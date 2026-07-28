//! Driving a webview from Rust, without ArkTS on the path.
//!
//! `native_interface_arkweb.h` exposes the parts of ArkWeb that matter to this
//! bridge as plain C, addressed by a **web tag** — a name ArkTS gives the
//! controller (`new webview.WebviewController('slot3')`) and Rust then uses to
//! talk to that webview directly:
//!
//! ```text
//! OH_NativeArkWeb_RunJavaScript(tag, js, cb)          evaluate into the page
//! OH_NativeArkWeb_RegisterJavaScriptProxy(tag, ...)   expose an object to JS
//! OH_NativeArkWeb_SetJavaScriptProxyValidCallback     know when it is live
//! ```
//!
//! # What this removes
//!
//! Every reply and every event used to travel Rust → ArkTS → `runJavaScript`.
//! That crossed a language boundary and, before the push channel, waited for a
//! 250 ms poll as well. With this, Rust evaluates into the page itself and
//! ArkTS is not involved in a bridge call at all.
//!
//! # What it does not remove
//!
//! ArkTS still declares the `Web` component, because **there is no
//! `ARKUI_NODE_WEB`** — the NDK has no web node, so Rust cannot create one. It
//! can drive one that already exists. That is the same boundary the whole web
//! slot design sits on, and it is not movable from here.
//!
//! Four tools also remain ArkTS-only because they have no NDK at all: the file
//! picker, the clipboard, runtime permission requests, and BLE scanning.

use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};

extern "C" {
    fn dlopen(file: *const c_char, mode: i32) -> *mut std::ffi::c_void;
    fn dlsym(handle: *mut std::ffi::c_void, name: *const c_char) -> *mut std::ffi::c_void;
}

type RunJs = unsafe extern "C" fn(*const c_char, *const c_char, *mut std::ffi::c_void);

static RUN_JS: std::sync::OnceLock<Option<RunJs>> = std::sync::OnceLock::new();
/// Logged once, so a device without the interface says so exactly one time
/// rather than on every reply.
static WARNED: AtomicBool = AtomicBool::new(false);

fn run_js_fn() -> Option<RunJs> {
    *RUN_JS.get_or_init(|| {
        // libohweb is the ArkWeb native interface. dlopen'd rather than linked
        // for the reason #26 established: an unresolved DT_NEEDED symbol kills
        // the whole library, and this is an optimisation, not a requirement --
        // the ArkTS path still works if it is absent.
        for name in ["libohweb.so", "libarkweb_core.so", "libnweb_ohos.so"] {
            let Ok(c) = CString::new(name) else { continue };
            let h = unsafe { dlopen(c.as_ptr(), 2) };
            if h.is_null() {
                continue;
            }
            let Ok(sym) = CString::new("OH_NativeArkWeb_RunJavaScript") else {
                continue;
            };
            let p = unsafe { dlsym(h, sym.as_ptr()) };
            if !p.is_null() {
                crate::log(&format!("arkweb: native RunJavaScript via {name}"));
                return Some(unsafe { std::mem::transmute::<*mut std::ffi::c_void, RunJs>(p) });
            }
        }
        crate::log("arkweb: no native RunJavaScript; replies will go via ArkTS");
        None
    })
}

/// The tag ArkTS gives slot `id`'s controller. One place, because both sides
/// have to agree and a mismatch is silent — the call simply does nothing.
pub fn web_tag(slot: u32) -> String {
    format!("slot{slot}")
}

/// Has native evaluation been shown to actually reach a page?
///
/// It has not, on this device, and the distinction matters more than usual
/// because `OH_NativeArkWeb_RunJavaScript` returns **void**. There is no
/// success value to check, so an implementation that reports "sent" after
/// calling it is really reporting "I called something" -- and that is exactly
/// what happened here: the symbol resolved from libohweb.so, every reply took
/// the native route, none of them arrived, and the whole card sat on ellipses
/// because the fallback never fired.
///
/// The tag is the likely gap. ArkTS names the controller
/// (`new webview.WebviewController('slot3')`), but a controller only binds to a
/// webview once the component using it is created, and the native side may key
/// on something set at component construction rather than on the controller.
/// `OH_NativeArkWeb_SetJavaScriptProxyValidCallback` exists precisely so native
/// code can learn when a tag becomes live, which suggests the binding is not
/// immediate.
///
/// Until a probe proves a round trip, this stays off. A delivery path that
/// silently drops everything is far worse than one that costs a language
/// boundary, and there is no way to tell them apart from the return value.
const NATIVE_DELIVERY_PROVEN: bool = false;

/// Evaluate `js` in the page behind `slot`. Returns whether the caller may
/// consider it delivered -- see [`NATIVE_DELIVERY_PROVEN`].
pub fn run_js(slot: u32, js: &str) -> bool {
    if !NATIVE_DELIVERY_PROVEN {
        if !WARNED.swap(true, Ordering::Relaxed) {
            crate::log("arkweb: native eval available but unproven; using the ArkTS path");
        }
        return false;
    }
    let Some(f) = run_js_fn() else {
        return false;
    };
    let (Ok(tag), Ok(code)) = (CString::new(web_tag(slot)), CString::new(js)) else {
        return false;
    };
    unsafe { f(tag.as_ptr(), code.as_ptr(), std::ptr::null_mut()) };
    true
}
