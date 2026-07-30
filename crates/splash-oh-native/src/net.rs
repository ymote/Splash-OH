//! Network capability for the Splash VM.
//!
//! The whole point of this module is that the *DSL* fetches its own data — the
//! weather card's numbers and its satellite map are pulled from the internet at
//! render time, nothing baked into the HAP. The actual transport is
//! OpenHarmony's native HTTP stack (`net_http.h`), reached through `shim.cpp`;
//! here we only wrap the flat C ABI in something safe.

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicU64, Ordering};

/// The thread that must never block: whichever one renders.
///
/// `http_get` is synchronous. Called from a `build()` it stalls the UI, and if
/// that build is running on the ArkUI event thread it also writes any web slot
/// it declares into the wrong thread's storage, so the surface is torn down
/// again. Both failures are silent -- the weather card rendered a white
/// rectangle for three debugging rounds before the cause was found.
///
/// So the render threads register themselves, and a blocking fetch from one is
/// a panic rather than a mystery.
static UI_THREADS: AtomicU64 = AtomicU64::new(0);

fn thread_bit() -> u64 {
    // Cheap per-thread identity: the low bits of the thread id, as a bitmask.
    // Collisions only cost a false positive on a thread that should not be
    // fetching anyway, which is the safe direction to be wrong in.
    let id = format!("{:?}", std::thread::current().id());
    let h = id
        .bytes()
        .fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
    1u64 << (h % 64)
}

/// Mark the calling thread as one that renders. Called from every entry point
/// that can reach `build()`.
pub fn mark_ui_thread() {
    UI_THREADS.fetch_or(thread_bit(), Ordering::Relaxed);
}

fn on_ui_thread() -> bool {
    UI_THREADS.load(Ordering::Relaxed) & thread_bit() != 0
}

extern "C" {
    /// Blocking GET. Returns the HTTP status code (negative on transport error).
    /// When `out_buf` is non-null the body is malloc'd into `*out_buf`/`*out_len`
    /// and must be released with `splash_free`.
    fn splash_http_get(url: *const c_char, out_buf: *mut *mut c_char, out_len: *mut c_int)
        -> c_int;
    fn splash_free(p: *mut c_char);
}

/// Blocking GET returning `(status, body)`. `status` is the HTTP code, or a
/// negative sentinel on a transport/timeout error; `body` is the raw response.
pub fn http_get(url: &str) -> (i32, Option<Vec<u8>>) {
    if on_ui_thread() {
        // Loud on purpose. A silent stall here is the single most expensive
        // failure mode this codebase has produced; better a crash that names
        // itself than another white rectangle.
        let msg = format!(
            "net::http_get called on a render thread ({url}). \
             Blocking HTTP must not happen inside build(): fetch on a worker \
             and have build() read a cache -- see apps::weather_web::prefetch."
        );
        crate::log(&msg);
        if cfg!(debug_assertions) {
            panic!("{msg}");
        }
    }
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

// ---------------------------------------------------------------------------
// Capability parity with the makepad host.
//
// The 2026-07-30 portability test lowered ONE semantic plan to both makepad Splash
// DSL and this backend. It worked — but the plan's SunMoon section could not
// render here at all, because the porting cost turned out not to be the widgets:
// makepad exposes thirty-odd `sys.*` helpers and this backend injected five
// (fetch_num, fetch_fmt, fetch_weekday, invoke, sget). Nothing yielded a moon
// phase, a daylight fraction, a geocoded coordinate or a forecast extent, so the
// card rendered an explicit "unavailable on this backend" notice.
//
// These close that gap. Each mirrors a `sys.*` helper in
// octos-one/aichat/widgets/src/splash.rs — same semantics, same reasoning — so a
// plan lowered to either backend resolves the same values.

/// Seconds since the Unix epoch.
fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Mean synodic month — new moon to new moon — in seconds.
const SYNODIC_SECS: f64 = 29.530_588_853 * 86_400.0;
/// A known new moon: 2000-01-06 18:14 UTC.
const NEW_MOON_EPOCH: f64 = 947_182_440.0;

/// Position in the synodic cycle, 0..1: 0 new, 0.25 first quarter, 0.5 full.
///
/// The MEAN cycle, not a full lunar theory — the true phase wanders by up to about
/// half a day. Invisible in a rendered disc or a rounded percentage, and it keeps
/// an ephemeris out of the backend.
fn moon_phase_fraction() -> f64 {
    let f = ((now_unix_secs() as f64 - NEW_MOON_EPOCH) % SYNODIC_SECS) / SYNODIC_SECS;
    if f < 0.0 {
        f + 1.0
    } else {
        f
    }
}

/// `moonphase(field)` -> the current 月相. No network, so it never shows a
/// placeholder. `field`: "name" | "name_zh" | "illumination" | "phase".
pub fn moonphase(field: &str) -> String {
    let f = moon_phase_fraction();
    match field.trim() {
        "name" | "name_zh" | "name_cn" => {
            let zh = field.trim() != "name";
            // The eight principal phases. The four exact ones name a narrow window
            // around the instant; the rest is crescent or gibbous.
            let (en, cn) = if f < 0.0335 || f >= 0.9665 {
                ("New Moon", "新月")
            } else if f < 0.2165 {
                ("Waxing Crescent", "蛾眉月")
            } else if f < 0.2835 {
                ("First Quarter", "上弦月")
            } else if f < 0.4665 {
                ("Waxing Gibbous", "盈凸月")
            } else if f < 0.5335 {
                ("Full Moon", "满月")
            } else if f < 0.7165 {
                ("Waning Gibbous", "亏凸月")
            } else if f < 0.7835 {
                ("Last Quarter", "下弦月")
            } else {
                ("Waning Crescent", "残月")
            };
            (if zh { cn } else { en }).to_string()
        }
        // Illuminated fraction is (1 - cos(2*pi*phase)) / 2 — 0 at new, 1 at full,
        // correctly non-linear between.
        "illumination" | "illum" => {
            format!(
                "{}",
                ((1.0 - (std::f64::consts::TAU * f).cos()) * 50.0).round() as i64
            )
        }
        _ => format!("{f:.2}"),
    }
}

/// Numeric form, for a shader uniform or any arithmetic.
pub fn moonnum(field: &str) -> f64 {
    let f = moon_phase_fraction();
    match field.trim() {
        "illumination" | "illum" => (1.0 - (std::f64::consts::TAU * f).cos()) * 50.0,
        _ => f,
    }
}

/// "HH:MM" or an ISO datetime -> minutes since local midnight.
fn hhmm_minutes(s: &str) -> Option<f64> {
    let t = s.trim();
    let time = if t.len() >= 16 && t.as_bytes().get(10) == Some(&b'T') {
        &t[11..16]
    } else {
        t
    };
    let (h, m) = time.split_once(':')?;
    Some(h.trim().parse::<f64>().ok()? * 60.0 + m.trim().parse::<f64>().ok()?)
}

/// `daylight(url)` -> fraction of daylight elapsed: 0 at sunrise, 1 at sunset.
/// Negative before sunrise and >1 after sunset, which a caller reads as night.
///
/// "Now" comes from the DEVICE CLOCK shifted by the response's own
/// `utc_offset_seconds`, NOT from a timestamp in the response — that response is
/// cached, so its own idea of "now" is frozen at whenever the card first fetched.
/// Sunrise, sunset and the offset are all stable for the day; only the instant
/// must be live.
///
/// `url` must be an open-meteo forecast including `daily=sunrise,sunset` and
/// `timezone=auto`.
pub fn daylight(url: &str) -> f64 {
    let inner = || -> Option<f64> {
        let v = cached_json(url)?;
        let rise = hhmm_minutes(walk(&v, "daily.sunrise", 0)?.as_str()?)?;
        let set = hhmm_minutes(walk(&v, "daily.sunset", 0)?.as_str()?)?;
        let offset = walk(&v, "utc_offset_seconds", -1)?.as_f64()?;
        let local = (now_unix_secs() as f64 + offset).rem_euclid(86_400.0) / 60.0;
        let span = set - rise;
        if span <= 0.0 {
            return None;
        } // polar day or night
        Some((local - rise) / span)
    };
    inner().unwrap_or(0.5)
}

/// `weekextent(url, path, want_max)` -> the min or max of a 7-element daily array.
///
/// Exists because the CARD cannot know it: every temperature is a live fetch, so a
/// generating model asked for the week's range guesses at numbers it has never
/// seen — and guesses badly, which clamps a gradient to one end of its ramp.
pub fn week_extent(url: &str, path: &str, want_max: bool) -> Option<f64> {
    let v = cached_json(url)?;
    let mut acc: Option<f64> = None;
    for i in 0..7 {
        let Some(n) = walk(&v, path, i).and_then(|x| x.as_f64()) else {
            continue;
        };
        acc = Some(match acc {
            None => n,
            Some(a) if want_max => a.max(n),
            Some(a) => a.min(n),
        });
    }
    acc
}

/// `geocode(name, field)` -> a fact about a place NAME. Never let a card carry a
/// coordinate: one recalled by a model is an invented number exactly like a
/// recalled temperature, plausible for a famous city and fabricated elsewhere.
///
/// `language` is not cosmetic — open-meteo searches its index PER LANGUAGE, so
/// "上海" with `language=en` returns nothing while `language=zh` returns Shanghai.
/// It therefore follows the script of the query.
pub fn geocode(name: &str, field: &str) -> Option<String> {
    let q = name.trim();
    let cjk = q.chars().any(|c| {
        matches!(c,
        '\u{3040}'..='\u{30FF}' | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}')
    });
    let lang = if cjk { "zh" } else { "en" };
    let enc: String = q
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={enc}&count=1&language={lang}&format=json"
    );
    let v = cached_json(&url)?;
    let path = match field.trim() {
        "lat" => "results.0.latitude",
        "lon" => "results.0.longitude",
        "name" => "results.0.name",
        "country" => "results.0.country",
        "timezone" => "results.0.timezone",
        other => {
            return walk(&v, &format!("results.0.{other}"), -1).map(|x| {
                x.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| x.to_string())
            })
        }
    };
    let x = walk(&v, path, -1)?;
    Some(
        x.as_str()
            .map(str::to_string)
            .unwrap_or_else(|| x.to_string()),
    )
}

/// Numeric geocode, for the coordinates that anchor a data URL.
pub fn geocodenum(name: &str, field: &str) -> f64 {
    geocode(name, field)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(-9999.0)
}
