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
pub mod apps;
pub mod arkui;
pub mod bench;
pub mod bridge;
pub mod catalog;
pub mod dsl;
pub mod mem;
pub mod net;
pub mod webslot;
pub mod wechat;

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

// ---------------------------------------------------------------------------
// Memory harness. Both sides read the same RSS counter in the same process, so
// there is nothing to normalise between them.
// ---------------------------------------------------------------------------

/// Resident set size, KiB.
#[napi(js_name = "rssKb")]
pub fn rss_kb() -> u32 {
    mem::rss_kb() as u32
}

/// Peak resident set size, KiB — what an OOM kill would have been judged on.
#[napi(js_name = "peakRssKb")]
pub fn peak_rss_kb() -> u32 {
    mem::peak_rss_kb() as u32
}

/// Build `n` more nodes through the NDK and hold them. Returns the total held.
#[napi(js_name = "memHold")]
pub fn mem_hold(n: u32) -> u32 {
    mem::hold(n as usize) as u32
}

/// Drop every held node. Returns how many were dropped.
#[napi(js_name = "memRelease")]
pub fn mem_release() -> u32 {
    mem::release() as u32
}

/// Log one labelled sample, so the whole ramp lands in `hilog` in order.
#[napi(js_name = "memLog")]
pub fn mem_log(label: String, held: u32) {
    log(&format!(
        "mem {label}: held={held} rss={} kB peak={} kB",
        mem::rss_kb(),
        mem::peak_rss_kb()
    ));
}

// ---------------------------------------------------------------------------
// The WeChat demo. See `wechat/mod.rs`.
// ---------------------------------------------------------------------------

/// Build the app for its current navigation state and mount it. Returns
/// [nodes built, µs].
#[napi(js_name = "wechatRender")]
pub fn wechat_render() -> Vec<f64> {
    // DEMO: the ArkTS entry page (Index.ets) hardwires this call as its mount
    // path. Show the LLM-generated weather card (native ArkUI, evaluated from
    // the Splash DSL in assets/weather.splash) instead of the WeChat benchmark.
    let node = crate::dsl::build_weather();
    app::set_root(node);
    vec![0.0, 0.0]
}

/// Web surfaces the current tree declared, as "id|url|x|y|w|h".
///
/// ArkTS positions a real `Web` component at each of these, above the
/// `ContentSlot`. There is no `ARKUI_NODE_WEB`, so this is the only way a
/// Splash tree can contain a webview -- see `webslot.rs`.
#[napi(js_name = "webSlots")]
pub fn web_slots() -> Vec<String> {
    webslot::encoded()
}

/// Evaluate a tiny script through the Splash VM and report what came back.
///
/// A smoke test for the VM itself, not for any app. The four ported apps build
/// their trees in Rust and never touch the interpreter, so a broken or swapped
/// VM would not show up in them at all -- it would only surface later, in a
/// `.splash` card, as a blank screen. Worth one call to rule out.
#[napi(js_name = "dslSelfTest")]
pub fn dsl_self_test() -> String {
    let src = r#"
fn argb(a, r, g, b) { return ((a * 256 + r) * 256 + g) * 256 + b }
let rows = []
for i in [1, 2, 3] { rows.push({t: "text", text: "row " + i, size: 14, w: 200, h: 20}) }
{t: "column", w: 200, h: 80, bg: argb(255, 20, 20, 30), c: rows}
"#;
    match dsl::build(src) {
        Some(_) => {
            let msg = "dsl selftest: ok (fn, for, array push, string+number, node tree)";
            log(msg);
            msg.to_string()
        }
        None => {
            let msg = "dsl selftest: FAILED — the VM did not produce a tree";
            log(msg);
            msg.to_string()
        }
    }
}

/// A page called `splash.invoke(tool, args)`. Fire-and-forget; the answer
/// comes back through `webBridgeDrain`.
#[napi(js_name = "webBridgeInvoke")]
pub fn web_bridge_invoke(slot: u32, call_id: String, tool: String, args: String) {
    bridge::invoke(slot, call_id, tool, args);
}

/// Finished calls, as `slot|callId|payloadJson`. ArkTS evaluates each back into
/// its page with `splash._resolve`.
#[napi(js_name = "webBridgeDrain")]
pub fn web_bridge_drain() -> Vec<String> {
    bridge::drain()
}

/// True once, if background data landed and the current app should redraw.
/// Polled by ArkTS alongside the web-slot list.
#[napi(js_name = "appTakeDirty")]
pub fn app_take_dirty() -> u32 {
    if apps::weather_web::take_dirty() {
        1
    } else {
        0
    }
}

/// The markup for a generated slot. Fetched once by id rather than carried in
/// the slot list, which ArkTS polls.
#[napi(js_name = "webSlotHtml")]
pub fn web_slot_html(id: u32) -> String {
    webslot::html_for(id)
}

/// Mount the YouTube app's native chrome bar into the slot. The video content
/// itself is an ArkTS `Web` component in Index.ets — OpenHarmony's WebView
/// cannot live in the native ArkUI node tree, so Splash-OH renders the app
/// shell and the Web is laid out beneath it.
#[napi(js_name = "youtubeRender")]
pub fn youtube_render() {
    let node = crate::dsl::build_youtube();
    app::set_root(node);
}

/// Build every screen once and keep them all alive, for the memory arm.
#[napi(js_name = "wechatKeepAll")]
pub fn wechat_keep_all() -> u32 {
    wechat::keep_all() as u32
}

/// Data the ArkTS twins render, so both sides draw the same content.
/// Keyed as "<app>.<what>"; each entry is pipe-separated fields.
#[napi(js_name = "appData")]
pub fn app_data(key: String) -> Vec<String> {
    use apps::{taobao, tiktok, wonderous};
    match key.as_str() {
        "taobao.products" => taobao::PRODUCTS
            .iter()
            .map(|(a, b, c, d)| format!("{a}|{b}|{c}|{d}"))
            .collect(),
        "taobao.tabs" => taobao::TABS
            .iter()
            .map(|(a, b)| format!("{a}|{b}"))
            .collect(),
        "tiktok.reels" => tiktok::REELS
            .iter()
            .map(|(a, b, c, d, e)| format!("{a}|{b}|{c}|{d}|{e}"))
            .collect(),
        "wonderous.wonders" => wonderous::WONDERS
            .iter()
            .map(|(a, b, c)| format!("{a}|{b}|{c}"))
            .collect(),
        "wonderous.tabs" => wonderous::TABS
            .iter()
            .map(|(a, b)| format!("{a}|{b}"))
            .collect(),
        "wonderous.sections" => wonderous::sections()
            .iter()
            .map(|(a, b)| format!("{a}|{b}"))
            .collect(),
        "wonderous.artifacts" => wonderous::artifacts_list()
            .iter()
            .map(|s| s.to_string())
            .collect(),
        "wonderous.timeline" => wonderous::timeline_list()
            .iter()
            .map(|(a, b)| format!("{a}|{b}"))
            .collect(),
        _ => Vec::new(),
    }
}

/// Switch which app is on screen and mount it. Returns [nodes, µs].
#[napi(js_name = "appRender")]
pub fn app_render(app: String) -> Vec<f64> {
    apps::set_app(apps::App::from_id(&app));
    app::set_wechat_active(true);
    let (node, n, us) = apps::build();
    app::set_root(node);
    vec![n as f64, us]
}

/// Build one route of one app without mounting it. Returns [nodes, µs].
#[napi(js_name = "appTime")]
pub fn app_time(app: String, tab: u32, route: String) -> Vec<f64> {
    let (n, us) = apps::build_route(apps::App::from_id(&app), tab as usize, &route);
    vec![n as f64, us]
}

/// The tour for an app, as "tab|route" entries.
#[napi(js_name = "appTour")]
pub fn app_tour(app: String) -> Vec<String> {
    apps::App::from_id(&app)
        .tour()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Build every screen of an app and keep them, for the memory arm.
#[napi(js_name = "appKeepAll")]
pub fn app_keep_all(app: String) -> u32 {
    apps::keep_all(apps::App::from_id(&app)) as u32
}

#[napi(js_name = "appDropKept")]
pub fn app_drop_kept() -> u32 {
    apps::drop_kept() as u32
}

/// Whether the native header toggle was tapped since the last call. Polled by
/// ArkTS, which then swaps which implementation owns the surface.
#[napi(js_name = "wechatTakeToggle")]
pub fn wechat_take_toggle() -> u32 {
    if wechat::take_toggle() {
        1
    } else {
        0
    }
}

/// Detach the native tree and give up the surface, so the ArkTS
/// implementation can mount its own into the same slot.
#[napi(js_name = "wechatDetach")]
pub fn wechat_detach() {
    app::set_wechat_active(false);
    app::detach_root();
}

/// Feed a click id in and re-render if it changed the navigation state.
/// Returns 1 if it did.
#[napi(js_name = "wechatClick")]
pub fn wechat_click(target: i32) -> u32 {
    if wechat::handle(target) {
        let (node, _, _) = wechat::build();
        app::set_root(node);
        1
    } else {
        0
    }
}

/// Build one route without mounting it, for timing. Returns [nodes, µs].
#[napi(js_name = "wechatTime")]
pub fn wechat_time(tab: u32, route: String, chat_id: u32) -> Vec<f64> {
    let r = match route.as_str() {
        "chat" => wechat::Route::Chat(chat_id as u64),
        "moments" => wechat::Route::Moments,
        "addcontact" => wechat::Route::AddContact,
        "myprofile" => wechat::Route::MyProfile,
        _ => wechat::Route::Root,
    };
    let (n, us) = wechat::build_timed(tab as usize, r);
    vec![n as f64, us]
}

/// The data the ArkTS implementation renders, so both draw the same content.
#[napi(js_name = "wechatChats")]
pub fn wechat_chats() -> Vec<String> {
    wechat::db::CHATS
        .iter()
        .map(|c| {
            format!(
                "{}|{}|{}|{}",
                c.username,
                c.preview.text(),
                c.timestamp,
                c.avatar
            )
        })
        .collect()
}

/// Asset lists the ArkTS implementation needs, so both render the same icons.
/// Each entry is "label|file".
#[napi(js_name = "wechatAssets")]
pub fn wechat_assets(which: String) -> Vec<String> {
    let pairs: &[(&str, &str)] = match which.as_str() {
        "tabs" => wechat::db::TAB_ICONS,
        "discover" => wechat::db::DISCOVER,
        "profile" => wechat::db::PROFILE,
        "contacts" => wechat::db::CONTACT_ACTIONS,
        _ => &[],
    };
    pairs.iter().map(|(a, b)| format!("{a}|{b}")).collect()
}

/// Contact names paired with the avatar each should use.
#[napi(js_name = "wechatContacts")]
pub fn wechat_contacts() -> Vec<String> {
    const POOL: &[&str] = &[
        "user1.png",
        "user2.png",
        "user3.png",
        "user4.png",
        "user5.png",
        "user6.png",
    ];
    let mut out = Vec::new();
    let mut k = 0usize;
    for (initial, names) in wechat::db::CONTACT_GROUPS {
        out.push(format!("#|{initial}"));
        for n in *names {
            out.push(format!("{n}|{}", POOL[k % POOL.len()]));
            k += 1;
        }
    }
    out
}

/// Moments feed as "author|body|avatar|photo".
#[napi(js_name = "wechatMoments")]
pub fn wechat_moments() -> Vec<String> {
    wechat::db::MOMENTS
        .iter()
        .map(|(a, b, av, p)| format!("{a}|{b}|{av}|{p}"))
        .collect()
}

/// Messages for a chat, as "direction|text|avatar".
#[napi(js_name = "wechatMessages")]
pub fn wechat_messages(chat_id: u32) -> Vec<String> {
    (0..wechat::db::MESSAGES_PER_CHAT)
        .map(|i| {
            let m = wechat::db::message(chat_id as u64, i);
            let d = if matches!(m.direction, wechat::db::Direction::Outgoing) {
                "o"
            } else {
                "i"
            };
            let av = if matches!(m.direction, wechat::db::Direction::Outgoing) {
                wechat::db::MY_AVATAR
            } else {
                wechat::db::peer_avatar(chat_id as u64)
            };
            format!("{d}|{}|{av}", m.text)
        })
        .collect()
}

/// One line of results, so everything lands in `hilog` in order.
#[napi(js_name = "wechatLog")]
pub fn wechat_log(line: String) {
    log(&format!("wechat {line}"));
}

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
                let (ng, t) =
                    s.cv.wait_timeout(g, std::time::Duration::from_millis(2000))
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
