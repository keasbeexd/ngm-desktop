//! What the app remembers: the game folder (a small settings file in the
//! app's own folder) and this computer's key (the system's secure store:
//! Windows Credential Manager, macOS Keychain, Linux Secret Service).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::time::Duration;

const SERVICE: &str = "app.ngmpeon.desktop";
const ACCOUNT: &str = "device-key";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// WoW: Forever's `_forever_` folder
    pub wow: Option<PathBuf>,
    /// fingerprints of the update codes already sent (the newest few)
    #[serde(default)]
    pub sent: Vec<u64>,
    /// per guild: the events this computer has seen (for "new event" notifications)
    #[serde(default)]
    pub seen_events: std::collections::HashMap<String, Vec<String>>,
    /// keep NGM Addon up to date (the player's choice; off unless ticked)
    #[serde(default)]
    pub auto_addon: bool,
    /// keep NGM Desktop itself up to date (on unless the player unticks it)
    #[serde(default = "yes")]
    pub auto_app: bool,
    /// start with the computer, into the tray (on unless the player unticks it)
    #[serde(default = "yes")]
    pub start_at_login: bool,
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            wow: None,
            sent: vec![],
            seen_events: Default::default(),
            auto_addon: false,
            auto_app: true,
            start_at_login: true,
        }
    }
}

pub fn load(dir: &Path) -> Settings {
    fs::read_to_string(dir.join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, s: &Settings) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("Couldn't save settings: {e}"))?;
    let text = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    fs::write(dir.join("settings.json"), text).map_err(|e| format!("Couldn't save settings: {e}"))
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| format!("Couldn't open the system's secure store: {e}"))
}

/// How long to wait for the secure store. A locked one (Linux: the login
/// keyring) can wait on an unlock prompt for as long as it likes.
const KEY_WAIT: Duration = Duration::from_secs(10);

/// The key once read: the store is asked once per run, not at every redraw.
static KEY: Mutex<Option<String>> = Mutex::new(None);
/// A read that didn't answer in time: the next call waits on it again
/// instead of asking the store once more.
static WAITING: Mutex<Option<mpsc::Receiver<Option<String>>>> = Mutex::new(None);

pub const KEY_STUCK: &str = "The system's secure store didn't answer, so NGM Desktop can't reach NGM Peon. If it's locked (on Linux, the login keyring), unlock it, then press Send now.";

/// This computer's key. None when there's none, or when the secure store
/// didn't answer in time (then `key_waiting` is true).
pub fn key() -> Option<String> {
    if let Some(k) = KEY.lock().unwrap().clone() {
        return Some(k);
    }
    let mut waiting = WAITING.lock().unwrap();
    let rx = waiting.take().unwrap_or_else(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(entry().ok().and_then(|e| e.get_password().ok()));
        });
        rx
    });
    match rx.recv_timeout(KEY_WAIT) {
        Ok(k) => {
            KEY.lock().unwrap().clone_from(&k);
            k
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            *waiting = Some(rx);
            None
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => None,
    }
}

/// The secure store hasn't answered yet (it may be locked).
pub fn key_waiting() -> bool {
    WAITING.lock().unwrap().is_some()
}

pub fn set_key(key: &str) -> Result<(), String> {
    entry()?
        .set_password(key)
        .map_err(|e| format!("Couldn't save to the system's secure store: {e}"))?;
    *WAITING.lock().unwrap() = None;
    *KEY.lock().unwrap() = Some(key.to_string());
    Ok(())
}

pub fn forget_key() {
    *KEY.lock().unwrap() = None;
    if let Ok(e) = entry() {
        let _ = e.delete_credential();
    }
}
