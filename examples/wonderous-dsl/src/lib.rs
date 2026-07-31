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
use wonderous::data::{Anchor, WONDERS};
use wonderous::artifact_data::ARTIFACTS;
use wonderous::places::{COLLECTIBLES, PLACES};
use wonderous::details::GALLERY_PHOTOS;
use wonderous::editorial_data::EDITORIAL;
use wonderous::screens::INTRO;
use wonderous::timeline_data::TIMELINES;

const SRC: &str = include_str!("../assets/wonderous.splash");

/// Seconds since the arm was first built.
fn elapsed() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

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
    // The wonder's illustration, as raw layer data. The script does the
    // placement arithmetic -- anchor, height factor, fractional offsets -- as
    // it should: baking the geometry here would leave the DSL arm drawing a
    // layout Rust had already decided, which is the thing this arm exists to
    // avoid.
    s.push_str(&format!("let w_bg = {}\n", wo.bg));
    s.push_str(&format!("let w_fg = {}\n", wo.fg));
    s.push_str(&format!("let w_line1 = \"{}\"\n", esc(wo.line1)));
    s.push_str(&format!("let w_line2 = \"{}\"\n", esc(wo.line2)));
    s.push_str(&format!("let w_article = \"{}\"\n", esc(wo.article)));
    s.push_str(&format!("let w_em1 = {}\n", wo.em1));
    s.push_str(&format!("let w_em2 = {}\n", wo.em2));
    s.push_str(&format!("let w_count = {}\n", WONDERS.len()));
    s.push_str(&format!(
        "let w_texture = \"resource://RAWFILE/wonders/{}/texture.png\"\n",
        esc(wo.dir)
    ));
    s.push_str("let pieces = [\n");
    for p in wo.pieces {
        let anchor = match p.anchor {
            Anchor::Center => 0,
            Anchor::TopLeft => 1,
            Anchor::TopCenter => 2,
            Anchor::TopRight => 3,
            Anchor::BottomLeft => 4,
            Anchor::BottomCenter => 5,
            Anchor::BottomRight => 6,
        };
        // `fg` marks the layers that overhang the title. Chichen Itza's two are
        // not named `foreground-*`, but that is where the app draws them.
        let fg = i32::from(
            p.file.starts_with("foreground") || p.file == "top-left.png" || p.file == "top-right.png",
        );
        s.push_str(&format!(
            "  [\"resource://RAWFILE/wonders/{}/{}\", {}, {}, {}, {}, {}, {}, {}, {}, {}],\n",
            esc(wo.dir),
            esc(p.file),
            p.aspect,
            p.height_factor,
            p.min_h,
            anchor,
            p.frac_x,
            p.frac_y,
            p.off_x,
            p.off_y,
            fg
        ));
    }
    s.push_str("]\n");

    // What the details tabs need. Bound per screen rather than wholesale: the
    // editorial text alone runs to tens of kilobytes a wonder, and the script
    // is re-parsed on every build.
    let e = &EDITORIAL[wonder % EDITORIAL.len()];
    s.push_str(&format!("let e_sub = \"{}\"\n", esc(e.sub_title)));
    s.push_str(&format!("let e_region = \"{}\"\n", esc(e.region)));
    s.push_str("let e_paras = [\n");
    for para in [
        e.history1,
        e.history2,
        e.construction1,
        e.construction2,
        e.location1,
        e.location2,
    ] {
        s.push_str(&format!("  \"{}\",\n", esc(para)));
    }
    s.push_str("]\n");
    s.push_str(&format!("let e_quote = \"{}\"\n", esc(e.quote_top)));
    s.push_str(&format!("let e_quote2 = \"{}\"\n", esc(e.quote_bottom)));
    s.push_str(&format!("let e_author = \"{}\"\n", esc(e.quote_author)));

    s.push_str("let artifacts = [\n");
    for a in ARTIFACTS[wonder % ARTIFACTS.len()] {
        s.push_str(&format!(
            "  [\"{}\", \"{}\", \"{}\", \"{}\"],\n",
            esc(a.id),
            esc(a.title),
            esc(a.date),
            esc(a.culture)
        ));
    }
    s.push_str("]\n");

    s.push_str("let events = [\n");
    for ev in TIMELINES[wonder % TIMELINES.len()].events {
        s.push_str(&format!("  [{}, \"{}\"],\n", ev.year, esc(ev.text)));
    }
    s.push_str("]\n");

    s.push_str(&format!("let gallery_n = {}\n", GALLERY_PHOTOS));
    s.push_str(&format!(
        "let gallery_base = \"resource://RAWFILE/wonders/{}/gallery/\"\n",
        esc(wo.dir)
    ));
    s.push_str("let icon_base = \"resource://RAWFILE/wonders/_common/icons/\"\n");

    // All eight, for the menu, and the twenty-four collectibles for the
    // collection. Both are short lists that every screen after the home may
    // need, so they are bound whatever screen is current.
    s.push_str("let all_wonders = [\n");
    for w in WONDERS {
        s.push_str(&format!(
            "  [\"{}\", \"{}\", {}],\n",
            esc(w.title),
            esc(w.region),
            w.bg
        ));
    }
    s.push_str("]\n");
    s.push_str("let collectibles = [\n");
    for (i, c) in COLLECTIBLES.iter().enumerate() {
        s.push_str(&format!(
            "  [\"{}\", {}, \"resource://RAWFILE/wonders/_common/collectibles/{}.png\", {}],\n",
            esc(c.title),
            c.wonder,
            esc(c.icon),
            i32::from(wonderous::collectibles::is_found(i))
        ));
    }
    s.push_str("]\n");
    s.push_str(&format!("let found_n = {}\n", wonderous::collectibles::found_count()));
    // The clock. The DSL has no tween and no controller, so the only thing that
    // can make a screen move is re-evaluating it against a changing value --
    // the same way the flutter kit animates. `wonderousDslTick` drives it.
    s.push_str(&format!("let t = {:.3}\n", elapsed()));
    s.push_str(&format!("let cloud_seed = {}\n", wo.cloud_seed));
    s.push_str(&format!("let art_sel = {}\n", ART_SEL.load(Relaxed)));
    s.push_str(&format!("let photo_sel = {}\n", PHOTO_SEL.load(Relaxed)));
    // The film and the map. YouTube's mobile watch page rather than an embed:
    // /embed gives the player no page origin and it answers error 153.
    let (video, lat, lng) = PLACES[wonder % PLACES.len()];
    s.push_str(&format!(
        "let video_src = \"https://m.youtube.com/watch?v={}\"\n",
        esc(video)
    ));
    let d = 0.004;
    s.push_str(&format!(
        "let map_src = \"https://www.openstreetmap.org/export/embed.html?bbox={},{},{},{}&layer=mapnik&marker={},{}\"\n",
        lng - d, lat - d, lng + d, lat + d, lat, lng
    ));
    s
}

/// Build the screen the app is on.
pub fn build(screen: i32, wonder: usize, tab: i32, page: i32, w: f32, h: f32) -> Option<Node> {
    // The previous tree's handles die with it, and starting a drift on a freed
    // node is a use-after-free rather than a missing animation.
    splash_oh_arkui::dsl::clear_drifts();
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
static ART_SEL: AtomicI32 = AtomicI32::new(0);
/// The middle of the 5x5 wall, which is where the app opens it.
static PHOTO_SEL: AtomicI32 = AtomicI32::new(13);

/// Tap ids. The same numbers the other two arms use, so a tour written against
/// one drives all three.
pub const T_INTRO_NEXT: i32 = 7300;
pub const T_INTRO_ENTER: i32 = 7301;

pub const S_HOME: i32 = 1;
pub const T_MENU: i32 = 7103;
pub const T_PREV: i32 = 7101;
pub const T_NEXT: i32 = 7102;
pub const T_DETAILS: i32 = 7104;
pub const S_DETAILS: i32 = 2;
pub const T_TAB: i32 = 7200;
pub const T_HOME: i32 = 7210;
pub const S_MENU: i32 = 3;
pub const S_COLLECTION: i32 = 4;
pub const S_TIMELINE: i32 = 5;
pub const S_SEARCH: i32 = 6;
pub const S_ARTIFACT: i32 = 7;
pub const S_PHOTO: i32 = 8;
pub const S_FOUND: i32 = 9;
pub const T_MENU_CLOSE: i32 = 7310;
pub const T_MENU_BASE: i32 = 7320;
pub const T_CLOSE: i32 = 7340;
pub const T_COLLECTION: i32 = 7341;
pub const T_TIMELINE: i32 = 7342;
pub const T_BROWSE: i32 = 7372;
pub const T_ART_OPEN: i32 = 7373;
pub const T_PHOTO_OPEN: i32 = 7375;
pub const T_VIEWER_CLOSE: i32 = 7450;
pub const T_VIEWER_PREV: i32 = 7451;
pub const T_VIEWER_NEXT: i32 = 7452;
pub const T_COLLECT: i32 = 7460;
pub const T_FOUND_CLOSE: i32 = 7490;
pub const T_VIDEO: i32 = 7454;
pub const T_MAPS: i32 = 7453;
pub const T_HOME_SWIPE: i32 = 7150;
pub const T_SCROLLED: i32 = 7160;
pub const T_PHOTO_SWIPE: i32 = 7170;
pub const S_VIDEO: i32 = 10;
pub const S_MAPS: i32 = 11;

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
        T_PREV => {
            let n = WONDERS.len() as i32;
            WONDER.store((WONDER.load(Relaxed) + n - 1) % n, Relaxed);
            true
        }
        T_NEXT => {
            let n = WONDERS.len() as i32;
            WONDER.store((WONDER.load(Relaxed) + 1) % n, Relaxed);
            true
        }
        T_DETAILS => {
            SCREEN.store(S_DETAILS, Relaxed);
            TAB.store(0, Relaxed);
            true
        }
        T_HOME => {
            SCREEN.store(S_HOME, Relaxed);
            true
        }
        id if (T_TAB..T_TAB + 4).contains(&id) => {
            TAB.store(id - T_TAB, Relaxed);
            true
        }
        T_MENU => {
            SCREEN.store(S_MENU, Relaxed);
            true
        }
        T_MENU_CLOSE => {
            SCREEN.store(S_HOME, Relaxed);
            true
        }
        // Closing a viewer goes back to whatever opened it. The details screen
        // is the only thing that opens one, so that is where it returns.
        T_CLOSE | T_VIEWER_CLOSE | T_FOUND_CLOSE => {
            SCREEN.store(S_DETAILS, Relaxed);
            true
        }
        T_COLLECTION => {
            SCREEN.store(S_COLLECTION, Relaxed);
            true
        }
        T_TIMELINE => {
            SCREEN.store(S_TIMELINE, Relaxed);
            true
        }
        T_BROWSE => {
            SCREEN.store(S_SEARCH, Relaxed);
            true
        }
        T_ART_OPEN => {
            SCREEN.store(S_ARTIFACT, Relaxed);
            true
        }
        T_PHOTO_OPEN => {
            SCREEN.store(S_PHOTO, Relaxed);
            true
        }
        T_VIDEO => {
            SCREEN.store(S_VIDEO, Relaxed);
            true
        }
        T_MAPS => {
            SCREEN.store(S_MAPS, Relaxed);
            true
        }
        // The article moved. Nothing is rebuilt: the hero's translate is set
        // straight on the mounted node, so this runs at whatever rate the
        // scroll reports rather than at a rebuild's.
        // The wall: a drag moves the selection one cell, and the wall slides
        // so the new one is centred. left/right step by one, up/down by a row.
        id if id > T_PHOTO_SWIPE && id <= T_PHOTO_SWIPE + 4 => {
            const GRID: i32 = 5;
            let n = GRID * GRID;
            let d = id - T_PHOTO_SWIPE;
            let sel = PHOTO_SEL.load(Relaxed);
            let (col, row) = (sel % GRID, sel / GRID);
            let (nc, nr) = match d {
                1 => ((col + 1).min(GRID - 1), row),
                2 => ((col - 1).max(0), row),
                3 => (col, (row + 1).min(GRID - 1)),
                _ => (col, (row - 1).max(0)),
            };
            PHOTO_SEL.store((nr * GRID + nc).rem_euclid(n), Relaxed);
            true
        }
        T_SCROLLED => {
            unsafe { splash_oh_arkui::dsl::apply_parallax() };
            return None;
        }
        // The shim reports a drag as base + 1..4: left, right, up, down.
        // Swiping left brings the next wonder in, as it does in the app.
        id if id == T_HOME_SWIPE + 1 || id == T_HOME_SWIPE + 2 => {
            let n = WONDERS.len() as i32;
            let step = if id == T_HOME_SWIPE + 1 { 1 } else { n - 1 };
            WONDER.store((WONDER.load(Relaxed) + step) % n, Relaxed);
            true
        }
        id if id == T_HOME_SWIPE + 3 => {
            // Up, from the home screen, opens the details -- the app's own
            // gesture for it.
            SCREEN.store(S_DETAILS, Relaxed);
            TAB.store(0, Relaxed);
            true
        }
        T_VIEWER_PREV => {
            let n = GALLERY_PHOTOS as i32;
            PHOTO_SEL.store((PHOTO_SEL.load(Relaxed) + n - 1) % n, Relaxed);
            true
        }
        T_VIEWER_NEXT => {
            let n = GALLERY_PHOTOS as i32;
            PHOTO_SEL.store((PHOTO_SEL.load(Relaxed) + 1) % n, Relaxed);
            true
        }
        id if (T_MENU_BASE..T_MENU_BASE + 8).contains(&id) => {
            WONDER.store(id - T_MENU_BASE, Relaxed);
            SCREEN.store(S_HOME, Relaxed);
            true
        }
        id if (T_COLLECT..T_COLLECT + 24).contains(&id) => {
            wonderous::collectibles::discover((id - T_COLLECT) as usize);
            ART_SEL.store(id - T_COLLECT, Relaxed);
            SCREEN.store(S_FOUND, Relaxed);
            true
        }
        _ => false,
    };
    if !handled {
        splash_oh_arkui::log(&format!("wonderous/dsl: unhandled tap {id}"));
        return None;
    }
    splash_oh_arkui::log(&format!(
        "wonderous/dsl: tap {id} -> screen {} wonder {} tab {} page {}",
        SCREEN.load(Relaxed),
        WONDER.load(Relaxed),
        TAB.load(Relaxed),
        PAGE.load(Relaxed)
    ));
    current()
}

/// Whether the current screen should be re-described on the clock.
///
/// The home screen is: its clouds drift. Everything else is at rest.
///
/// # What the cadence costs, measured rather than assumed
///
/// The DSL has no tween and no way to say that one node changed, so movement
/// means describing the whole screen again -- illustration layers included.
/// At 33 ms that is destructive: an ArkUI image node recreated every frame
/// never lives long enough to decode, and the clouds drifted beautifully over
/// a wonder that had vanished. At 100 ms the illustration survives intact.
///
/// So the rebuild path tops out around 10 frames a second here, which is not
/// what the original does. Matching it properly means not rebuilding at all:
/// `arkui::animate` and the `translate` attribute exist, and a node given a
/// drift could be interpolated natively at 60 without the tree being touched.
/// That needs the walker to hand mounted node handles back to the host, which
/// it currently does not -- a real addition to the DSL rather than a tuning of
/// this app.
pub fn animates() -> bool {
    // Nothing re-describes itself any more. Motion is native now: `drift`
    // hands a node to ArkUI and it interpolates at the display's rate, so the
    // tick only has to restart a cycle when one finishes.
    false
}

/// How many drifts the current tree declared.
pub fn drift_count() -> usize {
    splash_oh_arkui::dsl::drift_count()
}

/// Start the drifts the current tree declared, and return how many.
///
/// # Safety
/// Must be called with that tree mounted.
pub unsafe fn start_drifts() -> usize {
    splash_oh_arkui::dsl::start_drifts()
}

/// Put the app back at the start, for a fresh mount.
pub fn reset() {
    SCREEN.store(S_INTRO, Relaxed);
    WONDER.store(0, Relaxed);
    TAB.store(0, Relaxed);
    PAGE.store(0, Relaxed);
    PHOTO_SEL.store(13, Relaxed);
}
