//! Step 3, by itself: keep NGM Peon's news (the code the dashboard's Sync
//! tab shows for /ngm peon, per guild) in the app's addon, so any login or
//! /reload brings it in. Checks NGM Peon every 10 minutes while WoW runs
//! (a cheap "anything new?"), after each send, and when the app starts.
//! A small desktop notification says when there's a new event.

use crate::busy::Busy;
use crate::{companion, peon, store};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

const EVERY: Duration = Duration::from_secs(10 * 60);

/// A check older than this counts as hung (each request gives up after 20 s).
const STALE: Duration = Duration::from_secs(5 * 60);

#[derive(Default)]
pub struct News {
    /// per guild ("us/Nootropic"): NGM Peon's code and its ETag
    held: Mutex<HashMap<String, (String, String)>>,
    busy: Busy,
    pub last: Mutex<Option<Checked>>,
}

/// The last check, for the window.
#[derive(Debug, Clone, Serialize)]
pub struct Checked {
    /// Unix seconds
    pub at: u64,
    /// upcoming events in the codes kept for the game
    pub events: usize,
    /// what went wrong, if it did
    pub error: Option<String>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Is WoW running? (Only process names are looked at.)
pub fn game_running() -> bool {
    let Ok(mut sys) = sysinfo::System::new() else {
        return false;
    };
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.processes().values().any(|p| {
        let n = p.name().to_string_lossy().to_ascii_lowercase();
        n == "wow.exe" || n == "wow" || n == "world of warcraft"
    })
}

/// Checks every 10 minutes while WoW runs; nothing at all otherwise
/// (one sleeping thread).
pub fn start(handle: &AppHandle) {
    let h = handle.clone();
    std::thread::spawn(move || {
        let mut ticks: u32 = 5; // NGM Addon's own update: soon after start, then hourly (6 ticks)
        loop {
            std::thread::sleep(EVERY);
            ticks += 1;
            if game_running() {
                tauri::async_runtime::block_on(fetch(&h));
            } else if ticks >= 6 {
                ticks = 0;
                tauri::async_runtime::block_on(crate::update_addon(&h, false));
            }
        }
    });
}

/// Gets NGM Peon's newest codes for every guild this account is linked in
/// and puts them in the app's addon. Tells about new events.
pub async fn fetch(handle: &AppHandle) {
    let app = handle.state::<crate::App>();
    let Some(_held) = app.news.busy.take(STALE) else {
        return; // already checking
    };
    let checked = match fetch_in(handle, &app).await {
        Outcome::Skip => None,
        Outcome::Done(events) => Some(Checked {
            at: now(),
            events,
            error: None,
        }),
        Outcome::Failed(e) => Some(Checked {
            at: now(),
            events: 0,
            error: Some(e),
        }),
    };
    if let Some(c) = checked {
        *app.news.last.lock().unwrap() = Some(c);
        let _ = handle.emit("synced", ());
    }
}

/// How a check went.
enum Outcome {
    /// not set up yet (no game folder, or not connected): nothing to say
    Skip,
    /// the upcoming events now kept for the game
    Done(usize),
    Failed(String),
}

async fn fetch_in(handle: &AppHandle, app: &crate::App) -> Outcome {
    let Some(forever) = app.settings.lock().unwrap().wow.clone() else {
        return Outcome::Skip;
    };
    let Some(key) = store::key() else {
        return if store::key_waiting() {
            Outcome::Failed(store::KEY_STUCK.into())
        } else {
            Outcome::Skip
        };
    };
    let guilds = match crate::my_guilds(app, &key, false).await {
        Ok(g) => g,
        Err(e) => return Outcome::Failed(e.to_string()),
    };
    let mut changed = false;
    let mut problem = None;
    for g in guilds.iter().filter(|g| g.role.is_some()) {
        let id = format!("{}/{}", g.region, g.name);
        let etag = app.news.held.lock().unwrap().get(&id).map(|(_, e)| e.clone());
        match peon::data(&key, &g.region, &g.name, etag.as_deref()).await {
            Ok(Some((code, etag))) if companion::valid_code(&code) => {
                announce(handle, app, &id, &code);
                app.news.held.lock().unwrap().insert(id, (code, etag));
                changed = true;
            }
            Ok(Some(_)) => problem = Some(format!("<{}>: NGM Peon's code didn't look right.", g.name)),
            Ok(None) => {} // unchanged
            Err(e) => problem = Some(format!("<{}>: {e}", g.name)),
        }
    }
    let codes: Vec<String> = {
        let held = app.news.held.lock().unwrap();
        let mut ids: Vec<&String> = held.keys().collect();
        ids.sort();
        ids.into_iter().map(|k| held[k].0.clone()).collect()
    };
    if changed {
        if let Err(e) = companion::set_codes(&forever, &codes) {
            return Outcome::Failed(e);
        }
    }
    match problem {
        Some(p) => Outcome::Failed(p),
        None => Outcome::Done(codes.iter().map(|c| events_in(c).len()).sum()),
    }
}

/// A desktop notification for each event NGM Peon has that this computer
/// hasn't seen yet (none the first time a guild is checked).
fn announce(handle: &AppHandle, app: &crate::App, guild: &str, code: &str) {
    let events = events_in(code);
    let mut s = app.settings.lock().unwrap();
    let first = !s.seen_events.contains_key(guild);
    let seen = s.seen_events.entry(guild.to_string()).or_default();
    let new: Vec<&(String, String, String)> = events.iter().filter(|(id, _, _)| !seen.contains(id)).collect();
    if !first {
        for (_, kind, title) in &new {
            let what = match kind.as_str() {
                "raid" => "raid",
                "dungeon" => "dungeon",
                _ => "event",
            };
            let _ = handle
                .notification()
                .builder()
                .title(format!("New {what}: {title}"))
                .body("In WoW: Reload and Sync (or /reload) to see it in the NGM Addon's Upcoming Events.")
                .show();
        }
    }
    *seen = events.into_iter().map(|(id, _, _)| id).collect();
    let _ = store::save(&app.dir, &s);
}

/// The events in an NGMP1 code: (id, kind, title) from its `E` lines.
pub fn events_in(code: &str) -> Vec<(String, String, String)> {
    let Some(body) = code.split('.').nth(1) else {
        return vec![];
    };
    let Some(bytes) = base64url(body) else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&bytes);
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.first() == Some(&"E") && f.len() >= 6).then(|| {
                let title: String = f[5].chars().filter(|c| !c.is_control()).take(80).collect();
                (f[1].to_string(), f[2].to_string(), title)
            })
        })
        .collect()
}

fn base64url(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        } as u32)
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        acc = (acc << 6) | val(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_events_in_a_code() {
        // ngm-hub test/gameupdate.test.ts EVENT_VECTOR: one raid
        let code = "NGMP1.dXMKTm9vdHJvcGljCmhhcwl0b2Y3eGMKRQkxNglyYWlkCXRvaW8wMAk1MAlNb2x0ZW4gQ29yZQlCcmluZyBmaXJlIHJlc2lzdC4gT24gdGltZSEJcmFpZHMJTm9vdHJvcGljCWM.87cb2139";
        assert_eq!(
            events_in(code),
            vec![("16".to_string(), "raid".to_string(), "Molten Core".to_string())]
        );
        assert!(events_in("NGMP1.@@@.00000000").is_empty());
        assert!(events_in("nonsense").is_empty());
    }
}
