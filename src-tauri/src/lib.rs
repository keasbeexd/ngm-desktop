//! NGM Desktop. Lives in the tray; its window finds WoW: Forever and
//! connects this computer to NGM Peon. When the game saves NGM's data, it
//! sends NGM Peon the update code NGM left (sync.rs). Bringing NGM Peon's
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
    /// NGM's own updates: what's installed, the newest, and how the last try went
    addon: Mutex<AddonState>,
}

#[derive(Default, Clone, Serialize)]
struct AddonState {
    latest: Option<String>,
    message: Option<String>,
}

/// What the window shows about NGM itself.
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
    /// WoW accounts with NGM's data in that folder
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
        match peon::me(&key).await {
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
                sync::run(&handle).await; // anything already waiting
            }
            Ok(view(&app, err).await)
        }
        Err(e) => Ok(view(&app, Some(e.to_string())).await),
    }
}

/// Sends whatever NGM left that wasn't sent yet.
#[tauri::command]
async fn sync_now(handle: AppHandle, app: State<'_, App>) -> Result<View, String> {
    let sent = sync::run(&handle).await;
    let mut v = view(&app, None).await;
    if !sent {
        v.note = Some(
            "Nothing new to send. WoW saves NGM's update when you log out or /reload, so do one of those in game."
                .into(),
        );
    }
    Ok(v)
}

/// Checks GitHub for a newer NGM and, when `now` or the box is ticked,
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
    if news::game_running() {
        return set(
            Some(rel.version.clone()),
            Some(format!("NGM {} is out. It installs once WoW is closed.", rel.version)),
        );
    }
    let result = match addon::download(&rel).await {
        Ok(bytes) => addon::install(&forever, &bytes),
        Err(e) => Err(e),
    };
    match result {
        Ok(()) => {
            set(
                Some(rel.version.clone()),
                Some(format!("Updated NGM to {}.", rel.version)),
            );
            let _ = companion::ensure(&forever); // its game version follows NGM's
        }
        Err(e) => set(Some(rel.version), Some(e)),
    }
    let _ = handle.emit("synced", ());
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
            });
            if let Some(p) = wow {
                if let Err(e) = sync::start(app.handle(), &p) {
                    eprintln!("{e}");
                }
            }
            news::start(app.handle()); // every few minutes, only while WoW runs

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
            open_link
        ])
        .run(tauri::generate_context!())
        .expect("NGM Desktop couldn't start");
}
