//! Native device facts, straight from the OpenHarmony NDK.
//!
//! These are plain C APIs — `const char *` getters and out-param calls — so
//! unlike ArkUI they need no C++ shim, only `extern "C"` declarations and the
//! right `-l` flags in `build.rs`.
//!
//! The point of routing them through the bridge is that a web surface cannot
//! learn any of this on its own. A page in a browser gets a user-agent string
//! and `devicePixelRatio`; a page in a Splash web slot can ask what the phone
//! actually is, how the panel is really configured, and what the battery is
//! doing — because Rust asks the system and hands back the answer.
//!
//! Three libraries, all in the SDK's aarch64 sysroot:
//!
//! | lib | what |
//! |---|---|
//! | `libdeviceinfo_ndk.z.so` | brand, model, OS version, ABI, security patch |
//! | `libnative_display_manager.so` | size, density, refresh rate, rotation |
//! | `libohbattery_info.so` | charge level and power source |

use crate::bridge::json_str;
use std::ffi::CStr;
use std::os::raw::c_char;

// deviceinfo_ndk — every one of these returns a static string owned by the
// system, so they are read and copied, never freed.
extern "C" {
    fn OH_GetDeviceType() -> *const c_char;
    fn OH_GetManufacture() -> *const c_char;
    fn OH_GetBrand() -> *const c_char;
    fn OH_GetMarketName() -> *const c_char;
    fn OH_GetProductModel() -> *const c_char;
    fn OH_GetHardwareModel() -> *const c_char;
    fn OH_GetAbiList() -> *const c_char;
    fn OH_GetSecurityPatchTag() -> *const c_char;
    fn OH_GetDisplayVersion() -> *const c_char;
    fn OH_GetOSFullName() -> *const c_char;
    fn OH_GetSdkApiVersion() -> i32;
    fn OH_GetDistributionOSName() -> *const c_char;
    fn OH_GetDistributionOSVersion() -> *const c_char;
}

// native_display_manager — out-param style, 0 (DISPLAY_MANAGER_OK) on success.
extern "C" {
    fn OH_NativeDisplayManager_GetDefaultDisplayWidth(w: *mut i32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayHeight(h: *mut i32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayRotation(r: *mut i32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayOrientation(o: *mut i32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayRefreshRate(hz: *mut u32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayDensityDpi(dpi: *mut i32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayVirtualPixelRatio(r: *mut f32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayScaledDensity(d: *mut f32) -> i32;
}

// ohbattery_info
extern "C" {
    fn OH_BatteryInfo_GetCapacity() -> i32;
    fn OH_BatteryInfo_GetPluggedType() -> i32;
}

/// Read a system-owned C string. Null and invalid UTF-8 both become empty
/// rather than panicking — a missing device fact is not worth taking the
/// process down for.
fn s(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// What the phone is. JSON object.
pub fn info(slot: u32) -> String {
    format!(
        "{{\"platform\":\"OpenHarmony\",\"renderer\":\"ArkUI NDK via Rust\",\"slot\":{slot},\
         \"brand\":{},\"manufacturer\":{},\"marketName\":{},\"productModel\":{},\
         \"hardwareModel\":{},\"deviceType\":{},\"osFullName\":{},\"displayVersion\":{},\
         \"distroName\":{},\"distroVersion\":{},\"sdkApiVersion\":{},\"abiList\":{},\
         \"securityPatch\":{}}}",
        json_str(&s(unsafe { OH_GetBrand() })),
        json_str(&s(unsafe { OH_GetManufacture() })),
        json_str(&s(unsafe { OH_GetMarketName() })),
        json_str(&s(unsafe { OH_GetProductModel() })),
        json_str(&s(unsafe { OH_GetHardwareModel() })),
        json_str(&s(unsafe { OH_GetDeviceType() })),
        json_str(&s(unsafe { OH_GetOSFullName() })),
        json_str(&s(unsafe { OH_GetDisplayVersion() })),
        json_str(&s(unsafe { OH_GetDistributionOSName() })),
        json_str(&s(unsafe { OH_GetDistributionOSVersion() })),
        unsafe { OH_GetSdkApiVersion() },
        json_str(&s(unsafe { OH_GetAbiList() })),
        json_str(&s(unsafe { OH_GetSecurityPatchTag() })),
    )
}

/// Wrap an out-param call: `Some(v)` on DISPLAY_MANAGER_OK, `None` otherwise.
///
/// Several of these can legitimately fail (`ERROR_NO_PERMISSION` is 201), and a
/// failed read must not be reported as a zero — a refresh rate of 0 looks like
/// a fact rather than a missing one.
fn dm<T: Default>(f: impl FnOnce(*mut T) -> i32) -> Option<T> {
    let mut v = T::default();
    if f(&mut v as *mut T) == 0 {
        Some(v)
    } else {
        None
    }
}

fn num<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "null".into())
}

/// How the panel is actually configured. JSON object.
pub fn display() -> String {
    let w = dm(|p| unsafe { OH_NativeDisplayManager_GetDefaultDisplayWidth(p) });
    let h = dm(|p| unsafe { OH_NativeDisplayManager_GetDefaultDisplayHeight(p) });
    let rot = dm(|p| unsafe { OH_NativeDisplayManager_GetDefaultDisplayRotation(p) });
    let orient = dm(|p| unsafe { OH_NativeDisplayManager_GetDefaultDisplayOrientation(p) });
    let hz = dm(|p| unsafe { OH_NativeDisplayManager_GetDefaultDisplayRefreshRate(p) });
    let dpi = dm(|p| unsafe { OH_NativeDisplayManager_GetDefaultDisplayDensityDpi(p) });
    let ratio = dm(|p| unsafe { OH_NativeDisplayManager_GetDefaultDisplayVirtualPixelRatio(p) });
    let scaled = dm(|p| unsafe { OH_NativeDisplayManager_GetDefaultDisplayScaledDensity(p) });

    let rot_name = match rot {
        Some(0) => "0°",
        Some(1) => "90°",
        Some(2) => "180°",
        Some(3) => "270°",
        _ => "unknown",
    };
    let orient_name = match orient {
        Some(0) => "portrait",
        Some(1) => "landscape",
        Some(2) => "portrait-inverted",
        Some(3) => "landscape-inverted",
        _ => "unknown",
    };

    format!(
        "{{\"width\":{},\"height\":{},\"refreshRate\":{},\"densityDpi\":{},\
         \"pixelRatio\":{},\"scaledDensity\":{},\"rotation\":{},\"orientation\":{}}}",
        num(w),
        num(h),
        num(hz),
        num(dpi),
        num(ratio),
        num(scaled),
        json_str(rot_name),
        json_str(orient_name),
    )
}

/// Charge level and power source. JSON object.
pub fn battery() -> String {
    let plugged = unsafe { OH_BatteryInfo_GetPluggedType() };
    let source = match plugged {
        0 => "none",
        1 => "ac",
        2 => "usb",
        3 => "wireless",
        _ => "unknown",
    };
    format!(
        "{{\"capacity\":{},\"pluggedType\":{},\"charging\":{}}}",
        unsafe { OH_BatteryInfo_GetCapacity() },
        json_str(source),
        plugged >= 1 && plugged <= 3,
    )
}

// time_service and notification. Two more single-call kits, grouped here
// rather than given files of their own — a module per extern block would be
// more files than facts.
extern "C" {
    fn OH_TimeService_GetTimeZone(buf: *mut c_char, len: u32) -> i32;
    fn OH_Notification_IsNotificationEnabled() -> bool;
}

/// System time and zone. JSON object.
///
/// The zone is the point: `Intl.DateTimeFormat().resolvedOptions().timeZone` in
/// a page reports what the *webview* was configured with, which is not
/// necessarily what the user set on the phone. This asks the time service.
pub fn time() -> String {
    let mut buf = [0 as c_char; 128];
    let zone = if unsafe { OH_TimeService_GetTimeZone(buf.as_mut_ptr(), buf.len() as u32) } == 0 {
        s(buf.as_ptr())
    } else {
        String::new()
    };
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!(
        "{{\"timeZone\":{},\"unixSeconds\":{}}}",
        json_str(&zone),
        unix
    )
}

/// Whether the user has notifications switched on for this app.
///
/// Read-only: the native NotificationKit exposes exactly this one call, so a
/// page can find out whether posting would be seen — and cannot post.
pub fn notifications_enabled() -> String {
    format!("{{\"enabled\":{}}}", unsafe {
        OH_Notification_IsNotificationEnabled()
    })
}
