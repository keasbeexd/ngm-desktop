//! Linux, when run as an AppImage: an entry in the app launcher (with its
//! icon), pointing at the AppImage, so it opens like any other app, without
//! a terminal. Kept current at every start (the AppImage may have moved or
//! been updated). Also the login entry for "Start with my computer".

use std::fs;
use std::path::{Path, PathBuf};

const ID: &str = "app.ngmpeon.desktop";

fn put(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if fs::read(path).map(|b| b == bytes).unwrap_or(false) {
        return Ok(());
    }
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

/// The launcher entry for this AppImage (Exec quoted for spaces).
pub fn entry(appimage: &Path) -> String {
    let exec = appimage
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!(
        "[Desktop Entry]\nType=Application\nName=NGM Desktop\nComment=Keeps NGM Peon and the NGM Addon in sync\nExec=\"{exec}\"\nIcon={ID}\nTerminal=false\nCategories=Game;Utility;\nStartupWMClass=ngm-desktop\n"
    )
}

/// The program to start: the AppImage, or the installed program. None in a
/// development build (it would point at a build folder).
fn program() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        return None;
    }
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
}

/// The login entry ("Start with my computer"): starts hidden, in the tray.
pub fn login_entry(program: &Path) -> String {
    let exec = program.display().to_string().replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "[Desktop Entry]\nType=Application\nName=NGM Desktop\nComment=Keeps NGM Peon and the NGM Addon in sync\nExec=\"{exec}\" --hidden\nIcon={ID}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    )
}

/// Turns starting with the computer on or off. Err with why it couldn't.
pub fn set_login(on: bool) -> Result<(), String> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Err("No home folder.".into());
    };
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or(home.join(".config"));
    let file = config.join(format!("autostart/{ID}.desktop"));
    if !on {
        let _ = fs::remove_file(&file);
        return Ok(());
    }
    let Some(program) = program() else {
        return Err("Starts with the computer only once installed from a release.".into());
    };
    put(&file, login_entry(&program).as_bytes()).map_err(|e| format!("Couldn't add the login entry: {e}"))
}

/// Adds or updates the launcher entry when running as an AppImage.
pub fn ensure() {
    let Some(appimage) = std::env::var_os("APPIMAGE").map(PathBuf::from) else {
        return;
    };
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or(home.join(".local/share"));
    let icon = data.join(format!("icons/hicolor/128x128/apps/{ID}.png"));
    let desktop = data.join(format!("applications/{ID}.desktop"));
    let _ = put(&icon, include_bytes!("../icons/128x128.png"));
    let _ = put(&desktop, entry(&appimage).as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_points_at_the_appimage_without_a_terminal() {
        let e = entry(Path::new("/home/me/Apps/NGM Desktop.AppImage"));
        assert!(e.contains("Exec=\"/home/me/Apps/NGM Desktop.AppImage\"\n"));
        assert!(e.contains("Terminal=false\n") && e.contains("Icon=app.ngmpeon.desktop\n"));
        assert!(entry(Path::new("/a\"b")).contains("Exec=\"/a\\\"b\""));
        let l = login_entry(Path::new("/home/me/NGM Desktop.AppImage"));
        assert!(l.contains("Exec=\"/home/me/NGM Desktop.AppImage\" --hidden\n") && l.contains("Terminal=false\n"));
    }
}
