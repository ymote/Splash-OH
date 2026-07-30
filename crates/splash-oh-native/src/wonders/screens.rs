//! The screens either side of the wonder pager: intro, the home menu, the
//! collection, and one artifact.
//!
//! Copy is the app's own, from `app_en.arb`. Structure follows
//! `intro_screen.dart`, `home_menu.dart`, `collection_screen.dart` and
//! `artifact_details_screen.dart`.

use super::data::WONDERS;
use super::editorial_data::EDITORIAL;
use crate::arkui::{attr, ty, Node};
use crate::ui::*;

const APP: &str = "wonders";
const SHEET: u32 = 0xFFF8ECE5;
const BODY: u32 = 0xFF514F4D;
const GREY_STRONG: u32 = 0xFF272625;
const ACCENT: u32 = 0xFFE4935D;
const DISPLAY: &str = "YesevaOne";
const SERIF_UI: &str = "TenorSans";
const BODY_FONT: &str = "Raleway";

pub const INTRO_NEXT: i32 = 7300;
pub const INTRO_ENTER: i32 = 7301;
pub const MENU_CLOSE: i32 = 7310;
pub const MENU_BASE: i32 = 7320;
pub const COLLECTION_CLOSE: i32 = 7340;
pub const MENU_COLLECTION: i32 = 7341;
pub const MENU_TIMELINE: i32 = 7342;
pub const ARTIFACT_CLOSE: i32 = 7350;

/// The three onboarding pages, verbatim from the app.
pub const INTRO: &[(&str, &str, &str)] = &[
    (
        "Journey to the past",
        "Navigate the intersection of time, art, and culture.",
        "intro-statue.jpg",
    ),
    (
        "Explore places",
        "Uncover remarkable human-made structures from around the world.",
        "intro-camel.jpg",
    ),
    (
        "Discover artifacts",
        "Learn about cultures throughout time by examining things they left behind.",
        "intro-petra.jpg",
    ),
];

/// The onboarding carousel: a masked photograph, a title, a line of copy, and
/// the control that moves on.
pub fn intro(page: usize, w: f32, h: f32) -> Option<Node> {
    let p = &INTRO[page.min(INTRO.len() - 1)];
    let mut root = stack(w, h, GREY_STRONG)?;

    let ph = h * 0.52;
    root = root.child(
        photo(APP, &format!("_common/{}", p.2), w, ph, 0.0)?
            .f32v_attr(attr::position(), &[0.0, 0.0]),
    );
    // The app masks the photograph into a soft shape; a gradient scrim to the
    // page colour is the closest thing available without a mask node.
    root =
        root.child(col(w, ph * 0.45, 0xCC272625)?.f32v_attr(attr::position(), &[0.0, ph * 0.55]));

    root = root.child(
        text("WONDEROUS", 13.0, ACCENT, w, 26.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h * 0.60]),
    );
    root = root.child(
        text(p.0, 32.0, SHEET, w - 60.0, 48.0)?
            .string_attr(attr::font_family(), DISPLAY)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[30.0, h * 0.65]),
    );
    root = root.child(
        text(p.1, 15.0, 0xCCF8ECE5, w - 80.0, 76.0)?
            .string_attr(attr::font_family(), BODY_FONT)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[40.0, h * 0.72]),
    );

    // Page dots.
    let (d, gap) = (7.0, 11.0);
    let total = INTRO.len() as f32 * d + (INTRO.len() as f32 - 1.0) * gap;
    let mut dots = row(w, 24.0, 0x00000000)?
        .f32v_attr(attr::position(), &[0.0, h * 0.84])
        .f32v_attr(attr::padding(), &[8.0, 0.0, 0.0, (w - total) / 2.0]);
    for i in 0..INTRO.len() {
        dots = dots.child(
            col(d, d, if i == page { SHEET } else { 0x59F8ECE5 })?
                .radius(d / 2.0)
                .f32v_attr(attr::margin(), &[0.0, gap / 2.0, 0.0, gap / 2.0]),
        );
    }
    root = root.child(dots);

    let last = page + 1 == INTRO.len();
    let label = if last { "ENTER" } else { "NEXT" };
    let id = if last { INTRO_ENTER } else { INTRO_NEXT };
    root = root.child(
        col(w * 0.5, 52.0, ACCENT)?
            .radius(26.0)
            .f32v_attr(attr::position(), &[w * 0.25, h * 0.89])
            .child(
                text(label, 14.0, GREY_STRONG, w * 0.5, 52.0)?
                    .string_attr(attr::font_family(), SERIF_UI)
                    .i32_attr(attr::text_align(), 1)
                    .f32v_attr(attr::padding(), &[18.0, 0.0, 0.0, 0.0]),
            ),
    );
    root = root.child(super::hits(
        w,
        h,
        &[(w * 0.25, h * 0.89, w * 0.5, 52.0, id)],
    )?);
    Some(root)
}

/// The menu behind the hamburger: every wonder as a row, the current one
/// marked, over the current wonder's own colour.
pub fn menu(current: usize, w: f32, h: f32) -> Option<Node> {
    let wonder = &WONDERS[current % WONDERS.len()];
    let mut root = stack(w, h, wonder.bg)?;

    root = root.child(
        text("WONDEROUS", 13.0, ACCENT, w, 26.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h * 0.07]),
    );

    // Ten rows have to fit between the wordmark and the bottom: the eight
    // wonders plus Collection and Timeline. At a fixed 74 vp the last two fell
    // past the bottom edge and could not be reached at all.
    let top = h * 0.125;
    let row_h = ((h * 0.86 - top) / 10.5).clamp(44.0, 74.0);
    let mut menu_hits: Vec<(f32, f32, f32, f32, i32)> = Vec::new();
    for (i, wo) in WONDERS.iter().enumerate() {
        let y = top + row_h * i as f32;
        let on = i == current % WONDERS.len();
        let mut r = row(w, row_h, 0x00000000)?.f32v_attr(attr::position(), &[0.0, y]);
        r = r.child(
            photo(
                APP,
                &format!("{}/button.png", wo.dir),
                row_h * 0.62,
                row_h * 0.62,
                row_h * 0.31,
            )?
            .f32v_attr(attr::margin(), &[row_h * 0.19, 14.0, 0.0, 26.0]),
        );
        let mut t = col(w - 110.0, row_h, 0x00000000)?;
        t = t.child(
            text(
                wo.title,
                21.0,
                if on { SHEET } else { 0xB3F8ECE5 },
                w - 110.0,
                32.0,
            )?
            .string_attr(attr::font_family(), DISPLAY)
            .f32v_attr(attr::padding(), &[14.0, 0.0, 0.0, 0.0]),
        );
        t = t.child(
            text(EDITORIAL[i].region, 12.0, 0x8CF8ECE5, w - 110.0, 20.0)?
                .string_attr(attr::font_family(), BODY_FONT),
        );
        r = r.child(t);
        root = root.child(r);
        menu_hits.push((0.0, y, w, row_h, MENU_BASE + i as i32));
    }

    // The two destinations that are not wonders, as the app lists them under
    // the eight.
    let extras_y = top + row_h * WONDERS.len() as f32 + row_h * 0.15;
    for (i, (label, glyph, id)) in [
        ("Collection", "icon-collection.png", MENU_COLLECTION),
        ("Timeline", "icon-timeline.png", MENU_TIMELINE),
    ]
    .iter()
    .enumerate()
    {
        let y = extras_y + row_h * i as f32;
        root = root.child(
            icon(APP, &format!("_common/icons/{glyph}"), 22.0)?
                .f32v_attr(attr::position(), &[38.0, y + row_h * 0.25]),
        );
        root = root.child(
            text(label, 16.0, 0xCCF8ECE5, w - 90.0, 26.0)?
                .string_attr(attr::font_family(), SERIF_UI)
                .f32v_attr(attr::position(), &[86.0, y + row_h * 0.22]),
        );
        menu_hits.push((0.0, y, w, row_h, *id));
    }

    root = root.child(
        stack(46.0, 46.0, 0xB3272625)?
            .radius(23.0)
            .f32v_attr(attr::position(), &[w - 66.0, h * 0.055])
            .child(icon(APP, "_common/icons/icon-close.png", 20.0)?),
    );
    root = root.child(super::hits(
        w,
        h,
        &[(w - 76.0, h * 0.045, 66.0, 66.0, MENU_CLOSE)],
    )?);
    Some(root)
}

/// The collection: every wonder's artifact tile on a dark grid, which is what
/// `collection_screen.dart` shows once anything has been found.
pub fn collection(w: f32, h: f32) -> Option<Node> {
    let mut root = stack(w, h, GREY_STRONG)?;
    root = root.child(
        text("COLLECTION", 13.0, ACCENT, w, 26.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h * 0.06]),
    );
    root = root.child(
        text("Artifacts you have discovered", 14.0, 0x99F8ECE5, w, 24.0)?
            .string_attr(attr::font_family(), BODY_FONT)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h * 0.10]),
    );

    let pad = 20.0;
    let gap = 10.0;
    let cell = (w - pad * 2.0 - gap * 2.0) / 3.0;
    let top = h * 0.16;
    for (i, wo) in WONDERS.iter().enumerate() {
        let (cx, cy) = (i % 3, i / 3);
        let x = pad + (cell + gap) * cx as f32;
        let y = top + (cell + 34.0) * cy as f32;
        root = root.child(
            stack(cell, cell, 0xFF1E1B18)?
                .radius(6.0)
                .f32v_attr(attr::position(), &[x, y])
                .child(photo(
                    APP,
                    &format!("{}/photo-3.jpg", wo.dir),
                    cell,
                    cell,
                    6.0,
                )?),
        );
        root = root.child(
            text(wo.title, 11.0, 0x99F8ECE5, cell, 26.0)?
                .string_attr(attr::font_family(), BODY_FONT)
                .i32_attr(attr::text_align(), 1)
                .f32v_attr(attr::position(), &[x, y + cell + 5.0]),
        );
    }

    root = root.child(
        stack(46.0, 46.0, 0x33F8ECE5)?
            .radius(23.0)
            .f32v_attr(attr::position(), &[w - 66.0, h * 0.045])
            .child(icon(APP, "_common/icons/icon-close.png", 20.0)?),
    );
    root = root.child(super::hits(
        w,
        h,
        &[(w - 76.0, h * 0.035, 66.0, 66.0, COLLECTION_CLOSE)],
    )?);
    Some(root)
}

/// One artifact, full screen: the piece on a pale ground with its name, date
/// and a line about it — `artifact_details_screen.dart`.
pub fn artifact(index: usize, w: f32, h: f32) -> Option<Node> {
    let wo = &WONDERS[index % WONDERS.len()];
    let e = &EDITORIAL[index % EDITORIAL.len()];
    let mut root = stack(w, h, SHEET)?;

    let ih = h * 0.46;
    root = root.child(
        photo(APP, &format!("{}/photo-3.jpg", wo.dir), w, ih, 0.0)?
            .f32v_attr(attr::position(), &[0.0, 0.0]),
    );

    root = root.child(
        text(wo.title, 30.0, GREY_STRONG, w - 56.0, 44.0)?
            .string_attr(attr::font_family(), DISPLAY)
            .f32v_attr(attr::position(), &[28.0, ih + 26.0]),
    );
    root = root.child(
        text(e.region, 13.0, ACCENT, w - 56.0, 24.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .f32v_attr(attr::position(), &[28.0, ih + 74.0]),
    );
    root = root
        .child(col(w - 56.0, 1.0, 0x33272625)?.f32v_attr(attr::position(), &[28.0, ih + 106.0]));
    root = root.child(
        text(e.callout1, 15.0, BODY, w - 56.0, h * 0.3)?
            .string_attr(attr::font_family(), BODY_FONT)
            .f32v_attr(attr::position(), &[28.0, ih + 124.0]),
    );

    root = root.child(
        stack(46.0, 46.0, 0x33272625)?
            .radius(23.0)
            .f32v_attr(attr::position(), &[w - 66.0, h * 0.045])
            .child(icon(APP, "_common/icons/icon-close.png", 20.0)?),
    );
    root = root.child(super::hits(
        w,
        h,
        &[(w - 76.0, h * 0.035, 66.0, 66.0, ARTIFACT_CLOSE)],
    )?);
    let _ = ty::text();
    Some(root)
}
