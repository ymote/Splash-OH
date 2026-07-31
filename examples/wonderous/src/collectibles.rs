//! The collectibles hidden through the app, and the screen that shows one off.
//!
//! `collectibles_logic.dart`: three per wonder, twenty-four in all, each in one
//! of three states — lost, discovered, explored. A lost one is drawn as a small
//! icon tucked into the artwork; tapping it opens the found screen and marks it
//! discovered, and the icon collapses away for good. The collection screen is
//! the list of what has been found.
//!
//! State is kept for the life of the process and written to the app's own
//! sandbox so it survives a restart, which is what makes it a collection rather
//! than a session.

use super::places::COLLECTIBLES;
use splash_oh_native::arkui::{attr, Node};
use splash_oh_native::ui::*;
use std::sync::Mutex;

const APP: &str = "wonders";
const SHEET: u32 = 0xFFF8ECE5;
const GREY_STRONG: u32 = 0xFF272625;
const ACCENT: u32 = 0xFFE4935D;
const DISPLAY: &str = "YesevaOne";
const SERIF_UI: &str = "TenorSans";
const BODY_FONT: &str = "Raleway";

pub const COLLECT_BASE: i32 = 7460;
pub const FOUND_CLOSE: i32 = 7490;

/// Which of the twenty-four have been found. Index matches `COLLECTIBLES`.
static FOUND: Mutex<[bool; 24]> = Mutex::new([false; 24]);
/// The one the found screen is showing.
static SHOWING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

/// Where the found set is written.
///
/// An OHOS app's own sandbox. Which directory that is depends on how the
/// module is packaged, so both shapes are tried and the first that accepts a
/// write is used; `hdc shell` cannot read either, which is the point of a
/// sandbox and also why this is logged rather than checked from outside.
const STORES: &[&str] = &[
    "/data/storage/el2/base/haps/entry/files/wonders-collectibles",
    "/data/storage/el2/base/files/wonders-collectibles",
    "/data/storage/el1/base/files/wonders-collectibles",
];

/// Read the found set back at startup. Missing or unreadable means nothing has
/// been found yet, which is the right answer for a first run.
pub fn load() {
    for path in STORES {
        let Ok(s) = std::fs::read_to_string(path) else {
            continue;
        };
        if let Ok(mut g) = FOUND.lock() {
            for (i, c) in s.trim().chars().take(24).enumerate() {
                g[i] = c == '1';
            }
        }
        splash_oh_native::log(&format!("wonders/collectibles: loaded from {path}"));
        return;
    }
    splash_oh_native::log("wonders/collectibles: nothing stored yet");
}

fn save() {
    let bits: String = FOUND
        .lock()
        .map(|g| g.iter().map(|b| if *b { '1' } else { '0' }).collect())
        .unwrap_or_default();
    for path in STORES {
        // `write` will not create a missing directory, and on this device the
        // module-scoped one does not exist.
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(path, &bits) {
            Ok(()) => {
                splash_oh_native::log(&format!("wonders/collectibles: saved to {path}"));
                return;
            }
            Err(e) => splash_oh_native::log(&format!("wonders/collectibles: {path}: {e}")),
        }
    }
}

pub fn is_found(i: usize) -> bool {
    FOUND.lock().map(|g| g[i % 24]).unwrap_or(false)
}

pub fn found_count() -> usize {
    FOUND
        .lock()
        .map(|g| g.iter().filter(|b| **b).count())
        .unwrap_or(0)
}

/// Mark one found and remember it.
pub fn discover(i: usize) {
    if let Ok(mut g) = FOUND.lock() {
        g[i % 24] = true;
    }
    SHOWING.store(i % 24, std::sync::atomic::Ordering::Relaxed);
    save();
}

pub fn showing() -> usize {
    SHOWING.load(std::sync::atomic::Ordering::Relaxed)
}

/// The three that belong to a wonder, in the order the app lists them.
pub fn for_wonder(w: usize) -> Vec<usize> {
    COLLECTIBLES
        .iter()
        .enumerate()
        .filter(|(_, c)| c.wonder == w % 8)
        .map(|(i, _)| i)
        .collect()
}

/// Which cell of the 5×5 wall hides this wonder's gallery collectible —
/// `_getCollectibleIndex`, which puts it in a different corner per wonder so
/// you have to pan to find it.
pub fn gallery_cell(w: usize) -> usize {
    const GRID: usize = 5;
    const N: usize = GRID * GRID;
    match w % 8 {
        // Chichen Itza, Petra
        4 | 2 => 0,
        // Colosseum, Pyramids
        3 | 0 => GRID - 1,
        // Christ the Redeemer, Machu Picchu
        7 | 5 => N - 1,
        // Great Wall, Taj Mahal
        _ => N - GRID,
    }
}

/// The little icon a lost collectible is drawn as.
pub fn badge(i: usize, size: f32) -> Option<Node> {
    let c = &COLLECTIBLES[i % COLLECTIBLES.len()];
    Some(
        stack(size, size, 0x33F8ECE5)?
            .radius(size / 2.0)
            .child(icon(
                APP,
                &format!("_common/collectibles/{}.png", c.icon),
                size * 0.55,
            )?)
            .on_event(
                splash_oh_native::arkui::event::click(),
                COLLECT_BASE + i as i32,
            ),
    )
}

/// `collectible_found_screen.dart`: the piece you just found, its name, and
/// how many of the twenty-four that makes.
pub fn found_screen(w: f32, h: f32) -> Option<Node> {
    let i = showing().min(COLLECTIBLES.len() - 1);
    let c = &COLLECTIBLES[i];
    let mut root = stack(w, h, 0xF21E1B18)?;

    let ih = h * 0.42;
    root = root.child(
        Node::new(splash_oh_native::arkui::ty::image())?
            .width(w * 0.62)
            .height(ih)
            .radius(8.0)
            .string_attr(
                attr::image_src(),
                &super::corpus::thumb_url(c.artifact_id.parse().unwrap_or(0)),
            )
            // ARKUI_OBJECT_FIT_CONTAIN.
            .i32_attr(attr::image_fit(), 0)
            .f32v_attr(attr::position(), &[w * 0.19, h * 0.16]),
    );

    root = root.child(
        text("COLLECTIBLE FOUND", 12.0, ACCENT, w, 22.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h * 0.62]),
    );
    let lines = if c.title.chars().count() > 22 {
        2.0
    } else {
        1.0
    };
    root = root.child(
        text(c.title, 26.0, SHEET, w - 56.0, 36.0 * lines)?
            .string_attr(attr::font_family(), DISPLAY)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::padding(), &[0.0, 28.0, 0.0, 28.0])
            .f32v_attr(attr::position(), &[0.0, h * 0.66]),
    );
    root = root.child(
        text(
            &format!("{} of {} found", found_count(), COLLECTIBLES.len()),
            13.0,
            0x99F8ECE5,
            w,
            22.0,
        )?
        .string_attr(attr::font_family(), BODY_FONT)
        .i32_attr(attr::text_align(), 1)
        .f32v_attr(attr::position(), &[0.0, h * 0.66 + 36.0 * lines + 12.0]),
    );

    root = root.child(
        col(w * 0.5, 48.0, GREY_STRONG)?
            .radius(4.0)
            .f32v_attr(attr::position(), &[w * 0.25, h - 96.0])
            .child(
                text("CONTINUE", 12.0, SHEET, w * 0.5, 48.0)?
                    .string_attr(attr::font_family(), SERIF_UI)
                    .i32_attr(attr::text_align(), 1)
                    .f32v_attr(attr::padding(), &[17.0, 0.0, 0.0, 0.0]),
            ),
    );
    root = root.child(super::hits(
        w,
        h,
        &[(w * 0.25, h - 96.0, w * 0.5, 48.0, FOUND_CLOSE)],
    )?);
    Some(root)
}
