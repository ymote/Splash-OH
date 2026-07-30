//! The Met's public collection API, which is where Wonderous gets an
//! artifact's details.
//!
//! `artifact_api_service.dart` fetches
//! `collectionapi.metmuseum.org/public/collection/v1/objects/{id}` and reads
//! the same nine fields out of it. The list of ids and the thumbnails ship with
//! the app; the record behind an id does not, and the detail screen has five
//! fields on it that exist only in the response.
//!
//! Fetching happens on a worker. `net::http_get` panics if it is called from a
//! render thread, and it is right to: a blocking GET inside `build()` is the
//! most expensive mistake available here.

use std::sync::Mutex;

const BASE: &str = "https://collectionapi.metmuseum.org/public/collection/v1/objects";

/// The fields `_InfoColumn` puts on screen.
#[derive(Clone, Default)]
pub struct Record {
    pub culture: String,
    pub title: String,
    pub date: String,
    pub period: String,
    pub country: String,
    pub medium: String,
    pub dimensions: String,
    pub classification: String,
}

impl Record {
    /// The rows the app lists, in its order, minus the ones the Met left blank
    /// — it skips empty values rather than printing an empty row.
    pub fn rows(&self) -> Vec<(&'static str, &str)> {
        [
            ("Date", self.date.as_str()),
            ("Period", self.period.as_str()),
            ("Geography", self.country.as_str()),
            ("Medium", self.medium.as_str()),
            ("Dimensions", self.dimensions.as_str()),
            ("Classification", self.classification.as_str()),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .collect()
    }
}

static CACHE: Mutex<Vec<(String, Record)>> = Mutex::new(Vec::new());
/// Ids with a worker already running, so a rebuild mid-flight does not start a
/// second fetch of the same object.
static INFLIGHT: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Set when a record lands, so the screen showing it can be drawn again.
static DIRTY: Mutex<bool> = Mutex::new(false);

/// True once, if a record arrived since the last call.
pub fn take_dirty() -> bool {
    DIRTY
        .lock()
        .map(|mut d| std::mem::replace(&mut *d, false))
        .unwrap_or(false)
}

/// The record for `id`, if it has arrived.
pub fn record(id: &str) -> Option<Record> {
    CACHE
        .lock()
        .ok()?
        .iter()
        .find(|(k, _)| k == id)
        .map(|(_, v)| v.clone())
}

/// Start fetching `id` unless it is already here or already on its way.
pub fn prefetch(id: &str) {
    if record(id).is_some() {
        return;
    }
    match INFLIGHT.lock() {
        Ok(mut f) if !f.iter().any(|k| k == id) => f.push(id.to_string()),
        _ => return,
    }
    let id = id.to_string();
    std::thread::spawn(move || {
        let got = fetch(&id);
        if let Some(rec) = got {
            if let Ok(mut c) = CACHE.lock() {
                c.retain(|(k, _)| *k != id);
                c.push((id.clone(), rec));
            }
            if let Ok(mut d) = DIRTY.lock() {
                *d = true;
            }
            crate::log(&format!("wonders/met: {id} ready"));
        } else {
            crate::log(&format!("wonders/met: {id} unavailable"));
        }
        if let Ok(mut f) = INFLIGHT.lock() {
            f.retain(|k| *k != id);
        }
    });
}

fn fetch(id: &str) -> Option<Record> {
    let (code, body) = crate::net::http_get_string(&format!("{BASE}/{id}"));
    if code != 200 {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(&body?).ok()?;
    let s = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string()
    };
    Some(Record {
        culture: s("culture"),
        title: s("title"),
        date: s("objectDate"),
        period: s("period"),
        country: s("country"),
        medium: s("medium"),
        // The Met calls it `dimensions`; the app's parser asks for `dimension`
        // and so always gets an empty string. Asking for the real key is what
        // the screen wants, and the app's own layout leaves room for it.
        dimensions: s("dimensions"),
        classification: s("classification"),
    })
}
