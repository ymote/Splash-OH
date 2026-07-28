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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

extern "C" {
    fn dlopen(file: *const c_char, mode: i32) -> *mut std::ffi::c_void;
    fn dlsym(handle: *mut std::ffi::c_void, name: *const c_char) -> *mut std::ffi::c_void;
}

type RunJs = unsafe extern "C" fn(*const c_char, *const c_char, extern "C" fn(*const c_char));

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

/// Slots whose tag has answered a probe, as a bitmask.
///
/// `RunJavaScript` returns void, so "did it work" cannot be read from the call.
/// It does take a callback, though, and the callback carries the evaluated
/// result — so a page that is reachable will answer a trivial expression, and
/// one that is not will simply never call back. That is the only reliable
/// signal available, and it is what gates native delivery.
static PROVEN: AtomicU64 = AtomicU64::new(0);
/// Slots a probe has been fired at, so it is fired once rather than per reply.
static PROBED: AtomicU64 = AtomicU64::new(0);

fn bit(slot: u32) -> u64 {
    1u64 << (slot % 64)
}

/// The probe's callback. Receives the evaluated result as a string.
///
/// It carries no user data and no tag, so it cannot say *which* slot answered.
/// One probe is therefore in flight at a time and the slot is remembered
/// alongside it — with several outstanding, an answer could not be attributed.
extern "C" fn on_probe_result(result: *const c_char) {
    let slot = PROBING.load(Ordering::SeqCst);
    let text = if result.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(result) }
            .to_string_lossy()
            .into_owned()
    };
    PROVEN.fetch_or(bit(slot as u32), Ordering::SeqCst);
    crate::log(&format!(
        "arkweb: slot {slot} answered a native probe with {text:?} — delivering natively"
    ));
}

static PROBING: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// How many probes to fire before concluding the tag will never resolve.
///
/// Retried rather than fired once, because the first attempt happens about a
/// second after start and a controller only binds to a webview when the
/// component using it is created — a single early probe cannot distinguish
/// "not yet" from "never".
const MAX_PROBES: u32 = 24;
static PROBE_ATTEMPTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static LAST_PROBE: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// Ask a slot whether it is reachable natively.
fn probe(slot: u32) {
    // Spaced in time, not just counted. The first attempt at this burned all
    // 24 tries inside 10 ms, because replies arrive in a burst at startup --
    // which tested one instant 24 times rather than testing "later" at all.
    {
        let mut last = match LAST_PROBE.lock() {
            Ok(l) => l,
            Err(_) => return,
        };
        let now = std::time::Instant::now();
        if let Some(t) = *last {
            if now.duration_since(t) < std::time::Duration::from_millis(900) {
                return;
            }
        }
        *last = Some(now);
    }
    let n = PROBE_ATTEMPTS.fetch_add(1, Ordering::SeqCst);
    if n >= MAX_PROBES {
        if n == MAX_PROBES {
            crate::log(&format!(
                "arkweb: {MAX_PROBES} probes unanswered; the controller webTag does not \
                 resolve natively on this build. Staying on the ArkTS path."
            ));
        }
        return;
    }
    let _ = PROBED.fetch_or(bit(slot), Ordering::SeqCst);
    let Some(f) = run_js_fn() else { return };
    let (Ok(tag), Ok(code)) = (CString::new(web_tag(slot)), CString::new("1+1")) else {
        return;
    };
    PROBING.store(slot, Ordering::SeqCst);
    crate::log(&format!("arkweb: probing slot {slot} natively"));
    unsafe { f(tag.as_ptr(), code.as_ptr(), on_probe_result) };
}

/// Evaluate `js` in the page behind `slot`.
///
/// Returns whether the caller may consider it delivered. Native delivery is
/// used only for a slot that has answered a probe: `RunJavaScript` returns void,
/// so an implementation that assumes success reports "I called something"
/// rather than "it arrived" — which is exactly how an earlier version of this
/// silently dropped every reply and left the whole card on ellipses.
pub fn run_js(slot: u32, js: &str) -> bool {
    if run_js_fn().is_none() {
        if !WARNED.swap(true, Ordering::Relaxed) {
            crate::log("arkweb: no native RunJavaScript; using the ArkTS path");
        }
        return false;
    }
    if PROVEN.load(Ordering::SeqCst) & bit(slot) == 0 {
        // Not yet shown reachable. Start the probe and let this reply go the
        // ArkTS way; the next one benefits if the probe answers.
        probe(slot);
        return false;
    }
    let (Ok(tag), Ok(code)) = (CString::new(web_tag(slot)), CString::new(js)) else {
        return false;
    };
    unsafe { run_js_fn().unwrap()(tag.as_ptr(), code.as_ptr(), noop_callback) };
    true
}

/// Delivery does not want the result, but the parameter is not optional.
extern "C" fn noop_callback(_result: *const c_char) {}
