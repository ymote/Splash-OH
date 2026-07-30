//! Wonderous, sized to the screen it is actually on.
//!
//! The other apps here are laid out against a fixed 402×780 vp page, which is
//! close enough for a benchmark and wrong for this: Wonderous is full-bleed
//! artwork, and a frame that does not reach the edges reads immediately as
//! broken. The Pura X is 1320×2120 px where the Mate 70 is 1320×2760, so the
//! page has to come from the display rather than from a constant.

use splash_oh_native::arkui::Node;
use splash_oh_native::wonders;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

extern "C" {
    fn OH_NativeDisplayManager_GetDefaultDisplayWidth(w: *mut i32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayHeight(h: *mut i32) -> i32;
    fn OH_NativeDisplayManager_GetDefaultDisplayVirtualPixelRatio(r: *mut f32) -> i32;
}

/// The display in vp, or the old fixed page if the manager will not answer.
pub fn page() -> (f32, f32) {
    let (mut pw, mut ph, mut ratio) = (0i32, 0i32, 0f32);
    let ok = unsafe {
        OH_NativeDisplayManager_GetDefaultDisplayWidth(&mut pw) == 0
            && OH_NativeDisplayManager_GetDefaultDisplayHeight(&mut ph) == 0
            && OH_NativeDisplayManager_GetDefaultDisplayVirtualPixelRatio(&mut ratio) == 0
    };
    if ok && pw > 0 && ph > 0 && ratio > 0.1 {
        // The display is not the content area. ArkUI hands the page the space
        // below the status bar, so laying out against the full display height
        // pushes everything down by that much -- which put the intro's button
        // and the home screen's chevron past the bottom edge, where they drew
        // but could not be tapped.
        const STATUS_BAR_VP: f32 = 20.0;
        // And the navigation gesture bar at the bottom.
        //
        // The system takes touches in a strip across the bottom centre before
        // the app sees them. That is why the middle cell of the tab bar was
        // dead while the cells either side worked: it sat under the gesture
        // area. It survived every in-app fix — laid out or positioned, with or
        // without a hit-test mode, adjacent or gapped — because the app was
        // never receiving those events at all.
        const GESTURE_BAR_VP: f32 = 24.0;
        (
            pw as f32 / ratio,
            ph as f32 / ratio - STATUS_BAR_VP - GESTURE_BAR_VP,
        )
    } else {
        (splash_oh_native::ui::W, splash_oh_native::ui::PAGE_H)
    }
}

/// Which screen is showing.
///
/// The app opens on the intro, goes to the home pager, and from there into a
/// wonder's details, the menu, the collection or one artifact.
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Intro(usize),
    Home,
    Details(usize),
    Menu,
    Collection,
    Artifact,
    Timeline,
    Search(Option<usize>),
}

static SCREEN: Mutex<Screen> = Mutex::new(Screen::Intro(0));

fn screen() -> Screen {
    *SCREEN.lock().unwrap_or_else(|e| e.into_inner())
}
fn go(s: Screen) {
    if let Ok(mut g) = SCREEN.lock() {
        *g = s;
    }
}

/// Kept so the details screens can ask which tab is up.
static TAB: AtomicUsize = AtomicUsize::new(usize::MAX);

fn tab() -> Option<usize> {
    match TAB.load(Ordering::Relaxed) {
        usize::MAX => None,
        t => Some(t),
    }
}

/// Handle a tap. `true` if it was ours and the tree should be rebuilt.
pub fn handle(target: i32) -> bool {
    // A scroll tick moves the hero directly and does not rebuild: the tree is
    // unchanged, only two attributes on nodes that are already mounted.
    if target == wonders::details::SCROLL_TICK {
        wonders::details::apply_parallax();
        return false;
    }
    let n = wonders::data::WONDERS.len();
    // Menu rows pick a wonder and return to its home page.
    if target >= wonders::screens::MENU_BASE && target < wonders::screens::MENU_BASE + n as i32 {
        set((target - wonders::screens::MENU_BASE) as usize);
        go(Screen::Home);
        return true;
    }
    if target >= wonders::tabbar::TAB_BASE
        && target < wonders::tabbar::TAB_BASE + wonders::tabbar::TABS.len() as i32
    {
        TAB.store(
            (target - wonders::tabbar::TAB_BASE) as usize,
            Ordering::Relaxed,
        );
        return true;
    }
    if target >= wonders::search::CHIP_BASE
        && target < wonders::search::CHIP_BASE + wonders::search::CHIPS as i32
    {
        go(Screen::Search(Some(
            (target - wonders::search::CHIP_BASE) as usize,
        )));
        return true;
    }
    match target {
        wonders::details::BROWSE_TAP => {
            go(Screen::Search(None));
            true
        }
        wonders::search::SEARCH_CLOSE => {
            go(Screen::Details(2));
            true
        }
        wonders::details::ARTIFACT_NEXT | wonders::details::ARTIFACT_PREV => {
            let list = wonders::artifact_data::ARTIFACTS[current() % 8];
            if !list.is_empty() {
                let n = list.len();
                let cur = wonders::details::artifact_sel() % n;
                let next = if target == wonders::details::ARTIFACT_NEXT {
                    (cur + 1) % n
                } else {
                    (cur + n - 1) % n
                };
                wonders::details::set_artifact_sel(next);
            }
            true
        }
        wonders::screens::INTRO_NEXT => {
            if let Screen::Intro(p) = screen() {
                go(Screen::Intro(
                    (p + 1).min(wonders::screens::INTRO.len() - 1),
                ));
            }
            true
        }
        wonders::screens::INTRO_ENTER => {
            go(Screen::Home);
            true
        }
        wonders::home::MENU_TAP => {
            go(Screen::Menu);
            true
        }
        wonders::screens::MENU_CLOSE => {
            go(Screen::Home);
            true
        }
        wonders::screens::MENU_COLLECTION => {
            go(Screen::Collection);
            true
        }
        wonders::screens::MENU_TIMELINE => {
            go(Screen::Timeline);
            true
        }
        wonders::timeline::TIMELINE_CLOSE => {
            go(Screen::Home);
            true
        }
        wonders::screens::COLLECTION_CLOSE | wonders::screens::ARTIFACT_CLOSE => {
            go(Screen::Home);
            true
        }
        wonders::tabbar::HOME_TAP => {
            TAB.store(usize::MAX, Ordering::Relaxed);
            go(Screen::Home);
            true
        }
        // The chevron and the title both open the details, as they do in the app.
        wonders::home::DETAILS_TAP => {
            TAB.store(0, Ordering::Relaxed);
            go(Screen::Details(0));
            true
        }
        wonders::home::NEXT_TAP if screen() == Screen::Home => {
            set((current() + 1) % n);
            true
        }
        wonders::home::PREV_TAP if screen() == Screen::Home => {
            set((current() + n - 1) % n);
            true
        }
        _ => false,
    }
}

pub fn build() -> Option<Node> {
    let (w, h) = page();
    match screen() {
        Screen::Intro(p) => wonders::screens::intro(p, w, h),
        Screen::Menu => wonders::screens::menu(current(), w, h),
        Screen::Collection => wonders::screens::collection(w, h),
        Screen::Timeline => wonders::timeline::build(current(), w, h),
        Screen::Search(c) => wonders::search::build(current(), c, w, h),
        Screen::Artifact => wonders::screens::artifact(current(), w, h),
        Screen::Details(_) => wonders::details::build(current(), tab().unwrap_or(0), w, h),
        Screen::Home => wonders::home::build(current(), w, h),
    }
}

static INDEX: AtomicUsize = AtomicUsize::new(0);

pub fn current() -> usize {
    INDEX.load(Ordering::Relaxed)
}
pub fn set(i: usize) {
    INDEX.store(i, Ordering::Relaxed);
}
