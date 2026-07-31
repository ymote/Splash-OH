//! Wonderous described in the Splash DSL.
//!
//! The third arm, and the one that answers a question the other two cannot.
//! `examples/wonderous` is hand-written Rust calling the ArkUI NDK;
//! `WonderousArkTs.ets` is ArkTS building the same widgets through typeNode.
//! Both describe the interface in a general-purpose language. This one
//! describes it as data, evaluated on device by the Splash VM.
//!
//! # What lives where
//!
//! The script owns the layout: the arithmetic, the per-screen dispatch, the
//! loops. This module owns only the seam — it binds where the app is and what
//! the current screen needs, evaluates, and maps taps back to state.
//!
//! # Why the data is injected rather than written in the script
//!
//! The DSL source is parsed and evaluated on every build. The wonders' text
//! runs to hundreds of kilobytes; embedding it would mean re-parsing all of it
//! to draw one screen. Binding just the slice the screen needs keeps each
//! evaluation proportional to what is on it. The tables come from the Rust arm
//! rather than a copy, so the two cannot drift.

use splash_oh_arkui::arkui::Node;
use wonderous::data::WONDERS;
use wonderous::screens::INTRO;

const SRC: &str = include_str!("../assets/wonderous.splash");

/// Screen ids, matching the other two arms.
pub const S_INTRO: i32 = 0;

/// Escape a string for a DSL double-quoted literal.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Everything the script needs to know, as `let` bindings prepended to it.
///
/// The same shape `dsl::build_screen` uses for the catalog: there is no scope
/// object in this VM, so host state arrives as ordinary top-level lets.
fn prelude(screen: i32, wonder: usize, tab: i32, page: i32, w: f32, h: f32) -> String {
    let wo = &WONDERS[wonder % WONDERS.len()];
    let p = &INTRO[(page as usize).min(INTRO.len() - 1)];
    let mut s = String::new();
    s.push_str(&format!("let screen = {screen}\n"));
    s.push_str(&format!("let wonder = {wonder}\n"));
    s.push_str(&format!("let tab = {tab}\n"));
    s.push_str(&format!("let page = {page}\n"));
    s.push_str(&format!("let W = {w}\n"));
    s.push_str(&format!("let H = {h}\n"));
    // The wonder on screen.
    s.push_str(&format!("let w_title = \"{}\"\n", esc(wo.title)));
    s.push_str(&format!("let w_region = \"{}\"\n", esc(wo.region)));
    s.push_str(&format!("let w_dir = \"{}\"\n", esc(wo.dir)));
    // The intro page on screen.
    s.push_str(&format!("let intro_count = {}\n", INTRO.len()));
    s.push_str(&format!("let intro_title = \"{}\"\n", esc(p.0)));
    s.push_str(&format!("let intro_body = \"{}\"\n", esc(p.1)));
    // The walker hands `src` to the image node untouched, so the rawfile
    // scheme belongs here rather than in the script -- a screen should not have
    // to know how this platform names its bundled assets.
    s.push_str(&format!(
        "let intro_photo = \"resource://RAWFILE/wonders/_common/{}\"\n",
        esc(p.2)
    ));
    s
}

/// Build the screen the app is on.
pub fn build(screen: i32, wonder: usize, tab: i32, page: i32, w: f32, h: f32) -> Option<Node> {
    let src = format!("{}{}", prelude(screen, wonder, tab, page, w, h), SRC);
    splash_oh_arkui::dsl::build(&src)
}

/// How many nodes the last build produced, and how long the whole thing took.
///
/// Reported rather than inferred: a script that fails to evaluate returns
/// nothing in microseconds, which reads as a spectacular result if the count
/// is not checked. It cost one wrong measurement to learn that.
pub fn build_timed(
    screen: i32,
    wonder: usize,
    tab: i32,
    page: i32,
    w: f32,
    h: f32,
) -> (Option<Node>, usize, u128) {
    splash_oh_arkui::ui::reset_count();
    let t = std::time::Instant::now();
    let node = build(screen, wonder, tab, page, w, h);
    let us = t.elapsed().as_micros();
    (node, splash_oh_arkui::ui::count(), us)
}

// ---- where the app is ---------------------------------------------------
//
// Held here rather than passed in, because a tap arrives as a bare id through
// `app::Router` and has to be able to move the app on its own.

use std::sync::atomic::{AtomicI32, Ordering::Relaxed};

static SCREEN: AtomicI32 = AtomicI32::new(S_INTRO);
static WONDER: AtomicI32 = AtomicI32::new(0);
static TAB: AtomicI32 = AtomicI32::new(0);
static PAGE: AtomicI32 = AtomicI32::new(0);

/// Tap ids. The same numbers the other two arms use, so a tour written against
/// one drives all three.
pub const T_INTRO_NEXT: i32 = 7300;
pub const T_INTRO_ENTER: i32 = 7301;

pub const S_HOME: i32 = 1;

const W: f32 = 406.15;
const H: f32 = 805.23;

/// Build whatever the state says is current.
pub fn current() -> Option<Node> {
    build(
        SCREEN.load(Relaxed),
        WONDER.load(Relaxed) as usize,
        TAB.load(Relaxed),
        PAGE.load(Relaxed),
        W,
        H,
    )
}

/// A tap. Returns the new tree, or `None` if the id was not ours -- the caller
/// leaves the screen alone rather than blanking it.
pub fn route(id: i32) -> Option<Node> {
    let handled = match id {
        T_INTRO_NEXT => {
            PAGE.fetch_add(1, Relaxed);
            true
        }
        T_INTRO_ENTER => {
            SCREEN.store(S_HOME, Relaxed);
            PAGE.store(0, Relaxed);
            true
        }
        _ => false,
    };
    if !handled {
        splash_oh_arkui::log(&format!("wonderous/dsl: unhandled tap {id}"));
        return None;
    }
    splash_oh_arkui::log(&format!(
        "wonderous/dsl: tap {id} -> screen {} page {}",
        SCREEN.load(Relaxed),
        PAGE.load(Relaxed)
    ));
    current()
}

/// Put the app back at the start, for a fresh mount.
pub fn reset() {
    SCREEN.store(S_INTRO, Relaxed);
    WONDER.store(0, Relaxed);
    TAB.store(0, Relaxed);
    PAGE.store(0, Relaxed);
}
