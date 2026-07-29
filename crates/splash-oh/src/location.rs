//! Location, from `liblocation_ndk`.
//!
//! The first capability here that needs a **user_grant** permission. Everything
//! before it was either free (`sensor.list`, `net.info`), settled by a line in
//! `module.json5` (`ACCELEROMETER`, `VIBRATE`), or flatly unavailable
//! (`CAPTURE_SCREEN`). This one is granted or refused by the person holding the
//! phone, at runtime, which makes it the first tool whose availability is not a
//! property of the build.
//!
//! That has a consequence worth stating: `location.get` can fail for four
//! genuinely different reasons, and collapsing them would make the failure
//! unactionable.
//!
//! | code | meaning | what fixes it |
//! |---|---|---|
//! | 201 | permission not granted | ask the user |
//! | 3301100 | location switch off | user turns it on in settings |
//! | 3301000 | service unavailable | nothing, retry later |
//! | 801 | not supported | nothing, wrong device |
//!
//! # One-shot on a subscription API
//!
//! Like the sensors, there is no getter. `OH_Location_StartLocating` registers a
//! callback the location service invokes on its own thread; a single fix means
//! start, wait, stop. Unlike the sensors this can legitimately take seconds — a
//! cold GNSS fix is not instant — so the default timeout is generous and the
//! caller can raise it.

use crate::bridge::json_str;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// All thirteen fields, because this is returned **by value**.
///
/// An earlier version declared the first eight and stopped. That is not a
/// missing-feature bug, it is memory corruption: on AArch64 a struct this large
/// comes back through the indirect result convention -- the caller passes a
/// hidden pointer to a buffer and the callee writes into it. Declaring 64 bytes
/// where the callee writes 104 means every call scribbles 40 bytes past the end
/// of the caller's buffer.
///
/// It never fired only because the location callback never ran: the device
/// cannot see enough satellites indoors, so GetBasicInfo was never reached. A
/// fix arriving would have corrupted the stack of whichever thread took it.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct BasicInfo {
    latitude: f64,
    longitude: f64,
    altitude: f64,
    accuracy: f64,
    speed: f64,
    direction: f64,
    time_for_fix: i64,
    time_since_boot: i64,
    altitude_accuracy: f64,
    speed_accuracy: f64,
    direction_accuracy: f64,
    uncertainty_of_time_since_boot: i64,
    /// `Location_SourceType`, a C enum and therefore int32. The struct carries
    /// four bytes of tail padding after it, which `repr(C)` reproduces.
    location_source_type: i32,
}

extern "C" {
    fn OH_Location_IsLocatingEnabled(enabled: *mut bool) -> i32;
    fn OH_Location_StartLocating(config: *const c_void) -> i32;
    fn OH_Location_StopLocating(config: *const c_void) -> i32;
    fn OH_Location_CreateRequestConfig() -> *mut c_void;
    fn OH_Location_DestroyRequestConfig(config: *mut c_void);
    fn OH_LocationRequestConfig_SetUseScene(config: *mut c_void, scene: i32);
    fn OH_LocationRequestConfig_SetPowerConsumptionScene(config: *mut c_void, scene: i32);
    fn OH_LocationRequestConfig_SetInterval(config: *mut c_void, interval: i32);
    fn OH_LocationRequestConfig_SetCallback(
        config: *mut c_void,
        cb: extern "C" fn(*mut c_void, *mut c_void),
        user: *mut c_void,
    );
    fn OH_LocationInfo_GetBasicInfo(info: *mut c_void) -> BasicInfo;
}

/// `Location_SourceType` from the header.
fn source_name(t: i32) -> &'static str {
    match t {
        1 => "gnss",
        2 => "network",
        3 => "indoor",
        4 => "rtk",
        _ => "unknown",
    }
}

fn code_message(rc: i32) -> String {
    match rc {
        201 => "permission not granted — call permission.request first".into(),
        401 => "invalid request parameters".into(),
        801 => "location is not supported on this device".into(),
        3301000 => "location service unavailable".into(),
        3301100 => "location is switched off in system settings".into(),
        _ => format!("location failed ({rc})"),
    }
}

/// Whether the system location switch is on. Distinct from whether this app
/// holds the permission — a page shown "no location" deserves to know which of
/// the two it is, since only one of them it can do anything about.
pub fn enabled() -> Result<String, String> {
    let mut on = false;
    let rc = unsafe { OH_Location_IsLocatingEnabled(&mut on) };
    if rc != 0 {
        return Err(code_message(rc));
    }
    Ok(format!("{{\"enabled\":{on}}}"))
}

static FIX: Mutex<Option<BasicInfo>> = Mutex::new(None);
static GOT: AtomicBool = AtomicBool::new(false);
/// One request in flight. Two overlapping subscriptions would race on FIX.
static LOCATING: Mutex<()> = Mutex::new(());

extern "C" fn on_location(info: *mut c_void, _user: *mut c_void) {
    if info.is_null() {
        return;
    }
    let basic = unsafe { OH_LocationInfo_GetBasicInfo(info) };
    if let Ok(mut f) = FIX.lock() {
        *f = Some(basic);
    }
    GOT.store(true, Ordering::SeqCst);
}

/// A single fix. JSON object, or an error naming which of the four reasons.
///
/// Blocks until the service reports a position or `timeout_ms` elapses, so it
/// must run on a worker.
pub fn get(timeout_ms: u64, scene: &str) -> Result<String, String> {
    let _one_at_a_time = LOCATING.lock().map_err(|_| "location lock poisoned")?;

    let config = unsafe { OH_Location_CreateRequestConfig() };
    if config.is_null() {
        return Err("could not create a location request".into());
    }

    let result = (|| {
        // The scene decides which provider answers, and that turned out to
        // matter more than expected.
        //
        // DAILY_LIFE_SERVICE looks like the obvious choice for a card that just
        // wants to know roughly where it is — but on device the request landed
        // on the *passive* provider ("PassiveAbility: enter RequestRecord" in
        // the log), which by design only forwards fixes some other app asked
        // for. With nothing else locating, nothing ever arrives. So the scene
        // is a parameter and the caller states what it is willing to spend.
        let use_scene = match scene {
            "navigation" => 0x0401,
            "sport" => 0x0402,
            "transport" => 0x0403,
            _ => 0x0404, // daily life service
        };
        unsafe {
            OH_LocationRequestConfig_SetUseScene(config, use_scene);
            OH_LocationRequestConfig_SetInterval(config, 1);
            OH_LocationRequestConfig_SetCallback(config, on_location, std::ptr::null_mut());
        }

        GOT.store(false, Ordering::SeqCst);
        if let Ok(mut f) = FIX.lock() {
            *f = None;
        }

        let rc = unsafe { OH_Location_StartLocating(config) };
        if rc != 0 {
            return Err(code_message(rc));
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        while !GOT.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        unsafe { OH_Location_StopLocating(config) };

        if !GOT.load(Ordering::SeqCst) {
            return Err(format!("no fix within {timeout_ms} ms"));
        }
        let b = FIX
            .lock()
            .ok()
            .and_then(|mut f| f.take())
            .ok_or("fix vanished")?;
        // The extra fields are reported now that they are actually read --
        // an accuracy figure alongside a position is the difference between
        // "somewhere near here" and a number a caller can reason about.
        Ok(format!(
            "{{\"latitude\":{:.6},\"longitude\":{:.6},\"altitude\":{:.1},\
             \"accuracy\":{:.1},\"altitudeAccuracy\":{:.1},\"speed\":{:.2},\
             \"speedAccuracy\":{:.2},\"direction\":{:.1},\"timestamp\":{},\
             \"source\":{}}}",
            b.latitude,
            b.longitude,
            b.altitude,
            b.accuracy,
            b.altitude_accuracy,
            b.speed,
            b.speed_accuracy,
            b.direction,
            b.time_for_fix,
            json_str(source_name(b.location_source_type))
        ))
    })();

    unsafe { OH_Location_DestroyRequestConfig(config) };
    result
}

/// Nearest city in `weather_web`'s list, so the weather card can start where
/// the phone is instead of always on Tokyo.
///
/// Straight-line on lat/lon without projecting. At the scale that matters here —
/// picking the closest of four cities on different continents — the error from
/// treating degrees as flat is nowhere near enough to change the answer, and a
/// great-circle formula would be precision this decision cannot use.
pub fn nearest_city(lat: f64, lon: f64) -> usize {
    let cities = crate::apps::weather_web::CITIES;
    let mut best = 0usize;
    let mut best_d = f64::MAX;
    for (i, (_, clat, clon)) in cities.iter().enumerate() {
        let dlat = clat - lat;
        // Longitude degrees narrow towards the poles; without this a city at
        // the same longitude difference would look equally far at any latitude.
        let dlon = (clon - lon) * lat.to_radians().cos();
        let d = dlat * dlat + dlon * dlon;
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

// ---- a fix the DSL can ask for during a render ------------------------------
//
// `get` blocks until the service answers, so a screen cannot call it while its
// tree is being built — that is the UI thread, and a location request takes
// seconds. This is the same shape as the sensor fix: the answer is cached, the
// refresh happens on a worker, and the caller gets whatever is known now.
//
// A screen therefore shows "locating" on first mount and the real position on a
// later one. That is honest about what a location request costs, and it is why
// compass_app re-mounts against a clock like the animation demos do.

static CACHE: Mutex<Option<String>> = Mutex::new(None);
static REFRESHING: AtomicBool = AtomicBool::new(false);
/// Set when a fix (or a failure) lands, so the screen showing it redraws.
static DIRTY: AtomicBool = AtomicBool::new(false);

/// True once, if a location result arrived since the last call.
pub fn take_dirty() -> bool {
    DIRTY.swap(false, Ordering::SeqCst)
}

/// "on" or "off", for a screen that shows the answer to a person.
///
/// `enabled()` returns the JSON the web bridge wants. A DSL screen has no JSON
/// parser, so it would print `{"enabled":true}` at a reader — which is what it
/// did.
pub fn enabled_word() -> String {
    match enabled() {
        Ok(j) if j.contains("true") => "on".into(),
        Ok(_) => "off".into(),
        Err(e) => e,
    }
}

/// The last known fix, kicking off a refresh when there is none in flight.
///
/// Never blocks. `cached()` is a line to show; `cached_state()` is "ok",
/// "pending" or "error" so a screen can branch without parsing it — the DSL has
/// no JSON and no substring test, so anything it needs to decide on has to
/// arrive as a value it can compare with `==`.
pub fn cached() -> String {
    if let Ok(c) = CACHE.lock() {
        if let Some(v) = c.as_ref() {
            return v.clone();
        }
    }
    kick();
    "locating…".to_string()
}

/// "ok" once a fix has landed, "error" if the request failed, else "pending".
pub fn cached_state() -> String {
    if let Ok(c) = CACHE.lock() {
        if let Some(v) = c.as_ref() {
            return if v.starts_with("lat ") { "ok" } else { "error" }.to_string();
        }
    }
    "pending".to_string()
}

/// Start a refresh unless one is already running.
///
/// `swap` rather than load-then-store: two mounts land close together and both
/// would otherwise spawn, and `get` serialises on LOCATING anyway, so the
/// second would sit holding a thread for the length of the first.
fn kick() {
    if REFRESHING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        // 8s was not enough for a cold fix indoors; the device answered
        // "no fix within 8000 ms" every time. This runs on a worker and
        // nothing waits on it, so it can afford to be patient.
        let answer = match get(30_000, "navigation") {
            Ok(v) => summarise(&v),
            Err(e) => e,
        };
        if let Ok(mut c) = CACHE.lock() {
            *c = Some(answer);
        }
        REFRESHING.store(false, Ordering::SeqCst);
        // Nothing re-mounts a static screen on its own. Without this the
        // compass card renders "locating…" once and keeps it forever, however
        // long the fix takes — the screen would be stale rather than wrong,
        // which is the harder kind of wrong to notice.
        DIRTY.store(true, Ordering::SeqCst);
    });
}

/// One line out of the fix JSON, for a screen that can only show a string.
fn summarise(json: &str) -> String {
    let num = |key: &str| -> Option<f64> {
        let at = json.find(&format!("\"{key}\":"))? + key.len() + 3;
        json[at..]
            .split(|c: char| c == ',' || c == '}')
            .next()?
            .trim()
            .parse()
            .ok()
    };
    match (num("latitude"), num("longitude")) {
        (Some(lat), Some(lon)) => match num("accuracy") {
            Some(a) => format!("lat {lat:.4}, lon {lon:.4}  ±{a:.0}m"),
            None => format!("lat {lat:.4}, lon {lon:.4}"),
        },
        _ => json.to_string(),
    }
}
