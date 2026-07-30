//! A wonder's illustration, composed from its layers as native ArkUI nodes.
//!
//! Wonderous draws each wonder as three or four stacked PNGs over a flat
//! colour — a sun, the monument, and foreground foliage that frames it. The
//! layers are the app's; the composition is done here, from the same rules.
//!
//! # The rule, from `illustration_piece.dart`
//!
//! ```text
//! height = max(minHeight, frameHeight * heightFactor)
//! width  = height * aspectRatio          (BoxFit.fitHeight)
//! x, y   = anchor(alignment) + (fracX * width, fracY * height) + offset
//! ```
//!
//! `initialOffset` and `initialScale` are intro-animation terms multiplied by
//! `(1 - anim)`, so at rest they are zero and are not modelled. `zoomAmt` is
//! driven by scroll position, which this does not have either.
//!
//! # Why absolute positioning
//!
//! A `Stack` centres its children, and these pieces are deliberately not
//! centred — the sun sits high and to one side, the foliage hangs off the
//! bottom corners. `NODE_POSITION` places each piece by computed coordinate,
//! which is what the rule above produces.
//!
//! Pieces routinely extend past the frame; the foreground leaves are meant to
//! run off the edges. The frame therefore clips, and only at its own bounds.

use super::data::{Anchor, Piece, Wonder};
use crate::arkui::{attr, ty, Node};

const APP: &str = "wonders";

/// Place one piece, and give back its node.
fn piece(w: &Wonder, p: &Piece, frame_w: f32, frame_h: f32) -> Option<Node> {
    let h = (frame_h * p.height_factor).max(p.min_h);
    let width = h * p.aspect;

    // Where the piece sits before its own translation.
    let (ax, ay) = match p.anchor {
        Anchor::Center => ((frame_w - width) / 2.0, (frame_h - h) / 2.0),
        Anchor::TopLeft => (0.0, 0.0),
        Anchor::TopCenter => ((frame_w - width) / 2.0, 0.0),
        Anchor::TopRight => (frame_w - width, 0.0),
        Anchor::BottomLeft => (0.0, frame_h - h),
        Anchor::BottomCenter => ((frame_w - width) / 2.0, frame_h - h),
        Anchor::BottomRight => (frame_w - width, frame_h - h),
    };

    let x = ax + p.frac_x * width + p.off_x;
    let y = ay + p.frac_y * h + p.off_y;

    Some(
        Node::new(ty::image())?
            .width(width)
            .height(h)
            .string_attr(
                attr::image_src(),
                &format!("resource://RAWFILE/{APP}/{}/{}", w.dir, p.file),
            )
            // FILL rather than CONTAIN: the node is already the image's exact
            // aspect, and CONTAIN would letterbox on any rounding difference.
            .i32_attr(attr::image_fit(), 3)
            .f32v_attr(attr::position(), &[x, y]),
    )
}

/// Is this piece part of the foreground?
///
/// Wonderous splits every illustration into bg / mg / fg builders and draws the
/// wonder's name *between* mg and fg, so the foliage crosses in front of the
/// title. Nothing in the asset names says which is which except the names
/// themselves, and they are consistent across all eight.
fn is_foreground(file: &str) -> bool {
    file.starts_with("foreground")
}

/// The whole illustration, with `overlay` sitting between the mid-ground and
/// the foreground.
///
/// That ordering is the whole reason this takes an overlay rather than letting
/// the caller stack one on top: drawn above everything, the title floats over
/// the leaves and the composition reads flat.
pub fn illustration_with(
    w: &Wonder,
    frame_w: f32,
    frame_h: f32,
    overlay: Option<Node>,
) -> Option<Node> {
    let mut root = Node::new(ty::stack())?
        .width(frame_w)
        .height(frame_h)
        // fgColor, not bgColor. Wonderous fills the hero with the *foreground*
        // colour and uses bgColor for the chrome behind it; using bg here turns
        // Christ the Redeemer's coral sky into a dark green.
        .bg(w.fg)
        .i32_attr(attr::clip(), 1);

    // The paper texture Wonderous lays over the flat colour: a white mask
    // tinted per wonder and faded. Both the tint and the fade are baked into
    // the asset, because ArkUI has no srcIn colour filter for an image node.
    if let Some(t) = Node::new(ty::image()) {
        root = root.child(
            t.width(frame_w)
                .height(frame_h)
                .string_attr(
                    attr::image_src(),
                    &format!("resource://RAWFILE/{APP}/{}/texture.png", w.dir),
                )
                // COVER: the texture is a full-bleed wash, not a placed piece.
                .i32_attr(attr::image_fit(), 1)
                .f32v_attr(attr::position(), &[0.0, 0.0]),
        );
    }

    // Three clouds drift over the background. Wonderous places them with a
    // seeded `Random`, three per wonder, from these ranges:
    //   x     -200 .. width - 100
    //   y       50 .. height - 50
    //   scale   .7 .. 1
    //   flipX/flipY  either
    //
    // The ranges and the count are the app's; the generator is not. Dart's
    // `Random` is explicitly implementation-defined, so the same seed cannot be
    // replayed from Rust. This uses the app's per-wonder seed through a small
    // deterministic generator instead: stable across runs and per wonder, but
    // the three clouds do not land on the app's exact pixels.
    let mut rng = w.cloud_seed.wrapping_mul(2_654_435_761).wrapping_add(1);
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        (rng >> 8) as f32 / 16_777_216.0
    };
    for _ in 0..3 {
        let cx = -200.0 + next() * (frame_w - 100.0 + 200.0);
        let cy = 50.0 + next() * (frame_h - 100.0);
        let scale = 0.7 + next() * 0.3;
        let size = 500.0 * scale * (frame_w / 400.0).min(1.4);
        if let Some(c) = Node::new(ty::image()) {
            root = root.child(
                c.width(size)
                    .height(size * 0.5)
                    .string_attr(
                        attr::image_src(),
                        &format!("resource://RAWFILE/{APP}/_common/cloud-white.png"),
                    )
                    .i32_attr(attr::image_fit(), 0)
                    .f32v_attr(attr::opacity(), &[0.55])
                    .f32v_attr(attr::position(), &[cx, cy]),
            );
        }
    }

    for p in w.pieces.iter().filter(|p| !is_foreground(p.file)) {
        if let Some(node) = piece(w, p, frame_w, frame_h) {
            root = root.child(node);
        }
    }
    if let Some(o) = overlay {
        root = root.child(o);
    }
    for p in w.pieces.iter().filter(|p| is_foreground(p.file)) {
        if let Some(node) = piece(w, p, frame_w, frame_h) {
            root = root.child(node);
        }
    }
    Some(root)
}

/// The illustration on its own.
pub fn illustration(w: &Wonder, frame_w: f32, frame_h: f32) -> Option<Node> {
    illustration_with(w, frame_w, frame_h, None)
}
