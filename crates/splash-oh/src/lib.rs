//! Splash-OH — render a UI tree to OpenHarmony **native ArkUI widgets** from
//! Rust, with no ArkTS in the loop.
//!
//! ArkTS's entire role is to hand over one `NodeContent` slot at startup. After
//! `mount()` returns, every widget in the app was created, configured, laid out
//! and event-wired by this library. There are no per-widget and no per-frame
//! ArkTS calls.
//!
//! For what that is actually worth, measured on device, see `bench.rs` — the
//! answer is smaller than this repo originally claimed (~2.5× on construction,
//! not ~45×) and the real argument is about contention rather than raw speed.

pub mod app;
pub mod arkui;
pub mod bench;
pub mod catalog;
pub mod dsl;

use arkui::NodeContentHandle;
use napi_derive_ohos::napi;
use napi_ohos::threadsafe_function::{
    ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi_ohos::{Env, JsFunction, JsObject, NapiRaw};
use std::sync::{Condvar, Mutex, OnceLock};

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


// ---------------------------------------------------------------------------
// Benchmark plumbing.
//
// Measurement B is timed in ArkTS and reported down here. Measurement C times
// the napi boundary in the expensive direction (native -> JS) by posting to
// the JS thread and blocking until JS has run and called back.
// ---------------------------------------------------------------------------

/// A JS function the bridge benchmark calls. It builds `n` widgets in ArkTS
/// and then calls `bridgeDone`.
static BRIDGE: OnceLock<ThreadsafeFunction<u32, ErrorStrategy::Fatal>> = OnceLock::new();

/// Completion signal for one bridge crossing. A plain counter plus a condvar:
/// the worker records the count it wants and sleeps until JS gets there.
struct Signal {
    count: Mutex<u64>,
    cv: Condvar,
}
static SIGNAL: OnceLock<Signal> = OnceLock::new();
fn signal() -> &'static Signal {
    SIGNAL.get_or_init(|| Signal {
        count: Mutex::new(0),
        cv: Condvar::new(),
    })
}

/// Measurement B: ArkTS timed itself building `n` nodes, `trials_ms` per trial.
#[napi(js_name = "reportArkts")]
pub fn report_arkts(n: u32, trials_ms: Vec<f64>) {
    bench::record_arkts(n, trials_ms);
    app::rebuild();
}

/// Does nothing. Used to measure what one JS -> native napi call costs on its
/// own, which is the cheap direction: a direct call, not a queued post.
#[napi(js_name = "noop")]
pub fn noop() {}

/// Warm both paths before any timing starts.
#[napi(js_name = "rustWarmup")]
pub fn rust_warmup() {
    bench::rust_warmup();
}

/// One Rust trial. `kind` 0 = node with five attributes, 1 = node alone,
/// 2 = one setAttribute call (returns µs per call, not per node). Called once per event-loop tick so neither side
/// monopolises the JS thread.
#[napi(js_name = "rustTrial")]
pub fn rust_trial(kind: u32) -> f64 {
    bench::rust_trial(kind)
}

/// Everything, once ArkTS has collected both sides.
#[napi(js_name = "reportAll")]
#[allow(clippy::too_many_arguments)]
pub fn report_all(
    rust_full: Vec<f64>,
    rust_create: Vec<f64>,
    arkts_full: Vec<f64>,
    arkts_create: Vec<f64>,
    rust_attr: f64,
    arkts_attr: f64,
    crossing: f64,
    empty_loop: f64,
) {
    bench::record_all(
        rust_full,
        rust_create,
        arkts_full,
        arkts_create,
        rust_attr,
        arkts_attr,
        crossing,
        empty_loop,
    );
    app::rebuild();
}

/// Hand Rust a JS function so it can measure the native -> JS direction.
#[napi(js_name = "setBridge")]
pub fn set_bridge(cb: JsFunction) -> napi_ohos::Result<()> {
    let tsfn: ThreadsafeFunction<u32, ErrorStrategy::Fatal> =
        cb.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;
    let ok = BRIDGE.set(tsfn).is_ok();
    log(&format!("bench C: bridge registered = {ok}"));
    Ok(())
}

/// Called from JS when one bridge crossing has finished its work.
#[napi(js_name = "bridgeDone")]
pub fn bridge_done() {
    let s = signal();
    let mut g = s.count.lock().unwrap();
    *g += 1;
    s.cv.notify_all();
}

/// Redraw after the worker thread has filled in measurement C. Must be called
/// from JS, because only the JS thread may touch the ArkUI tree.
#[napi(js_name = "benchRefresh")]
pub fn bench_refresh() {
    app::rebuild();
}

/// Measurement C. Runs on a worker thread — it blocks waiting for the JS
/// thread, so running it on the JS thread would deadlock instantly.
#[napi(js_name = "runBridgeBench")]
pub fn run_bridge_bench() {
    std::thread::spawn(|| {
        let Some(tsfn) = BRIDGE.get() else {
            crate::log("bench C: no bridge registered, skipping");
            return;
        };
        crate::log("bench C: starting");
        let s = signal();

        // Block until JS has completed `target` crossings in total — but never
        // forever. If the JS side does not acknowledge, this thread used to
        // park permanently and measurement C simply never appeared, with no
        // error anywhere. A bounded wait turns that into a reported failure.
        let wait_until = |target: u64| -> bool {
            let mut g = s.count.lock().unwrap();
            while *g < target {
                let (ng, t) = s
                    .cv
                    .wait_timeout(g, std::time::Duration::from_millis(2000))
                    .unwrap();
                g = ng;
                if t.timed_out() && *g < target {
                    return false;
                }
            }
            true
        };

        // Same widget count both ways, so the two numbers are directly
        // comparable: M crossings of one widget, versus one crossing of M.
        const M: u32 = 200;
        const REPS: usize = 5;
        /// Sentinel: acknowledge without doing any work (the control).
        const EMPTY: u32 = 0xFFFF_FFFE;
        /// Sentinel: hop onto the JS thread purely to redraw.
        const REFRESH: u32 = 0xFFFF_FFFF;

        // One awaited crossing carrying `n` widgets. Awaiting each one is the
        // point: firing them all and waiting once at the end measures pipelined
        // throughput, which is a different and much flatterier number than the
        // round-trip latency an interactive caller actually experiences.
        let crossing = |n: u32| -> Option<f64> {
            let base = *s.count.lock().unwrap();
            let t0 = std::time::Instant::now();
            tsfn.call(n, ThreadsafeFunctionCallMode::Blocking);
            if !wait_until(base + 1) {
                return None;
            }
            Some(t0.elapsed().as_nanos() as f64 / 1000.0)
        };

        let median = |mut v: Vec<f64>| -> f64 {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v[v.len() / 2]
        };

        for _ in 0..8 {
            if crossing(1).is_none() {
                crate::log(
                    "bench C: JS never acknowledged a crossing within 2 s — \
                     measurement C abandoned",
                );
                return;
            }
        }

        // Control: a crossing that carries no work at all, so JS acknowledges
        // it immediately. This is the pure round-trip latency of the boundary,
        // and subtracting it from the others separates "what the bridge costs"
        // from "what the widget costs".
        let collect = |f: &dyn Fn() -> Option<f64>, n: usize| -> Option<Vec<f64>> {
            (0..n).map(|_| f()).collect()
        };
        let Some(empty_v) = collect(&|| crossing(EMPTY), M as usize) else {
            crate::log("bench C: timed out during the control");
            return;
        };
        let empty = median(empty_v);
        crate::log(&format!("bench C control: empty crossing {empty:.1} µs"));

        // Unbatched: one awaited crossing per widget.
        let Some(unbatched_v) = collect(&|| crossing(1), M as usize) else {
            crate::log("bench C: timed out during the unbatched pass");
            return;
        };
        let unbatched = median(unbatched_v);

        // Batched: one awaited crossing carrying all M widgets.
        let Some(batched_v) = collect(&|| crossing(M).map(|v| v / M as f64), REPS) else {
            crate::log("bench C: timed out during the batched pass");
            return;
        };
        let batched = median(batched_v);

        bench::record_bridge(empty, unbatched, batched);

        // Hop back to the JS thread to redraw — only it may touch the tree.
        tsfn.call(REFRESH, ThreadsafeFunctionCallMode::Blocking);
    });
}
