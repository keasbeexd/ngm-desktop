//! NGM Desktop (milestone 2: the skeleton). Lives in the tray; its window
//! finds WoW: Forever and connects this computer to NGM Peon. Sending and
//! bringing back the addon's data comes in milestones 3 and 4.

mod peon;
mod store;
mod wow;

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_dialog::DialogExt;

struct App {
    settings: Mutex<store::Settings>,
    dir: PathBuf,
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

fn set_wow(app: &App, path: Option<PathBuf>) -> Result<(), String> {
    let mut s = app.settings.lock().unwrap();
    s.wow = path;
    store::save(&app.dir, &s)
}

#[tauri::command]
async fn state(app: State<'_, App>) -> Result<View, String> {
    Ok(view(&app, None).await)
}

/// Looks in the usual places.
#[tauri::command]
async fn find_wow(app: State<'_, App>) -> Result<View, String> {
    match wow::find() {
        Some(p) => {
            let err = set_wow(&app, Some(p)).err();
            Ok(view(&app, err).await)
        }
        None => Ok(view(&app, Some("Couldn't find WoW: Forever. Choose its folder.".into())).await),
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
            let err = set_wow(&app, Some(p)).err();
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
async fn connect(app: State<'_, App>, code: String) -> Result<View, String> {
    let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).take(16).collect();
    if code.len() != 8 {
        return Ok(view(&app, Some("The code has 8 letters and numbers, like ABCD-EFGH.".into())).await);
    }
    match peon::link(&code, &device_name()).await {
        Ok(key) => {
            let err = store::set_key(&key).err();
            Ok(view(&app, err).await)
        }
        Err(e) => Ok(view(&app, Some(e.to_string())).await),
    }
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
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            app.manage(App {
                settings: Mutex::new(store::load(&dir)),
                dir,
            });

            let open = MenuItem::with_id(app, "open", "Open NGM Desktop", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().cloned().expect("the app's icon"))
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
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            state, find_wow, choose_wow, connect, disconnect
        ])
        .run(tauri::generate_context!())
        .expect("NGM Desktop couldn't start");
}
