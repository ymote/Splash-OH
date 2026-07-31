//! Persistent key-value storage for web surfaces, in the app sandbox.
//!
//! A page in a Splash web slot cannot use `localStorage` the way a browser page
//! can: the slot is re-created whenever the DSL rebuilds, and generated pages
//! arrive through `loadData` under a synthetic `baseUrl`, so whatever origin the
//! storage was scoped to is not reliably the same one next time. State that
//! survives has to live where the app lives.
//!
//! One JSON file under `files/`, rewritten whole. That is the right shape at
//! this size — a card storing a few dozen settings is not a database, and a
//! whole-file rewrite cannot leave a half-updated record behind the way an
//! append log can.

use crate::bridge::json_str;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Where the file lives. `el2/base/files` is the app's own directory — the
/// probe in `apps/files.rs` confirmed on device that this is what OHOS gives an
/// app, rather than the Android-shaped `haps/entry/files` that does not exist.
const STORE: &str = "/data/storage/el2/base/files/prefs.json";

/// Caps, because a page decides what goes in here.
const MAX_KEYS: usize = 500;
const MAX_VALUE: usize = 64 * 1024;

/// Loaded once and kept, so a read is not a file read. `None` until first use.
static CACHE: Mutex<Option<BTreeMap<String, String>>> = Mutex::new(None);

fn load() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(STORE).unwrap_or_default();
    serde_json::from_str::<BTreeMap<String, String>>(&text).unwrap_or_default()
}

fn with<T>(f: impl FnOnce(&mut BTreeMap<String, String>) -> T) -> T {
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = Some(load());
    }
    f(guard.as_mut().expect("just populated"))
}

/// Write the map out. Via a temp file and a rename, so a crash mid-write leaves
/// the previous contents rather than a truncated file that will not parse.
fn flush(map: &BTreeMap<String, String>) -> Result<(), String> {
    let body = serde_json::to_string(map).map_err(|e| e.to_string())?;
    let tmp = format!("{STORE}.tmp");
    std::fs::write(&tmp, body).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, STORE).map_err(|e| e.to_string())
}

/// One value, or JSON null when absent.
pub fn get(key: &str) -> String {
    with(|m| match m.get(key) {
        Some(v) => json_str(v),
        None => "null".into(),
    })
}

/// Every key. Useful to a page that wants to know what it stored last run.
pub fn keys() -> String {
    with(|m| {
        format!(
            "[{}]",
            m.keys().map(|k| json_str(k)).collect::<Vec<_>>().join(",")
        )
    })
}

pub fn set(key: &str, value: &str) -> Result<String, String> {
    if key.is_empty() {
        return Err("empty key".into());
    }
    if value.len() > MAX_VALUE {
        return Err(format!("value larger than {MAX_VALUE} bytes"));
    }
    with(|m| {
        if !m.contains_key(key) && m.len() >= MAX_KEYS {
            return Err(format!("at the {MAX_KEYS}-key limit"));
        }
        m.insert(key.to_string(), value.to_string());
        flush(m)?;
        Ok(format!("{{\"key\":{},\"stored\":true}}", json_str(key)))
    })
}

pub fn remove(key: &str) -> Result<String, String> {
    with(|m| {
        let existed = m.remove(key).is_some();
        flush(m)?;
        Ok(format!(
            "{{\"key\":{},\"removed\":{}}}",
            json_str(key),
            existed
        ))
    })
}
