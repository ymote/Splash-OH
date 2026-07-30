//! Wonderous, rebuilt from the Flutter app with native ArkUI components.
//!
//! Not the benchmark port in `wonderous.rs` — that one approximates a makepad
//! translation for node-count comparison. This is the real app: the same eight
//! wonders, the same artwork, the same layout rules, read out of
//! `gskinnerTeam/flutter-wonderous-app` rather than eyeballed.
//!
//! No Flutter, no makepad, no ArkTS widgets. Every node here is created through
//! the ArkUI NDK from Rust.

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
/// Two things had to be true at once. `NODE_POSITION` moves where a node draws
/// and ArkUI keeps hit-testing it where it was laid out, so a target has to be
/// placed by layout — a spacer above it, a spacer beside it. But a full-frame
/// column per target means the last one added covers all the others, and only
/// it ever receives anything.
///
/// So all of a screen's targets go into a single full-frame column, stacked in
/// order of y with the gaps between them as spacers. Both were found on the
/// device: positioned targets drew correctly and never fired, and once each had
/// its own overlay only the last one worked.
///
/// `targets` is `(x, y, w, h, id)`, and does not need to be sorted.
pub fn hits(
    frame_w: f32,
    frame_h: f32,
    targets: &[(f32, f32, f32, f32, i32)],
) -> Option<crate::arkui::Node> {
    use crate::ui::{col, row, spacer, tap_col};
    let mut ts: Vec<&(f32, f32, f32, f32, i32)> = targets.iter().collect();
    ts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut column = col(frame_w, frame_h, 0x00000000)?;
    let mut y = 0.0f32;
    for (tx, ty, tw, th, id) in ts {
        // Targets that overlap in y cannot both be laid out in one flow; the
        // later one is dropped rather than silently shifting the rest down.
        if *ty < y {
            continue;
        }
        if *ty > y {
            column = column.child(spacer(frame_w, ty - y)?);
        }
        let mut line = row(frame_w, *th, 0x00000000)?;
        if *tx > 0.0 {
            line = line.child(spacer(*tx, *th)?);
        }
        line = line.child(tap_col(*tw, *th, 0x00000000, *id)?);
        column = column.child(line);
        y = ty + th;
    }
    Some(column)
}
