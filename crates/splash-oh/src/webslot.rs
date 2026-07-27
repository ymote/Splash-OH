//! Web surfaces the DSL can place, as a first-class node type.
//!
//! # Why this is not an ArkUI node
//!
//! There is no `ARKUI_NODE_WEB`. All 48 node types in `native_node.h` were
//! checked; the NDK exposes no web component at all, the same gap as video. So
//! Rust cannot create a web view the way it creates a `Text` or a `Column`, and
//! a webview in a Splash tree has to be an **ArkTS `Web` component positioned
//! on top of the native tree**.
//!
//! # How the hole is cut
//!
//! The DSL emits a `{t: "web", url: ..., w, h}` node. Rust builds a
//! transparent placeholder of exactly that size so the native layout reserves
//! the space, and records the geometry here. ArkTS reads the record and puts a
//! real `Web` at those coordinates in a `Stack` above the `ContentSlot`.
//!
//! This works because of a property this codebase already has: native ArkUI
//! nodes do not auto-size, so the DSL states every width and height explicitly.
//! That means Rust knows the geometry at build time and does not have to wait
//! for a layout pass to find out where the hole ended up.
//!
//! # What this replaces
//!
//! A build-time `YOUTUBE_MODE` flag that swapped the whole page layout for a
//! hardcoded `Web({src: 'https://www.youtube.com/embed/...'})`. That could only
//! ever be one webview, at one fixed position, with a URL ArkTS owned. A DSL
//! that cannot say *what* to load is not driving anything.

use std::cell::RefCell;

/// A web surface the DSL asked for, in vp, relative to the page.
#[derive(Clone)]
pub struct WebSlot {
    pub id: u32,
    pub url: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

thread_local! {
    /// Slots declared by the tree currently being built. Cleared at the start
    /// of every build, because a stale slot leaves a webview floating over a
    /// screen that no longer has one.
    static SLOTS: RefCell<Vec<WebSlot>> = const { RefCell::new(Vec::new()) };
    static NEXT_ID: RefCell<u32> = const { RefCell::new(1) };
}

pub fn reset() {
    SLOTS.with(|s| s.borrow_mut().clear());
}

/// Record a web surface. Returns its id, which ArkTS uses to address the
/// controller for `loadUrl` / `runJavaScript` / back-forward.
pub fn declare(url: &str, x: f32, y: f32, w: f32, h: f32) -> u32 {
    let id = NEXT_ID.with(|n| {
        let mut n = n.borrow_mut();
        let v = *n;
        *n = v.wrapping_add(1).max(1);
        v
    });
    SLOTS.with(|s| {
        s.borrow_mut().push(WebSlot {
            id,
            url: url.to_string(),
            x,
            y,
            w,
            h,
        })
    });
    id
}

pub fn slots() -> Vec<WebSlot> {
    SLOTS.with(|s| s.borrow().clone())
}

/// Serialised for the napi boundary as `id|url|x|y|w|h`.
pub fn encoded() -> Vec<String> {
    slots()
        .iter()
        .map(|s| format!("{}|{}|{}|{}|{}|{}", s.id, s.url, s.x, s.y, s.w, s.h))
        .collect()
}
