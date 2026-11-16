//! Linux, when run as an AppImage: an entry in the app launcher (with its
//! icon), pointing at the AppImage, so it opens like any other app, without
//! a terminal. Kept current at every start (the AppImage may have moved or
//! been updated). Removed only by the player (it's a plain .desktop file).

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
    }
}
