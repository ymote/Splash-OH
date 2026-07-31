//! The flutter/samples catalog, built in Rust straight into ArkUI nodes.
//!
//! This is the ArkUI *implementation* of the catalog, not a rendering of a
//! shared description of it. There used to be one `.splash` kit authored in
//! Splash-Makepad and vendored here, walked by `dsl.rs` — one source, two
//! backends. That bought consistency at the price of every screen being written
//! to the intersection of what both toolkits can express, and the seams showed:
//! eleven `pick_if(st.backend == "arkui", …)` branches, a `restfill()` helper
//! that had to be 1 here and 0 there, and three picker screens drawing fake
//! wheels because the *other* backend has no picker.
//!
//! So the two are separate now. This half is free to use ArkUI's own widgets
//! with no regard for what makepad can do, in the same idiom as the four ported
//! apps beside it (`taobao.rs`, `tiktok.rs`, `wonderous.rs`, `wechat/`): Rust
//! builds the tree, ArkUI owns the widgets, nothing is evaluated at runtime.
//!
//! ## What carries over and what does not
//!
//! The *capabilities* carry over unchanged, because they were always Rust:
//! `state` for controls that remember, `host_invoke` for platform reads,
//! `web_declare` for a composited WebView. What does not carry over is the DSL —
//! no `sget`, no `tapto` strings, no `act_toggle`. A control names a tap id and
//! the handler below decides what it means, which is how the other four apps
//! have always worked.
//!
//! ## Tap ids
//!
//! 200s, 300s, 999, 1000, 2000s and 3000s are taken by the other apps and the
//! router. This takes 50000 up: routes are interned as they are met (the catalog
//! navigates by *name*, and an ArkUI event carries only an i32), and state
//! actions get their own block above them.

use splash_oh_native::arkui::{attr, ty, Node};

mod index;

/// Page width in vp, set once by the host at mount.
///
/// Not a constant. `ui::W` is 402 — the reference width the four benchmark apps
/// were built against — and copying it here left a 38vp strip of bare surface
/// down the right edge of every screen on a 440vp display. The DSL kit never had
/// this problem because it sized in percentages; a Rust build has to be told.
///
/// The bridge knows the answer (`device::display` reads the physical width and
/// the virtual pixel ratio) and this crate must not depend on the bridge, so it
/// arrives through a setter — the same inversion `set_router` and
/// `set_host_invoke` use.
static PAGE_W: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(402);

pub fn set_page_width(vp: f32) {
    if vp > 200.0 && vp < 4000.0 {
        PAGE_W.store(vp as u32, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The page width in vp.
#[allow(non_snake_case)]
pub fn W() -> f32 {
    PAGE_W.load(std::sync::atomic::Ordering::Relaxed) as f32
}

/// Tap ids for a route the catalog can navigate to.
pub const ROUTE_BASE: i32 = 50_000;
/// Tap ids for "change this piece of state", resolved by `action_for`.
pub const ACTION_BASE: i32 = 52_000;
/// Pop one level of the current route.
pub const BACK: i32 = 51_999;

// ---- Material 3 baseline tokens ---------------------------------------------
// The real ones, from the M3 spec's baseline scheme — the same values the DSL
// kit carried, because they are the sample's own and not a preference.
pub const SURFACE: u32 = 0xFFFEF7FF;
pub const SURF_CONT: u32 = 0xFFF3EDF7;
pub const SURF_HIGHEST: u32 = 0xFFE6E0E9;
pub const SURF_LOWEST: u32 = 0xFFFFFFFF;
pub const ON_SURFACE: u32 = 0xFF1D1B20;
pub const ON_SURF_VAR: u32 = 0xFF49454F;
pub const PRIMARY: u32 = 0xFF6750A4;
pub const ON_PRIMARY: u32 = 0xFFFFFFFF;
pub const PRI_CONT: u32 = 0xFFEADDFF;
pub const ON_PRI_CONT: u32 = 0xFF21005D;
pub const OUTLINE_VAR: u32 = 0xFFCAC4D0;
pub const ERROR: u32 = 0xFFB3261E;

// ---- the screens ------------------------------------------------------------
// Each entry is a directory of flutter/samples: its route prefix, the label the
// index shows, and whether it responds or is only a note. Order is the index's.
pub struct Screen {
    pub route: &'static str,
    pub label: &'static str,
    /// True when the screen does something — live data, motion, or a control.
    pub responds: bool,
}

pub const SCREENS: &[Screen] = &[
    Screen {
        route: "material_3_demo",
        label: "Material 3 Demo",
        responds: true,
    },
    Screen {
        route: "cupertino_gallery",
        label: "Cupertino Gallery",
        responds: true,
    },
    Screen {
        route: "date_planner",
        label: "Date Planner",
        responds: true,
    },
    Screen {
        route: "platform_design",
        label: "Platform Design",
        responds: true,
    },
    Screen {
        route: "animations",
        label: "Animations",
        responds: true,
    },
    Screen {
        route: "form_app",
        label: "Form App",
        responds: true,
    },
    Screen {
        route: "navigation_and_routing",
        label: "Navigation and Routing",
        responds: true,
    },
    Screen {
        route: "compass_app",
        label: "Compass App",
        responds: true,
    },
    Screen {
        route: "desktop_photo_search",
        label: "Desktop Photo Search",
        responds: true,
    },
    Screen {
        route: "dynamic_theme",
        label: "Dynamic Theme",
        responds: true,
    },
    Screen {
        route: "testing_app",
        label: "Testing App",
        responds: true,
    },
    Screen {
        route: "add_to_app",
        label: "Add to App",
        responds: true,
    },
    Screen {
        route: "asset_transformation",
        label: "Asset Transformation",
        responds: true,
    },
    Screen {
        route: "background_isolate_channels",
        label: "Background Isolate Channels",
        responds: true,
    },
    Screen {
        route: "google_maps",
        label: "Google Maps",
        responds: true,
    },
    Screen {
        route: "pedometer",
        label: "Pedometer",
        responds: true,
    },
    Screen {
        route: "platform_channels",
        label: "Platform Channels",
        responds: true,
    },
    Screen {
        route: "platform_view_swift",
        label: "Platform View Swift",
        responds: true,
    },
    Screen {
        route: "simple_sdf",
        label: "Simple SDF",
        responds: true,
    },
    Screen {
        route: "simple_shader",
        label: "Simple Shader",
        responds: true,
    },
    Screen {
        route: "web_embedding",
        label: "Web Embedding",
        responds: true,
    },
    Screen {
        route: "analysis_defaults",
        label: "Analysis Defaults",
        responds: false,
    },
    Screen {
        route: "android_splash_screen",
        label: "Android Splash Screen",
        responds: false,
    },
    Screen {
        route: "docs",
        label: "docs",
        responds: false,
    },
    Screen {
        route: "ios_app_clip",
        label: "iOS App Clip",
        responds: false,
    },
    Screen {
        route: "tool",
        label: "tool",
        responds: false,
    },
    Screen {
        route: "veggieseasons",
        label: "Veggie Seasons",
        responds: false,
    },
];

// ---- route interning --------------------------------------------------------
// The catalog navigates by name — "date_planner/maya" — and an ArkUI event
// carries an i32. So a route is interned when the tree that links to it is
// built, and the id is an index into that list. The list is cleared per build,
// which is safe only because a tap is delivered against the tree that is
// currently mounted.

thread_local! {
    static ROUTES: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static ACTIONS: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn intern(route: &str) -> i32 {
    ROUTES.with(|r| {
        let mut v = r.borrow_mut();
        match v.iter().position(|s| s == route) {
            Some(i) => ROUTE_BASE + i as i32,
            None => {
                v.push(route.to_string());
                ROUTE_BASE + (v.len() - 1) as i32
            }
        }
    })
}

/// The route a tap id names, if it names one.
pub fn route_for(id: i32) -> Option<String> {
    if id < ROUTE_BASE || id >= ACTION_BASE {
        return None;
    }
    ROUTES.with(|r| r.borrow().get((id - ROUTE_BASE) as usize).cloned())
}

fn intern_action(action: &str) -> i32 {
    ACTIONS.with(|a| {
        let mut v = a.borrow_mut();
        match v.iter().position(|s| s == action) {
            Some(i) => ACTION_BASE + i as i32,
            None => {
                v.push(action.to_string());
                ACTION_BASE + (v.len() - 1) as i32
            }
        }
    })
}

/// The state action a tap id names, if it names one.
pub fn action_for(id: i32) -> Option<String> {
    if id < ACTION_BASE {
        return None;
    }
    ACTIONS.with(|a| a.borrow().get((id - ACTION_BASE) as usize).cloned())
}

// ---- widgets ----------------------------------------------------------------
// `ui.rs` has sixteen helpers and no controls — it was written for the four
// benchmark apps, which are lists and images. The catalog is mostly controls,
// so they live here rather than being bolted onto a module the benchmark
// depends on.

pub fn col(w: f32, h: f32, bg: u32) -> Option<Node> {
    Some(Node::new(ty::column())?.width(w).height(h).bg(bg))
}

/// A column that is as tall as its children. Most of the catalog is this: a
/// fixed height clips descenders, which is how the first pass shipped with
/// every paragraph cut off at the baseline.
pub fn col_fit(w: f32, bg: u32) -> Option<Node> {
    Some(Node::new(ty::column())?.width(w).bg(bg))
}

pub fn row(w: f32, h: f32, bg: u32) -> Option<Node> {
    Some(Node::new(ty::row())?.width(w).height(h).bg(bg))
}

pub fn row_fit(w: f32, bg: u32) -> Option<Node> {
    Some(Node::new(ty::row())?.width(w).bg(bg))
}

/// A scroll that claims the space its fixed-size siblings leave.
///
/// Not a fixed height. A scroll with a bar under it and no bounded height gets
/// a viewport that runs past the bottom of the display: it reaches its own
/// scroll end with rows still below the screen and the bar never appears. The
/// Cupertino list lost its last two entries that way.
pub fn scroll_rest() -> Option<Node> {
    Some(
        Node::new(ty::scroll())?
            .f32_attr(attr::width_percent(), 1.0)
            .f32_attr(attr::layout_weight(), 1.0),
    )
}

pub fn text(s: &str, size: f32, weight: i32, color: u32, w: f32, h: f32) -> Option<Node> {
    Some(
        Node::new(ty::text())?
            .text(s)
            .font_size(size)
            .font_weight(weight)
            .font_color(color)
            .width(w)
            .height(h),
    )
}

/// A paragraph that wraps and sizes itself to the wrapped result.
///
/// ArkUI measures a Text to zero without a width, and will not grow a fixed
/// height to fit the wrap — so the height is estimated from the character count
/// at roughly 2.2 characters per vp, the same rule the DSL kit used.
pub fn para(s: &str, size: f32, color: u32, w: f32) -> Option<Node> {
    let per_line = (w / (size * 0.5)).max(1.0);
    let lines = (s.chars().count() as f32 / per_line).ceil().max(1.0);
    text(s, size, 400, color, w, lines * size * 1.45 + 6.0)
}

/// A tappable wrapper. ArkUI delivers a click from any node with a handler, so
/// a whole row can be the target — which is what Flutter's `CheckboxListTile`
/// and `SwitchListTile` do, and what a 18x18 box needs, being well under the
/// 48dp a finger wants.
pub fn tap_row(w: f32, h: f32, bg: u32, route: &str) -> Option<Node> {
    Some(row(w, h, bg)?.on_event(splash_oh_native::arkui::event::click(), intern(route)))
}

pub fn tap_row_fit(w: f32, bg: u32, route: &str) -> Option<Node> {
    Some(row_fit(w, bg)?.on_event(splash_oh_native::arkui::event::click(), intern(route)))
}

/// A row whose tap changes state instead of navigating. The action string is
/// the same grammar `state::apply` already parses — `key=!`, `key=+1`, `key=~n`.
pub fn act_row(w: f32, h: f32, bg: u32, action: &str) -> Option<Node> {
    Some(row(w, h, bg)?.on_event(
        splash_oh_native::arkui::event::click(),
        intern_action(action),
    ))
}

pub fn act_row_fit(w: f32, bg: u32, action: &str) -> Option<Node> {
    Some(row_fit(w, bg)?.on_event(
        splash_oh_native::arkui::event::click(),
        intern_action(action),
    ))
}

// The three selected-state attributes and the two colour overrides below are
// carried over from the DSL walker rather than rediscovered. ArkUI's defaults
// are wrong for Material in two specific ways it already paid to learn: a
// checkbox draws as a CIRCLE, which reads as a radio button, and neither control
// picks up the app's primary colour on its own.
pub fn checkbox(on: bool) -> Option<Node> {
    Some(
        Node::new(ty::checkbox())?
            .width(18.0)
            .height(18.0)
            // Material's box is `_kEdgeSize` 18 with a RoundedRectangleBorder
            // (material/checkbox.dart). 1 is ROUNDED_SQUARE.
            .i32_attr(attr::checkbox_shape(), 1)
            .u32_attr(attr::checkbox_color(), PRIMARY)
            .i32_attr(attr::checkbox_select(), i32::from(on)),
    )
}

pub fn toggle(on: bool) -> Option<Node> {
    Some(
        Node::new(ty::toggle())?
            .width(52.0)
            .height(32.0)
            .u32_attr(attr::toggle_color(), PRIMARY)
            .i32_attr(attr::toggle_value(), i32::from(on)),
    )
}

pub fn radio(on: bool) -> Option<Node> {
    Some(
        Node::new(ty::radio())?
            .width(20.0)
            .height(20.0)
            .i32_attr(attr::radio_checked(), i32::from(on)),
    )
}

/// A slider at `v` in 0..=1.
pub fn slider(v: f32, w: f32) -> Option<Node> {
    Some(
        Node::new(ty::slider())?
            .width(w)
            .height(32.0)
            .f32_attr(attr::slider_min(), 0.0)
            .f32_attr(attr::slider_max(), 1.0)
            .f32_attr(attr::slider_value(), v),
    )
}

pub fn divider(w: f32) -> Option<Node> {
    Some(col(w, 1.0, OUTLINE_VAR)?)
}

pub fn gap(h: f32) -> Option<Node> {
    Some(col(1.0, h, 0)?)
}

/// The top bar: a back chevron and a title. `parent` is where back goes.
pub fn appbar(title: &str, parent: &str) -> Option<Node> {
    let mut bar = row(W(), 56.0, SURF_CONT)?;
    bar = bar.child(
        tap_row(48.0, 56.0, SURF_CONT, parent)?
            .child(text("\u{2039}", 26.0, 400, ON_SURFACE, 48.0, 56.0)?),
    );
    bar = bar.child(text(title, 22.0, 400, ON_SURFACE, W() - 56.0, 56.0)?);
    Some(bar)
}

/// A titled card, the shape every "section" on these screens uses.
pub fn section(title: &str, kids: Vec<Node>) -> Option<Node> {
    let iw = W() - 64.0;
    let mut card = col_fit(W() - 32.0, SURF_CONT)?.radius(12.0).padding(16.0);
    card = card.child(text(title, 16.0, 500, ON_SURFACE, iw, 26.0)?);
    card = card.child(gap(8.0)?);
    for k in kids {
        card = card.child(k);
    }
    Some(card)
}

// ---- the page ---------------------------------------------------------------

/// Build a route. `""` and `"index"` are the catalog list.
pub fn build(route: &str) -> Option<Node> {
    ROUTES.with(|r| r.borrow_mut().clear());
    ACTIONS.with(|a| a.borrow_mut().clear());
    let route = if route.is_empty() { "index" } else { route };
    match route {
        "index" => index::build(),
        // Each directory answers for itself and its sub-routes. Filled in as
        // the screens are ported; anything not yet here falls back to the index
        // rather than mounting nothing, so a half-finished catalog is navigable.
        _ => index::build(),
    }
}
