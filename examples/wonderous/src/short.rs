//! The illustration as the editorial screen draws it — `shortMode`.
//!
//! `WonderIllustrationConfig` carries a `shortMode` flag, and every wonder's
//! `*_illustration.dart` branches on it: the texture is scaled up three or four
//! times, the sun or moon is moved to sit inside a much shorter frame, and the
//! wonder itself is pushed down rather than up. `editorial_screen.dart` uses it
//! for the band across the top of an article, 250 high on a short screen.
//!
//! Only the background and mid-ground are drawn — `_TopIllustration` builds
//! `bg` and `mg` and no `fg` — and no clouds, which belong to the home screen.
//!
//! The numbers are transcribed from the `config.shortMode ? a : b` arms, one
//! wonder at a time. `H` in a comment means the app's `context.heightPx`, the
//! whole screen rather than this band.

use super::data::{Anchor, Piece, Wonder, WONDERS};
use splash_oh_arkui::arkui::{attr, ty, Node};
use splash_oh_arkui::ui::*;

const APP: &str = "wonders";

/// `editorial_screen.dart`: 250 when the screen is short, 280 otherwise. Every
/// phone this runs on is short by its measure.
pub const HERO_H: f32 = 250.0;

/// One piece in short mode. `off_y_frac` is the part of the offset the app
/// writes as `context.heightPx * k`, which is the screen and not this band.
struct Short {
    file: &'static str,
    height_factor: f32,
    min_h: f32,
    anchor: Anchor,
    frac_x: f32,
    frac_y: f32,
    off_x: f32,
    off_y_frac: f32,
    /// Texture scale for this wonder, the `IllustrationTexture(scale:)` arm.
    texture_scale: f32,
}

/// Background then mid-ground, per wonder, in the order `WONDERS` lists them.
const SHORT: &[[Short; 2]] = &[
    // Pyramids of Giza
    [
        Short {
            file: "moon.png",
            height_factor: 0.15,
            min_h: 100.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.0,
            off_x: 120.0,
            off_y_frac: -0.05,
            texture_scale: 4.0,
        },
        Short {
            file: "pyramids.png",
            height_factor: 0.5,
            min_h: 300.0,
            anchor: Anchor::Center,
            frac_x: 0.015,
            frac_y: 0.17,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 4.0,
        },
    ],
    // Great Wall of China
    [
        Short {
            file: "sun.png",
            height_factor: 0.07,
            min_h: 120.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.0,
            off_x: -40.0,
            off_y_frac: -0.06,
            texture_scale: 4.0,
        },
        Short {
            file: "great-wall.png",
            height_factor: 0.45,
            min_h: 250.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.15,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 4.0,
        },
    ],
    // Petra — the moon is anchored to the top, and the mid-ground fills the
    // band rather than the .8 it takes on the home screen.
    [
        Short {
            file: "moon.png",
            height_factor: 0.15,
            min_h: 50.0,
            anchor: Anchor::TopCenter,
            frac_x: -0.7,
            frac_y: 0.0,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 4.0,
        },
        Short {
            file: "petra.png",
            height_factor: 0.65,
            min_h: 500.0,
            anchor: Anchor::BottomCenter,
            frac_x: 0.0,
            frac_y: 0.025,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 4.0,
        },
    ],
    // Colosseum
    [
        Short {
            file: "sun.png",
            height_factor: 0.25,
            min_h: 100.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.0,
            off_x: 50.0,
            off_y_frac: -0.07,
            texture_scale: 3.0,
        },
        Short {
            file: "colosseum.png",
            height_factor: 0.6,
            min_h: 200.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.10,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 3.0,
        },
    ],
    // Chichen Itza — the mid-ground's offset is a flat 70, not a fraction.
    [
        Short {
            file: "sun.png",
            height_factor: 0.4,
            min_h: 200.0,
            anchor: Anchor::Center,
            frac_x: 0.55,
            frac_y: 0.2,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 4.0,
        },
        Short {
            file: "chichen.png",
            height_factor: 0.4,
            min_h: 180.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.0,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 4.0,
        },
    ],
    // Machu Picchu
    [
        Short {
            file: "sun.png",
            height_factor: 0.15,
            min_h: 100.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.0,
            off_x: 150.0,
            off_y_frac: -0.08,
            texture_scale: 3.0,
        },
        Short {
            file: "machu-picchu.png",
            height_factor: 0.65,
            min_h: 230.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.12,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 3.0,
        },
    ],
    // Taj Mahal — the reflecting pool is drawn only when the frame is tall.
    [
        Short {
            file: "sun.png",
            height_factor: 0.3,
            min_h: 140.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.0,
            off_x: -100.0,
            off_y_frac: -0.02,
            texture_scale: 3.0,
        },
        Short {
            file: "taj-mahal.png",
            height_factor: 0.6,
            min_h: 230.0,
            anchor: Anchor::Center,
            frac_x: 0.0,
            frac_y: 0.12,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 3.0,
        },
    ],
    // Christ the Redeemer
    [
        Short {
            file: "sun.png",
            height_factor: 0.25,
            min_h: 120.0,
            anchor: Anchor::Center,
            frac_x: 0.7,
            frac_y: -0.5,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 3.5,
        },
        Short {
            file: "redeemer.png",
            height_factor: 1.0,
            min_h: 0.0,
            anchor: Anchor::BottomCenter,
            frac_x: 0.0,
            frac_y: 0.5,
            off_x: 0.0,
            off_y_frac: 0.0,
            texture_scale: 3.5,
        },
    ],
];

/// The aspect of a file, read off the wonder's own piece table so the two
/// cannot disagree about the same image.
fn aspect_of(w: &Wonder, file: &str) -> f32 {
    w.pieces
        .iter()
        .find(|p| p.file == file)
        .map(|p| p.aspect)
        .unwrap_or(1.0)
}

/// The band across the top of an article: the wonder's colour, its texture,
/// its sky piece and the wonder itself, clipped to `HERO_H`.
///
/// `screen_h` is the whole screen, because that is what the app's offsets are a
/// fraction of.
pub fn hero(index: usize, w: f32, screen_h: f32) -> Option<Node> {
    let wonder = &WONDERS[index % WONDERS.len()];
    let short = &SHORT[index % SHORT.len()];
    let mut root = col(w, HERO_H, wonder.fg)?.i32_attr(attr::clip(), 1);

    // The texture, scaled up as the short-mode arm asks. COVER on a box this
    // much larger than the band is the same as Flutter scaling the tile.
    let ts = short[0].texture_scale;
    if let Some(t) = Node::new(ty::image()) {
        root = root.child(
            t.width(w * ts)
                .height(HERO_H * ts)
                .string_attr(
                    attr::image_src(),
                    &format!("resource://RAWFILE/{APP}/{}/texture.png", wonder.dir),
                )
                .i32_attr(attr::image_fit(), 1)
                .f32v_attr(
                    attr::position(),
                    &[-w * (ts - 1.0) / 2.0, -HERO_H * (ts - 1.0) / 2.0],
                ),
        );
    }

    for s in short {
        let p = Piece {
            file: s.file,
            aspect: aspect_of(wonder, s.file),
            height_factor: s.height_factor,
            min_h: s.min_h,
            anchor: s.anchor,
            frac_x: s.frac_x,
            frac_y: s.frac_y,
            off_x: s.off_x,
            off_y: screen_h * s.off_y_frac,
        };
        if let Some(node) = super::illustration::piece_node(wonder, &p, w, HERO_H) {
            root = root.child(node);
        }
    }
    Some(root)
}
