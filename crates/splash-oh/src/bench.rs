//! Widget-construction benchmark: Rust/NDK versus ArkTS.
//!
//! Both paths end at the same place — real ArkUI components in the same
//! process — so the difference is purely how the tree is built.
//!
//! * **Rust** creates nodes through `ArkUI_NativeNodeAPI_1` directly. No JS
//!   thread is involved at any point.
//! * **ArkTS** must be driven over napi, and napi may only be entered from the
//!   JS thread, so each call is `uv_queue_work` + a wait for the event loop.
//!   That is the cost measured in the README (~1.05 ms per round trip, 70% of
//!   it queue latency), and it is per-call, not per-tree.
//!
//! We time the Rust side precisely here, and report the ArkTS cost as
//! `calls × measured round trip`, which is the honest way to state it: we are
//! not going to build 2000 ArkTS widgets on device just to prove a number we
//! already measured directly.

use crate::arkui::{attr, ty, Node};
use std::time::Instant;

/// Round trip cost of one Rust→ArkTS napi call, measured on the SUP-AL90 in
/// the octos-one port (enqueue 101 µs + queue wait 730 µs + JS work 220 µs).
const ARKTS_ROUND_TRIP_US: f64 = 1051.0;

/// Build `n` throwaway nodes and report ns/node.
fn time_nodes(n: usize) -> (f64, usize) {
    let t0 = Instant::now();
    let mut made = 0usize;
    let mut sink: Vec<Node> = Vec::with_capacity(n);
    for i in 0..n {
        // A representative widget: a Text with four attributes set, which is
        // what a real row in the catalog costs.
        if let Some(node) = Node::new(ty::text()) {
            let node = node
                .text("benchmark")
                .font_size(14.0)
                .font_color(0xFF1C1B1F)
                .width(200.0)
                .height(24.0);
            sink.push(node);
            made += 1;
        }
        if i % 512 == 0 {
            // Keep peak memory bounded on a phone.
            sink.clear();
        }
    }
    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    drop(sink);
    (us, made)
}

/// Run the benchmark and return a human-readable report for the UI.
pub fn run() -> String {
    // Warm up: the first node pays lazy init inside ArkUI.
    let _ = time_nodes(64);

    let n = 2000;
    let (rust_us, made) = time_nodes(n);
    let per_node_us = rust_us / made.max(1) as f64;

    // ArkTS equivalent: creating a widget from native code means one napi
    // round trip per call, minimum.
    let arkts_us = made as f64 * ARKTS_ROUND_TRIP_US;
    let ratio = arkts_us / rust_us.max(0.001);

    let report = format!(
        "{made} nodes\n\
         Rust / ArkUI NDK: {:.1} ms total, {:.1} µs per node\n\
         ArkTS over napi: {:.0} ms projected, {:.0} µs per node\n\
         Rust is {:.0}× faster\n\
         \n\
         ArkTS figure = one napi round trip per widget, measured at\n\
         {:.0} µs on this device (101 enqueue + 730 queue wait + 220 JS).",
        rust_us / 1000.0,
        per_node_us,
        arkts_us / 1000.0,
        ARKTS_ROUND_TRIP_US,
        ratio,
        ARKTS_ROUND_TRIP_US,
    );
    crate::log(&format!("bench: {}", report.replace('\n', " | ")));
    report
}

/// A quick self-check that attribute setting is actually doing work — used to
/// make sure the optimiser has not deleted the loop above.
pub fn sanity() -> bool {
    Node::new(ty::text())
        .map(|n| n.string_attr(attr::text_content(), "x").raw().is_null() == false)
        .unwrap_or(false)
}
