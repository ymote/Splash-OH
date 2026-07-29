//! The tool registry, and the argument type a tool receives.
//!
//! # Why this crate exists
//!
//! Every bridge tool used to be an arm of one `match` in `bridge.rs`. That
//! works, and it means the only way to add a capability is to edit the bridge —
//! so nobody outside this repository can add one at all. A framework whose
//! extension point is "modify the framework" does not have an extension point.
//!
//! # Why it is a separate crate rather than a module
//!
//! `splash-oh` is a `cdylib`. A `cdylib` is a final artifact: nothing links
//! *against* it, so a plugin crate cannot depend on it, and a registry living
//! inside it would be unreachable from anywhere else. The registry and the
//! types a tool needs therefore live here, in an `rlib` that both the plugin
//! and the bridge can depend on:
//!
//! ```text
//! splash-oh-plugin-demo  (rlib)  ─┐
//!                                 ├─> splash-oh-core (rlib)
//! splash-oh              (cdylib) ┘        the registry
//!   the app; links the plugins in
//! ```
//!
//! The direction matters. The plugin does not know about the bridge, the
//! bridge does not know about the plugin, and the `cdylib` — the app — is what
//! decides which plugins are part of this build. That is the same shape Tauri
//! uses, and it is the reason the crate split had to come before the registry
//! rather than after it.
//!
//! # What a tool is
//!
//! A function from JSON arguments to a JSON result, or an error message. That
//! covers the synchronous majority. Tools that must park a call and answer
//! later — the picker, the clipboard, a permission prompt — still live in the
//! bridge, because parking needs the reply channel and the reply channel needs
//! napi. Widening this to cover them is the next question, not a solved one.

use std::sync::Mutex;

/// A tool's arguments, as JSON.
///
/// Always JSON: the shim stringifies whatever a page passes, so a tool can
/// deserialize into a type rather than guessing at a shape.
pub struct Args(String);

impl Args {
    pub fn new(json: String) -> Self {
        Args(json)
    }

    /// Deserialize into `T`, naming the failure rather than falling back to a
    /// default that hides the mistake.
    pub fn parse<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_str(&self.0).map_err(|e| format!("bad arguments: {e}"))
    }

    /// The raw JSON, for the few tools that genuinely want it.
    pub fn raw(&self) -> &str {
        &self.0
    }

    /// A single string argument. `invoke('echo', 'hi')` arrives as `"hi"`, so
    /// this is a parse rather than the quote-trimming every tool used to do by
    /// hand — and it is correct for a string containing a quote, which the
    /// trimming was not.
    pub fn text(&self) -> String {
        self.parse::<String>().unwrap_or_else(|_| self.0.clone())
    }
}

/// How a tool answers.
///
/// Handed to the tool rather than returned by it, so a tool that has to wait --
/// a network call, a file picker, anything that parks -- can move this to
/// another thread and answer whenever the answer arrives. A tool that already
/// knows just calls `ok` and returns.
///
/// The page is holding a promise on the other end of this, so **it must be
/// answered**. Dropping one without answering used to be a page that waits
/// forever; now `Drop` answers with an error, which is a bad outcome the caller
/// can see rather than a hang they cannot.
pub struct Responder {
    slot: u32,
    call_id: String,
    answered: bool,
}

impl Responder {
    pub fn new(slot: u32, call_id: String) -> Self {
        Responder {
            slot,
            call_id,
            answered: false,
        }
    }

    /// Answer with a JSON payload.
    pub fn ok(mut self, json: impl Into<String>) {
        self.answered = true;
        send(
            self.slot,
            std::mem::take(&mut self.call_id),
            Ok(json.into()),
        );
    }

    /// Answer with a failure. The page's promise rejects.
    pub fn err(mut self, msg: impl Into<String>) {
        self.answered = true;
        send(
            self.slot,
            std::mem::take(&mut self.call_id),
            Err(msg.into()),
        );
    }

    /// Which surface asked, for a tool that cares.
    pub fn slot(&self) -> u32 {
        self.slot
    }
}

impl Drop for Responder {
    fn drop(&mut self) {
        if !self.answered {
            send(
                self.slot,
                std::mem::take(&mut self.call_id),
                Err("the tool did not answer".into()),
            );
        }
    }
}

/// How an answer gets back to the page.
///
/// Installed by the bridge at startup, because the reply channel needs napi and
/// this crate cannot have napi -- a plugin has to be able to depend on it from
/// a host build.
pub type ReplyFn = fn(u32, String, Result<String, String>);
static REPLY: Mutex<Option<ReplyFn>> = Mutex::new(None);

pub fn set_reply(f: ReplyFn) {
    if let Ok(mut g) = REPLY.lock() {
        *g = Some(f);
    }
}

fn send(slot: u32, call_id: String, result: Result<String, String>) {
    let f = REPLY.lock().ok().and_then(|g| *g);
    match f {
        Some(f) => f(slot, call_id, result),
        // Only reachable if a tool answers before mount installed the channel,
        // which would be a wiring mistake rather than a runtime condition.
        None => eprintln!("splash-oh-core: no reply channel; dropped answer for {call_id}"),
    }
}

/// What a tool does. It receives its arguments and the means to answer.
pub type ToolFn = fn(&Args, Responder);

pub struct Tool {
    pub name: &'static str,
    /// One line, so a registry listing is legible without reading the code.
    pub summary: &'static str,
    pub call: ToolFn,
}

#[derive(Default)]
pub struct Registry {
    tools: Vec<Tool>,
}

impl Registry {
    /// Add a tool. Returns whether it was added.
    ///
    /// A duplicate name is refused rather than overwriting: two plugins
    /// claiming one name is a build mistake, and letting the later one win
    /// would decide it silently and differently depending on link order.
    pub fn add(&mut self, name: &'static str, summary: &'static str, call: ToolFn) -> bool {
        if self.tools.iter().any(|t| t.name == name) {
            return false;
        }
        self.tools.push(Tool {
            name,
            summary,
            call,
        });
        true
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|t| t.name).collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// The process-wide registry.
///
/// A plain `Mutex` rather than anything link-time: `linkme`-style distributed
/// slices depend on section behaviour that is not proven on this target, and a
/// registration that silently fails to be collected would present as a tool
/// that simply is not there. Explicit registration at startup is duller and
/// cannot half-work.
static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

/// Register plugins. Called once, at mount.
pub fn with_registry_mut<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    let mut g = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Registry::default))
}

/// Does a plugin claim this name?
///
/// Asked *before* a `Responder` exists, and that ordering is the whole point. A
/// Responder answers with an error when dropped unanswered, so building one
/// speculatively and letting it fall out of scope on a miss rejects the call --
/// which is what happened when this took a Responder and returned a bool: every
/// built-in tool started failing with "the tool did not answer", because the
/// guard fired before the bridge's own dispatch got a turn.
pub fn claims(name: &str) -> bool {
    REGISTRY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .is_some_and(|r| r.get(name).is_some())
}

/// Call a plugin's tool. Only sound after [`claims`] has said yes.
///
/// Returning says nothing about whether the tool has answered — that is what
/// `Responder` is for. A deferred tool has taken it to another thread by now.
pub fn dispatch(name: &str, args: &Args, responder: Responder) {
    let call = {
        let g = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
        match g.as_ref().and_then(|r| r.get(name)) {
            Some(t) => t.call,
            None => {
                // Only reachable if the registry changed between claims() and
                // here. Answering beats dropping: the page gets a reason.
                responder.err(format!("no tool named {name:?}"));
                return;
            }
        }
    };
    // Called with the lock released: a tool is arbitrary code and may well
    // invoke something that wants the registry again.
    call(args, responder);
}

/// Registered tool names, for `plugin.list`.
pub fn registered() -> Vec<&'static str> {
    REGISTRY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|r| r.names())
        .unwrap_or_default()
}

/// Check the registry's own invariants, and report them as a line of text.
///
/// `Registry::add` refusing a duplicate is a rule with real consequences —
/// two plugins claiming one name, resolved by link order, would be a bug that
/// changed shape between builds. A rule stated only in a doc comment and an
/// `if` is a rule nobody has watched work, so this runs it.
pub fn self_test() -> String {
    let mut r = Registry::default();
    let first = r.add("selftest.tool", "first", |_, resp| resp.ok("1"));
    let dup = r.add("selftest.tool", "second, should be refused", |_, resp| {
        resp.ok("2")
    });
    let other = r.add("selftest.other", "a different name", |_, resp| resp.ok("3"));

    // Which function a name resolves to, without calling it: a Responder needs
    // a live reply channel and this runs before one is guaranteed.
    let first_wins = r.get("selftest.tool").map(|t| t.summary) == Some("first");
    let called = if first_wins { "1" } else { "" };

    let ok = first && !dup && other && r.len() == 2 && called == "1";
    format!(
        "registry selftest: {} (added={first} duplicate_refused={} other={other} len={} \
         first_wins={})",
        if ok { "ok" } else { "FAILED" },
        !dup,
        r.len(),
        called == "1"
    )
}

/// The bridge shim, as bare JavaScript with no `<script>` wrapper.
///
/// It lives here rather than beside the bridge because two different
/// consumers need it and only one of them can build a cdylib for a phone:
/// the app serves it at `/__splash.js`, and a developer running a Vite dev
/// server needs the same bytes on disk. `splash-oh-cli shim` writes them.
pub const SHIM_JS: &str = r#"(function(){
  var n = 0, pending = {};
  window.splash = {
    // ARGS ARE ALWAYS JSON. Whatever is passed is stringified, so a tool
    // always receives valid JSON and can deserialize into a type instead of
    // guessing. Results come back already decoded, as before.
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
        // Always stringified. It used to pass a string through raw, which
        // meant `args` was JSON for some calls and not others, and every tool
        // had to guess -- see Args in bridge.rs for the two bugs that caused.
        window.splash_native.invoke(id, tool,
          JSON.stringify(args === undefined ? null : args));
      });
    },
    // Subscribe to events Rust sends with no call outstanding. The direction
    // that did not exist before: a page can now be told things.
    on: function (name, cb) {
      (this._l[name] = this._l[name] || []).push(cb);
      return this;
    },
    off: function (name) { delete this._l[name]; return this; },
    _l: {},
    _event: function (name, payload) {
      var fns = this._l[name];
      if (!fns) { return; }
      for (var i = 0; i < fns.length; i++) {
        try { fns[i](payload); } catch (e) { /* one bad listener must not stop the rest */ }
      }
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
"#;
