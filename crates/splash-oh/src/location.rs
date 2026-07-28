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

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct BasicInfo {
    latitude: f64,
    longitude: f64,
    altitude: f64,
    accuracy: f64,
    speed: f64,
    direction: f64,
    time_stamp: i64,
    time_since_boot: i64,
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
        Ok(format!(
            "{{\"latitude\":{:.6},\"longitude\":{:.6},\"altitude\":{:.1},\
             \"accuracy\":{:.1},\"speed\":{:.2},\"direction\":{:.1},\"timestamp\":{}}}",
            b.latitude, b.longitude, b.altitude, b.accuracy, b.speed, b.direction, b.time_stamp
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
