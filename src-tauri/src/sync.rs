//! Step 2, by itself: watch NGM's saved data, and when the game rewrites it
//! (logout or /reload), send the update codes NGM left in it to NGM Peon.
//!
//! Idle otherwise: a file watcher, no timers. Waits until the game has
//! finished writing (a few quiet seconds), then sends each code once.

use crate::{companion, peon, saved, store};
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const QUIET: Duration = Duration::from_secs(3);
const REMEMBER: usize = 50; // codes already sent, so a restart doesn't send them again

/// The last send, for the window.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Last {
    /// Unix seconds
    pub at: u64,
    pub ok: bool,
    pub lines: Vec<String>,
}

#[derive(Default)]
pub struct Sync {
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
    pub last: Mutex<Option<Last>>,
    /// the app's addon was just added: the game must restart to load it
    pub helper_new: Mutex<bool>,
    running: Mutex<bool>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn fingerprint(code: &str) -> u64 {
    let mut h = DefaultHasher::new();
    code.hash(&mut h);
    h.finish()
}

fn describe(s: &peon::Sent) -> String {
    let what = match s.stored {
        0 => "nothing new".to_string(),
        1 => "1 change taken".to_string(),
        n => format!("{n} changes taken"),
    };
    let mut line = format!("<{}>: {what}", s.guild);
    if s.ranks {
        line.push_str(", ranks updated");
    }
    if s.role == "member" {
        line.push_str(" (your own characters)");
    }
    if s.refused > 0 {
        line.push_str(&format!(", {} not taken", s.refused));
    }
    line
}

/// Watches this game folder (replacing any earlier watch) and adds the
/// app's addon. Sends whatever's waiting right away.
pub fn start(handle: &AppHandle, forever: &Path) -> Result<(), String> {
    let app = handle.state::<crate::App>();
    *app.sync.watcher.lock().unwrap() = None;
    if companion::ensure(forever)? {
        *app.sync.helper_new.lock().unwrap() = true;
    }
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(e) = res {
            if e.paths
                .iter()
                .any(|p| p.file_name().map(|n| n == crate::wow::SAVED).unwrap_or(false))
            {
                let _ = tx.send(());
            }
        }
    })
    .map_err(|e| format!("Couldn't watch the game folder: {e}"))?;
    watcher
        .watch(&forever.join("WTF").join("Account"), RecursiveMode::Recursive)
        .map_err(|e| format!("Couldn't watch the game folder: {e}"))?;
    *app.sync.watcher.lock().unwrap() = Some(watcher);

    // one thread per watch: wait for a change, then for the game to finish
    // writing, then send (it ends when the watch is replaced or dropped)
    let h = handle.clone();
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            while rx.recv_timeout(QUIET).is_ok() {}
            let h2 = h.clone();
            tauri::async_runtime::spawn(async move { run(&h2).await });
        }
    });
    let h = handle.clone();
    tauri::async_runtime::spawn(async move { run(&h).await });
    Ok(())
}

/// Sends every code not sent yet. Nothing without a key or a game folder.
/// True when something was sent (or tried).
pub async fn run(handle: &AppHandle) -> bool {
    let app = handle.state::<crate::App>();
    {
        let mut running = app.sync.running.lock().unwrap();
        if *running {
            return true; // already sending
        }
        *running = true;
    }
    let result = send(&app).await;
    *app.sync.running.lock().unwrap() = false;
    let sent = result.is_some();
    if let Some(last) = result {
        *app.sync.last.lock().unwrap() = Some(last);
        let _ = handle.emit("synced", ());
    }
    sent
}

async fn send(app: &crate::App) -> Option<Last> {
    let key = store::key()?;
    let forever: PathBuf = app.settings.lock().unwrap().wow.clone()?;
    let mut lines = Vec::new();
    let mut ok = true;
    let mut any = false;
    for file in saved::files(&forever) {
        for code in saved::codes(&file) {
            let f = fingerprint(&code);
            if app.settings.lock().unwrap().sent.contains(&f) {
                continue;
            }
            any = true;
            match peon::update(&key, &code).await {
                Ok(s) => {
                    lines.push(describe(&s));
                    let mut st = app.settings.lock().unwrap();
                    st.sent.push(f);
                    let extra = st.sent.len().saturating_sub(REMEMBER);
                    st.sent.drain(..extra);
                    let _ = store::save(&app.dir, &st);
                }
                Err(e) => {
                    ok = false;
                    lines.push(e.to_string());
                    if matches!(e, peon::Error::NotConnected(_)) {
                        store::forget_key();
                        break;
                    }
                }
            }
        }
    }
    any.then(|| Last { at: now(), ok, lines })
}
