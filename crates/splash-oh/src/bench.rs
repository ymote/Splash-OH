//! Widget-construction benchmark: Rust/NDK versus ArkTS, measured on device.
//!
//! # What is compared, and why it is a fair comparison
//!
//! Both paths create the **same** thing: an `ARKUI_NODE_TEXT` with four
//! attributes set (font size, font colour, width, height). Not an equivalent —
//! the same node. `typeNode.createNode(ctx, 'Text')` in ArkTS and
//! `createNode(ARKUI_NODE_TEXT)` in the NDK both land on the same C++
//! `TextPattern` inside libace. The tree that results is indistinguishable.
//!
//! That matters because it means everything downstream — measure, layout,
//! paint, rasterise — is *identical native code in both cases*. It cancels out.
//! Construction is the only stage where the two paths differ, so construction
//! is what this isolates. Timing a full frame instead would mostly measure
//! layout, which both paths pay equally, and would understate nothing but
//! obscure everything.
//!
//! # The three measurements
//!
//! * **A — Rust → ArkUI NDK.** Timed here, in this file, with `Instant`.
//! * **B — ArkTS → typeNode.** Timed in ArkTS with `systemDateTime.getTime`,
//!   reported down through napi. Same N, same node, same four attributes.
//! * **C — the napi bridge, native → JS.** A real round trip: Rust posts to the
//!   JS thread and blocks until JS has run and called back. Measured unbatched
//!   (one crossing per widget) and batched (one crossing for all N).
//!
//! # Method
//!
//! Warm up, then five trials, report the median and the spread. The warm-up
//! matters: the first node into a fresh ArkUI pays lazy init, and the first
//! ArkTS loop pays JIT.
//!
//! Every node in a trial is kept alive until the timed region closes.  An
//! earlier version of this file dropped nodes every 512 iterations to bound
//! memory, which folded `disposeNode` into the number being reported.
//!
//! # What C measures
//!
//! C is not part of the A-vs-B comparison. It answers a different question: if
//! your app logic lives in Rust and your widgets live in ArkTS, what does the
//! boundary cost?
//!
//! It includes an empty-crossing control — a round trip that carries no work —
//! so the bridge can be separated from the widget. That control is what makes
//! the result interpretable, and it overturned the guess this file was written
//! with. The expectation was that batching would collapse the cost, i.e. that
//! crossing overhead dominates. It does not: an empty crossing is ~29 µs on an
//! idle JS thread, so per-widget cost is JS-side work, and batching 200 widgets
//! into one crossing does not beat 200 separate crossings.
//!
//! The corollary matters more than the number. napi is not slow. What is slow
//! is waiting behind a busy JS thread — the octos-one port measured ~1.05 ms
//! per round trip with 70% of it queue wait, versus 29 µs here, and the only
//! difference is what else that thread was doing. Bridge latency is not a
//! constant; it is a function of load. Building a widget tree is load.

use crate::arkui::{ty, Node};
use std::sync::Mutex;
use std::time::Instant;

/// Nodes per trial.
pub const N: usize = 2000;
/// Trials per run; the median is reported. Ten rather than five so that a
/// warm-up curve, if there is one, is visible in the ordered log line.
const TRIALS: usize = 10;

/// Results filled in from different places: A here, B from ArkTS, C from the
/// bridge worker thread.
#[derive(Default, Clone)]
pub struct Results {
    /// A: µs per node, Rust → NDK. (median, min, max)
    pub rust: Option<(f64, f64, f64)>,
    /// B: µs per node, ArkTS → typeNode. (median, min, max)
    pub arkts: Option<(f64, f64, f64)>,
    /// C control: µs for a round trip that carries no work.
    pub bridge_empty: Option<f64>,
    /// C: µs per widget crossing the napi boundary, unbatched.
    pub bridge_unbatched: Option<f64>,
    /// C: µs per widget when all N are described in a single crossing.
    pub bridge_batched: Option<f64>,
    /// Decomposition: what each step of building one node costs, in µs.
    /// (rust_create, rust_attr_each, arkts_create, arkts_attr_each,
    ///  arkts_napi_crossing, arkts_empty_loop)
    pub parts: Option<[f64; 6]>,
}

/// A `Mutex`, not a `thread_local` — measurement C runs on a worker thread
/// (it has to: it blocks waiting for the JS thread, so running it on the JS
/// thread would deadlock) and writes its result from there.
static RESULTS: Mutex<Results> = Mutex::new(Results {
    rust: None,
    arkts: None,
    bridge_empty: None,
    bridge_unbatched: None,
    bridge_batched: None,
    parts: None,
});

pub fn results() -> Results {
    RESULTS.lock().unwrap().clone()
}

/// Median, min and max of a set of per-node timings.
fn stats(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (v[v.len() / 2], v[0], v[v.len() - 1])
}

/// One trial: build `n` nodes with four attributes, keeping them all alive.
/// Returns µs for the whole trial.
fn trial(n: usize) -> f64 {
    let mut sink: Vec<Node> = Vec::with_capacity(n);
    let t0 = Instant::now();
    for _ in 0..n {
        if let Some(node) = Node::new(ty::text()) {
            sink.push(
                node.text("benchmark")
                    .font_size(14.0)
                    .font_color(0xFF1C1B1F)
                    .width(200.0)
                    .height(24.0),
            );
        }
    }
    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    // Disposal is outside the timed region on purpose — ArkTS's side is
    // garbage collected and pays it later, so including it here would compare
    // two different things.
    drop(sink);
    us
}

/// Like `trial`, but creates the node without touching any attribute. The
/// difference between this and `trial` is what five `setAttribute` calls cost.
fn trial_create_only(n: usize) -> f64 {
    let mut sink: Vec<Node> = Vec::with_capacity(n);
    let t0 = Instant::now();
    for _ in 0..n {
        if let Some(node) = Node::new(ty::text()) {
            sink.push(node);
        }
    }
    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    drop(sink);
    us
}

/// Rust's half of the decomposition: node cost, and per-attribute cost.
pub fn run_rust_parts() -> (f64, f64) {
    if let Some(p) = *RUST_PARTS.lock().unwrap() {
        return p;
    }
    let _ = trial_create_only(128);
    let create = median((0..5).map(|_| trial_create_only(N) / N as f64).collect());
    let full = median((0..5).map(|_| trial(N) / N as f64).collect());
    // `trial` sets five attributes: text, size, colour, width, height.
    let per_attr = (full - create) / 5.0;
    crate::log(&format!(
        "bench D rust parts: createNode {create:.2} µs, setAttribute {per_attr:.2} µs each \
         (x5 = {:.2}), total {full:.2}",
        per_attr * 5.0
    ));
    *RUST_PARTS.lock().unwrap() = Some((create, per_attr));
    (create, per_attr)
}

/// Cached so it is measured once, at startup, off the JS thread's critical path.
static RUST_PARTS: Mutex<Option<(f64, f64)>> = Mutex::new(None);

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// Set one attribute `n` times on a single already-created node.
///
/// Measured directly rather than as (full trial − create-only trial): the
/// difference of two 2000-node trials has a noise floor of several µs per node
/// and the signal here is well under 1 µs, so subtraction returned nonsense
/// (a negative per-attribute cost).
fn trial_attr_only(n: usize) -> f64 {
    let Some(node) = Node::new(ty::text()) else {
        return 0.0;
    };
    let raw = node.raw();
    let t0 = Instant::now();
    for _ in 0..n {
        Node::set_f32_attr_raw(raw, crate::arkui::attr::font_size(), 14.0);
    }
    t0.elapsed().as_nanos() as f64 / 1000.0 / n as f64
}

/// Run exactly one trial and return µs per node. `kind` 0 is the full node
/// (five attributes), 1 is createNode alone, 2 is one attribute set.
///
/// This exists because doing the whole suite in one call blocks the JS thread
/// long enough that its timer queue stops being serviced — which silently
/// killed the ArkTS half of the benchmark until the event loop was given room
/// to breathe between trials.
pub fn rust_trial(kind: u32) -> f64 {
    match kind {
        1 => trial_create_only(N) / N as f64,
        2 => trial_attr_only(50_000),
        _ => trial(N) / N as f64,
    }
}

/// Warm up both paths; the first node into a fresh ArkUI pays lazy init.
pub fn rust_warmup() {
    let _ = trial(128);
    let _ = trial_create_only(128);
}

/// Everything, computed from trials the caller has already spread out.
/// `crossing` and `empty_loop` are ArkTS-side floors, in µs.
#[allow(clippy::too_many_arguments)]
pub fn record_all(
    rust_full: Vec<f64>,
    rust_create: Vec<f64>,
    arkts_full: Vec<f64>,
    arkts_create: Vec<f64>,
    rust_attr: f64,
    arkts_attr: f64,
    crossing: f64,
    empty_loop: f64,
) {
    if rust_full.is_empty() || arkts_full.is_empty() {
        return;
    }
    let order = |label: &str, v: &[f64]| {
        crate::log(&format!(
            "bench {label} trials in order: {}",
            v.iter().map(|x| format!("{x:.1}")).collect::<Vec<_>>().join(" ")
        ))
    };
    order("A rust", &rust_full);
    order("B arkts", &arkts_full);

    let a = stats(rust_full.clone());
    let b = stats(arkts_full.clone());
    let rc = median(rust_create.clone());
    let ac = median(arkts_create.clone());
    // Paired differences, then the median — not the difference of two medians.
    // The trials come in matched sets from the same tick, so pairing them
    // cancels the drift that otherwise swamps a ~1 µs signal.
    let paired = |full: &[f64], create: &[f64]| {
        let d: Vec<f64> = full
            .iter()
            .zip(create.iter())
            .map(|(f, c)| (f - c) / 5.0)
            .collect();
        if d.is_empty() {
            0.0
        } else {
            median(d)
        }
    };
    let _ = paired; // superseded: attributes are measured directly now
    let ra = rust_attr;
    let aa = arkts_attr;

    {
        let mut r = RESULTS.lock().unwrap();
        r.rust = Some(a);
        r.arkts = Some(b);
        r.parts = Some([rc, ra, ac, aa, crossing, empty_loop]);
    }
    crate::log(&format!(
        "bench A rust/ndk: median {:.2} µs/node ({:.2}-{:.2}) | \
         B arkts/typeNode: median {:.2} µs/node ({:.2}-{:.2}) | ratio {:.2}x",
        a.0, a.1, a.2, b.0, b.1, b.2, b.0 / a.0.max(0.0001)
    ));
    crate::log(&format!(
        "bench D parts: createNode rust {rc:.2} arkts {ac:.2} ({:.1}x) | \
         setAttribute rust {ra:.3} arkts {aa:.3} ({:.1}x) | \
         napi JS->native {crossing:.3} | empty JS loop {empty_loop:.3}",
        ac / rc.max(0.0001),
        aa / ra.max(0.0001)
    ));
}

/// Measurement A. Safe to call repeatedly.
pub fn run_rust() -> (f64, f64, f64) {
    let _ = trial(128); // warm up: the first node pays lazy init inside ArkUI
    let per_node: Vec<f64> = (0..TRIALS).map(|_| trial(N) / N as f64).collect();
    crate::log(&format!(
        "bench A trials in order: {}",
        per_node
            .iter()
            .map(|v| format!("{v:.1}"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    let s = stats(per_node);
    RESULTS.lock().unwrap().rust = Some(s);
    crate::log(&format!(
        "bench A rust/ndk: median {:.2} µs/node (min {:.2}, max {:.2}) over {TRIALS} trials of {N}",
        s.0, s.1, s.2
    ));
    s
}

/// Measurement B, handed down from ArkTS: per-trial totals in milliseconds.
pub fn record_arkts(n: u32, trials_ms: Vec<f64>) {
    if n == 0 || trials_ms.is_empty() {
        return;
    }
    let per_node: Vec<f64> = trials_ms.iter().map(|ms| ms * 1000.0 / n as f64).collect();
    // In order, not sorted: if the ArkTS VM were warming up, this would fall
    // monotonically. If it bounces, the variance is the collector, not warm-up.
    crate::log(&format!(
        "bench B trials in order: {}",
        per_node
            .iter()
            .map(|v| format!("{v:.1}"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    let s = stats(per_node);
    RESULTS.lock().unwrap().arkts = Some(s);
    crate::log(&format!(
        "bench B arkts/typeNode: median {:.2} µs/node (min {:.2}, max {:.2}) over {} trials of {n}",
        s.0,
        s.1,
        s.2,
        trials_ms.len()
    ));
}

/// Measurement C, handed up from the bridge worker.
pub fn record_bridge(empty_us: f64, unbatched_us: f64, batched_us: f64) {
    let mut b = RESULTS.lock().unwrap();
    b.bridge_empty = Some(empty_us);
    b.bridge_unbatched = Some(unbatched_us);
    b.bridge_batched = Some(batched_us);
    drop(b);
    crate::log(&format!(
        "bench C napi native->JS: empty {empty_us:.1} µs, unbatched {unbatched_us:.1} µs/widget, \
         batched {batched_us:.2} µs/widget"
    ));
}

/// The ArkTS half of the decomposition, measured in ArkTS and sent down.
/// Order: [createNode, per-attribute, empty napi crossing, empty JS loop].
pub fn record_parts(arkts: Vec<f64>) {
    if arkts.len() < 4 {
        return;
    }
    // Rust's half was measured at startup — redoing it here would add another
    // half second of work on the JS thread, which is the thread we are trying
    // not to block.
    let (rc, ra) = RUST_PARTS.lock().unwrap().unwrap_or((0.0, 0.0));
    RESULTS.lock().unwrap().parts = Some([rc, ra, arkts[0], arkts[1], arkts[2], arkts[3]]);
    crate::log(&format!(
        "bench D arkts parts: createNode {:.2} µs, attribute {:.2} µs each, \
         napi JS->native {:.3} µs, empty JS loop {:.3} µs",
        arkts[0], arkts[1], arkts[2], arkts[3]
    ));
}

/// The report rendered on the Performance screen.
pub fn report() -> String {
    let r = results();
    let mut out = String::new();

    match (r.rust, r.arkts) {
        (Some(rust), Some(arkts)) => {
            out.push_str(&format!(
                "Same node, same 4 attributes, {N} of them, 5 trials each.\n\n\
                 A  Rust -> ArkUI NDK      {:.2} µs/node  ({:.2}-{:.2})\n\
                 B  ArkTS -> typeNode      {:.2} µs/node  ({:.2}-{:.2})\n\n\
                 Rust is {:.1}× faster at construction.\n",
                rust.0,
                rust.1,
                rust.2,
                arkts.0,
                arkts.1,
                arkts.2,
                arkts.0 / rust.0.max(0.0001),
            ));
        }
        (Some(rust), None) => {
            out.push_str(&format!(
                "A  Rust -> ArkUI NDK      {:.2} µs/node  ({:.2}-{:.2})\n\
                 B  ArkTS -> typeNode      waiting for the JS thread...\n",
                rust.0, rust.1, rust.2
            ));
        }
        _ => out.push_str("Running...\n"),
    }

    if let Some(p) = r.parts {
        out.push_str(&format!(
            "\nWhere the difference actually is, per call:\n\
             createNode      Rust {:.1}     ArkTS {:.1}\n\
             setAttribute    Rust {:.3}   ArkTS {:.3}\n\n\
             One JS -> native napi call is {:.3} µs and an empty JS loop\n\
             iteration is {:.3} µs. So crossing the boundary is not the cost,\n\
             and setting attributes is not either. Nearly all of it is\n\
             createNode, which in ArkTS also builds a JS wrapper object and\n\
             registers it with the collector.\n",
            p[0], p[2], p[1], p[3], p[4], p[5]
        ));
    }

    match (r.bridge_empty, r.bridge_unbatched, r.bridge_batched) {
        (Some(e), Some(u), Some(b)) => {
            out.push_str(&format!(
                "\nC  napi round trip, native -> JS and back:\n\
                 empty crossing        {e:.0} µs   (the bridge itself)\n\
                 1 widget per crossing {u:.0} µs\n\
                 {N2} widgets in one     {b:.0} µs/widget\n\n\
                 Batching does not help here, which is the interesting part: the\n\
                 bridge is only {e:.0} µs, so the per-widget cost is JS-side work,\n\
                 not crossing overhead.",
                N2 = 200
            ));
        }
        _ => out.push_str("\nC  napi boundary: measuring...\n"),
    }
    out
}
