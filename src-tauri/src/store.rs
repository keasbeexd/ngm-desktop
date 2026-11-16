//! What the app remembers: the game folder (a small settings file in the
//! app's own folder) and this computer's key (the system's secure store:
//! Windows Credential Manager, macOS Keychain, Linux Secret Service).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SERVICE: &str = "app.ngmpeon.desktop";
const ACCOUNT: &str = "device-key";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// WoW: Forever's `_forever_` folder
    pub wow: Option<PathBuf>,
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

pub fn key() -> Option<String> {
    entry().ok()?.get_password().ok()
}

pub fn set_key(key: &str) -> Result<(), String> {
    entry()?
        .set_password(key)
        .map_err(|e| format!("Couldn't save to the system's secure store: {e}"))
}

pub fn forget_key() {
    if let Ok(e) = entry() {
        let _ = e.delete_credential();
    }
}
