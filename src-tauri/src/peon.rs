//! Talking to NGM Peon (ngmpeon.app/api/app, see ngm-hub's src/app/api.ts).
//! HTTPS only, to one address, with this computer's key.

use serde::{Deserialize, Serialize};
use std::time::Duration;

const BASE: &str = "https://ngmpeon.app/api/app";

/// Where to send requests: always ngmpeon.app, except in a development
/// build, which may point at a local NGM Peon (NGM_PEON_URL).
fn base() -> String {
    #[cfg(debug_assertions)]
    if let Ok(url) = std::env::var("NGM_PEON_URL") {
        return format!("{}/api/app", url.trim_end_matches('/'));
    }
    BASE.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Guild {
    pub region: String,
    pub name: String,
    /// "gm", "officer", "member", or none (no linked characters there now)
    pub role: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    /// The key isn't (or is no longer) valid: connect again.
    NotConnected(String),
    /// Something NGM Peon said, or the network: show it.
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotConnected(m) | Error::Other(m) => f.write_str(m),
        }
    }
}

fn client() -> Result<reqwest::Client, Error> {
    reqwest::Client::builder()
        .user_agent(concat!("NGM-Desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .https_only(!cfg!(debug_assertions))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| Error::Other(format!("Couldn't start: {e}")))
}

#[derive(Deserialize)]
struct Problem {
    error: String,
}

async fn fail(r: reqwest::Response) -> Error {
    let status = r.status();
    let msg = r
        .json::<Problem>()
        .await
        .map(|p| p.error)
        .unwrap_or_else(|_| format!("NGM Peon answered {status}."));
    if status == reqwest::StatusCode::UNAUTHORIZED {
        Error::NotConnected(msg)
    } else {
        Error::Other(msg)
    }
}

fn offline(e: reqwest::Error) -> Error {
    Error::Other(if e.is_timeout() {
        "NGM Peon didn't answer in time. Check your internet connection.".into()
    } else {
        "Couldn't reach NGM Peon. Check your internet connection.".into()
    })
}

/// A one-time code from the dashboard for this computer's key.
pub async fn link(code: &str, name: &str) -> Result<String, Error> {
    #[derive(Deserialize)]
    struct Linked {
        key: String,
    }
    let r = client()?
        .post(format!("{}/link", base()))
        .json(&serde_json::json!({ "code": code, "name": name }))
        .send()
        .await
        .map_err(offline)?;
    if !r.status().is_success() {
        return Err(fail(r).await);
    }
    let k: Linked = r
        .json()
        .await
        .map_err(|_| Error::Other("NGM Peon's answer didn't make sense.".into()))?;
    if !k.key.starts_with("ngmd_") || k.key.len() > 100 {
        return Err(Error::Other("NGM Peon's answer didn't make sense.".into()));
    }
    Ok(k.key)
}

/// The guilds this account is linked in.
pub async fn me(key: &str) -> Result<Vec<Guild>, Error> {
    #[derive(Deserialize)]
    struct Me {
        guilds: Vec<Guild>,
    }
    let r = client()?
        .get(format!("{}/me", base()))
        .bearer_auth(key)
        .send()
        .await
        .map_err(offline)?;
    if !r.status().is_success() {
        return Err(fail(r).await);
    }
    let m: Me = r
        .json()
        .await
        .map_err(|_| Error::Other("NGM Peon's answer didn't make sense.".into()))?;
    Ok(m.guilds)
}

/// What NGM Peon took from an update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sent {
    pub guild: String,
    pub role: String,
    pub stored: u32,
    pub unchanged: u32,
    pub refused: u32,
    pub ranks: bool,
}

/// Step 2: an update code from NGM's saved data.
pub async fn update(key: &str, code: &str) -> Result<Sent, Error> {
    let r = client()?
        .post(format!("{}/update", base()))
        .bearer_auth(key)
        .json(&serde_json::json!({ "code": code }))
        .send()
        .await
        .map_err(offline)?;
    if !r.status().is_success() {
        return Err(fail(r).await);
    }
    r.json()
        .await
        .map_err(|_| Error::Other("NGM Peon's answer didn't make sense.".into()))
}
