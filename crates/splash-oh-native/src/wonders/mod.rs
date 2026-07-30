//! Wonderous, rebuilt from the Flutter app with native ArkUI components.
//!
//! Not the benchmark port in `wonderous.rs` — that one approximates a makepad
//! translation for node-count comparison. This is the real app: the same eight
//! wonders, the same artwork, the same layout rules, read out of
//! `gskinnerTeam/flutter-wonderous-app` rather than eyeballed.
//!
//! No Flutter, no makepad, no ArkTS widgets. Every node here is created through
//! the ArkUI NDK from Rust.

pub mod artifact_data;
pub mod corpus;
pub mod dart_random;
pub mod data;
pub mod details;
pub mod editorial_data;
pub mod home;
pub mod illustration;
pub mod met;
pub mod places;
pub mod screens;
pub mod search;
pub mod search_data;
pub mod short;
pub mod tabbar;
pub mod timeline;
pub mod timeline_data;
pub mod viewers;

/// The id a screen root reports the moment ArkUI mounts it.
///
/// Declared here rather than beside its only user in `splash-oh`, so that
/// `ids_are_unique` can see it: a constant the test cannot reach is a constant
/// that can collide silently.
pub const SCREEN_APPEAR: i32 = 7440;

/// Every tap target on a screen, in one overlay.
///
/// Three facts, each of which hid the next.
///
/// 1. `NODE_POSITION` moves where a node draws and ArkUI keeps hit-testing it
///    where it was laid out, so a positioned target draws right and never
///    fires.
/// 2. Giving each target its own full-frame Column to lay it out means the last
///    one added covers all the others, and only it receives anything.
/// 3. Putting them all in one Column in a single vertical flow drops any two
///    that share a y — which is exactly the home pager's left and right halves.
///
/// So: one full-frame Column, targets grouped into rows by y, and within a row
/// laid out left to right with spacers. All three were found on the device, the
/// third from a log line reporting the dropped target by id.
///
/// `targets` is `(x, y, w, h, id)` and need not be sorted.
pub fn hits(
    frame_w: f32,
    frame_h: f32,
    targets: &[(f32, f32, f32, f32, i32)],
) -> Option<crate::arkui::Node> {
    hits_swipe(frame_w, frame_h, targets, None)
}

/// `hits`, plus a swipe base.
///
/// A drag has to be measured on the node that receives it, and over most of a
/// screen that node is one of these targets rather than the artwork under
/// them. Every target therefore also carries the touch event, and the shim
/// turns the drag into `base + 1..4` -- left, right, up, down.
pub fn hits_swipe(
    frame_w: f32,
    frame_h: f32,
    targets: &[(f32, f32, f32, f32, i32)],
    swipe: Option<i32>,
) -> Option<crate::arkui::Node> {
    use crate::arkui::attr;
    use crate::ui::{col, row, spacer, tap_col};

    /// `ARKUI_HIT_TEST_MODE_NONE`: this node never takes the touch itself.
    const HIT_NONE: i32 = 3;

    let mut ts: Vec<(f32, f32, f32, f32, i32)> = targets.to_vec();
    ts.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Group into bands: anything starting before the current band ends joins it.
    let mut bands: Vec<Vec<(f32, f32, f32, f32, i32)>> = Vec::new();
    for t in ts {
        match bands.last_mut() {
            Some(b) if t.1 < b.iter().map(|x| x.1 + x.3).fold(f32::MIN, f32::max) => b.push(t),
            _ => bands.push(vec![t]),
        }
    }

    // Kept: this line is what found the menu passing a one-element literal
    // where it meant to pass its ten targets. A screen whose taps do nothing
    // looks identical whether the targets are misplaced or were never built.
    crate::log(&format!(
        "wonders/hits: {} target(s) in {} band(s)",
        targets.len(),
        bands.len()
    ));
    // 4. The overlay is full-frame, so by default it eats every gesture aimed
    //    at whatever is underneath — the details screen would not scroll at
    //    all. `NONE` means "I don't take the touch, my children still can", so
    //    the frame and the padding between bands stay transparent to the
    //    Scroll behind them and only the targets themselves take a touch.
    let mut column = col(frame_w, frame_h, 0x00000000)?.i32_attr(attr::hit_test(), HIT_NONE);
    let mut cursor = 0.0f32;
    for band in bands {
        let top = band.iter().map(|t| t.1).fold(f32::MAX, f32::min);
        let bottom = band.iter().map(|t| t.1 + t.3).fold(f32::MIN, f32::max);
        if top > cursor {
            column =
                column.child(spacer(frame_w, top - cursor)?.i32_attr(attr::hit_test(), HIT_NONE));
        }
        let mut line = row(frame_w, bottom - top, 0x00000000)?.i32_attr(attr::hit_test(), HIT_NONE);
        let mut x = 0.0f32;
        for (tx, _, tw, th, id) in band {
            // A band whose members overlap horizontally cannot be laid out in
            // one row; the overflow pushes the whole row past the frame and
            // then nothing in it is reachable. Skipping loudly beats a screen
            // where every tap silently misses.
            if tx + tw > frame_w + 0.5 || tx < x {
                crate::log(&format!(
                    "wonders/hits: target {id} does not fit its band (x {tx}..{}) — skipped",
                    tx + tw
                ));
                continue;
            }
            if tx > x {
                line = line.child(spacer(tx - x, th)?.i32_attr(attr::hit_test(), HIT_NONE));
            }
            let mut t = tap_col(tw, th, 0x00000000, id)?;
            if let Some(base) = swipe {
                t = t.on_event(crate::arkui::event::touch(), base);
            }
            line = line.child(t);
            x = tx + tw;
        }
        column = column.child(line);
        cursor = bottom;
    }
    Some(column)
}

#[cfg(test)]
mod tests {
    /// Every tap id, in one list.
    ///
    /// Two screens once shared 7340-7342, and because the dispatcher matches on
    /// constants rather than literals nothing complained: the collection's
    /// close button was read as a pan of a photo wall that was not on screen,
    /// and the screen simply stopped closing. A screen whose button does
    /// nothing looks exactly like a mislaid tap target, so this is worth a test
    /// rather than care.
    const IDS: &[(&str, i32)] = &[
        ("home::PREV_TAP", super::home::PREV_TAP),
        ("home::NEXT_TAP", super::home::NEXT_TAP),
        ("home::MENU_TAP", super::home::MENU_TAP),
        ("home::DETAILS_TAP", super::home::DETAILS_TAP),
        ("tabbar::HOME_TAP", super::tabbar::HOME_TAP),
        ("screens::INTRO_NEXT", super::screens::INTRO_NEXT),
        ("screens::INTRO_ENTER", super::screens::INTRO_ENTER),
        ("screens::MENU_CLOSE", super::screens::MENU_CLOSE),
        (
            "screens::COLLECTION_CLOSE",
            super::screens::COLLECTION_CLOSE,
        ),
        ("screens::MENU_COLLECTION", super::screens::MENU_COLLECTION),
        ("screens::MENU_TIMELINE", super::screens::MENU_TIMELINE),
        ("screens::ARTIFACT_CLOSE", super::screens::ARTIFACT_CLOSE),
        ("timeline::TIMELINE_CLOSE", super::timeline::TIMELINE_CLOSE),
        ("details::PHOTO_UP", super::details::PHOTO_UP),
        ("details::PHOTO_DOWN", super::details::PHOTO_DOWN),
        ("details::PHOTO_LEFT", super::details::PHOTO_LEFT),
        ("details::PHOTO_RIGHT", super::details::PHOTO_RIGHT),
        ("details::ARTIFACT_PREV", super::details::ARTIFACT_PREV),
        ("details::ARTIFACT_NEXT", super::details::ARTIFACT_NEXT),
        ("details::BROWSE_TAP", super::details::BROWSE_TAP),
        ("details::ARTIFACT_OPEN", super::details::ARTIFACT_OPEN),
        ("details::SCROLL_TICK", super::details::SCROLL_TICK),
        ("SCREEN_APPEAR", super::SCREEN_APPEAR),
        ("search::SEARCH_CLOSE", super::search::SEARCH_CLOSE),
        ("search::SEARCH_TYPED", super::search::SEARCH_TYPED),
        ("search::RANGE_TOGGLE", super::search::RANGE_TOGGLE),
        ("search::RANGE_START", super::search::RANGE_START),
        ("search::RANGE_END", super::search::RANGE_END),
    ];

    /// The ids that are the base of a run, and how long the run is.
    const RANGES: &[(&str, i32, i32)] = &[
        (
            "tabbar::TAB_BASE",
            super::tabbar::TAB_BASE,
            super::tabbar::TABS.len() as i32,
        ),
        (
            "screens::MENU_BASE",
            super::screens::MENU_BASE,
            super::data::WONDERS.len() as i32,
        ),
        (
            "search::CHIP_BASE",
            super::search::CHIP_BASE,
            super::search::CHIPS as i32,
        ),
        // A swipe base is reported as base + 1..4.
        ("home::HOME_SWIPE", super::home::HOME_SWIPE, 5),
        ("details::PHOTO_SWIPE", super::details::PHOTO_SWIPE, 5),
        ("details::ARTIFACT_SWIPE", super::details::ARTIFACT_SWIPE, 5),
    ];

    #[test]
    fn ids_are_unique() {
        let mut all: Vec<(&str, i32)> = IDS.to_vec();
        for (name, base, len) in RANGES {
            for i in 0..*len {
                all.push((name, base + i));
            }
        }
        all.sort_by_key(|(_, id)| *id);
        for w in all.windows(2) {
            assert_ne!(
                w[0].1, w[1].1,
                "{} and {} both use id {}",
                w[0].0, w[1].0, w[0].1
            );
        }
    }
}
