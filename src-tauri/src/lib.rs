//! NGM Desktop. Lives in the tray; its window finds WoW: Forever and
//! connects this computer to NGM Peon. When the game saves NGM Addon's data, it
//! sends NGM Peon the update code NGM Addon left (sync.rs). Bringing NGM Peon's
//! data back comes in milestone 4.

mod addon;
mod companion;
mod news;
mod peon;
mod saved;
mod store;
mod sync;
mod wow;

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, Theme, WindowEvent};
use tauri_plugin_dialog::DialogExt;

struct App {
    settings: Mutex<store::Settings>,
    dir: PathBuf,
    sync: sync::Sync,
    news: news::News,
    /// NGM Addon's own updates: what's installed, the newest, and how the last try went
    addon: Mutex<AddonState>,
    /// NGM Desktop's own updates: how the last check went
    app_update: Mutex<Option<String>>,
    /// the guilds this account is linked in, and when NGM Peon said so
    guilds: Mutex<Option<(std::time::Instant, Vec<peon::Guild>)>>,
}

/// How long the guild list is trusted before asking NGM Peon again (the
/// window redraws often; NGM Peon allows 120 requests an hour per computer).
const GUILDS_FOR: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// The guilds this account is linked in: remembered for a while, asked again
/// when `fresh` (after connecting) or when old.
async fn my_guilds(app: &App, key: &str, fresh: bool) -> Result<Vec<peon::Guild>, peon::Error> {
    if !fresh {
        if let Some((at, g)) = app.guilds.lock().unwrap().as_ref() {
            if at.elapsed() < GUILDS_FOR {
                return Ok(g.clone());
            }
        }
    }
    let g = peon::me(key).await?;
    *app.guilds.lock().unwrap() = Some((std::time::Instant::now(), g.clone()));
    Ok(g)
}

#[derive(Default, Clone, Serialize)]
struct AddonState {
    latest: Option<String>,
    message: Option<String>,
}

/// What the window shows about NGM Addon.
#[derive(Serialize)]
struct AddonView {
    installed: Option<String>,
    latest: Option<String>,
    auto: bool,
    message: Option<String>,
    /// a newer release than the one installed
    behind: bool,
}

/// Everything the window shows.
#[derive(Serialize)]
struct View {
    wow: Option<String>,
    /// WoW accounts with NGM Addon's data in that folder
    accounts: Vec<String>,
    connected: bool,
    guilds: Vec<peon::Guild>,
    error: Option<String>,
    /// the last time the app sent NGM Peon an update
    last: Option<sync::Last>,
    /// the app's addon was just added: restart WoW once
    helper_new: bool,
    /// a passing remark (not an error)
    note: Option<String>,
    addon: AddonView,
    /// NGM Desktop itself: its version, whether it updates itself, the last word
    app: AppView,
}

#[derive(Serialize)]
struct AppView {
    version: &'static str,
    auto: bool,
    message: Option<String>,
}

async fn view(app: &App, error: Option<String>) -> View {
    let wow = app.settings.lock().unwrap().wow.clone();
    let accounts = wow.as_deref().map(wow::accounts).unwrap_or_default();
    let mut v = View {
        wow: wow.map(|p| p.display().to_string()),
        accounts,
        connected: false,
        guilds: vec![],
        error,
        last: app.sync.last.lock().unwrap().clone(),
        helper_new: *app.sync.helper_new.lock().unwrap(),
        note: None,
        app: AppView {
            version: env!("CARGO_PKG_VERSION"),
            auto: app.settings.lock().unwrap().auto_app,
            message: app.app_update.lock().unwrap().clone(),
        },
        addon: {
            let a = app.addon.lock().unwrap().clone();
            let s = app.settings.lock().unwrap();
            let installed = s.wow.as_deref().and_then(addon::installed);
            let behind = matches!((&a.latest, &installed), (Some(l), Some(i)) if addon::newer(l, i));
            AddonView {
                installed,
                behind,
                latest: a.latest,
                auto: s.auto_addon,
                message: a.message,
            }
        },
    };
    if let Some(key) = store::key() {
        match my_guilds(app, &key, false).await {
            Ok(g) => {
                v.connected = true;
                v.guilds = g;
            }
            Err(peon::Error::NotConnected(m)) => {
                store::forget_key(); // removed on the dashboard: start over
                v.error = v.error.or(Some(m));
            }
            Err(e) => {
                v.connected = true; // still connected, just not reachable now
                v.error = v.error.or(Some(e.to_string()));
            }
        }
    }
    v
}

// Saves the game folder and starts watching it.
fn set_wow(handle: &AppHandle, app: &App, path: PathBuf) -> Result<(), String> {
    {
        let mut s = app.settings.lock().unwrap();
        s.wow = Some(path.clone());
        store::save(&app.dir, &s)?;
    }
    sync::start(handle, &path)
}

#[tauri::command]
async fn state(app: State<'_, App>) -> Result<View, String> {
    Ok(view(&app, None).await)
}

/// Looks in the usual places.
#[tauri::command]
async fn find_wow(handle: AppHandle, app: State<'_, App>) -> Result<View, String> {
    match wow::find() {
        Some(p) => {
            let err = set_wow(&handle, &app, p).err();
            Ok(view(&app, err).await)
        }
        None => Ok(view(
            &app,
            Some("Couldn't find WoW: Forever on this computer. Choose its folder.".into()),
        )
        .await),
    }
}

/// The player picks the folder.
#[tauri::command]
async fn choose_wow(handle: AppHandle, app: State<'_, App>) -> Result<View, String> {
    let picked = handle
        .dialog()
        .file()
        .set_title("Choose your World of Warcraft folder")
        .blocking_pick_folder();
    let Some(picked) = picked.and_then(|f| f.into_path().ok()) else {
        return Ok(view(&app, None).await);
    };
    match wow::normalize(&picked) {
        Some(p) => {
            let err = set_wow(&handle, &app, p).err();
            Ok(view(&app, err).await)
        }
        None => Ok(view(
            &app,
            Some(
                "That isn't WoW: Forever's folder. Choose the World of Warcraft folder (the one with _forever_ in it)."
                    .into(),
            ),
        )
        .await),
    }
}

/// A one-time code from the dashboard.
#[tauri::command]
async fn connect(handle: AppHandle, app: State<'_, App>, code: String) -> Result<View, String> {
    let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).take(16).collect();
    if code.len() != 8 {
        return Ok(view(&app, Some("The code has 8 letters and numbers, like ABCD-EFGH.".into())).await);
    }
    match peon::link(&code, &device_name()).await {
        Ok(key) => {
            let err = store::set_key(&key).err();
            if err.is_none() {
                let _ = my_guilds(&app, &key, true).await; // a new account: ask afresh
                sync::run(&handle).await; // anything already waiting
            }
            Ok(view(&app, err).await)
        }
        Err(e) => Ok(view(&app, Some(e.to_string())).await),
    }
}

/// Sends whatever NGM Addon left that wasn't sent yet.
#[tauri::command]
async fn sync_now(handle: AppHandle, app: State<'_, App>) -> Result<View, String> {
    let sent = sync::run(&handle).await;
    let mut v = view(&app, None).await;
    if !sent {
        v.note = Some(
            "Nothing new to send. WoW saves NGM Addon's update when you log out or /reload, so do one of those in game."
                .into(),
        );
    }
    Ok(v)
}

/// Checks GitHub for a newer NGM Addon and, when `now` or the box is ticked,
/// installs it (only while WoW is closed).
async fn update_addon(handle: &AppHandle, now: bool) {
    let app = handle.state::<App>();
    let (forever, auto) = {
        let s = app.settings.lock().unwrap();
        (s.wow.clone(), s.auto_addon)
    };
    let Some(forever) = forever else { return };
    let set = |latest: Option<String>, message: Option<String>| {
        let mut a = app.addon.lock().unwrap();
        if latest.is_some() {
            a.latest = latest;
        }
        a.message = message;
    };
    let rel = match addon::latest().await {
        Ok(r) => r,
        Err(e) => return set(None, Some(e)),
    };
    let have = addon::installed(&forever);
    let behind = have.as_deref().map(|h| addon::newer(&rel.version, h)).unwrap_or(true);
    if !behind || !(now || auto) {
        return set(Some(rel.version), None);
    }
    // WoW may be running: addons load their files at login and /reload, so
    // the new version takes over then (a changed file list: restart once)
    let running = news::game_running();
    let result = match addon::download(&rel).await {
        Ok(bytes) => addon::install(&forever, &bytes),
        Err(e) => Err(e),
    };
    match result {
        Ok(files_changed) => {
            let next = match (running, files_changed) {
                (false, _) => "",
                (true, false) => " Type /reload in WoW to load it.",
                (true, true) => " Restart WoW once to load it (its file list changed).",
            };
            set(
                Some(rel.version.clone()),
                Some(format!("Updated NGM Addon to {}.{next}", rel.version)),
            );
            let _ = companion::ensure(&forever); // its game version follows NGM Addon's
        }
        Err(e) => set(Some(rel.version), Some(e)),
    }
    let _ = handle.emit("synced", ());
}

/// NGM Desktop's own update: only files signed with our update key (the
/// updater checks the signature against the public key in tauri.conf.json),
/// from the newest published release on GitHub. Installs and restarts when
/// `now` or the box is ticked. Only in installed builds, never `npm run dev`.
async fn update_self(handle: &AppHandle, now: bool) {
    use tauri_plugin_updater::UpdaterExt;
    let app = handle.state::<App>();
    let say = |m: Option<String>| *app.app_update.lock().unwrap() = m;
    if cfg!(debug_assertions) {
        return say(Some("Updates itself only once installed from a release.".into()));
    }
    let auto = app.settings.lock().unwrap().auto_app;
    let checked = match handle.updater() {
        Ok(u) => u.check().await,
        Err(e) => return say(Some(format!("Couldn't check for updates: {e}"))),
    };
    match checked {
        Ok(None) => say(None),
        Ok(Some(update)) if now || auto => {
            say(Some(format!("Installing NGM Desktop {}...", update.version)));
            let _ = handle.emit("synced", ());
            match update.download_and_install(|_, _| {}, || {}).await {
                Ok(()) => handle.restart(),
                Err(e) => say(Some(format!("Couldn't install NGM Desktop {}: {e}", update.version))),
            }
        }
        Ok(Some(update)) => say(Some(format!("NGM Desktop {} is out.", update.version))),
        Err(_) => say(Some("Couldn't reach GitHub to check for NGM Desktop updates.".into())),
    }
    let _ = handle.emit("synced", ());
}

#[tauri::command]
async fn set_auto_app(handle: AppHandle, app: State<'_, App>, on: bool) -> Result<View, String> {
    {
        let mut s = app.settings.lock().unwrap();
        s.auto_app = on;
        store::save(&app.dir, &s)?;
    }
    if on {
        update_self(&handle, false).await;
    }
    Ok(view(&app, None).await)
}

#[tauri::command]
async fn check_app(handle: AppHandle, app: State<'_, App>) -> Result<View, String> {
    update_self(&handle, true).await;
    Ok(view(&app, None).await)
}

#[tauri::command]
async fn set_auto_addon(handle: AppHandle, app: State<'_, App>, on: bool) -> Result<View, String> {
    {
        let mut s = app.settings.lock().unwrap();
        s.auto_addon = on;
        store::save(&app.dir, &s)?;
    }
    if on {
        update_addon(&handle, false).await;
    }
    Ok(view(&app, None).await)
}

/// "Check now" / "Update now".
#[tauri::command]
async fn check_addon(handle: AppHandle, app: State<'_, App>) -> Result<View, String> {
    update_addon(&handle, true).await;
    Ok(view(&app, None).await)
}

/// The links in the window: fixed addresses only, opened in the browser.
#[tauri::command]
fn open_link(handle: AppHandle, which: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let url = match which.as_str() {
        "site" => "https://ngmpeon.app",
        "dashboard" => "https://ngmpeon.app/dashboard",
        "bug" => "https://github.com/keasbeexd/nootropic-guild-manager/issues/new?template=bug.yml",
        "feature" => "https://github.com/keasbeexd/nootropic-guild-manager/issues/new?template=feature.yml",
        _ => return Err("Unknown link".into()),
    };
    handle.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Forgets this computer's key here (remove it on the dashboard too).
#[tauri::command]
async fn disconnect(app: State<'_, App>) -> Result<View, String> {
    store::forget_key();
    *app.guilds.lock().unwrap() = None;
    Ok(view(&app, None).await)
}

/// How this computer shows on the dashboard: its system and network name.
fn device_name() -> String {
    let os = if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "Mac"
    } else {
        "Linux"
    };
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .map(|h| {
            h.trim()
                .chars()
                .filter(|c| !c.is_control())
                .take(24)
                .collect::<String>()
        })
        .filter(|h| !h.is_empty());
    match host {
        Some(h) => format!("{os} ({h})"),
        None => format!("{os} computer"),
    }
}

/// The tray icon: the banner's silhouette, white on dark taskbars and black
/// on light ones. macOS recolours it by itself (a "template" image).
fn tray_icon(theme: Theme) -> Image<'static> {
    let bytes: &'static [u8] = if cfg!(target_os = "macos") || theme == Theme::Light {
        include_bytes!("../icons/tray-black.png")
    } else {
        include_bytes!("../icons/tray-white.png")
    };
    Image::from_bytes(bytes).expect("the tray icon")
}

fn system_theme(handle: &AppHandle) -> Theme {
    handle
        .get_webview_window("main")
        .and_then(|w| w.theme().ok())
        .unwrap_or(Theme::Dark)
}

fn show(handle: &AppHandle) {
    if let Some(w) = handle.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        // a second launch just shows the window that's already running
        .plugin(tauri_plugin_single_instance::init(|handle, _args, _cwd| show(handle)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let settings = store::load(&dir);
            let wow = settings.wow.clone();
            app.manage(App {
                settings: Mutex::new(settings),
                dir,
                sync: sync::Sync::default(),
                news: news::News::default(),
                addon: Mutex::new(AddonState::default()),
                guilds: Mutex::new(None),
                app_update: Mutex::new(None),
            });
            if let Some(p) = wow {
                if let Err(e) = sync::start(app.handle(), &p) {
                    eprintln!("{e}");
                }
            }
            news::start(app.handle()); // every 10 minutes, only while WoW runs
                                       // NGM Desktop's own update: a minute after start, then every 6 hours
            let h = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(60));
                loop {
                    tauri::async_runtime::block_on(update_self(&h, false));
                    std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
                }
            });

            let open = MenuItem::with_id(app, "open", "Open NGM Desktop", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::with_id("tray")
                .icon(tray_icon(system_theme(app.handle())))
                .icon_as_template(true)
                .tooltip("NGM Desktop")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|handle, event| match event.id.as_ref() {
                    "open" => show(handle),
                    "quit" => handle.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        // closing the window keeps the app in the tray (Quit is in its menu)
        .on_window_event(|window, event| {
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let _ = window.hide();
                    api.prevent_close();
                }
                // light or dark switched: the tray icon follows
                WindowEvent::ThemeChanged(theme) => {
                    if let Some(tray) = window.app_handle().tray_by_id("tray") {
                        let _ = tray.set_icon(Some(tray_icon(*theme)));
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            state,
            find_wow,
            choose_wow,
            connect,
            sync_now,
            disconnect,
            set_auto_addon,
            check_addon,
            set_auto_app,
            check_app,
            open_link
        ])
        .run(tauri::generate_context!())
        .expect("NGM Desktop couldn't start");
}
