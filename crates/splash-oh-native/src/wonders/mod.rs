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
pub mod data;
pub mod details;
pub mod editorial_data;
pub mod home;
pub mod illustration;
pub mod screens;
pub mod tabbar;
pub mod timeline;
pub mod timeline_data;

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
    use crate::ui::{col, row, spacer, tap_col};

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
    let mut column = col(frame_w, frame_h, 0x00000000)?;
    let mut cursor = 0.0f32;
    for band in bands {
        let top = band.iter().map(|t| t.1).fold(f32::MAX, f32::min);
        let bottom = band.iter().map(|t| t.1 + t.3).fold(f32::MIN, f32::max);
        if top > cursor {
            column = column.child(spacer(frame_w, top - cursor)?);
        }
        let mut line = row(frame_w, bottom - top, 0x00000000)?;
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
                line = line.child(spacer(tx - x, th)?);
            }
            line = line.child(tap_col(tw, th, 0x00000000, id)?);
            x = tx + tw;
        }
        column = column.child(line);
        cursor = bottom;
    }
    Some(column)
}
