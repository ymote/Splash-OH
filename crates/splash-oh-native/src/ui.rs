//! Widget builders shared by every ported app.
//!
//! These were pulled out of the WeChat port when three more apps arrived and
//! all four needed the same primitives. Nothing here is app-specific: it is the
//! thin layer between `arkui::Node` and a screen description.
//!
//! Every builder bumps a counter, so an app can report how many nodes it built
//! and the ArkTS implementation of the same app can be checked against it. A
//! comparison where one side quietly builds a smaller tree is the failure mode
//! worth guarding against, so it is checked rather than assumed.

use crate::arkui::{attr, event, ty, Node};
use std::cell::RefCell;

thread_local! {
    static COUNT: RefCell<usize> = const { RefCell::new(0) };
}

pub fn reset_count() {
    COUNT.with(|c| *c.borrow_mut() = 0);
}
pub fn count() -> usize {
    COUNT.with(|c| *c.borrow())
}
fn bump() {
    COUNT.with(|c| *c.borrow_mut() += 1);
}

/// Logical screen width in vp, matching the reference apps' 400-ish layouts.
pub const W: f32 = 402.0;
/// Usable page height, less the status and gesture bars.
pub const PAGE_H: f32 = 780.0;

pub fn text(s: &str, size: f32, color: u32, w: f32, h: f32) -> Option<Node> {
    bump();
    Some(
        Node::new(ty::text())?
            .text(s)
            .font_size(size)
            .font_color(color)
            .width(w)
            .height(h),
    )
}

/// Text with a weight, for titles and prices.
pub fn text_w(s: &str, size: f32, color: u32, w: f32, h: f32, weight: i32) -> Option<Node> {
    Some(text(s, size, color, w, h)?.font_weight(weight))
}

pub fn col(w: f32, h: f32, bg: u32) -> Option<Node> {
    bump();
    Some(Node::new(ty::column())?.width(w).height(h).bg(bg))
}

pub fn row(w: f32, h: f32, bg: u32) -> Option<Node> {
    bump();
    Some(Node::new(ty::row())?.width(w).height(h).bg(bg))
}

/// Overlapping children, for the media overlays TikTok and Wonderous use.
pub fn stack(w: f32, h: f32, bg: u32) -> Option<Node> {
    bump();
    Some(Node::new(ty::stack())?.width(w).height(h).bg(bg))
}

/// A tappable container. `tap` is the id ArkUI hands back on click.
pub fn tap_row(w: f32, h: f32, bg: u32, tap: i32) -> Option<Node> {
    Some(row(w, h, bg)?.on_event(event::click(), tap))
}

pub fn tap_col(w: f32, h: f32, bg: u32, tap: i32) -> Option<Node> {
    Some(col(w, h, bg)?.on_event(event::click(), tap))
}

/// A photo. Keeps a placeholder fill behind it so a still-decoding file reads
/// as a grey box rather than a hole.
pub fn photo(app: &str, file: &str, w: f32, h: f32, radius: f32) -> Option<Node> {
    bump();
    Some(
        Node::new(ty::image())?
            .width(w)
            .height(h)
            .bg(0xFFD8D8D8)
            .radius(radius)
            .string_attr(
                attr::image_src(),
                &format!("resource://RAWFILE/{app}/{file}"),
            )
            // ARKUI_OBJECT_FIT_COVER
            .i32_attr(attr::image_fit(), 1),
    )
}

/// A monochrome or flat icon. No placeholder fill — a grey square behind a
/// glyph hides the glyph, and hides a failure to load with it.
pub fn icon(app: &str, file: &str, size: f32) -> Option<Node> {
    bump();
    Some(
        Node::new(ty::image())?
            .width(size)
            .height(size)
            .string_attr(
                attr::image_src(),
                &format!("resource://RAWFILE/{app}/{file}"),
            )
            // ARKUI_OBJECT_FIT_CONTAIN — glyphs must not be cropped.
            .i32_attr(attr::image_fit(), 0),
    )
}

/// A scrolling body pinned to the top. A Scroll centres content shorter than
/// itself by default, which drops a short page half way down the screen.
pub fn scroll(h: f32) -> Option<Node> {
    bump();
    Some(
        Node::new(ty::scroll())?
            .width(W)
            .height(h)
            // ARKUI_ALIGNMENT_TOP
            .i32_attr(attr::alignment(), 1),
    )
}

pub fn spacer(w: f32, h: f32) -> Option<Node> {
    col(w, h, 0)
}

pub fn divider(w: f32, color: u32) -> Option<Node> {
    col(w, 1.0, color)
}
