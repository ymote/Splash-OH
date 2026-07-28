//! Memory: what does one widget cost, and does it come back?
//!
//! The timing benchmark (`bench.rs`) concluded that ArkTS's extra cost is JS
//! object churn — `typeNode.createNode` builds a wrapper object, registers a
//! finalizer and wires up cross-language reference tracking, none of which the
//! NDK path has. That is a claim about *allocation*, so it predicts two things
//! that this file can check:
//!
//! 1. **ArkTS should cost more resident bytes per node**, because it holds a JS
//!    object per widget on top of the same native node.
//! 2. **ArkTS should give the memory back late, and Rust immediately** — one is
//!    a collector, the other is a `drop`.
//!
//! Both are falsifiable, which is the point of writing them down first.
//!
//! # Method
//!
//! RSS from `/proc/self/status`, sampled around each phase, on the same process
//! for both paths so there is nothing to normalise between them.
//!
//! Nodes are held in a `thread_local` — they are created on the UI thread and
//! must be destroyed there, and `Node` is full of raw ArkUI pointers, so it is
//! neither `Send` nor safe to free anywhere else.
//!
//! # What RSS can and cannot tell you
//!
//! RSS is what the process actually has resident, which is the number that
//! decides whether the device kills you. It is also noisy: the allocator does
//! not return freed pages to the kernel promptly, so a phase that frees
//! everything can still show flat RSS. That makes "memory came back" hard to
//! prove and "memory did not come back" easy to prove — asymmetric, and worth
//! remembering before reading a reclaim number as a leak.

use crate::arkui::{ty, Node};
use std::cell::RefCell;

thread_local! {
    /// Nodes deliberately kept alive so their cost shows up in RSS.
    static HELD: RefCell<Vec<Node>> = const { RefCell::new(Vec::new()) };
}

/// Resident set size in KiB, or 0 if `/proc` is not readable.
pub fn rss_kb() -> u64 {
    let Ok(s) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    s.lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Peak RSS in KiB — the high-water mark the kernel remembers, which is what
/// an out-of-memory kill would have been judged against.
pub fn peak_rss_kb() -> u64 {
    let Ok(s) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    s.lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Build `n` more nodes and hold them. Returns how many are held in total.
///
/// Four attributes each, the same node the timing benchmark builds, so the two
/// results describe the same object.
pub fn hold(n: usize) -> usize {
    HELD.with(|h| {
        let mut v = h.borrow_mut();
        v.reserve(n);
        for _ in 0..n {
            if let Some(node) = Node::new(ty::text()) {
                v.push(
                    node.text("benchmark")
                        .font_size(14.0)
                        .font_color(0xFF1C1B1F)
                        .width(200.0)
                        .height(24.0),
                );
            }
        }
        v.len()
    })
}

/// Drop everything held. Returns how many were dropped.
pub fn release() -> usize {
    HELD.with(|h| {
        let v = std::mem::take(&mut *h.borrow_mut());
        let n = v.len();
        drop(v);
        n
    })
}

/// How many are currently held.
pub fn held() -> usize {
    HELD.with(|h| h.borrow().len())
}
