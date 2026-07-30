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
        // Measured on the Pura X rather than assumed: the first row of app
        // pixels on a details screen, whose hero starts at page y = 0, is at
        // display y = 119 px, which at ratio 3 is 39.7 vp.
        const STATUS_BAR_VP: f32 = 39.7;
        // And the navigation gesture bar at the bottom.
        //
        // The system takes touches in a strip across the bottom centre before
        // the app sees them. That is why the middle cell of the tab bar was
        // dead while the cells either side worked: it sat under the gesture
        // area. It survived every in-app fix — laid out or positioned, with or
        // without a hit-test mode, adjacent or gapped — because the app was
        // never receiving those events at all.
        //
        // Only the sum of the two matters here — the system imposes the top
        // inset itself and the page only chooses its height — but they are
        // split so the numbers can be checked against a screenshot.
        const GESTURE_BAR_VP: f32 = 4.3;
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

/// The four directions the shim reports, as offsets from a swipe base.
const SWIPE_LEFT: i32 = 1;
const SWIPE_RIGHT: i32 = 2;
const SWIPE_UP: i32 = 3;
const SWIPE_DOWN: i32 = 4;

/// `Some(direction)` if `target` is a swipe on `base`.
fn swipe_of(target: i32, base: i32) -> Option<i32> {
    match target - base {
        d @ SWIPE_LEFT..=SWIPE_DOWN => Some(d),
        _ => None,
    }
}

/// Move the home pager, wrapping. Only meaningful on the home screen.
///
/// Returns false: all eight wonders are already mounted, so the page change is
/// a cross-fade over the tree that is there rather than a new one.
fn step(d: i32) -> bool {
    if screen() != Screen::Home {
        return false;
    }
    let n = wonders::data::WONDERS.len();
    let next = ((current() + n) as i32 + d) as usize % n;
    set(next);
    wonders::home::fade_to(next);
    false
}

/// Move the artifact carousel, wrapping as the app's `PageView` does.
fn step_artifact(d: i32) -> bool {
    let list = wonders::artifact_data::ARTIFACTS[current() % 8];
    if list.is_empty() {
        return false;
    }
    let n = list.len();
    let cur = wonders::details::artifact_sel() % n;
    let next = ((cur + n) as i32 + d) as usize % n;
    wonders::details::set_artifact_sel(next);
    // Every piece is already mounted, so this collapses the carousel around
    // the new one rather than rebuilding the screen.
    wonders::details::collapse_carousel(next);
    false
}

/// Handle a tap. `true` if it was ours and the tree should be rebuilt.
pub fn handle(target: i32) -> bool {
    // A scroll tick moves the hero directly and does not rebuild: the tree is
    // unchanged, only two attributes on nodes that are already mounted.
    if target == wonders::details::SCROLL_TICK {
        wonders::details::apply_parallax();
        return false;
    }
    // A keystroke in the search field: read the text back and redraw the
    // results if it actually changed.
    if target == wonders::search::SEARCH_TYPED {
        return wonders::search::read_typed();
    }
    if target == SCREEN_APPEAR {
        fade_in_screen();
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
    // Swipes. The shim measures the drag and reports base + 1..4 for left,
    // right, up and down; each of these is the same move as the tap next to it.
    if let Some(d) = swipe_of(target, wonders::home::HOME_SWIPE) {
        return match d {
            SWIPE_LEFT => {
                step(1);
                true
            }
            SWIPE_RIGHT => {
                step(-1);
                true
            }
            _ => false,
        };
    }
    if let Some(d) = swipe_of(target, wonders::details::PHOTO_SWIPE) {
        // `_handleSwipe`: a horizontal swipe moves one cell against the drag,
        // a vertical one moves a whole row.
        let (dx, dy) = match d {
            SWIPE_LEFT => (1, 0),
            SWIPE_RIGHT => (-1, 0),
            SWIPE_UP => (0, 1),
            _ => (0, -1),
        };
        return wonders::details::move_photo_sel(dx, dy);
    }
    if let Some(d) = swipe_of(target, wonders::details::ARTIFACT_SWIPE) {
        return match d {
            SWIPE_LEFT => step_artifact(1),
            SWIPE_RIGHT => step_artifact(-1),
            _ => false,
        };
    }
    match target {
        // Tapping a peeking edge of the photo wall pans it one cell, the same
        // move the app makes on a swipe in that direction.
        wonders::details::PHOTO_UP
        | wonders::details::PHOTO_DOWN
        | wonders::details::PHOTO_LEFT
        | wonders::details::PHOTO_RIGHT => {
            let (dx, dy) = match target {
                wonders::details::PHOTO_UP => (0, -1),
                wonders::details::PHOTO_DOWN => (0, 1),
                wonders::details::PHOTO_LEFT => (-1, 0),
                _ => (1, 0),
            };
            wonders::details::move_photo_sel(dx, dy)
        }
        wonders::details::ARTIFACT_OPEN => {
            go(Screen::Artifact);
            true
        }
        wonders::details::BROWSE_TAP => {
            go(Screen::Search(None));
            true
        }
        wonders::search::SEARCH_CLOSE => {
            wonders::search::clear_field();
            go(Screen::Details(2));
            true
        }
        wonders::details::ARTIFACT_NEXT => step_artifact(1),
        wonders::details::ARTIFACT_PREV => step_artifact(-1),
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
        wonders::screens::COLLECTION_CLOSE => {
            go(Screen::Home);
            true
        }
        // Closing an artifact goes back to the carousel it was opened from,
        // not to the home screen.
        wonders::screens::ARTIFACT_CLOSE => {
            go(Screen::Details(2));
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
        wonders::home::NEXT_TAP => step(1),
        wonders::home::PREV_TAP => step(-1),
        _ => false,
    }
}

/// The id the screen root reports the moment it is mounted.
///
/// A route change in Wonderous fades; the new tree cannot be animated before
/// it exists, so it is built transparent and asks to be faded in as soon as
/// ArkUI tells it that it is on screen.
const SCREEN_APPEAR: i32 = 7440;
/// `$styles.times.fast`, the app's route transition.
const SCREEN_FADE_MS: i32 = 200;
/// The newest screen root. An appear from an older tree would find this
/// pointing at the tree that is actually on screen, which is the one that
/// should be opaque, so there is nothing to guard against.
static SCREEN_ROOT: AtomicUsize = AtomicUsize::new(0);

fn fade_in_screen() {
    let root = SCREEN_ROOT.load(Ordering::Relaxed);
    if root == 0 {
        return;
    }
    let n = root as splash_oh_native::arkui::NodeHandle;
    unsafe {
        splash_oh_native::arkui::animate(
            n,
            SCREEN_FADE_MS,
            splash_oh_native::arkui::CURVE_EASE_OUT,
            move || unsafe {
                Node::set_f32_attr_raw(n, splash_oh_native::arkui::attr::opacity(), 1.0)
            },
        )
    };
}

pub fn build() -> Option<Node> {
    let (w, h) = page();
    let node = build_screen(w, h)?;
    // Every screen fades in, including home: home changes wonder by
    // cross-fading in place rather than rebuilding, so it only ever appears
    // when arriving from somewhere else -- which is exactly when a route fade
    // is wanted.
    let node = node
        .f32_attr(splash_oh_native::arkui::attr::opacity(), 0.0)
        .on_event(splash_oh_native::arkui::event::appear(), SCREEN_APPEAR);
    SCREEN_ROOT.store(node.raw() as usize, Ordering::Relaxed);
    Some(node)
}

fn build_screen(w: f32, h: f32) -> Option<Node> {
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
