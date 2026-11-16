//! Finding WoW: Forever's folder (`World of Warcraft/_forever_`) and the
//! WoW accounts in it that have NGM's saved data.
//!
//! Only looks at folder names; never opens a file here.

use std::fs;
use std::path::{Path, PathBuf};

pub const FOREVER: &str = "_forever_";
/// NGM's saved data, per WoW account: WTF/Account/<account>/SavedVariables/<this>
pub const SAVED: &str = "NootropicGuildManager.lua";

/// The `_forever_` folder for a folder someone picked: `_forever_` itself,
/// the `World of Warcraft` folder above it, or the folder above that.
pub fn normalize(picked: &Path) -> Option<PathBuf> {
    let tries = [
        picked.to_path_buf(),
        picked.join(FOREVER),
        picked.join("World of Warcraft").join(FOREVER),
    ];
    tries.into_iter().find(|p| is_forever(p))
}

fn is_forever(p: &Path) -> bool {
    p.file_name().map(|n| n == FOREVER).unwrap_or(false) && (p.join("WTF").is_dir() || p.join("Interface").is_dir())
}

/// WoW accounts (folder names under WTF/Account) with NGM's saved data.
pub fn accounts(forever: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(dir) = fs::read_dir(forever.join("WTF").join("Account")) else {
        return out;
    };
    for e in dir.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name == "SavedVariables" {
            continue; // the folder for account-wide data of every account
        }
        if e.path().join("SavedVariables").join(SAVED).is_file() {
            out.push(name);
        }
    }
    out.sort();
    out
}

/// Where WoW usually lives on this kind of computer.
fn candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        for drive in b'C'..=b'H' {
            let root = PathBuf::from(format!("{}:\\", drive as char));
            for sub in ["Program Files (x86)", "Program Files", "Games", ""] {
                out.push(root.join(sub).join("World of Warcraft"));
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        out.push(PathBuf::from("/Applications/World of Warcraft"));
        if let Some(home) = home() {
            out.push(home.join("Applications").join("World of Warcraft"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        // WoW runs under Wine or Proton: look inside the usual prefixes
        if let Some(home) = home() {
            let mut prefixes: Vec<PathBuf> = vec![home.join(".wine")];
            for parent in [
                home.join("Games"),                        // Lutris and others
                home.join(".local/share/bottles/bottles"), // Bottles
                home.join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
                home.join(".local/share/Steam/steamapps/compatdata"), // Steam (Proton)
                home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata"),
            ] {
                if let Ok(dir) = fs::read_dir(&parent) {
                    for e in dir.flatten() {
                        prefixes.push(e.path());
                        prefixes.push(e.path().join("pfx")); // Steam
                    }
                }
            }
            for p in prefixes {
                for sub in ["Program Files (x86)", "Program Files"] {
                    out.push(p.join("drive_c").join(sub).join("World of Warcraft"));
                }
            }
        }
    }
    out
}

#[cfg(not(target_os = "windows"))]
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// The first `_forever_` folder found, preferring one with NGM's data.
pub fn find() -> Option<PathBuf> {
    let found: Vec<PathBuf> = candidates().iter().filter_map(|c| normalize(c)).collect();
    found
        .iter()
        .find(|p| !accounts(p).is_empty())
        .or(found.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ngm-desktop-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn picks_the_forever_folder_from_any_level_and_lists_accounts_with_ngm() {
        let root = temp("wow");
        let forever = root.join("World of Warcraft").join(FOREVER);
        fs::create_dir_all(forever.join("WTF/Account/ACCOUNT1/SavedVariables")).unwrap();
        fs::create_dir_all(forever.join("WTF/Account/ACCOUNT2/SavedVariables")).unwrap();
        fs::create_dir_all(forever.join("WTF/Account/SavedVariables")).unwrap();
        fs::write(forever.join("WTF/Account/ACCOUNT1/SavedVariables").join(SAVED), "x").unwrap();
        fs::write(forever.join("WTF/Account/SavedVariables").join(SAVED), "x").unwrap();
        for picked in [root.clone(), root.join("World of Warcraft"), forever.clone()] {
            assert_eq!(normalize(&picked), Some(forever.clone()));
        }
        assert_eq!(normalize(&root.join("nothing")), None);
        assert_eq!(accounts(&forever), vec!["ACCOUNT1".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_just_named_forever_is_not_enough() {
        let root = temp("fake");
        let fake = root.join(FOREVER);
        fs::create_dir_all(&fake).unwrap();
        assert_eq!(normalize(&fake), None);
        let _ = fs::remove_dir_all(&root);
    }
}
