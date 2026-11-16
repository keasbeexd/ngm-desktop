//! The app's own small addon, `Interface/AddOns/NootropicGuildManager_Peon/`:
//! the only place it writes in the game folder. Its being there tells NGM
//! that the app is installed, so NGM leaves its update code at logout for
//! the app to send. (Milestone 4 adds NGM Peon's data to it.)

use std::fs;
use std::path::{Path, PathBuf};

pub const NAME: &str = "NootropicGuildManager_Peon";
const FALLBACK_INTERFACE: &str = "16001"; // WoW: Forever, 2026

pub fn dir(forever: &Path) -> PathBuf {
    forever.join("Interface").join("AddOns").join(NAME)
}

/// The game version NGM itself says it's for (its TOC), so the game never
/// calls the app's addon out of date when NGM isn't.
fn interface(forever: &Path) -> String {
    let toc = forever
        .join("Interface")
        .join("AddOns")
        .join("NootropicGuildManager")
        .join("NootropicGuildManager.toc");
    fs::read_to_string(toc)
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("## Interface:").map(|v| v.trim().to_string()))
        })
        .filter(|v| !v.is_empty() && v.len() <= 40 && v.chars().all(|c| c.is_ascii_digit() || c == ','))
        .unwrap_or_else(|| FALLBACK_INTERFACE.to_string())
}

fn toc(forever: &Path) -> String {
    format!(
        "## Interface: {}\n## Title: NGM Peon |cff9d9d9d(desktop app)|r\n## Notes: Written by NGM Desktop: tells NGM the app is here (and, soon, brings NGM Peon's news). Don't edit; the app rewrites it.\n## Author: keasbeexd\n## Version: {}\n## Dependencies: NootropicGuildManager\n\nData.lua\n",
        interface(forever),
        env!("CARGO_PKG_VERSION")
    )
}

fn data() -> String {
    format!(
        "-- Written by NGM Desktop ({}). Don't edit: the app rewrites it.\nNGMPeonDesktop = {{ app = \"{}\" }}\n",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION")
    )
}

/// Writes a file only when it differs, through a temporary file, so the game
/// never sees half of one.
fn put(path: &Path, text: &str) -> std::io::Result<bool> {
    if fs::read_to_string(path).map(|t| t == text).unwrap_or(false) {
        return Ok(false);
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)?;
    Ok(true)
}

/// Makes sure the addon is there and current. True when it was just made
/// (the game loads new addons only when it starts).
pub fn ensure(forever: &Path) -> Result<bool, String> {
    let d = dir(forever);
    let new = !d.join(format!("{NAME}.toc")).is_file();
    fs::create_dir_all(&d).map_err(|e| format!("Couldn't add the app's addon: {e}"))?;
    put(&d.join(format!("{NAME}.toc")), &toc(forever)).map_err(|e| format!("Couldn't write the app's addon: {e}"))?;
    put(&d.join("Data.lua"), &data()).map_err(|e| format!("Couldn't write the app's addon: {e}"))?;
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_its_addon_with_ngms_game_version_and_only_when_needed() {
        let root = std::env::temp_dir().join(format!("ngm-desktop-companion-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let ngm = root.join("Interface/AddOns/NootropicGuildManager");
        fs::create_dir_all(&ngm).unwrap();
        fs::write(
            ngm.join("NootropicGuildManager.toc"),
            "## Interface: 16002\n## Title: NGM\n",
        )
        .unwrap();
        assert!(ensure(&root).unwrap(), "new");
        assert!(!ensure(&root).unwrap(), "already there");
        let toc = fs::read_to_string(dir(&root).join(format!("{NAME}.toc"))).unwrap();
        assert!(toc.starts_with("## Interface: 16002\n"));
        assert!(toc.contains("## Dependencies: NootropicGuildManager\n"));
        assert!(fs::read_to_string(dir(&root).join("Data.lua"))
            .unwrap()
            .contains("NGMPeonDesktop = {"));
        let _ = fs::remove_dir_all(&root);
    }
}
