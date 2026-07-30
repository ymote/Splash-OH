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
        (pw as f32 / ratio, ph as f32 / ratio)
    } else {
        (splash_oh_native::ui::W, splash_oh_native::ui::PAGE_H)
    }
}

/// Handle a tap. `true` if it was ours and the tree should be rebuilt.
pub fn handle(target: i32) -> bool {
    let n = wonders::data::WONDERS.len();
    match target {
        wonders::home::NEXT_TAP => {
            set((current() + 1) % n);
            true
        }
        wonders::home::PREV_TAP => {
            set((current() + n - 1) % n);
            true
        }
        _ => false,
    }
}

pub fn build() -> Option<Node> {
    let (w, h) = page();
    wonders::home::build(current(), w, h)
}

static INDEX: AtomicUsize = AtomicUsize::new(0);

pub fn current() -> usize {
    INDEX.load(Ordering::Relaxed)
}
pub fn set(i: usize) {
    INDEX.store(i, Ordering::Relaxed);
}
