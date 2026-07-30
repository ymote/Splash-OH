//! The JS ↔ Rust bridge, and the native capabilities behind it.
//!
//! This crate is the one ArkTS loads. It owns the napi surface, the web slots,
//! the capability gate and the 45 tools a page can call — and it depends on
//! `splash-oh-native` for the widget tree those pages sit in.
//!
//! # Why the split runs this way
//!
//! The dependency is one-directional and that is the whole reason it was worth
//! separating. Nothing in `splash-oh-native` mentions the bridge, a web slot,
//! an XComponent or ArkWeb. Every card here, by contrast, builds real ArkUI
//! chrome out of that crate's widgets — the browser's tab strip, the file
//! card's roots, the capability dashboard's header are native nodes with a web
//! surface positioned into the hole they leave.
//!
//! So: `splash-oh-native` renders; this crate exposes the phone to a page.
//! One `.so` still comes out, because ArkTS loads exactly one.

pub use splash_oh_native::{app, arkui, bench, catalog, dsl, log, mem, net};

pub mod apps;
pub mod arkweb;
pub mod assets;
pub mod audio;
pub mod bridge;
pub mod caps;
pub mod capture;
pub mod device;
pub mod image;
pub mod location;
pub mod netinfo;
pub mod prefs;
pub mod radio;
pub mod secure;
pub mod sensor;
pub mod webslot;
pub mod xcomp;

use napi_derive_ohos::napi;
use napi_ohos::threadsafe_function::{
    ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi_ohos::{Env, JsFunction, JsObject, NapiRaw};
use splash_oh_native::arkui::NodeContentHandle;
use splash_oh_native::wechat;
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
    // Tell the renderer how to route a tap. It deliberately does not know:
    // splash-oh-native has no idea what an app is, and wiring this here is what
    // keeps the dependency pointing one way.
    app::set_router(|target| {
        if apps::handle(target) {
            apps::build().0
        } else {
            None
        }
    });
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

    // Link the plugins this build ships into the registry. The cdylib is the
    // only place that can: a plugin crate cannot register itself into a library
    // it is not part of. See splash-oh-core's module docs.
    // Before any plugin can answer, it needs somewhere to answer to.
    splash_oh_core::set_reply(bridge::plugin_reply);
    splash_oh_core::with_registry_mut(|r| {
        splash_oh_plugin_demo::register(r);
    });
    log(&format!(
        "plugins: {} tool(s) registered: {:?}",
        splash_oh_core::registered().len(),
        splash_oh_core::registered()
    ));
    log(&splash_oh_core::self_test());
    log(&caps::self_test());
    assets::self_test();
    app::init(slot);
    // Hand the capability registry to the renderer's DSL. `platform_channels`
    // and `pedometer` are not "no analogue" samples on this stack — a script
    // calling the platform and getting a typed answer back is exactly what a
    // MethodChannel is, and the bridge already carries ~45 of these for web
    // pages. This exposes the same registry to the Splash DSL.
    // A `web` node in the DSL reserves space and asks for a real WebView there.
    // The flutter catalog is built in Rust, so it has to be told how wide the
    // page is — it cannot size in percentages the way the DSL kit did. `ui::W`
    // is 402, the reference width the benchmark apps were drawn against, and on
    // a 440vp display that left a bare strip down the right edge of every
    // screen.
    splash_oh_native::flutter::set_page_width(device::width_vp());
    splash_oh_native::set_web_reset(webslot::reset);
    splash_oh_native::set_web_declare(|src, x, y, w, h| {
        if let Some(path) = src.strip_prefix("app:") {
            webslot::declare_app(path, x, y, w, h)
        } else {
            webslot::declare(src, x, y, w, h)
        }
    });
    splash_oh_native::set_host_invoke(|tool| match tool {
        "device.info" => device::info(0),
        "device.display" => device::display(),
        "device.battery" => device::battery(),
        "device.time" => device::time(),
        "device.notifications" => device::notifications_enabled(),
        // compass_app: the sample is a travel planner, and the one thing a
        // travel planner on a phone can know that a mock cannot is where the
        // phone is. `enabled` is a cheap system-switch read; `fix` is the
        // cached position, refreshed on a worker, because a real request takes
        // seconds and this runs while the tree is being built.
        "location.enabled" => location::enabled_word(),
        "location.fix" => location::cached(),
        "location.state" => location::cached_state(),
        "sensor.list" => sensor::list().unwrap_or_else(|e| e),
        // 0 is ARKUI/OH's accelerometer id; the pedometer sample reads step
        // data, which is the same shape of platform call.
        "sensor.accelerometer" => sensor::sample(0, 400).unwrap_or_else(|e| e),
        "sensor.steps" => sensor::sample(sensor::type_from_name("pedometer").unwrap_or(266), 400)
            .unwrap_or_else(|e| e),
        // asset_transformation: the splash:// protocol resolves a request URL
        // to bytes at serve time, which is this stack's asset pipeline.
        "assets.shim" => {
            let a = assets::get("splash://app/__splash.js");
            format!(
                "{{\"path\":\"/__splash.js\",\"mime\":\"{}\",\"status\":{},\"bytes\":{}}}",
                a.mime,
                a.status,
                a.body.len()
            )
        }
        "assets.index" => {
            let a = assets::get("splash://app/index.html");
            format!(
                "{{\"path\":\"/index.html\",\"mime\":\"{}\",\"status\":{},\"bytes\":{}}}",
                a.mime,
                a.status,
                a.body.len()
            )
        }
        "assets.missing" => {
            let a = assets::get("splash://app/nope.bin");
            format!(
                "{{\"path\":\"/nope.bin\",\"mime\":\"{}\",\"status\":{},\"bytes\":{}}}",
                a.mime,
                a.status,
                a.body.len()
            )
        }
        // add_to_app: facts about this very embed.
        "embed.nodes" => format!("{}", splash_oh_native::ui::last_total()),
        // platform_view_swift: a real native rendering surface composited into
        // the same tree, which is what that sample is about.
        "surface.state" => xcomp::state(),
        "embed.shape" => "ArkTS hands over one NodeContent at startup; every \
node after that is created, configured, laid out and event-wired from Rust"
            .to_string(),
        // background_isolate_channels: a call that does its work off the UI
        // thread and returns when it is done.
        "thread.offmain" => {
            let t0 = std::time::Instant::now();
            let r = sensor::sample(0, 250).unwrap_or_else(|e| e);
            format!(
                "{{\"blocked_ui_ms\":{},\"answer\":{}}}",
                t0.elapsed().as_millis(),
                r.len()
            )
        }
        other => format!("no such tool: {other}"),
    });
    Ok(())
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
    let node = splash_oh_native::dsl::build_weather();
    app::set_root(node);
    vec![0.0, 0.0]
}

/// Web surfaces the current tree declared, as "id|url|x|y|w|h".
///
/// ArkTS positions a real `Web` component at each of these, above the
/// `ContentSlot`. There is no `ARKUI_NODE_WEB`, so this is the only way a
/// Splash tree can contain a webview -- see `webslot.rs`.
/// One asset from the shipped bundle, for the `splash://` scheme handler.
///
/// ArkTS owns `customizeSchemes`/`setWebSchemeHandler` because those are ArkTS
/// APIs; it owns none of the policy. It asks here for a URL and streams back
/// whatever comes out, including the 404 -- a custom scheme has no default
/// resolver, so a request this declines to answer is not a miss, it is a hang.
#[napi(js_name = "assetGet")]
pub fn asset_get(url: String) -> AssetReply {
    let a = assets::get(&url);
    AssetReply {
        mime: a.mime.to_string(),
        status: a.status as u32,
        body: a.body.into(),
    }
}

/// What `assetGet` hands back. `body` crosses as a napi Buffer, which ArkTS
/// receives as a Uint8Array -- see Index.ets for why the ArrayBuffer behind it
/// has to be sliced to the view rather than passed whole.
#[napi(object)]
pub struct AssetReply {
    pub mime: String,
    pub status: u32,
    pub body: napi_ohos::bindgen_prelude::Buffer,
}

/// Report where a slot's document actually came from.
///
/// Called by ArkTS when a page starts loading in a slot. Trust used to be a
/// property of the slot alone, which a navigation could invalidate without
/// anything noticing; this is how Rust finds out.
#[napi(js_name = "slotOrigin")]
pub fn slot_origin(slot: u32, origin: String) {
    webslot::set_observed_origin(slot, &origin);
}

/// The origin a slot is allowed to be on, or "" if it is an untrusted URL slot.
///
/// ArkTS asks so it can refuse the navigation outright rather than let the
/// document load and be disowned afterwards. Both checks exist on purpose: this
/// one stops the page arriving, and `is_trusted` stops it being believed if it
/// does.
#[napi(js_name = "slotExpectedOrigin")]
pub fn slot_expected_origin(slot: u32) -> String {
    webslot::expected_origin(slot).unwrap_or_default()
}

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

/// Picker requests a page has made, as `reqId|mode` where mode is
/// `folder` or `file`.
///
/// The one tool that runs the other way. `fs.list` established that user
/// storage is absent from the app's mount namespace rather than merely denied,
/// so the system picker — ArkTS-only, needing a UIAbilityContext — is the only
/// route to a user's own directories. Rust parks the call and ArkTS answers it.
#[napi(js_name = "pickDrain")]
pub fn pick_drain() -> Vec<String> {
    bridge::pick_drain()
}

/// ArkTS reporting a picker outcome. `payload` is a JSON array of
/// `{uri, path, name}` when `success`, otherwise a message.
///
/// `req_id` is a string because it crosses into JS, where a u64 past 2^53 would
/// come back rounded — the same reason call ids are.
#[napi(js_name = "pickResolve")]
pub fn pick_resolve(req_id: String, success: bool, payload: String) {
    bridge::pick_resolve(&req_id, success, payload);
}

/// Pushes bridge traffic into ArkTS the moment it exists.
///
/// Everything used to reach a page through a 250 ms poll, which put up to a
/// quarter second on every call regardless of how long the work took, and made
/// Rust -> page communication impossible: a page could only ever be told
/// something as the answer to a question it had asked.
///
/// A threadsafe function removes both limits at once. It is callable from any
/// worker, so a reply goes out when the work finishes, and an event can be sent
/// with no call outstanding at all.
static PUSH: OnceLock<ThreadsafeFunction<String, ErrorStrategy::Fatal>> = OnceLock::new();

/// Hand Rust the callback it delivers on. Called once at startup.
#[napi(js_name = "setPush")]
pub fn set_push(cb: JsFunction) -> napi_ohos::Result<()> {
    let tsfn: ThreadsafeFunction<String, ErrorStrategy::Fatal> =
        cb.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;
    let _ = PUSH.set(tsfn);
    bridge::set_pusher(push_line);
    Ok(())
}

/// The function `bridge` calls to deliver one line. Separate from PUSH so the
/// bridge module does not need the napi types.
fn push_line(line: String) -> bool {
    match PUSH.get() {
        Some(tsfn) => {
            // NonBlocking: a worker delivering a reply must not wait on the JS
            // thread's queue. Dropping under extreme pressure is better than
            // stalling the thread that produced the value.
            tsfn.call(line, ThreadsafeFunctionCallMode::NonBlocking);
            true
        }
        None => false,
    }
}

/// Slots whose page has reported its script running, as ids.
///
/// The only reliable "it rendered" signal: `loadData` returns without throwing
/// when it has silently painted nothing, and `onPageEnd` fires for the blank
/// origin a generated slot starts on, so neither distinguishes a live page from
/// a white one.
#[napi(js_name = "webSlotsPainted")]
pub fn web_slots_painted() -> Vec<String> {
    bridge::painted_drain()
}

/// True once, if background data landed and the current app should redraw.
/// Polled by ArkTS alongside the web-slot list.
#[napi(js_name = "appTakeDirty")]
pub fn app_take_dirty() -> u32 {
    // `|` not `||`: both flags have to be cleared, or whichever is checked
    // second stays set and every poll after this one reports dirty forever.
    if apps::weather_web::take_dirty()
        | location::take_dirty()
        | splash_oh_native::wonders::met::take_dirty()
    {
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
    let node = splash_oh_native::dsl::build_youtube();
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
    use splash_oh_native::{taobao, tiktok, wonderous};
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
///
/// This *resets* navigation — it is the "show me app X from the top" call. For
/// redrawing what is already there, use [`app_rerender`].
#[napi(js_name = "appRender")]
pub fn app_render(app: String) -> Vec<f64> {
    apps::set_app(apps::App::from_id(&app));
    app::set_wechat_active(true);
    let (node, n, us) = apps::build();
    app::set_root(node);
    vec![n as f64, us]
}

/// Mount one catalog screen by name. `""` is the index.
///
/// A verification affordance rather than a feature: the catalog's 28 screens
/// sit behind a scrolling index, and tapping through it to look at each one is
/// slow and misses the rows below the fold. This drives them directly, so a
/// screenshot sweep can cover all of them. `CATALOG_WALK_MS` in `Index.ets`
/// turns that sweep on.
///
/// An unknown name lands on the index rather than failing — the caller is a
/// capture script, and a silent no-op there would be harder to spot than a
/// screen that visibly is not the one asked for.
/// Handle the system back gesture: pop one level of the route.
///
/// The kit's routes are hierarchical strings — `cupertino_gallery/button` sits
/// under `cupertino_gallery`, which sits under the index — so "back" is just
/// dropping the last segment. Returns false at the index so the platform does
/// its own thing (leaving the app), which is what a back gesture should do
/// there.
///
/// Without this, ArkTS never implemented `onBackPress`, so the system back
/// gesture closed the app from any screen rather than navigating.
#[napi(js_name = "goBack")]
pub fn go_back() -> bool {
    let screen = app::current_screen();
    let cur = if screen.is_empty() {
        "index"
    } else {
        screen.as_str()
    };
    if cur == "index" {
        return false;
    }
    let parent = match cur.rfind('/') {
        Some(i) => cur[..i].to_string(),
        None => "index".to_string(),
    };
    app::set_screen_quiet(parent.clone());
    let node = splash_oh_native::dsl::build_flutter(&parent, false);
    app::set_root(node);
    true
}

/// One animation frame: re-mount only if the current screen animates.
///
/// ArkTS calls this on a short interval. It is a no-op on every static screen,
/// which is all of them but the animation demos.
#[napi(js_name = "animTick")]
pub fn anim_tick() -> bool {
    if !app::is_animating() {
        return false;
    }
    app::rebuild();
    true
}

#[napi(js_name = "catalogScreen")]
pub fn catalog_screen(name: String) -> u32 {
    apps::set_app(apps::App::Catalog);
    app::set_wechat_active(true);
    // The flutter kit routes by *string*, so the name passes straight through
    // rather than being looked up in CATALOG_SCREENS. Empty means the index.
    let route = if name.is_empty() {
        "index"
    } else {
        name.as_str()
    };
    // Record it, or the animation tick cannot tell what is on screen.
    app::set_screen_quiet(route.to_string());
    let node = splash_oh_native::dsl::build_flutter(route, false);
    let n = splash_oh_native::ui::count();
    splash_oh_native::ui::record_total(n);
    app::set_root(node);
    n as u32
}

/// Redraw whatever is on screen, leaving navigation alone. Returns [nodes, µs].
///
/// The dirty path used `appRender`, which goes through `set_app` and so resets
/// tab, sub, pushed and feed to zero. Any background data landing therefore
/// threw the user back to the first tab — and because the data that lands is
/// usually the data the tap asked for, **a tap appeared to do nothing at all**:
/// it registered, changed the tab, started a fetch, and the fetch's own arrival
/// undid it a moment later.
#[napi(js_name = "appRerender")]
pub fn app_rerender() -> Vec<f64> {
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
