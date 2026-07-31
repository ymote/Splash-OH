//! Sensors and haptics, from the OpenHarmony NDK.
//!
//! Two capabilities a web surface has no other route to. A browser page can
//! sometimes get `devicemotion` behind a permission prompt and nothing at all
//! for haptics; a page in a Splash web slot asks Rust, and Rust asks the system.
//!
//! # Why reading a sensor is not a getter
//!
//! There is no "read the accelerometer" call. The NDK is subscribe-only:
//! `OH_Sensor_Subscribe` registers a C callback that the sensor service invokes
//! on **its own thread** at the sampling interval. Getting one value therefore
//! means subscribing, waiting for a sample to land, and unsubscribing — which
//! is what [`sample`] does, on a worker so nothing on the UI thread waits for a
//! hardware tick.
//!
//! The callback cannot carry state, so the sample it captures goes into a
//! static and the waiting side picks it up. One in-flight read at a time,
//! guarded by a mutex, because two overlapping subscriptions to the same sensor
//! type would race on that static and hand each caller the other's reading.

use crate::bridge::json_str;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[repr(C)]
#[derive(Clone, Copy)]
struct VibratorAttribute {
    vibrator_id: i32,
    usage: i32,
}

extern "C" {
    fn OH_Vibrator_PlayVibration(duration: i32, attribute: VibratorAttribute) -> i32;
    fn OH_Vibrator_Cancel() -> i32;

    fn OH_Sensor_CreateInfos(count: u32) -> *mut *mut c_void;
    fn OH_Sensor_DestroyInfos(infos: *mut *mut c_void, count: u32) -> i32;
    fn OH_Sensor_GetInfos(infos: *mut *mut c_void, count: *mut u32) -> i32;
    fn OH_SensorInfo_GetName(info: *mut c_void, name: *mut c_char, len: *mut u32) -> i32;
    fn OH_SensorInfo_GetVendorName(info: *mut c_void, name: *mut c_char, len: *mut u32) -> i32;
    fn OH_SensorInfo_GetType(info: *mut c_void, ty: *mut i32) -> i32;

    fn OH_Sensor_CreateSubscriptionId() -> *mut c_void;
    fn OH_Sensor_DestroySubscriptionId(id: *mut c_void) -> i32;
    fn OH_SensorSubscriptionId_SetType(id: *mut c_void, ty: i32) -> i32;
    fn OH_Sensor_CreateSubscriptionAttribute() -> *mut c_void;
    fn OH_Sensor_DestroySubscriptionAttribute(a: *mut c_void) -> i32;
    fn OH_SensorSubscriptionAttribute_SetSamplingInterval(a: *mut c_void, ns: i64) -> i32;
    fn OH_Sensor_CreateSubscriber() -> *mut c_void;
    fn OH_Sensor_DestroySubscriber(s: *mut c_void) -> i32;
    fn OH_SensorSubscriber_SetCallback(
        s: *mut c_void,
        cb: extern "C" fn(*mut c_void, *mut c_void),
    ) -> i32;
    fn OH_Sensor_Subscribe(id: *mut c_void, attr: *mut c_void, sub: *mut c_void) -> i32;
    fn OH_Sensor_Unsubscribe(id: *mut c_void, sub: *mut c_void) -> i32;
    fn OH_SensorEvent_GetData(e: *mut c_void, data: *mut *mut f32, len: *mut u32) -> i32;
    fn OH_SensorEvent_GetAccuracy(e: *mut c_void, acc: *mut i32) -> i32;
}

/// The sensors worth naming. The enum has ~15 more; these are the ones a card
/// might plausibly use, and anything else still enumerates by number.
fn type_name(t: i32) -> &'static str {
    match t {
        1 => "accelerometer",
        2 => "gyroscope",
        5 => "ambient light",
        6 => "magnetic field",
        8 => "barometer",
        10 => "hall",
        12 => "proximity",
        256 => "orientation",
        257 => "gravity",
        258 => "linear acceleration",
        259 => "rotation vector",
        262 => "game rotation vector",
        265 => "pedometer detection",
        266 => "pedometer",
        278 => "heart rate",
        _ => "other",
    }
}

pub fn type_from_name(s: &str) -> Option<i32> {
    Some(match s {
        "accelerometer" => 1,
        "gyroscope" => 2,
        "ambient light" | "light" => 5,
        "magnetic field" | "magnetometer" => 6,
        "barometer" => 8,
        "proximity" => 12,
        "orientation" => 256,
        "gravity" => 257,
        "linear acceleration" => 258,
        "rotation vector" => 259,
        _ => return None,
    })
}

fn c_str(buf: &[c_char], len: u32) -> String {
    let n = (len as usize).min(buf.len());
    let bytes: Vec<u8> = buf[..n]
        .iter()
        .map(|&c| c as u8)
        .take_while(|&c| c != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// What sensors this phone has. JSON array of `{type, name, vendor, id}`.
pub fn list() -> Result<String, String> {
    let mut count: u32 = 0;
    if unsafe { OH_Sensor_GetInfos(std::ptr::null_mut(), &mut count) } != 0 {
        return Err("could not count sensors".into());
    }
    if count == 0 {
        return Ok("[]".into());
    }
    let infos = unsafe { OH_Sensor_CreateInfos(count) };
    if infos.is_null() {
        return Err("could not allocate sensor info".into());
    }
    if unsafe { OH_Sensor_GetInfos(infos, &mut count) } != 0 {
        unsafe { OH_Sensor_DestroyInfos(infos, count) };
        return Err("could not read sensors".into());
    }

    let mut out = Vec::new();
    for i in 0..count as isize {
        let info = unsafe { *infos.offset(i) };
        if info.is_null() {
            continue;
        }
        let mut ty: i32 = 0;
        unsafe { OH_SensorInfo_GetType(info, &mut ty) };

        // These take a buffer plus its length as an in/out parameter.
        let mut name = [0 as c_char; 128];
        let mut nlen = name.len() as u32;
        unsafe { OH_SensorInfo_GetName(info, name.as_mut_ptr(), &mut nlen) };
        let mut vendor = [0 as c_char; 128];
        let mut vlen = vendor.len() as u32;
        unsafe { OH_SensorInfo_GetVendorName(info, vendor.as_mut_ptr(), &mut vlen) };

        out.push(format!(
            "{{\"type\":{},\"kind\":{},\"name\":{},\"vendor\":{}}}",
            ty,
            json_str(type_name(ty)),
            json_str(&c_str(&name, nlen)),
            json_str(&c_str(&vendor, vlen))
        ));
    }
    unsafe { OH_Sensor_DestroyInfos(infos, count) };
    Ok(format!("[{}]", out.join(",")))
}

/// Where the callback drops its sample. See the module note: the C callback
/// takes no user pointer, so there is nowhere else to put it.
static SAMPLE: Mutex<Option<(Vec<f32>, i32)>> = Mutex::new(None);
static GOT: AtomicBool = AtomicBool::new(false);
/// One in-flight read at a time. Two overlapping subscriptions would race on
/// SAMPLE and hand each caller the other's reading.
static READING: Mutex<()> = Mutex::new(());

/// Where a running stream sends its samples, and whether one is running.
///
/// The C callback takes no user pointer, so the destination cannot travel with
/// the subscription and has to live here.
static STREAM_SLOT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static STREAMING: AtomicBool = AtomicBool::new(false);
static STREAM_KIND: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
static STREAM_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Samples waiting for the worker to format and send. Bounded, because a
/// stalled drainer must cost samples rather than memory.
#[allow(clippy::type_complexity)]
static PENDING: Mutex<Vec<(i32, Vec<f32>, u32)>> = Mutex::new(Vec::new());

extern "C" fn on_event(event: *mut c_void, _user: *mut c_void) {
    if event.is_null() {
        return;
    }
    let mut data: *mut f32 = std::ptr::null_mut();
    let mut len: u32 = 0;
    if unsafe { OH_SensorEvent_GetData(event, &mut data, &mut len) } != 0 || data.is_null() {
        return;
    }
    let vals = unsafe { std::slice::from_raw_parts(data, len as usize) }.to_vec();
    let mut acc: i32 = 0;
    unsafe { OH_SensorEvent_GetAccuracy(event, &mut acc) };
    // A stream hands the sample to a worker rather than sending it here.
    //
    // audio.rs states the rule for callbacks on service threads -- no locking,
    // no allocating -- and the first version of this broke it: formatting the
    // payload allocated four times and bridge::emit takes a mutex, all on the
    // sensor service's own thread. The sensor thread is not hard real-time the
    // way the audio one is, so nothing audibly broke, but a rule worth writing
    // down is worth not violating two commits later.
    //
    // Copying six floats into a bounded queue is the whole cost here; the
    // worker does the formatting and the send.
    if STREAMING.load(Ordering::Relaxed) {
        let n = STREAM_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if let Ok(mut q) = PENDING.try_lock() {
            // try_lock, not lock: if the drainer holds it, drop this sample
            // rather than stall the sensor service. A dropped sample at 20 Hz
            // is invisible; a blocked callback is not.
            if q.len() < 256 {
                q.push((STREAM_KIND.load(Ordering::Relaxed), vals, n));
            }
        }
        return;
    }
    if let Ok(mut s) = SAMPLE.lock() {
        *s = Some((vals, acc));
    }
    GOT.store(true, Ordering::SeqCst);
}

/// Subscribe, take one sample, unsubscribe. JSON `{kind, values, accuracy}`.
///
/// Blocks the calling thread for up to `timeout_ms` waiting for the sensor
/// service to deliver a tick, so it must run on a worker — never the UI thread.
pub fn sample(kind: i32, timeout_ms: u64) -> Result<String, String> {
    let _one_at_a_time = READING.lock().map_err(|_| "sensor lock poisoned")?;

    let id = unsafe { OH_Sensor_CreateSubscriptionId() };
    let attr = unsafe { OH_Sensor_CreateSubscriptionAttribute() };
    let sub = unsafe { OH_Sensor_CreateSubscriber() };
    if id.is_null() || attr.is_null() || sub.is_null() {
        return Err("could not create a subscription".into());
    }

    // Everything below must run the teardown, so the body is a closure and the
    // destroys happen on the way out whatever it returns.
    let result = (|| {
        unsafe { OH_SensorSubscriptionId_SetType(id, kind) };
        // 50 ms in nanoseconds: fast enough that one sample lands promptly,
        // slow enough not to flood for the single tick that is wanted.
        unsafe { OH_SensorSubscriptionAttribute_SetSamplingInterval(attr, 50_000_000) };
        unsafe { OH_SensorSubscriber_SetCallback(sub, on_event) };

        GOT.store(false, Ordering::SeqCst);
        if let Ok(mut s) = SAMPLE.lock() {
            *s = None;
        }

        let rc = unsafe { OH_Sensor_Subscribe(id, attr, sub) };
        if rc != 0 {
            // 201 is the NDK's permission error; say so rather than "failed",
            // because a missing permission is a different fix from a missing
            // sensor.
            return Err(if rc == 201 {
                format!("permission denied for {} sensor", type_name(kind))
            } else {
                format!("subscribe failed ({rc})")
            });
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        while !GOT.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        unsafe { OH_Sensor_Unsubscribe(id, sub) };

        if !GOT.load(Ordering::SeqCst) {
            return Err(format!("no sample within {timeout_ms} ms"));
        }
        let (vals, acc) = SAMPLE
            .lock()
            .ok()
            .and_then(|mut s| s.take())
            .ok_or("sample vanished")?;
        let nums: Vec<String> = vals.iter().map(|v| format!("{v:.4}")).collect();
        Ok(format!(
            "{{\"type\":{},\"kind\":{},\"values\":[{}],\"accuracy\":{}}}",
            kind,
            json_str(type_name(kind)),
            nums.join(","),
            acc
        ))
    })();

    unsafe {
        OH_Sensor_DestroySubscriptionId(id);
        OH_Sensor_DestroySubscriptionAttribute(attr);
        OH_Sensor_DestroySubscriber(sub);
    }
    result
}

/// Buzz for `ms`. Needs `ohos.permission.VIBRATE`.
///
/// Capped: a page should not be able to hold the motor on. `usage` is TOUCH,
/// which is what the system uses to decide whether to honour it under Do Not
/// Disturb — a UI tap is not an alarm and should not behave like one.
pub fn vibrate(ms: i32) -> Result<String, String> {
    let ms = ms.clamp(1, 2000);
    let rc = unsafe {
        OH_Vibrator_PlayVibration(
            ms,
            VibratorAttribute {
                vibrator_id: 0,
                usage: 5,
            },
        )
    };
    if rc == 0 {
        Ok(format!("{{\"ms\":{ms}}}"))
    } else if rc == 201 {
        Err("permission denied: ohos.permission.VIBRATE".into())
    } else {
        Err(format!("vibrate failed ({rc})"))
    }
}

pub fn vibrate_cancel() -> i32 {
    unsafe { OH_Vibrator_Cancel() }
}

/// Format and send whatever the callback has queued.
fn drain_pending(slot: u32) {
    let batch: Vec<(i32, Vec<f32>, u32)> = match PENDING.lock() {
        Ok(mut q) => q.drain(..).collect(),
        Err(_) => return,
    };
    for (kind, vals, n) in batch {
        let nums: Vec<String> = vals.iter().map(|v| format!("{v:.3}")).collect();
        crate::bridge::emit(
            slot,
            "sensor",
            &format!(
                "{{\"kind\":{},\"values\":[{}],\"n\":{}}}",
                json_str(type_name(kind)),
                nums.join(","),
                n
            ),
        );
    }
}

/// Stream a sensor for `ms`, emitting every sample as a `sensor` event.
///
/// The case `emit` exists for. An accelerometer at 20 Hz cannot travel through
/// a request/reply channel — the page would have to ask sixty times for three
/// seconds of data, and each answer would already be stale. Here Rust sends and
/// the page listens.
pub fn stream(kind: i32, slot: u32, ms: u64) -> Result<String, String> {
    let _one_at_a_time = READING.lock().map_err(|_| "sensor lock poisoned")?;
    let ms = ms.clamp(200, 10_000);

    let id = unsafe { OH_Sensor_CreateSubscriptionId() };
    let attr = unsafe { OH_Sensor_CreateSubscriptionAttribute() };
    let sub = unsafe { OH_Sensor_CreateSubscriber() };
    if id.is_null() || attr.is_null() || sub.is_null() {
        return Err("could not create a subscription".into());
    }

    let result = (|| {
        unsafe { OH_SensorSubscriptionId_SetType(id, kind) };
        // 50 ms -> 20 samples a second, which is a stream rather than a trickle
        // and still well inside what the bridge can carry.
        unsafe { OH_SensorSubscriptionAttribute_SetSamplingInterval(attr, 50_000_000) };
        unsafe { OH_SensorSubscriber_SetCallback(sub, on_event) };

        STREAM_SLOT.store(slot, Ordering::SeqCst);
        STREAM_KIND.store(kind, Ordering::SeqCst);
        STREAM_COUNT.store(0, Ordering::SeqCst);
        STREAMING.store(true, Ordering::SeqCst);

        let rc = unsafe { OH_Sensor_Subscribe(id, attr, sub) };
        if rc != 0 {
            STREAMING.store(false, Ordering::SeqCst);
            return Err(if rc == 201 {
                format!("permission denied for {} sensor", type_name(kind))
            } else {
                format!("subscribe failed ({rc})")
            });
        }
        // Drain on this thread, not the sensor's. Polled at 10 ms, which is
        // half the 50 ms sampling interval, so a sample waits 10 ms at worst.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
            drain_pending(slot);
        }
        STREAMING.store(false, Ordering::SeqCst);
        unsafe { OH_Sensor_Unsubscribe(id, sub) };
        // Anything the callback queued after the last poll.
        drain_pending(slot);

        let sent = STREAM_COUNT.load(Ordering::SeqCst);
        Ok(format!(
            "{{\"kind\":{},\"ms\":{},\"events\":{},\"hz\":{:.1}}}",
            json_str(type_name(kind)),
            ms,
            sent,
            sent as f64 * 1000.0 / ms as f64
        ))
    })();

    unsafe {
        OH_Sensor_DestroySubscriptionId(id);
        OH_Sensor_DestroySubscriptionAttribute(attr);
        OH_Sensor_DestroySubscriber(sub);
    }
    result
}
