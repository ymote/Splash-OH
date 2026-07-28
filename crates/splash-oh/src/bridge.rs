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

fn reply(slot: u32, call_id: String, payload: String) {
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
        // Round-trip check, so a page can prove the bridge is live.
        "echo" => reply(slot, call_id, ok(json_str(&args))),

        "device.info" => {
            let info = format!(
                "{{\"platform\":\"OpenHarmony\",\"renderer\":\"ArkUI NDK via Rust\",\"slot\":{slot}}}"
            );
            reply(slot, call_id, ok(info));
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

/// Evaluate a page-supplied script under limits and encode the result.
///
/// Deliberately a fresh `Runtime` per call. Sharing one across pages would let
/// a card leave state behind for the next caller, and the VM is cheap enough
/// that isolation is the better default.
fn eval_splash(source: &str, input: Option<&serde_json::Value>) -> String {
    let mut rt = match splash_core::Runtime::<(), ()>::new((), ()) {
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
        // A syntax rejection carries line/column, which is the whole reason to
        // go through splash-core rather than the bare VM. Pass it on.
        Err(e) => return err(&format!("{e:?}")),
    };

    if evaluation.suspended {
        return err("script suspended (it awaited a capability this host does not grant)");
    }

    match rt.script_value_as_json(evaluation.value, JSON_MAX_BYTES, JSON_MAX_DEPTH) {
        Ok(j) => ok(j.to_string()),
        // Functions, handles, cycles and non-finite numbers are not encodable.
        Err(e) => err(&format!("result not representable as JSON: {e:?}")),
    }
}

/// Replies waiting to be evaluated, as `slot|callId|payload`.
///
/// Payload is JSON and can contain `|`, so it is last and the ArkTS side splits
/// on the first two separators only.
pub fn drain() -> Vec<String> {
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
})();
</script>"#;
