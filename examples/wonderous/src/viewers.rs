//! The three fullscreen viewers the app opens over a screen.
//!
//! - `fullscreen_url_img_viewer.dart` — the selected gallery photograph, large,
//!   on black, with a back button and a pair of prev/next circles.
//! - `fullscreen_video_viewer.dart` — the editorial's film. The app embeds a
//!   YouTube iframe player; here that is an ArkWeb surface over the same rect,
//!   which is the only way to play YouTube either way.
//! - `fullscreen_maps_viewer.dart` — where the wonder is. The app uses Google
//!   Maps, which needs a key and is not on this platform, so this is
//!   OpenStreetMap at the same coordinates and a comparable zoom. Same picture,
//!   different cartographer.

use super::data::WONDERS;
use super::places::PLACES;
use splash_oh_native::arkui::{attr, Node};
use splash_oh_native::ui::*;

const APP: &str = "wonders";
const SHEET: u32 = 0xFFF8ECE5;

pub const VIEWER_CLOSE: i32 = 7450;
pub const VIEWER_PREV: i32 = 7451;
pub const VIEWER_NEXT: i32 = 7452;

/// The round back button the app puts in the top-left of every fullscreen
/// viewer, and the tap target for it.
fn close_button(w: f32, h: f32, targets: &mut Vec<(f32, f32, f32, f32, i32)>) -> Option<Node> {
    let _ = w;
    targets.push((14.0, h * 0.02, 66.0, 66.0, VIEWER_CLOSE));
    Some(
        stack(46.0, 46.0, 0x59000000)?
            .radius(23.0)
            .f32v_attr(attr::position(), &[24.0, h * 0.03])
            .child(icon(APP, "_common/icons/icon-close.png", 20.0)?),
    )
}

/// One gallery photograph, full screen — `FullscreenUrlImgViewer`.
///
/// `index` is the cell in the 5×5 wall, so paging here moves through the same
/// 24 photographs the wall is built from.
pub fn photo_viewer(wonder: usize, index: usize, w: f32, h: f32) -> Option<Node> {
    let wo = &WONDERS[wonder % WONDERS.len()];
    let n = super::details::GALLERY_PHOTOS;
    let i = index % n;
    let mut root = stack(w, h, 0xFF000000)?;

    // Fit the photograph to the frame rather than filling it: this is the view
    // you open to see the whole picture.
    root = root.child(
        photo(
            APP,
            &format!("{}/gallery/{:02}.jpg", wo.dir, i),
            w,
            h * 0.72,
            0.0,
        )?
        // ARKUI_OBJECT_FIT_CONTAIN.
        .i32_attr(attr::image_fit(), 0)
        // `photo` carries a pale placeholder so a loading tile is not a hole;
        // here it would letterbox the picture in grey on black.
        .bg(0xFF000000)
        .f32v_attr(attr::position(), &[0.0, h * 0.14]),
    );

    let mut targets: Vec<(f32, f32, f32, f32, i32)> = Vec::new();
    root = root.child(close_button(w, h, &mut targets)?);

    // Prev and next, as two circles at the bottom centre.
    let d = 52.0;
    let gap = 16.0;
    let y = h - d - 40.0;
    let x0 = (w - d * 2.0 - gap) / 2.0;
    for (k, (glyph, id)) in [
        ("icon-prev.png", VIEWER_PREV),
        ("icon-next-large.png", VIEWER_NEXT),
    ]
    .iter()
    .enumerate()
    {
        let x = x0 + (d + gap) * k as f32;
        root = root.child(
            stack(d, d, 0x33F8ECE5)?
                .radius(d / 2.0)
                .f32v_attr(attr::position(), &[x, y])
                .child(icon(APP, &format!("_common/icons/{glyph}"), 20.0)?),
        );
        targets.push((x, y, d, d, *id));
    }

    root = root.child(
        text(&format!("{} / {}", i + 1, n), 12.0, 0x99F8ECE5, w, 20.0)?
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, y - 28.0]),
    );

    root = root.child(super::hits(w, h, &targets)?);
    Some(root)
}

/// The editorial's film — `FullscreenVideoViewer`.
///
/// The player is a web surface because YouTube is: the app embeds an iframe
/// too. `web_declare` reserves the rect and the shell composites an ArkWeb view
/// over it; everything else on this screen is still a native node.
pub fn video_viewer(wonder: usize, w: f32, h: f32) -> Option<Node> {
    let (video_id, _, _) = PLACES[wonder % PLACES.len()];
    let mut root = stack(w, h, 0xFF000000)?;

    // The app gives the player a square in portrait (`aspect = 9/9`).
    let side = w;
    let top = (h - side) / 2.0;
    // YouTube's own mobile page, not an embed.
    //
    // Two embeds were tried first and neither plays here: navigating straight
    // to /embed gives the player no page origin and it answers error 153, and
    // hosting the iframe in generated markup gives it `https://localhost/`,
    // which it also refuses. The watch page has no such requirement, and it is
    // the same film.
    splash_oh_native::web_declare(
        &format!("https://m.youtube.com/watch?v={video_id}"),
        0.0,
        top,
        side,
        side,
    );

    let mut targets: Vec<(f32, f32, f32, f32, i32)> = Vec::new();
    root = root.child(close_button(w, h, &mut targets)?);
    root = root.child(super::hits(w, h, &targets)?);
    Some(root)
}

/// The strips left free of the map, top and bottom.
///
/// An ArkWeb surface is composited over the native tree, not inside it, so a
/// full-bleed map would bury the close button rather than sit under it. The app
/// can lay a transparent header over its map because both are Flutter widgets;
/// here the chrome has to have the screen to itself.
const HEADER: f32 = 76.0;
const CAPTION: f32 = 52.0;

/// Where the wonder is — `FullscreenMapsViewer`.
pub fn maps_viewer(wonder: usize, w: f32, h: f32) -> Option<Node> {
    let wo = &WONDERS[wonder % WONDERS.len()];
    let (_, lat, lng) = PLACES[wonder % PLACES.len()];
    let mut root = stack(w, h, 0xFF1E1B18)?;

    // A tenth of a degree either side is roughly the app's zoom 17 at this
    // size; the marker is the point itself.
    let d = 0.004;
    splash_oh_native::web_declare(
        &format!(
            "https://www.openstreetmap.org/export/embed.html?bbox={},{},{},{}&layer=mapnik&marker={lat},{lng}",
            lng - d,
            lat - d,
            lng + d,
            lat + d
        ),
        0.0,
        HEADER,
        w,
        h - HEADER - CAPTION,
    );

    let mut targets: Vec<(f32, f32, f32, f32, i32)> = Vec::new();
    root = root.child(close_button(w, h, &mut targets)?);
    root = root.child(
        text(wo.title, 13.0, SHEET, w, 24.0)?
            .string_attr(attr::font_family(), "TenorSans")
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h - 44.0]),
    );
    root = root.child(super::hits(w, h, &targets)?);
    Some(root)
}

/// The editorial's inline map — `_MapsSection`, aspect 1.65, which opens the
/// fullscreen map when tapped.
///
/// Built from OpenStreetMap raster tiles as ordinary Image nodes rather than a
/// web surface: a composited ArkWeb view sits at a fixed rectangle and would
/// stay put while the article scrolled past it. Tiles are just pictures, so
/// they scroll like everything else here.
pub const MAP_TAP: i32 = 7453;

/// Slippy-map tile coordinates for a point, at `z`.
fn tile_of(lat: f64, lng: f64, z: u32) -> (f64, f64) {
    let n = (1u32 << z) as f64;
    let x = (lng + 180.0) / 360.0 * n;
    let r = lat.to_radians();
    let y = (1.0 - ((r.tan() + 1.0 / r.cos()).ln()) / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

pub fn map_block(wonder: usize, w: f32) -> Option<Node> {
    let (_, lat, lng) = PLACES[wonder % PLACES.len()];
    let h = w / 1.65;
    let z = 15u32;
    let (tx, ty) = tile_of(lat, lng, z);

    let mut root = stack(w, h, 0xFFE8E3DC)?
        .i32_attr(attr::clip(), 1)
        .radius(8.0);
    // Enough 256-px tiles to cover the block, positioned so the wonder itself
    // lands in the middle.
    let t = 256.0f32;
    let cols = (w / t).ceil() as i32 + 1;
    let rows = (h / t).ceil() as i32 + 1;
    let (fx, fy) = (tx.fract() as f32, ty.fract() as f32);
    for dy in -(rows / 2 + 1)..=(rows / 2 + 1) {
        for dx in -(cols / 2 + 1)..=(cols / 2 + 1) {
            let (ix, iy) = (tx.floor() as i64 + dx as i64, ty.floor() as i64 + dy as i64);
            if iy < 0 || iy >= (1i64 << z) {
                continue;
            }
            let x = w / 2.0 - fx * t + dx as f32 * t;
            let y = h / 2.0 - fy * t + dy as f32 * t;
            if x > w || y > h || x + t < 0.0 || y + t < 0.0 {
                continue;
            }
            if let Some(n) = Node::new(splash_oh_native::arkui::ty::image()) {
                root = root.child(
                    n.width(t)
                        .height(t)
                        .string_attr(
                            attr::image_src(),
                            &format!("https://tile.openstreetmap.org/{z}/{ix}/{iy}.png"),
                        )
                        .f32v_attr(attr::position(), &[x, y]),
                );
            }
        }
    }
    // The marker, at the point itself.
    root = root.child(
        col(14.0, 14.0, 0xFFE4935D)?
            .radius(7.0)
            .f32_attr(attr::border_width(), 2.0)
            .u32_attr(attr::border_color(), SHEET)
            .f32v_attr(attr::position(), &[w / 2.0 - 7.0, h / 2.0 - 7.0]),
    );
    Some(root.on_event(splash_oh_native::arkui::event::click(), MAP_TAP))
}
