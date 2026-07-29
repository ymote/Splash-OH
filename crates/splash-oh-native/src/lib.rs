//! Splash DSL rendered to OpenHarmony **native ArkUI widgets**, with no ArkTS
//! in the loop.
//!
//! ArkTS's entire role is to hand over one `NodeContent` slot at startup. After
//! that, every widget was created, configured, laid out and event-wired by this
//! crate. There are no per-widget and no per-frame ArkTS calls.
//!
//! # Why this is a crate of its own
//!
//! It has no idea that webviews exist. The bridge, the web slots and the 45
//! native capability tools live in `splash-oh-webview`, which depends on this
//! one — and the dependency runs in exactly that direction, which is what made
//! the split worth doing. Nothing here references the bridge; every web card
//! over there builds its chrome out of the widgets defined here.
//!
//! For what the native path is actually worth against ArkTS, measured on
//! device, see [`bench`] — the answer is ~2.5x on construction, not the ~45x
//! this repo originally claimed.

pub mod app;
pub mod arkui;
pub mod bench;
pub mod catalog;
pub mod dsl;
pub mod mem;
pub mod net;
pub mod taobao;
pub mod tiktok;
pub mod ui;
pub mod wechat;
pub mod wonderous;

/// hilog, so this is debuggable on device without stdout (an OHOS app has none).
///
/// Public here, where it was `pub(crate)` before: the webview crate logs too,
/// and nothing should have to depend on the bridge to write a line.
pub fn log(msg: &str) {
    #[link(name = "hilog_ndk.z")]
    extern "C" {
        fn OH_LOG_Print(
            log_type: i32,
            level: i32,
            domain: u32,
            tag: *const std::os::raw::c_char,
            fmt: *const std::os::raw::c_char,
            ...
        ) -> i32;
    }
    if let Ok(c) = std::ffi::CString::new(msg) {
        unsafe {
            // `%{public}s` — a bare %s is redacted as <private> by hilog.
            OH_LOG_Print(
                0,
                4,
                0xAF01,
                c"SplashOH".as_ptr(),
                c"%{public}s".as_ptr(),
                c.as_ptr(),
            );
        }
    }
}

/// A host capability call: a tool name in, an answer out.
///
/// The renderer must not depend on the bridge — that one-directional rule is
/// the point of the crate split — so the capabilities are reached through a
/// hook the bridge installs at mount, exactly as `app::set_router` does for
/// navigation.
pub type HostInvoke = fn(&str) -> String;

static HOST_INVOKE: std::sync::Mutex<Option<HostInvoke>> = std::sync::Mutex::new(None);

/// Installed once by the crate that owns the capabilities.
pub fn set_host_invoke(f: HostInvoke) {
    if let Ok(mut h) = HOST_INVOKE.lock() {
        *h = Some(f);
    }
}

/// Call a capability by name. Answers with a marker when no host is installed,
/// so a screen renders on a backend that has none rather than failing.
pub fn host_invoke(tool: &str) -> String {
    match HOST_INVOKE.lock().ok().and_then(|h| *h) {
        Some(f) => f(tool),
        None => "unavailable".to_string(),
    }
}
