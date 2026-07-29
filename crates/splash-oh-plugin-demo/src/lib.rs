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

use splash_oh_core::{Args, Registry, Responder};

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
        |args: &Args, resp: Responder| match args.parse::<SumArgs>() {
            Ok(a) => resp.ok(format!("{}", a.a + a.b)),
            Err(e) => resp.err(e),
        },
    );
    r.add(
        "demo.reverse",
        "Reverse a string, to show a plugin returning JSON",
        |args: &Args, resp: Responder| {
            let rev: String = args.text().chars().rev().collect();
            resp.ok(serde_json::to_string(&rev).unwrap_or_else(|_| "\"\"".into()))
        },
    );
    // The one that could not exist before: a tool that answers later.
    //
    // The Responder moves onto the thread, so this function returns long before
    // the page hears anything. Any real waiting tool -- an HTTP call, a database
    // read, a file the user has yet to choose -- has this shape.
    r.add(
        "demo.delay",
        "Answer after a delay, to show a tool that does not return immediately",
        |args: &Args, resp: Responder| {
            let ms: u64 = args.parse::<DelayArgs>().map(|a| a.ms).unwrap_or(500);
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(ms.min(10_000)));
                resp.ok(format!("{{\"sleptMs\":{ms}}}"));
            });
        },
    );
    // A tool that answers nothing at all, to prove the page is not left hanging
    // when a plugin is wrong. The Responder's Drop turns it into a rejection.
    r.add(
        "demo.forget",
        "Drop the responder without answering, to show the promise still settles",
        |_args: &Args, _resp: Responder| {},
    );
}

#[derive(serde::Deserialize)]
struct DelayArgs {
    ms: u64,
}
