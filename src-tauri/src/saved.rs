//! Reading NGM Addon's saved data (`WTF/Account/<account>/SavedVariables/NootropicGuildManager.lua`).
//!
//! The file is never run: the app only looks for the update codes NGM Addon
//! leaves in it at logout, lines like
//!     ["desktopCode"] = "NGM2.<base64url>.<8 hex digits>",
//! and takes a code only if every character is one a code can have. NGM Addon
//! Peon checks the code's own checksum and everything in it again.

use std::fs;
use std::path::{Path, PathBuf};

const MARK: &str = "[\"desktopCode\"] = \"";
const MAX_FILE: u64 = 64 * 1024 * 1024;
const MAX_CODE: usize = 600_000; // NGM Peon takes up to 400,000

/// Every account's saved-data file in this `_forever_` folder.
pub fn files(forever: &Path) -> Vec<PathBuf> {
    crate::wow::accounts(forever)
        .into_iter()
        .map(|a| {
            forever
                .join("WTF")
                .join("Account")
                .join(a)
                .join("SavedVariables")
                .join(crate::wow::SAVED)
        })
        .collect()
}

/// The update codes in one saved-data file (one per guild the account's
/// characters are in, at most).
pub fn codes(file: &Path) -> Vec<String> {
    match fs::metadata(file) {
        Ok(m) if m.len() <= MAX_FILE => {}
        _ => return vec![],
    }
    let Ok(text) = fs::read_to_string(file) else {
        return vec![];
    };
    codes_in(&text)
}

fn codes_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(MARK) {
        rest = &rest[i + MARK.len()..];
        let Some(end) = rest.find('"') else { break };
        let code = &rest[..end];
        if valid(code) && !out.iter().any(|c| c == code) {
            out.push(code.to_string());
        }
        rest = &rest[end..];
    }
    out
}

fn valid(code: &str) -> bool {
    let parts: Vec<&str> = code.split('.').collect();
    code.len() <= MAX_CODE
        && parts.len() == 3
        && parts[0] == "NGM2"
        && !parts[1].is_empty()
        && parts[1]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && parts[2].len() == 8
        && parts[2].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_only_well_formed_codes() {
        let text = r#"
NootropicGuildManagerDB = {
["guilds"] = {
["Nootropic"] = {
["desktopCode"] = "NGM2.dXMKTm9vdHJvcGlj.13f06304",
},
["Other"] = {
["desktopCode"] = "NGM2.abc\\\"); os.execute(\"x.13f06304",
},
["Third"] = {
["desktopCode"] = "NGM1.dXM.13f06304",
["note"] = "[\"desktopCode\"] = \"NGM2.fake",
},
},
}
"#;
        assert_eq!(codes_in(text), vec!["NGM2.dXMKTm9vdHJvcGlj.13f06304".to_string()]);
        assert!(codes_in("").is_empty());
        assert!(codes_in(r#"["desktopCode"] = "NGM2.unterminated"#).is_empty());
    }
}
