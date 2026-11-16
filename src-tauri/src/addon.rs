//! Keeping NGM itself up to date (only when the player ticks the box): the
//! newest release from GitHub (keasbeexd/nootropic-guild-manager), checked
//! against GitHub's SHA-256 for that file, installed only while WoW is
//! closed, all at once (the old folder comes back if anything fails).

use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

const RELEASES: &str = "https://api.github.com/repos/keasbeexd/nootropic-guild-manager/releases/latest";
const DOWNLOADS: &str = "https://github.com/keasbeexd/nootropic-guild-manager/releases/download/";
const FOLDER: &str = "NootropicGuildManager";
const MAX_ZIP: u64 = 20 * 1024 * 1024;
const MAX_UNPACKED: u64 = 80 * 1024 * 1024;
const MAX_FILES: usize = 3000;

#[derive(Debug, Clone)]
pub struct Release {
    pub version: String,
    url: String,
    sha256: String,
    size: u64,
}

pub fn folder(forever: &Path) -> PathBuf {
    forever.join("Interface").join("AddOns").join(FOLDER)
}

/// The version installed (its TOC), if NGM is there.
pub fn installed(forever: &Path) -> Option<String> {
    let toc = fs::read_to_string(folder(forever).join(format!("{FOLDER}.toc"))).ok()?;
    toc.lines()
        .find_map(|l| l.strip_prefix("## Version:").map(|v| v.trim().to_string()))
        .filter(|v| !v.is_empty() && v.len() <= 30)
}

/// "3.0.8" -> [3,0,8]; a test build ("3.0.8-test5") counts as just before
/// its release.
fn key(v: &str) -> (Vec<u64>, bool) {
    let v = v.trim_start_matches('v');
    let (base, test) = match v.split_once('-') {
        Some((b, _)) => (b, true),
        None => (v, false),
    };
    (base.split('.').map(|n| n.parse().unwrap_or(0)).collect(), !test)
}

/// Is `latest` newer than `have`?
pub fn newer(latest: &str, have: &str) -> bool {
    key(latest) > key(have)
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("NGM-Desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(60))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5)) // GitHub sends downloads to its file host
        .build()
        .map_err(|e| e.to_string())
}

/// The newest release on GitHub.
pub async fn latest() -> Result<Release, String> {
    #[derive(serde::Deserialize)]
    struct Asset {
        name: String,
        size: u64,
        digest: Option<String>,
        browser_download_url: String,
    }
    #[derive(serde::Deserialize)]
    struct Rel {
        tag_name: String,
        assets: Vec<Asset>,
    }
    let r = client()?
        .get(RELEASES)
        .header("accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Couldn't reach GitHub to check for NGM updates.".to_string())?;
    if !r.status().is_success() {
        return Err(format!("GitHub answered {} when checking for NGM updates.", r.status()));
    }
    let rel: Rel = r
        .json()
        .await
        .map_err(|_| "GitHub's answer didn't make sense.".to_string())?;
    let version = rel.tag_name.trim_start_matches('v').to_string();
    let a = rel
        .assets
        .into_iter()
        .find(|a| a.name.ends_with("-forever.zip"))
        .ok_or("That NGM release has no WoW: Forever download.")?;
    let sha256 = a
        .digest
        .and_then(|d| d.strip_prefix("sha256:").map(str::to_string))
        .filter(|d| d.len() == 64 && d.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or("That NGM release has no checksum to check it against.")?;
    if !a.browser_download_url.starts_with(DOWNLOADS)
        || a.size > MAX_ZIP
        || !version.chars().all(|c| c.is_ascii_digit() || c == '.')
    {
        return Err("That NGM release doesn't look right; not installing it.".into());
    }
    Ok(Release {
        version,
        url: a.browser_download_url,
        sha256: sha256.to_lowercase(),
        size: a.size,
    })
}

/// Downloads a release and checks it is exactly the file GitHub lists.
pub async fn download(r: &Release) -> Result<Vec<u8>, String> {
    let resp = client()?
        .get(&r.url)
        .send()
        .await
        .map_err(|_| "Couldn't download the NGM update.".to_string())?;
    if !resp.status().is_success() {
        return Err(format!("GitHub answered {} for the NGM download.", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|_| "The NGM download was cut short.".to_string())?;
    if bytes.len() as u64 != r.size || bytes.len() as u64 > MAX_ZIP {
        return Err("The NGM download isn't the size GitHub lists; not installing it.".into());
    }
    let sum: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    if sum != r.sha256 {
        return Err("The NGM download doesn't match GitHub's checksum; not installing it.".into());
    }
    Ok(bytes.to_vec())
}

/// Unpacks only `NootropicGuildManager/...` into `to`, refusing anything
/// that would land outside it, and within size and file-count limits.
fn unpack(zip_bytes: &[u8], to: &Path) -> Result<(), String> {
    let bad = |m: &str| format!("The NGM download {m}; not installing it.");
    let mut zip = zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(|_| bad("isn't a proper zip"))?;
    if zip.len() > MAX_FILES {
        return Err(bad("has too many files"));
    }
    let mut total = 0u64;
    let mut any_toc = false;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|_| bad("is damaged"))?;
        let Some(path) = f.enclosed_name() else {
            return Err(bad("has a file in an odd place"));
        };
        let Ok(rest) = path.strip_prefix(FOLDER) else { continue }; // only NGM's folder
        total += f.size();
        if total > MAX_UNPACKED {
            return Err(bad("unpacks too big"));
        }
        let out = to.join(rest);
        if f.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if rest == Path::new(&format!("{FOLDER}.toc")) {
            any_toc = true;
        }
        if let Some(p) = out.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut data = Vec::new();
        f.by_ref()
            .take(MAX_UNPACKED)
            .read_to_end(&mut data)
            .map_err(|_| bad("is damaged"))?;
        fs::write(&out, data).map_err(|e| e.to_string())?;
    }
    if !any_toc {
        return Err(bad("isn't NGM"));
    }
    Ok(())
}

/// Puts a checked download in place of the installed NGM, all at once.
pub fn install(forever: &Path, zip_bytes: &[u8]) -> Result<(), String> {
    let addons = forever.join("Interface").join("AddOns");
    let staging = addons.join(".NootropicGuildManager.new");
    let old = addons.join(".NootropicGuildManager.old");
    let _ = fs::remove_dir_all(&staging);
    let _ = fs::remove_dir_all(&old);
    fs::create_dir_all(&staging).map_err(|e| format!("Couldn't update NGM: {e}"))?;
    if let Err(e) = unpack(zip_bytes, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    let live = folder(forever);
    let had = live.exists();
    if had {
        fs::rename(&live, &old).map_err(|e| format!("Couldn't update NGM (is WoW still open?): {e}"))?;
    }
    if let Err(e) = fs::rename(&staging, &live) {
        if had {
            let _ = fs::rename(&old, &live); // put it back
        }
        return Err(format!("Couldn't update NGM: {e}"));
    }
    let _ = fs::remove_dir_all(&old);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            for (name, body) in files {
                z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
                z.write_all(body.as_bytes()).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn compares_versions_with_test_builds_just_before_their_release() {
        assert!(newer("3.0.8", "3.0.7"));
        assert!(newer("3.0.8", "3.0.8-test5"));
        assert!(!newer("3.0.7", "3.0.8-test5"));
        assert!(!newer("3.0.8", "3.0.8"));
        assert!(newer("3.1.0", "3.0.12"));
        assert!(newer("v3.0.10", "3.0.9"));
    }

    #[test]
    fn installs_only_ngms_folder_and_refuses_tricks() {
        let root = std::env::temp_dir().join(format!("ngm-desktop-addon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let addons = root.join("Interface/AddOns");
        fs::create_dir_all(addons.join(FOLDER)).unwrap();
        fs::write(addons.join(FOLDER).join(format!("{FOLDER}.toc")), "## Version: 3.0.7\n").unwrap();
        fs::write(addons.join(FOLDER).join("old.lua"), "gone").unwrap();
        let good = zip_of(&[
            ("NootropicGuildManager/NootropicGuildManager.toc", "## Version: 3.0.8\n"),
            ("NootropicGuildManager/Core/Core.lua", "-- core"),
            ("OtherAddon/evil.lua", "x"),
        ]);
        install(&root, &good).unwrap();
        assert_eq!(installed(&root).as_deref(), Some("3.0.8"));
        assert!(addons.join(FOLDER).join("Core/Core.lua").is_file());
        assert!(!addons.join(FOLDER).join("old.lua").exists(), "a clean copy");
        assert!(!addons.join("OtherAddon").exists(), "nothing outside NGM's folder");
        // a path escaping the folder, or a zip without NGM: refused, NGM untouched
        let escape = zip_of(&[
            ("NootropicGuildManager/../../../evil.lua", "x"),
            ("NootropicGuildManager/NootropicGuildManager.toc", "## Version: 9\n"),
        ]);
        assert!(install(&root, &escape).is_err());
        assert!(install(&root, &zip_of(&[("Something/readme.txt", "x")])).is_err());
        assert!(install(&root, b"not a zip").is_err());
        assert_eq!(installed(&root).as_deref(), Some("3.0.8"));
        assert!(!root.join("evil.lua").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
