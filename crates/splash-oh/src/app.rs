//! App state: which screen is showing, event routing, and rebuilds.
//!
//! Navigation is the reason events matter. A tap arrives on the ArkUI event
//! thread with just a target id; we map that to a screen, re-evaluate the DSL
//! with the new screen bound, and swap the tree in the NodeContent slot.

use crate::arkui::{NodeContentHandle, Node};
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
    // The WeChat demo owns the surface once it is mounted, so its ids win.
    if WECHAT_ACTIVE.with(|w| *w.borrow()) {
        crate::wechat::handle(target_id);
        let (node, _, _) = crate::wechat::build();
        set_root(node);
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
        match new_root.mount_keep(slot) {
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

/// Re-evaluate the DSL for the current screen and swap the tree in.
pub fn rebuild() {
    let (slot, screen) = APP.with(|a| {
        let b = a.borrow();
        let app = b.as_ref().unwrap();
        (app.slot, app.screen.clone())
    });
    let bench = crate::bench::report();

    // DEMO: mount the LLM-generated weather card instead of the catalog. The
    // weather DSL is self-contained (data inlined), so screen/bench are unused.
    let _ = (&screen, &bench);
    let Some(new_root) = crate::dsl::build_weather() else {
        crate::log("app: DSL build failed");
        return;
    };

    APP.with(|a| {
        let mut b = a.borrow_mut();
        let app = b.as_mut().unwrap();
        // Detach the previous tree before dropping it, or ArkUI keeps
        // rendering nodes we are about to free.
        if let Some(old) = app.root.take() {
            unsafe { splash_content_remove(slot, old.raw()) };
            drop(old);
        }
        match new_root.mount_keep(slot) {
            Ok(node) => app.root = Some(node),
            Err(e) => crate::log(&format!("app: mount failed: {e}")),
        }
    });
}
