//! A Tauri-style JS → Rust bridge for web surfaces.
//!
//! Ported from the substrate in octos-one (`makepad.ets` +
//! `widgets/src/web_card.rs`), which reached this design first. A page inside a
//! Splash web slot calls:
//!
//! ```js
//! splash.invoke('device.info').then(info => …)
//! ```
//!
//! and Rust answers. That makes a webview a real part of the app rather than a
//! sealed rectangle: the page can use native capability it could never reach
//! from a browser sandbox, and — more usefully here — it can get its data from
//! Rust instead of opening a second, invisible network connection of its own.
//!
//! # Why it is shaped like this
//!
//! ArkWeb's `javaScriptProxy` can only expose **synchronous, void** methods. It
//! cannot return a promise and it cannot return a value asynchronously. So the
//! call is split in two, exactly as octos-one does it:
//!
//! 1. JS calls `splash_native.invoke(callId, tool, argsJson)` and gets nothing
//!    back. The shim parks a promise under `callId`.
//! 2. Rust does the work — on a worker thread, never on the caller's — and
//!    queues a result.
//! 3. ArkTS drains the queue and evaluates
//!    `splash._resolve(callId, payload)` back into the page, settling it.
//!
//! # Call ids are strings
//!
//! They are `u64` here and JS numbers are `f64`, so anything past 2^53 would
//! arrive rounded. octos-one learned this one the hard way; the ids are
//! stringified on both sides.

use crate::net;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Caps for anything a page hands to the VM.
///
/// A page is the least trusted thing in the process. `splash-core` is built to
/// be embedded under limits rather than trusted, so use them: a script that
/// loops forever, allocates without bound, or returns a megabyte of JSON is
/// stopped by the runtime instead of taking the thread with it.
///
/// # These are set explicitly, and tighter than the defaults
///
/// `Runtime::new` is not unbounded — I had implied it was, and it is not.
/// It applies `ExecutionLimits::default()`, which is already conservative:
/// 256 KB of source, an 8 MB script heap, 200k instructions, and 32/64 ms
/// soft/hard timeouts. A runaway script was never going to hang the process.
///
/// What the defaults are not is *calibrated to this caller*. They are sized for
/// a trusted script an app ships. The scripts arriving here come from a web
/// page, and the work they do is arithmetic over a handful of numbers — a
/// weather card finding the highest of six temperatures. Nothing legitimate
/// needs 200,000 instructions or a megabyte of heap, so the budget is cut to
/// what the real workload uses with room to spare. A limit set where the
/// legitimate work actually sits is the one that means something.
const EVAL_MAX_SOURCE: usize = 64 * 1024;
const JSON_MAX_BYTES: usize = 256 * 1024;
const JSON_MAX_DEPTH: usize = 32;

/// A finished call, waiting to be evaluated back into its page.
pub struct Reply {
    pub slot: u32,
    pub call_id: String,
    /// Already-encoded JSON: `{"ok":true,"data":…}` or `{"ok":false,"error":…}`.
    pub payload: String,
}

static REPLIES: Mutex<VecDeque<Reply>> = Mutex::new(VecDeque::new());

/// Calls that have been dispatched and not yet answered, as
/// (slot, call_id, deadline). Without this a wedged worker leaves a promise
/// pending for the life of the page, with nothing on the JS side able to
/// notice -- the page cannot time out a call it has no handle on.
static PENDING: Mutex<Vec<(u32, String, std::time::Instant)>> = Mutex::new(Vec::new());

/// How long any one call may take before the bridge answers for it.
///
/// Generous, because it is a backstop rather than a policy: the slowest
/// legitimate call is a cold location fix, which is allowed 30 s of its own.
const CALL_DEADLINE: std::time::Duration = std::time::Duration::from_secs(45);

fn track(slot: u32, call_id: &str) {
    if let Ok(mut p) = PENDING.lock() {
        p.push((
            slot,
            call_id.to_string(),
            std::time::Instant::now() + CALL_DEADLINE,
        ));
    }
}

fn untrack(slot: u32, call_id: &str) {
    if let Ok(mut p) = PENDING.lock() {
        p.retain(|(s, c, _)| !(*s == slot && c == call_id));
    }
}

/// Settle anything past its deadline. Driven from `drain`, which ArkTS already
/// polls -- no extra timer, and it runs on the thread that delivers replies.
fn expire_stale() {
    let now = std::time::Instant::now();
    let expired: Vec<(u32, String)> = match PENDING.lock() {
        Ok(mut p) => {
            let (dead, alive): (Vec<_>, Vec<_>) = p.drain(..).partition(|(_, _, d)| *d <= now);
            *p = alive;
            dead.into_iter().map(|(s, c, _)| (s, c)).collect()
        }
        Err(_) => Vec::new(),
    };
    for (slot, call_id) in expired {
        crate::log(&format!("bridge: call {call_id} on slot {slot} timed out"));
        // Straight onto the queue: reply() would re-enter the tracking that
        // just removed this entry.
        if let Ok(mut q) = REPLIES.lock() {
            q.push_back(Reply {
                slot,
                call_id,
                payload: err("timed out waiting for the native side"),
            });
        }
    }
}

/// Slots whose page has reported its script running. ArkTS drains this to know
/// which loads actually took, rather than inferring it from signals that cannot
/// tell a blank surface from a rendered one.
static PAINTED: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// Slots that have reported ready since the last call.
pub fn painted_drain() -> Vec<String> {
    match PAINTED.lock() {
        Ok(mut v) => v.drain(..).map(|s| s.to_string()).collect(),
        Err(_) => Vec::new(),
    }
}

fn reply(slot: u32, call_id: String, payload: String) {
    untrack(slot, &call_id);
    if let Ok(mut q) = REPLIES.lock() {
        // A page that stops draining should not grow this without bound.
        if q.len() > 256 {
            q.pop_front();
        }
        q.push_back(Reply {
            slot,
            call_id,
            payload,
        });
    }
}

fn ok(data: String) -> String {
    format!("{{\"ok\":true,\"data\":{data}}}")
}

fn err(msg: &str) -> String {
    format!("{{\"ok\":false,\"error\":{}}}", json_str(msg))
}

/// Minimal JSON string escaping. Enough for the values this bridge returns;
/// anything richer should be built by a real encoder.
pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Handle a call from a page. Returns immediately; the answer arrives via
/// [`drain`].
///
/// Anything that can block runs on a worker. The caller here is ArkTS's JS
/// thread, and blocking it is what `net::mark_ui_thread` exists to catch.
pub fn invoke(slot: u32, call_id: String, tool: String, args: String) {
    // Tracked before dispatch, so a tool that never replies is still answered.
    // slot.ready is fire-and-forget by design and has no promise waiting.
    if tool != "slot.ready" {
        track(slot, &call_id);
    }
    // The capability gate, enforced here rather than in ArkTS.
    //
    // ArkTS already declines to attach the proxy to an untrusted slot, but that
    // check lives next to the untrusted content and is therefore the wrong
    // place to rely on. octos-one puts it the same way: a JS-side check is
    // bypassable, so the trusted Rust side is what enforces it.
    if !crate::webslot::is_trusted(slot) {
        crate::log(&format!(
            "bridge: refused {tool} from untrusted slot {slot}"
        ));
        reply(
            slot,
            call_id,
            err("this surface is not permitted to call native tools"),
        );
        return;
    }

    match tool.as_str() {
        // A page reporting that its script is running. See SHIM: this is the
        // only signal that a generated page actually rendered, because
        // loadData succeeding and onPageEnd firing both happen when it did not.
        "slot.ready" => {
            if let Ok(mut v) = PAINTED.lock() {
                if !v.contains(&slot) {
                    v.push(slot);
                }
            }
        }

        // Round-trip check, so a page can prove the bridge is live.
        "echo" => reply(slot, call_id, ok(json_str(&args))),

        // Real device facts from libdeviceinfo_ndk, not a hardcoded string.
        // A page in a browser gets a user-agent; a page here can ask what the
        // phone actually is.
        "device.info" => reply(slot, call_id, ok(crate::device::info(slot))),

        // How the panel is really configured — size in px, density, refresh
        // rate, rotation. `devicePixelRatio` is the only part of this a web
        // page can normally see, and it is a rounded approximation of one field.
        "device.display" => reply(slot, call_id, ok(crate::device::display())),

        "device.battery" => reply(slot, call_id, ok(crate::device::battery())),

        // The zone is the point: Intl in a page reports what the webview
        // was configured with, not necessarily what the user set.
        "device.time" => reply(slot, call_id, ok(crate::device::time())),

        // Whether posting would be seen. Read-only -- the native kit
        // exposes this one call, so a page can ask and cannot post.
        "device.notifications" => reply(slot, call_id, ok(crate::device::notifications_enabled())),

        // What the phone can actually sense. Enumeration needs no permission,
        // so this works even where reading an individual sensor would not.
        "sensor.list" => reply(
            slot,
            call_id,
            match crate::sensor::list() {
                Ok(j) => ok(j),
                Err(e) => err(&e),
            },
        ),

        // One reading. Blocks until the sensor service ticks, so it runs on a
        // worker -- see sensor::sample.
        //
        // Args: a bare kind ("accelerometer"), or {"kind": "...", "timeoutMs": n}.
        "sensor.read" => {
            std::thread::spawn(move || {
                let (name, timeout) = if args.starts_with('{') {
                    let v: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
                    (
                        v.get("kind")
                            .and_then(|x| x.as_str())
                            .unwrap_or("accelerometer")
                            .to_string(),
                        v.get("timeoutMs").and_then(|x| x.as_u64()).unwrap_or(1500),
                    )
                } else {
                    (args.trim_matches('"').to_string(), 1500)
                };
                let payload = match crate::sensor::type_from_name(&name) {
                    Some(k) => match crate::sensor::sample(k, timeout.min(5000)) {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    },
                    None => err(&format!("unknown sensor: {name}")),
                };
                reply(slot, call_id, payload);
            });
        }

        // Can an app read the framebuffer? On Android the equivalent needs
        // READ_FRAME_BUFFER, which the shell user has and an app does not.
        // Whether OpenHarmony draws the line in the same place is a question
        // worth measuring rather than assuming -- so this reports the OS's own
        // refusal when it refuses. See capture.rs for why it returns a
        // description and an average colour rather than the frame.
        "screen.capture" => {
            std::thread::spawn(move || {
                reply(
                    slot,
                    call_id,
                    match crate::capture::screen() {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    },
                );
            });
        }

        // What an image is, from its header alone. Cheap: learning that a
        // 12 MP HEIF was picked costs a header read, and a page often needs
        // only that.
        "image.info" => {
            std::thread::spawn(move || {
                let v: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
                let path = v
                    .get("path")
                    .and_then(|x| x.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| args.trim_matches('"').to_string());
                reply(
                    slot,
                    call_id,
                    match crate::image::info(&path) {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    },
                );
            });
        }

        // Decode small, re-encode as JPEG, return a data: URI.
        //
        // This is what makes fs.pick worth having on a picture. A phone photo
        // is several megabytes and everything here crosses as one JSON string
        // that is then evaluated into the page -- a full-size image is the one
        // payload this channel genuinely cannot carry. Decoding at a reduced
        // size means the full bitmap is never materialised at all.
        //
        // Args: {"path": "...", "maxEdge": n, "quality": n}
        "image.thumbnail" => {
            std::thread::spawn(move || {
                let v: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
                let path = v
                    .get("path")
                    .and_then(|x| x.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| args.trim_matches('"').to_string());
                let edge = v.get("maxEdge").and_then(|x| x.as_u64()).unwrap_or(320) as u32;
                let q = v.get("quality").and_then(|x| x.as_u64()).unwrap_or(80) as u32;
                reply(
                    slot,
                    call_id,
                    match crate::image::thumbnail(&path, edge, q) {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    },
                );
            });
        }

        // Whether a Rust-created XComponent produced a real surface. The
        // question behind it is whether a camera preview needs ArkTS at all --
        // it does not, because ARKUI_NODE_XCOMPONENT exists where
        // ARKUI_NODE_WEB does not.
        "surface.state" => reply(slot, call_id, ok(crate::xcomp::state())),

        // Start a live preview into the surface Rust built. No ArkTS on the
        // path: the surface is an ARKUI_NODE_XCOMPONENT in the same native
        // tree, and the camera writes frames straight into it.
        //
        // Args: {"front": bool} — the surface id comes from xcomp, not the
        // page, so a page cannot aim the camera at someone else's surface.
        "camera.preview" => {
            std::thread::spawn(move || {
                let v: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
                let front = v.get("front").and_then(|x| x.as_bool()).unwrap_or(false);
                let st = crate::xcomp::surface_state();
                let payload = if !st.created || st.surface_id == 0 {
                    err("no native surface yet")
                } else {
                    match crate::image::preview_start(
                        st.surface_id,
                        st.width as u32,
                        st.height as u32,
                        front,
                    ) {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    }
                };
                reply(slot, call_id, payload);
            });
        }

        "camera.stop" => {
            std::thread::spawn(move || {
                reply(
                    slot,
                    call_id,
                    match crate::image::preview_stop() {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    },
                );
            });
        }

        // What cameras exist. Enumeration only -- capture is a session pipeline
        // needing ohos.permission.CAMERA, and knowing what the hardware is is
        // worth having before anything asks to use it.
        "camera.list" => {
            std::thread::spawn(move || {
                reply(
                    slot,
                    call_id,
                    match crate::image::cameras() {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    },
                );
            });
        }

        // Whether the system location switch is on. Deliberately separate
        // from whether this app holds the permission: a page told "no
        // location" deserves to know which of the two it is, since only one of
        // them it can do anything about.
        "location.enabled" => reply(
            slot,
            call_id,
            match crate::location::enabled() {
                Ok(j) => ok(j),
                Err(e) => err(&e),
            },
        ),

        // A single fix. Runs on a worker and can take seconds -- a cold fix is
        // not instant, which is why the timeout is generous and adjustable.
        //
        // Args: {} or {"timeoutMs": n}.
        "location.get" => {
            std::thread::spawn(move || {
                let v: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
                let timeout = v
                    .get("timeoutMs")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(8000)
                    .min(30_000);
                let scene = v
                    .get("scene")
                    .and_then(|x| x.as_str())
                    .unwrap_or("daily")
                    .to_string();
                reply(
                    slot,
                    call_id,
                    match crate::location::get(timeout, &scene) {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    },
                );
            });
        }

        // Which of the weather card's cities the phone is nearest, so that
        // card can start where you are instead of always on Tokyo.
        //
        // Args: {"lat": n, "lon": n}
        "location.nearestCity" => {
            let v: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
            let lat = v.get("lat").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let lon = v.get("lon").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let i = crate::location::nearest_city(lat, lon);
            let (name, clat, clon) = crate::apps::weather_web::CITIES[i];
            // Rough great-circle distance, only so the row can say how far off
            // the nearest city is -- "London (8900 km)" is honest about the
            // card showing somewhere else, where a bare name would not be.
            let dlat = (clat - lat).to_radians();
            let dlon = (clon - lon).to_radians() * lat.to_radians().cos();
            let km = 6371.0 * (dlat * dlat + dlon * dlon).sqrt();
            reply(
                slot,
                call_id,
                ok(format!(
                    "{{\"index\":{i},\"city\":{},\"km\":{km:.1}}}",
                    json_str(name)
                )),
            );
        }

        // Ask the user for runtime permissions. Goes through ArkTS because
        // requestPermissionsFromUser needs a UIAbilityContext -- the same
        // reason the picker does.
        //
        // Args: a JSON array of permission names.
        "permission.request" => {
            let list = if args.trim_start().starts_with('[') {
                args.clone()
            } else {
                format!("[{}]", json_str(args.trim_matches('"')))
            };
            park_arkts(slot, call_id, "permission.request", &list);
        }

        // Cellular network state: operator, technology, roaming. A page has
        // no route to any of this; the closest a browser offers is
        // navigator.connection.effectiveType, which is a guess from timings.
        "radio.cellular" => reply(
            slot,
            call_id,
            match crate::radio::cellular() {
                Ok(j) => ok(j),
                Err(e) => err(&e),
            },
        ),

        // Whether the Wi-Fi radio is on, which is not the same question as
        // whether it carries the default route -- that is net.info's.
        "radio.wifi" => reply(
            slot,
            call_id,
            match crate::radio::wifi() {
                Ok(j) => ok(j),
                Err(e) => err(&e),
            },
        ),

        // SHA-256 from the system crypto kit. The point of a hash is that
        // everyone computes the same one, so it uses the platform's rather
        // than a hand-rolled implementation.
        //
        // Args: {"text": "..."} or {"path": "..."}. The file form streams in
        // chunks, so a page can check what it was handed without the bytes
        // crossing the bridge at all.
        "crypto.sha256" => {
            std::thread::spawn(move || {
                let v: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
                let payload = if let Some(p) = v.get("path").and_then(|x| x.as_str()) {
                    crate::radio::sha256_file(p)
                } else {
                    let t = v
                        .get("text")
                        .and_then(|x| x.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| args.trim_matches('"').to_string());
                    crate::radio::sha256(t.as_bytes())
                };
                reply(
                    slot,
                    call_id,
                    match payload {
                        Ok(j) => ok(j),
                        Err(e) => err(&e),
                    },
                );
            });
        }

        // What the default route actually is. `navigator.onLine` is a boolean
        // that mostly means "the browser has not noticed a failure yet"; this
        // is the bearer, whether the system considers the link validated, and
        // whether a proxy sits in front of it.
        "net.info" => reply(slot, call_id, ok(crate::netinfo::info())),

        // Persistent key-value in the app sandbox. A page here cannot rely on
        // localStorage: the slot is re-created whenever the DSL rebuilds, and
        // generated pages arrive under a synthetic baseUrl, so the origin the
        // storage was scoped to is not reliably the same one next time.
        "prefs.get" => reply(slot, call_id, ok(crate::prefs::get(args.trim_matches('"')))),

        "prefs.keys" => reply(slot, call_id, ok(crate::prefs::keys())),

        // Args: {"key": "...", "value": "..."}
        "prefs.set" => {
            let v: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
            let k = v.get("key").and_then(|x| x.as_str()).unwrap_or("");
            let val = v.get("value").and_then(|x| x.as_str()).unwrap_or("");
            reply(
                slot,
                call_id,
                match crate::prefs::set(k, val) {
                    Ok(j) => ok(j),
                    Err(e) => err(&e),
                },
            );
        }

        "prefs.remove" => reply(
            slot,
            call_id,
            match crate::prefs::remove(args.trim_matches('"')) {
                Ok(j) => ok(j),
                Err(e) => err(&e),
            },
        ),

        // Haptics. A page in a browser has no route to the motor at all.
        "vibrate" => {
            let ms = args
                .trim_matches('"')
                .parse::<i32>()
                .ok()
                .or_else(|| {
                    serde_json::from_str::<serde_json::Value>(&args)
                        .ok()
                        .and_then(|v| v.get("ms").and_then(|x| x.as_i64()).map(|n| n as i32))
                })
                .unwrap_or(40);
            reply(
                slot,
                call_id,
                match crate::sensor::vibrate(ms) {
                    Ok(j) => ok(j),
                    Err(e) => err(&e),
                },
            );
        }

        "log" => {
            crate::log(&format!("page: {args}"));
            reply(slot, call_id, ok("true".into()));
        }

        // The interesting one: the page asks Rust to fetch, instead of opening
        // its own connection. One network path, one place to see failures.
        "http.get" => {
            std::thread::spawn(move || {
                let url = args.trim_matches('"').to_string();
                if let Err(e) = check_https_public(&url) {
                    reply(slot, call_id, err(&e));
                    return;
                }
                let (code, body) = net::http_get_string(&url);
                if code >= 200 && code < 300 {
                    reply(slot, call_id, ok(json_str(&body.unwrap_or_default())));
                } else {
                    reply(slot, call_id, err(&format!("http {code}")));
                }
            });
        }

        // Directory listing, so a page can browse the phone rather than only
        // the network. Names, sizes and kinds -- no contents: reading a file
        // out to a web surface is a strictly larger capability than seeing
        // that it exists, and nothing here needs it yet.
        //
        // Deliberately NOT path-restricted. The OS sandbox is the real
        // boundary and the point of this tool is to find out where it runs, so
        // a denial has to come back as a denial rather than be pre-empted by
        // an allowlist of paths guessed from the host side. Errors are
        // reported verbatim for the same reason.
        //
        // Args: a bare path string, or {"path": "..."}.
        "fs.list" => {
            std::thread::spawn(move || {
                let path = args
                    .strip_prefix('{')
                    .and_then(|_| serde_json::from_str::<serde_json::Value>(&args).ok())
                    .and_then(|v| v.get("path").and_then(|p| p.as_str()).map(String::from))
                    .unwrap_or_else(|| args.trim_matches('"').to_string());
                reply(slot, call_id, list_dir(&path));
            });
        }

        // One path's metadata. Smaller than `fs.list` and much smaller than
        // reading contents, and it answers the question a picked file raises:
        // the grant clearly resolves the path, but can Rust actually *use* it?
        // A size matching what the picker displayed says yes.
        "fs.stat" => {
            std::thread::spawn(move || {
                let path = args.trim_matches('"').to_string();
                reply(
                    slot,
                    call_id,
                    match std::fs::metadata(&path) {
                        Ok(m) => ok(format!(
                            "{{\"path\":{},\"dir\":{},\"size\":{},\"readonly\":{}}}",
                            json_str(&path),
                            m.is_dir(),
                            m.len(),
                            m.permissions().readonly()
                        )),
                        Err(e) => err(&format!("{e}")),
                    },
                );
            });
        }

        // File contents. Strictly larger than `fs.list` and `fs.stat`, and the
        // one the picker makes worth having: a granted URI resolves to a path
        // Rust can stat, so it is a path Rust can read, and without this a page
        // can be told a file exists and never see it.
        //
        // Capped, and binary-safe by declaring the encoding rather than
        // guessing. Text that is not valid UTF-8 comes back as base64 with
        // `"encoding":"base64"` instead of being silently lossy — a page that
        // renders replacement characters and calls it the file is worse than
        // one that is told it got bytes.
        //
        // Args: a bare path, or {"path": "...", "max": <bytes>}.
        "fs.read" => {
            std::thread::spawn(move || {
                let (path, max) = read_args(&args);
                reply(slot, call_id, read_file(&path, max));
            });
        }

        // The system folder/file picker.
        //
        // This one cannot be answered in Rust at all. `fs.list` showed why:
        // user storage is `absent` from the app's mount namespace, not merely
        // denied, so no permission makes /storage/media/... appear. The only
        // route to a user's Documents or Downloads is the system picker, which
        // is ArkTS-only and needs a UIAbilityContext.
        //
        // So the call is parked rather than answered. ArkTS drains it, shows
        // the picker, and calls back with URIs -- which makes this the first
        // tool whose answer travels Rust -> ArkTS -> Rust, the opposite of
        // every other one here.
        //
        // Args: {"mode":"folder"|"file"} — bare "folder" also works.
        "fs.pick" => {
            let mode = if args.contains("folder") {
                "folder"
            } else {
                "file"
            };
            park_arkts(slot, call_id, "pick", mode);
        }

        // The system clipboard. No NDK exists for the pasteboard, so this
        // takes the same Rust -> ArkTS -> Rust road the picker does.
        //
        // Reading someone's clipboard is a real capability -- it may hold a
        // password a moment after they copied one -- which is exactly why it is
        // behind the trust gate and reachable only from app-generated pages.
        "clipboard.read" => park_arkts(slot, call_id, "clipboard.read", ""),

        "clipboard.write" => {
            let text = if args.starts_with('{') {
                serde_json::from_str::<serde_json::Value>(&args)
                    .ok()
                    .and_then(|v| v.get("text").and_then(|x| x.as_str()).map(String::from))
                    .unwrap_or_default()
            } else {
                args.trim_matches('"').to_string()
            };
            park_arkts(slot, call_id, "clipboard.write", &text);
        }

        // The Splash half of the bridge: a page evaluates DSL and gets JSON
        // back. This is what makes it a JS <-> Rust/Splash bridge rather than
        // a JS -> Rust one -- the same language that describes the native tree
        // is reachable from the web surface, so a card can share logic with the
        // widgets around it instead of reimplementing it in JavaScript.
        //
        // Args: {"source": "<splash>", "input": <any json>}
        // `input` is injected as a global the script can read.
        "splash.eval" => {
            std::thread::spawn(move || {
                let parsed: serde_json::Value = match serde_json::from_str(&args) {
                    Ok(v) => v,
                    Err(e) => {
                        reply(slot, call_id, err(&format!("bad args: {e}")));
                        return;
                    }
                };
                let source = parsed.get("source").and_then(|v| v.as_str()).unwrap_or("");
                if source.is_empty() {
                    reply(slot, call_id, err("missing \"source\""));
                    return;
                }
                if source.len() > EVAL_MAX_SOURCE {
                    reply(slot, call_id, err("source too large"));
                    return;
                }
                reply(slot, call_id, eval_splash(source, parsed.get("input")));
            });
        }

        other => reply(slot, call_id, err(&format!("unknown tool: {other}"))),
    }
}

/// Hosts a page may reach through `http.get`.
///
/// Curated rather than open: the tool runs with the app's network identity, so
/// an open one turns every card into a proxy for reaching anything the phone
/// can reach. Suffix-matched on a label boundary, so `open-meteo.com` covers
/// `api.` and `geocoding-api.` but not `evil-open-meteo.com`.
const HTTP_GET_ALLOWED_HOSTS: &[&str] = &[
    "open-meteo.com",
    "api.open-meteo.com",
    "air-quality-api.open-meteo.com",
];

/// https-only, public hosts only. Blocks the SSRF shapes -- loopback, private
/// ranges, link-local, and `.internal` -- that would otherwise let a page use
/// this app to reach things on the device or the local network.
fn check_https_public(url: &str) -> Result<String, String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| "only https:// URLs are allowed".to_string())?;
    let host = rest
        .split(|c| c == '/' || c == '?' || c == '#')
        .next()
        .unwrap_or("")
        .split('@')
        .next_back()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if host.is_empty() {
        return Err("no host in URL".into());
    }
    let blocked = host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".internal")
        || host.ends_with(".local")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || host == "0.0.0.0"
        || host.starts_with("[")
        || (host.starts_with("172.")
            && host
                .split('.')
                .nth(1)
                .and_then(|o| o.parse::<u32>().ok())
                .is_some_and(|o| (16..=31).contains(&o)));
    if blocked {
        return Err(format!("host not permitted: {host}"));
    }
    let allowed = HTTP_GET_ALLOWED_HOSTS
        .iter()
        .any(|a| host == *a || host.ends_with(&format!(".{a}")));
    if !allowed {
        return Err(format!("host not on the allowlist: {host}"));
    }
    Ok(host)
}

// ---------------------------------------------------------------------------
// The ArkTS channel: Rust -> ArkTS -> Rust.
//
// Every other tool answers on a worker and queues the reply. Some cannot: the
// file picker needs a UIAbilityContext and the pasteboard has no NDK at all, so
// Rust has to ask the ArkTS side to do the work and wait to be told the answer.
// The call is parked with its (slot, call_id) so the eventual reply reaches the
// right promise in the right page.
//
// Generalised from what began as a picker-only queue. A second tool needing the
// same round trip made the shape obvious: the channel carries an *operation
// name* and an argument string, and ArkTS switches on the name. Adding a third
// is now an ArkTS case rather than a parallel set of statics.
//
// Request ids are strings on the wire for the same reason call ids are: they
// cross into JS, where a u64 past 2^53 would come back rounded.
// ---------------------------------------------------------------------------

/// Requests ArkTS has not yet picked up, as (req_id, op, args).
static PICK_QUEUE: Mutex<Vec<(u64, String, String)>> = Mutex::new(Vec::new());
/// Requests ArkTS is working on, as (req_id, slot, call_id). Kept separate from
/// the queue because the queue is emptied on drain and this must outlive it —
/// the user is looking at a picker UI in between, which can take as long as it
/// takes.
static PICK_INFLIGHT: Mutex<Vec<(u64, u32, String)>> = Mutex::new(Vec::new());
static PICK_SEQ: Mutex<u64> = Mutex::new(0);

fn park_arkts(slot: u32, call_id: String, op: &str, args: &str) {
    let id = match PICK_SEQ.lock() {
        Ok(mut n) => {
            *n += 1;
            *n
        }
        Err(_) => return,
    };
    // A user who dismisses pickers forever should not grow this list, but the
    // evicted entry is a call some page is still awaiting. Dropping it silently
    // left that promise pending for the life of the page; settle it instead.
    let evicted = match PICK_INFLIGHT.lock() {
        Ok(mut q) => {
            let old = if q.len() > 16 {
                Some(q.remove(0))
            } else {
                None
            };
            q.push((id, slot, call_id));
            old
        }
        Err(_) => None,
    };
    if let Some((_, s, c)) = evicted {
        reply(
            s,
            c,
            err("request dropped: too many ArkTS calls still open"),
        );
    }
    if let Ok(mut q) = PICK_QUEUE.lock() {
        q.push((id, op.to_string(), args.to_string()));
    }
}

/// Work waiting for ArkTS, as `reqId|op|args`. Args is last because it may
/// itself contain a separator.
pub fn pick_drain() -> Vec<String> {
    match PICK_QUEUE.lock() {
        Ok(mut q) => q
            .drain(..)
            .map(|(id, op, a)| format!("{id}|{op}|{a}"))
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// ArkTS reporting an outcome. `payload` is the tool's JSON result on success,
/// or a message on failure.
pub fn pick_resolve(req_id: &str, success: bool, payload: String) {
    let id: u64 = match req_id.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let found = PICK_INFLIGHT.lock().ok().and_then(|mut q| {
        q.iter()
            .position(|(i, _, _)| *i == id)
            .map(|ix| q.remove(ix))
    });
    let Some((_, slot, call_id)) = found else {
        crate::log(&format!("bridge: arkts {id} resolved with nothing waiting"));
        return;
    };
    if success {
        reply(slot, call_id, ok(payload));
    } else {
        reply(slot, call_id, err(&payload));
    }
}

/// Ceiling on `fs.read`, and its default when a page names none.
///
/// The whole payload rides through the reply queue as one JSON string and is
/// then evaluated into the page by `runJavaScript`, so a large file is paid for
/// three times over. A page that wants more than this wants streaming, which is
/// a different tool.
const READ_MAX: u64 = 1024 * 1024;

/// `(path, max_bytes)` from either a bare path string or `{path, max}`.
fn read_args(args: &str) -> (String, u64) {
    if args.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(args) {
            let p = v.get("path").and_then(|x| x.as_str()).unwrap_or_default();
            let m = v
                .get("max")
                .and_then(|x| x.as_u64())
                .unwrap_or(READ_MAX)
                .min(READ_MAX);
            return (p.to_string(), m);
        }
    }
    (args.trim_matches('"').to_string(), READ_MAX)
}

/// One file's contents, as UTF-8 text when it is text and base64 when it is not.
///
/// `truncated` is reported for the same reason `fs.list` reports it: a clipped
/// file that says nothing reads as a whole one.
fn read_file(path: &str, max: u64) -> String {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return err(&format!("{e}")),
    };
    if meta.is_dir() {
        return err("is a directory");
    }
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return err(&format!("{e}")),
    };
    let truncated = bytes.len() as u64 > max;
    let slice = if truncated {
        &bytes[..max as usize]
    } else {
        &bytes[..]
    };

    // Decide by what the bytes are, not by the extension: a .txt holding a JPEG
    // is still a JPEG, and String::from_utf8_lossy would hand the page a string
    // of replacement characters and call it the file.
    match std::str::from_utf8(slice) {
        Ok(text) => format!(
            "{{\"ok\":true,\"data\":{{\"path\":{},\"size\":{},\"encoding\":\"utf8\",\
             \"truncated\":{},\"content\":{}}}}}",
            json_str(path),
            meta.len(),
            truncated,
            json_str(text)
        ),
        Err(_) => format!(
            "{{\"ok\":true,\"data\":{{\"path\":{},\"size\":{},\"encoding\":\"base64\",\
             \"truncated\":{},\"content\":{}}}}}",
            json_str(path),
            meta.len(),
            truncated,
            json_str(&b64(slice))
        ),
    }
}

/// Base64, for bytes that are not text.
fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// How many entries one listing may carry. `/system/lib64` alone runs past
/// this, and the whole listing rides through the reply queue as one string.
const MAX_ENTRIES: usize = 500;

/// One directory, as JSON. Entries sorted directories-first then by name, which
/// is what a file browser wants and saves the page doing it.
///
/// Carries `truncated` when the cap bit, because a silently capped list reads
/// as a complete one — `/system/lib64` stopping at exactly 500 looked like a
/// fact about the directory rather than about this function.
///
/// A failure carries the OS's own message. That is the interesting part of this
/// tool: "Permission denied" and "No such file or directory" are different
/// answers about where an app's reach ends, and collapsing them to "failed"
/// would throw away the finding.
fn list_dir(path: &str) -> String {
    let dir = match std::fs::read_dir(path) {
        Ok(d) => d,
        Err(e) => return err(&format!("{e}")),
    };

    let mut entries: Vec<(bool, String, u64)> = Vec::new();
    let mut truncated = false;
    for e in dir.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        // A symlink to a directory should read as a directory, so ask the
        // entry rather than the metadata call that follows links lazily.
        let (is_dir, size) = match e.metadata() {
            Ok(m) => (m.is_dir(), m.len()),
            // Listable but not stat-able happens on OHOS system paths; show it
            // rather than dropping it, since its presence is the information.
            Err(_) => (false, 0),
        };
        entries.push((is_dir, name, size));
        if entries.len() >= MAX_ENTRIES {
            truncated = true;
            break;
        }
    }
    entries.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });

    let items: Vec<String> = entries
        .iter()
        .map(|(d, n, s)| format!("{{\"name\":{},\"dir\":{},\"size\":{}}}", json_str(n), d, s))
        .collect();

    let parent = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    ok(format!(
        "{{\"path\":{},\"parent\":{},\"truncated\":{},\"entries\":[{}]}}",
        json_str(path),
        json_str(&parent),
        truncated,
        items.join(",")
    ))
}

/// Evaluate a page-supplied script under limits and encode the result.
///
/// Deliberately a fresh `Runtime` per call. Sharing one across pages would let
/// a card leave state behind for the next caller, and the VM is cheap enough
/// that isolation is the better default.
fn page_limits() -> splash_core::ExecutionLimits {
    let d = splash_core::ExecutionLimits::default();
    splash_core::ExecutionLimits {
        // One number, not two: the source cap the tool already enforces.
        max_source_bytes: EVAL_MAX_SOURCE,
        // 8 MB -> 1 MB. A card computing over its own data does not need a
        // script heap larger than the whole page that sent it.
        max_heap_bytes: 1024 * 1024,
        // 1024 -> 64 frames. Deep recursion from a page is a mistake or an
        // attack; neither deserves a thousand frames.
        max_call_frames: 64,
        max_syntax_nesting: 32,
        // 200k -> 20k instructions. The weather card's loop over six values is
        // a few hundred; twenty thousand is two orders of magnitude of slack.
        instruction_limit: 20_000,
        // 32/64 ms -> 10/25 ms. These run on a worker, so the ceiling is about
        // not tying up a thread rather than about frame budget.
        soft_timeout: std::time::Duration::from_millis(10),
        hard_timeout: std::time::Duration::from_millis(25),
        ..d
    }
}

fn eval_splash(source: &str, input: Option<&serde_json::Value>) -> String {
    let mut rt = match splash_core::Runtime::<(), ()>::with_limits((), (), page_limits()) {
        Ok(r) => r,
        Err(e) => return err(&format!("runtime: {e:?}")),
    };

    if let Some(v) = input {
        if let Err(e) = rt.set_json_global("input", v, JSON_MAX_BYTES, JSON_MAX_DEPTH) {
            return err(&format!("input rejected: {e:?}"));
        }
    }

    let evaluation = match rt.eval(source) {
        Ok(e) => e,
        // Name the limit that stopped it. The error variants distinguish
        // SourceTooLarge from HeapLimitExceeded from a timeout, and a page
        // author fixing "too big" does something different from one fixing
        // "too slow" -- collapsing them to "failed" throws that away, the same
        // reason fs.list reports the OS errno verbatim.
        Err(e) => {
            let m = limit_message(&e);
            crate::log(&format!("splash.eval stopped: {m}"));
            return err(&m);
        }
    };

    if evaluation.suspended {
        return err("script suspended (it awaited a capability this host does not grant)");
    }

    match rt.script_value_as_json(evaluation.value, JSON_MAX_BYTES, JSON_MAX_DEPTH) {
        Ok(j) => ok(j.to_string()),
        // Functions, handles, cycles and non-finite numbers are not encodable.
        Err(e) => {
            crate::log(&format!("splash.eval result not encodable: {e:?}"));
            err(&format!("result not representable as JSON: {e:?}"))
        }
    }
}

/// Turn a runtime rejection into something a page author can act on.
fn limit_message(e: &impl std::fmt::Debug) -> String {
    let d = format!("{e:?}");
    let hint = if d.contains("SourceTooLarge") || d.contains("FormattedSourceTooLarge") {
        " — script is larger than the source limit"
    } else if d.contains("HeapLimitExceeded") {
        " — script allocated past the heap limit"
    } else if d.contains("StringLimitExceeded") {
        " — a single string exceeded the string limit"
    } else if d.contains("Timeout") || d.contains("Budget") || d.contains("Instruction") {
        " — script ran past its instruction or time budget"
    } else if d.contains("SyntaxRejected") {
        " — syntax rejected; the report carries line and column"
    } else {
        ""
    };
    format!("{d}{hint}")
}

/// Replies waiting to be evaluated, as `slot|callId|payload`.
///
/// Payload is JSON and can contain `|`, so it is last and the ArkTS side splits
/// on the first two separators only.
pub fn drain() -> Vec<String> {
    expire_stale();
    let mut out = Vec::new();
    if let Ok(mut q) = REPLIES.lock() {
        while let Some(r) = q.pop_front() {
            out.push(format!("{}|{}|{}", r.slot, r.call_id, r.payload));
        }
    }
    out
}

/// The shim injected into every generated page.
///
/// `splash_native` is the raw proxy ArkWeb injects: synchronous and void. This
/// wraps it in the promise API a page actually wants, and provides `_resolve`
/// for ArkTS to call back into.
pub const SHIM: &str = r#"<script>
(function(){
  var n = 0, pending = {};
  window.splash = {
    // NOTE ON ARGS: a string is passed through RAW, anything else is
    // stringified. So `invoke('echo', 'hi')` sends `hi`, not `"hi"` -- the
    // args a tool receives are not always valid JSON, and each tool handles
    // its own shape (http.get trims stray quotes, splash.eval parses). Tools
    // returning a string therefore return it decoded; JSON.parse on the way
    // out is a mistake, and was one.
    invoke: function (tool, args) {
      return new Promise(function (res, rej) {
        // Ids are strings: they are u64 in Rust and JS numbers are f64, so
        // anything past 2^53 would come back rounded.
        var id = String(++n);
        pending[id] = { res: res, rej: rej };
        if (!window.splash_native || !window.splash_native.invoke) {
          delete pending[id];
          rej(new Error('splash_native bridge not present'));
          return;
        }
        window.splash_native.invoke(id, tool,
          typeof args === 'string' ? args : JSON.stringify(args === undefined ? null : args));
      });
    },
    _resolve: function (id, payload) {
      var p = pending[String(id)];
      if (!p) { return; }
      delete pending[String(id)];
      if (payload && payload.ok) { p.res(payload.data); }
      else { p.rej(new Error((payload && payload.error) || 'call failed')); }
    },
    available: function () {
      return !!(window.splash_native && window.splash_native.invoke);
    }
  };
  // Tell the host this page is really running.
  //
  // ArkTS otherwise has to infer it, and both available signals lie: loadData
  // returns without throwing when it has silently rendered nothing, and
  // onPageEnd also fires for the blank origin the slot starts on, so a failed
  // load looks exactly like a successful one. Script executing is the only
  // witness that cannot be wrong about it -- if this line runs, the page is
  // there. Fire and forget; nothing waits on the reply.
  if (window.splash_native && window.splash_native.invoke) {
    window.splash_native.invoke('ready', 'slot.ready', '');
  }
})();
</script>"#;
