//! A plugin, existing to prove that a tool can live outside the bridge.
//!
//! This crate does not depend on `splash-oh`, cannot see `bridge.rs`, and knows
//! nothing about napi, ArkTS, web slots or the capability gate. It depends only
//! on `splash-oh-core`, which is the whole point: if a tool defined here is
//! callable from a page, then the bridge is extensible by someone who is not
//! editing the bridge.
//!
//! It is deliberately trivial. A plugin that did something interesting would
//! prove the interesting thing worked; this one is here to prove the *seam*
//! works, and a seam is easiest to see through something with nothing else in
//! it.

use splash_oh_core::{Args, Registry};

#[derive(serde::Deserialize)]
struct SumArgs {
    a: f64,
    b: f64,
}

/// Add this plugin's tools to the registry.
///
/// Called by the application crate, which is what links this code into the
/// final `.so`. Nothing collects it automatically, and that is on purpose —
/// see the note on `REGISTRY` in `splash-oh-core`.
pub fn register(r: &mut Registry) {
    r.add(
        "demo.sum",
        "Add two numbers, to show a typed plugin argument",
        |args: &Args| {
            let a: SumArgs = args.parse()?;
            Ok(format!("{}", a.a + a.b))
        },
    );
    r.add(
        "demo.reverse",
        "Reverse a string, to show a plugin returning JSON",
        |args: &Args| {
            let s: String = args.text();
            let rev: String = s.chars().rev().collect();
            Ok(serde_json::to_string(&rev).unwrap_or_else(|_| "\"\"".into()))
        },
    );
}
