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

/// What a tool does. Returns the JSON payload of a successful reply, or a
/// message explaining the refusal.
pub type ToolFn = fn(&Args) -> Result<String, String>;

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

/// Look a tool up and call it. `None` if no plugin claims the name, which is
/// what tells the bridge to fall through to its own dispatch.
pub fn dispatch(name: &str, args: &Args) -> Option<Result<String, String>> {
    let g = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    let call = g.as_ref()?.get(name)?.call;
    // Called with the lock released: a tool is arbitrary code and may well
    // invoke something that wants the registry again.
    drop(g);
    Some(call(args))
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
    let first = r.add("selftest.tool", "first", |_| Ok("1".into()));
    let dup = r.add("selftest.tool", "second, should be refused", |_| {
        Ok("2".into())
    });
    let other = r.add("selftest.other", "a different name", |_| Ok("3".into()));

    let called = r
        .get("selftest.tool")
        .map(|t| (t.call)(&Args::new("null".into())))
        .and_then(|v| v.ok())
        .unwrap_or_default();

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
