//! Splash-OH — render a UI tree to OpenHarmony **native ArkUI widgets** from
//! Rust, with no ArkTS in the loop.
//!
//! ArkTS's entire role is to hand over one `NodeContent` slot at startup. After
//! `mount()` returns, every widget in the app was created, configured, laid out
//! and event-wired by this library. There are no per-widget and no per-frame
//! ArkTS calls, which is the point: on the octos-one OHOS port a round trip
//! through ArkTS measured ~1.0 ms (70% of it just waiting for the JS event loop
//! to become free), and that cost scales with widget count.

pub mod app;
pub mod arkui;
pub mod bench;
pub mod catalog;
pub mod dsl;

use arkui::NodeContentHandle;
use napi_derive_ohos::napi;
use napi_ohos::{Env, JsObject, NapiRaw};

extern "C" {
    /// napi_value (an ArkTS `NodeContent`) -> native slot handle.
    fn OH_ArkUI_GetNodeContentFromNapiValue(
        env: napi_ohos::sys::napi_env,
        value: napi_ohos::sys::napi_value,
        handle: *mut NodeContentHandle,
    ) -> i32;
}

/// Called once from ArkTS with the page's `NodeContent`.
///
/// This is the ONLY ArkTS -> native entry point in the app.
#[napi(js_name = "mount")]
pub fn mount(env: Env, content: JsObject) -> napi_ohos::Result<()> {
    if let Err(e) = arkui::init() {
        log(&format!("splash-oh: {e}"));
        return Ok(());
    }

    let mut slot: NodeContentHandle = std::ptr::null_mut();
    let status = unsafe {
        OH_ArkUI_GetNodeContentFromNapiValue(env.raw(), content.raw(), &mut slot as *mut _)
    };
    if status != 0 || slot.is_null() {
        log("splash-oh: could not resolve NodeContent from the napi value");
        return Ok(());
    }

    app::init(slot);
    Ok(())
}

/// hilog, so this is debuggable on device without stdout (an OHOS app has none).
pub(crate) fn log(msg: &str) {
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
