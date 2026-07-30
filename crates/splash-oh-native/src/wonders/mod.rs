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

/// A tap target at a given place in the frame, placed by *layout*.
///
/// `NODE_POSITION` moves where a node draws and ArkUI keeps hit-testing it
/// where it was laid out. Every touch target positioned that way drew in the
/// right place and could not be tapped: the home chevron, all four tab cells,
/// the intro button. A full-frame Column with a spacer above the target puts
/// the target's layout box where it actually appears, so the two agree.
///
/// Verified rather than assumed — an unpositioned full-frame target fired while
/// the positioned one beside it never did.
pub fn hit(
    frame_w: f32,
    frame_h: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    id: i32,
) -> Option<crate::arkui::Node> {
    use crate::ui::{col, row, spacer, tap_col};
    let mut column = col(frame_w, frame_h, 0x00000000)?;
    if y > 0.0 {
        column = column.child(spacer(frame_w, y)?);
    }
    let mut line = row(frame_w, h, 0x00000000)?;
    if x > 0.0 {
        line = line.child(spacer(x, h)?);
    }
    line = line.child(tap_col(w, h, 0x00000000, id)?);
    column = column.child(line);
    Some(column)
}
