//! App state: which screen is showing, event routing, and rebuilds.
//!
//! Navigation is the reason events matter. A tap arrives on the ArkUI event
//! thread with just a target id; we map that to a screen, re-evaluate the DSL
//! with the new screen bound, and swap the tree in the NodeContent slot.

use crate::arkui::{Node, NodeContentHandle};
use std::cell::RefCell;
use std::os::raw::c_int;

extern "C" {
    fn splash_content_remove(content: NodeContentHandle, root: crate::arkui::NodeHandle) -> c_int;
    fn splash_set_event_handler(h: extern "C" fn(i32, i32));
}

/// Event ids are `SCREEN_BASE + index` for navigation, and small numbers for
/// in-demo controls. Keeping them in one place stops the DSL and Rust drifting.
pub const NAV_BASE: i32 = 1000;
pub const NAV_BACK: i32 = 999;
pub const BENCH_RUN: i32 = 900;

/// Every screen in the catalog. `id` is what the DSL switches on.
pub const SCREENS: &[(&str, &str)] = &[
    ("buttons", "Buttons"),
    ("fab", "Floating action button"),
    ("checkbox", "Checkbox"),
    ("radio", "Radio button"),
    ("switch", "Switch"),
    ("slider", "Slider"),
    ("progress", "Progress indicators"),
    ("textfield", "Text fields"),
    ("chips", "Chips"),
    ("cards", "Cards"),
    ("lists", "Lists"),
    ("dividers", "Dividers"),
    ("badges", "Badges"),
    ("tabs", "Tabs"),
    ("appbar", "Top app bar"),
    ("bottomnav", "Bottom navigation"),
    ("datepicker", "Date picker"),
    ("timepicker", "Time picker"),
    ("textpicker", "Text picker"),
    ("swiper", "Swiper / carousel"),
    ("grid", "Grid"),
    ("waterflow", "Water flow"),
    ("refresh", "Pull to refresh"),
    ("image", "Image"),
    ("typography", "Typography"),
    ("color", "Color"),
    ("layout", "Layout"),
    ("bench", "Performance"),
];

struct App {
    slot: NodeContentHandle,
    root: Option<Node>,
    /// "" is the index screen.
    screen: String,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// Whether the WeChat demo is the mounted surface rather than the catalog.
    static WECHAT_ACTIVE: RefCell<bool> = const { RefCell::new(false) };
}

/// Hand the surface to the WeChat demo (or back to the catalog).
pub fn set_wechat_active(on: bool) {
    WECHAT_ACTIVE.with(|w| *w.borrow_mut() = on);
}

/// Called once from `mount`.
pub fn init(slot: NodeContentHandle) {
    APP.with(|a| {
        *a.borrow_mut() = Some(App {
            slot,
            root: None,
            screen: String::new(),
        })
    });
    unsafe { splash_set_event_handler(on_event) };
    // Nothing heavy here. Benchmarking at startup blocked the JS thread long
    // enough that its timer queue stopped being serviced, which stopped the
    // ArkTS half of the benchmark from ever running. ArkTS drives the whole
    // suite now, one trial per event-loop tick.
    rebuild();
}

/// ArkUI event thread → here. Only the target id matters.
extern "C" fn on_event(target_id: i32, _event_type: i32) {
    // Checked FIRST, ahead of the router hand-off below.
    //
    // The flutter kit names its targets with route strings, interned during the
    // walk into ids at or above FLUTTER_NAV_BASE. Those ids belong to no other
    // app, so claiming them here is unambiguous — and it has to happen before
    // the `WECHAT_ACTIVE` branch, which returns unconditionally once a bridge
    // app is mounted. `catalogScreen` sets that flag, so every tap in the kit
    // was being handed to a router that knows nothing about route strings and
    // then dropped. Nothing was clickable.
    if let Some(route) = crate::dsl::flutter_route(target_id) {
        // A target is either somewhere to go or something to change. The kit
        // names the second kind "set:key=…" and it rides the same interning as
        // a route, so a control needs no new node attribute and both backends
        // get it from the one place a tap already lands.
        if crate::state::apply(&route) {
            rebuild();
        } else {
            set_screen(route);
        }
        return;
    }

    // Whichever ported app owns the surface handles the id and rebuilds.
    //
    // This used to call `wechat::handle` + `wechat::build` directly, from
    // before the other three apps existed. The effect was that tapping a tab
    // in Taobao, TikTok, Wonderous or the browser card rebuilt the *WeChat*
    // tree over the top of it. It went unnoticed because the benchmark drives
    // `build_route` rather than taps, and the on-device tour was driven by a
    // timer -- nothing in the harness ever exercised a tap outside WeChat.
    if WECHAT_ACTIVE.with(|w| *w.borrow()) {
        // Routed through a hook, not a direct call.
        //
        // This is the one place the renderer would otherwise have to know what
        // an app is. The router lives in splash-oh-webview -- it dispatches to
        // both the native demos and the web cards -- and a direct call here
        // would make the renderer depend on the bridge, inverting the whole
        // point of the split.
        if let Some(route) = router() {
            if let Some(node) = route(target_id) {
                set_root(Some(node));
            }
        }
        return;
    }
    match target_id {
        NAV_BACK => set_screen(String::new()),
        BENCH_RUN => {
            // The suite is driven from ArkTS so it can yield between trials;
            // this only redraws whatever has landed so far.
            rebuild();
        }
        id if id >= NAV_BASE => {
            let idx = (id - NAV_BASE) as usize;
            if let Some((s, _)) = SCREENS.get(idx) {
                set_screen((*s).to_string());
            }
        }
        // In-demo control taps are acknowledged but do not navigate; the
        // widgets are real, so ArkUI already handled the visual state itself.
        _ => {}
    }
}

/// The route currently mounted.
pub fn current_screen() -> String {
    APP.with(|a| {
        a.borrow()
            .as_ref()
            .map(|app| app.screen.clone())
            .unwrap_or_default()
    })
}

/// Record the current route without rebuilding.
///
/// `catalogScreen` builds and mounts its own tree, so it never went through
/// `set_screen` and `app.screen` stayed empty — which made `is_animating()`
/// always false and the animation tick a permanent no-op.
pub fn set_screen_quiet(s: String) {
    APP.with(|a| {
        if let Some(app) = a.borrow_mut().as_mut() {
            app.screen = s;
        }
    });
}

fn set_screen(s: String) {
    APP.with(|a| {
        if let Some(app) = a.borrow_mut().as_mut() {
            app.screen = s;
        }
    });
    rebuild();
}

/// Mount an already-built tree, replacing whatever is there.
///
/// The catalog builds its tree from the DSL; the WeChat demo builds its own in
/// Rust. Both end up here, because detaching the previous root before dropping
/// it is the part that must not be got wrong — ArkUI keeps rendering nodes we
/// are about to free otherwise.
/// Handle a tap and, if it changed anything, return the tree to mount.
///
/// `None` means the tap was not for this app and nothing should be rebuilt.
pub type Router = fn(i32) -> Option<Node>;

static ROUTER: std::sync::Mutex<Option<Router>> = std::sync::Mutex::new(None);

/// Installed once by the crate that owns the apps.
pub fn set_router(f: Router) {
    if let Ok(mut r) = ROUTER.lock() {
        *r = Some(f);
    }
}

fn router() -> Option<Router> {
    ROUTER.lock().ok().and_then(|r| *r)
}

pub fn set_root(new_root: Option<Node>) {
    let Some(new_root) = new_root else {
        crate::log("app: build produced nothing");
        return;
    };
    let slot = APP.with(|a| a.borrow().as_ref().map(|x| x.slot));
    let Some(slot) = slot else { return };
    APP.with(|a| {
        let mut b = a.borrow_mut();
        let app = b.as_mut().unwrap();
        if let Some(old) = app.root.take() {
            unsafe { splash_content_remove(slot, old.raw()) };
            drop(old);
        }
        match unsafe { new_root.mount_keep(slot) } {
            Ok(node) => app.root = Some(node),
            Err(e) => crate::log(&format!("app: mount failed: {e}")),
        }
    });
}

/// Remove the mounted tree without putting anything back, so another owner can
/// use the slot.
pub fn detach_root() {
    let slot = APP.with(|a| a.borrow().as_ref().map(|x| x.slot));
    let Some(slot) = slot else { return };
    APP.with(|a| {
        let mut b = a.borrow_mut();
        let app = b.as_mut().unwrap();
        if let Some(old) = app.root.take() {
            unsafe { splash_content_remove(slot, old.raw()) };
            drop(old);
        }
    });
}

/// Whether the current screen is one that moves.
///
/// Re-mounting rebuilds the whole native tree, so it is only worth doing on a
/// timer for screens that animate. Everything else is static and costs nothing.
pub fn is_animating() -> bool {
    APP.with(|a| {
        a.borrow()
            .as_ref()
            .is_some_and(|app| app.screen.starts_with("animations/"))
    })
}

/// Re-evaluate the DSL for the current screen and swap the tree in.
pub fn rebuild() {
    let (slot, screen) = APP.with(|a| {
        let b = a.borrow();
        let app = b.as_ref().unwrap();
        (app.slot, app.screen.clone())
    });
    let bench = crate::bench::report();

    // Mount the flutter/samples kit — the same `.splash` the makepad backend
    // renders, walked into ArkUI here. `screen` carries the route; empty means
    // the index. `bench` is unused by this kit.
    let _ = &bench;
    let route = if screen.is_empty() { "index" } else { &screen };

    // Where the old tree was scrolled to, read before it is dropped.
    //
    // Only carried across when the route is unchanged: a tap that ticks a
    // checkbox should leave you looking at the checkbox, and a tap that
    // navigates should start the new screen at the top, the way every phone
    // does it.
    let keep = crate::dsl::built_route() == route;
    let offset = if keep {
        let h = crate::dsl::scroll_node();
        if h.is_null() {
            None
        } else {
            crate::arkui::Node::get_f32(h, crate::arkui::attr::scroll_offset(), 1)
        }
    } else {
        None
    };

    let Some(new_root) = crate::dsl::build_flutter(route, false) else {
        crate::log("app: DSL build failed");
        return;
    };

    crate::ui::record_total(crate::ui::count());
    APP.with(|a| {
        let mut b = a.borrow_mut();
        let app = b.as_mut().unwrap();
        // Detach the previous tree before dropping it, or ArkUI keeps
        // rendering nodes we are about to free.
        if let Some(old) = app.root.take() {
            unsafe { splash_content_remove(slot, old.raw()) };
            drop(old);
        }
        match unsafe { new_root.mount_keep(slot) } {
            Ok(node) => app.root = Some(node),
            Err(e) => crate::log(&format!("app: mount failed: {e}")),
        }
    });

    // After mounting, not before: the node has no content to scroll over until
    // it is in the tree, and setting an offset on a zero-height Scroll is a
    // no-op that looks exactly like this working.
    if let Some(y) = offset {
        let h = crate::dsl::scroll_node();
        if !h.is_null() && y > 0.0 {
            crate::arkui::Node::set_f32v_raw(h, crate::arkui::attr::scroll_offset(), &[0.0, y]);
        }
    }
}
