//! Network capability for the Splash VM.
//!
//! The whole point of this module is that the *DSL* fetches its own data — the
//! weather card's numbers and its satellite map are pulled from the internet at
//! render time, nothing baked into the HAP. The actual transport is
//! OpenHarmony's native HTTP stack (`net_http.h`), reached through `shim.cpp`;
//! here we only wrap the flat C ABI in something safe.

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

extern "C" {
    /// Blocking GET. Returns the HTTP status code (negative on transport error).
    /// When `out_buf` is non-null the body is malloc'd into `*out_buf`/`*out_len`
    /// and must be released with `splash_free`.
    fn splash_http_get(url: *const c_char, out_buf: *mut *mut c_char, out_len: *mut c_int) -> c_int;
    fn splash_free(p: *mut c_char);
}

/// Blocking GET returning `(status, body)`. `status` is the HTTP code, or a
/// negative sentinel on a transport/timeout error; `body` is the raw response.
pub fn http_get(url: &str) -> (i32, Option<Vec<u8>>) {
    let Ok(c) = CString::new(url) else {
        return (-1, None);
    };
    let mut buf: *mut c_char = std::ptr::null_mut();
    let mut len: c_int = 0;
    let code = unsafe { splash_http_get(c.as_ptr(), &mut buf, &mut len) };
    let body = if !buf.is_null() {
        let out = if len > 0 {
            let slice = unsafe { std::slice::from_raw_parts(buf as *const u8, len as usize) };
            Some(slice.to_vec())
        } else {
            None
        };
        unsafe { splash_free(buf) };
        out
    } else {
        None
    };
    (code, body)
}

/// Convenience: GET and return the body as a UTF-8 string (lossy), if any.
pub fn http_get_string(url: &str) -> (i32, Option<String>) {
    let (code, body) = http_get(url);
    (code, body.map(|b| String::from_utf8_lossy(&b).into_owned()))
}

// ---- JSON data: fetch once, extract many -----------------------------------
// The card pulls ~30 fields out of two JSON responses. Fetching per-field would
// be absurd, so responses are cached by URL for the life of the process and the
// DSL just names a dotted path into the parsed tree.

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static JSON_CACHE: RefCell<HashMap<String, serde_json::Value>> = RefCell::new(HashMap::new());
}

/// Parsed JSON for `url`, fetched on first use and cached thereafter.
fn cached_json(url: &str) -> Option<serde_json::Value> {
    if let Some(v) = JSON_CACHE.with(|c| c.borrow().get(url).cloned()) {
        return Some(v);
    }
    let (code, body) = http_get(url);
    if code != 200 {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&body?).ok()?;
    JSON_CACHE.with(|c| c.borrow_mut().insert(url.to_string(), v.clone()));
    Some(v)
}

/// Walk a dotted path ("current.temperature_2m", "daily.time.0") into a value.
/// A numeric segment indexes an array; `idx >= 0` appends one more index (used
/// for the per-day forecast rows so the DSL needn't build path strings).
fn walk<'a>(v: &'a serde_json::Value, path: &str, idx: i32) -> Option<&'a serde_json::Value> {
    let mut cur = v;
    for seg in path.split('.') {
        if seg.is_empty() {
            continue;
        }
        cur = match seg.parse::<usize>() {
            Ok(i) => cur.get(i)?,
            Err(_) => cur.get(seg)?,
        };
    }
    if idx >= 0 {
        cur = cur.get(idx as usize)?;
    }
    Some(cur)
}

/// A number at `path` (+ optional array index) in the JSON at `url`.
pub fn fetch_num(url: &str, path: &str, idx: i32) -> Option<f64> {
    walk(&cached_json(url)?, path, idx)?.as_f64()
}

/// A number at `path`, rounded to an integer and rendered with `suffix`
/// ("22" + "°" = "22°"). Returns "--" when the field is missing, so the card
/// still lays out if a value is absent.
pub fn fetch_fmt(url: &str, path: &str, idx: i32, suffix: &str) -> String {
    match fetch_num(url, path, idx) {
        Some(n) => format!("{}{}", n.round() as i64, suffix),
        None => format!("--{}", suffix),
    }
}

/// The weekday name ("Tue") for an ISO date ("2026-07-27") at `path`.
pub fn fetch_weekday(url: &str, path: &str, idx: i32) -> Option<String> {
    let v = cached_json(url)?;
    let s = walk(&v, path, idx)?.as_str()?;
    let y: i32 = s.get(0..4)?.parse().ok()?;
    let m: i32 = s.get(5..7)?.parse().ok()?;
    let d: i32 = s.get(8..10)?.parse().ok()?;
    // Sakamoto's algorithm: 0 = Sunday.
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    let dow = (yy + yy / 4 - yy / 100 + yy / 400 + t[(m - 1) as usize] + d) % 7;
    let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    Some(names[dow as usize].to_string())
}
