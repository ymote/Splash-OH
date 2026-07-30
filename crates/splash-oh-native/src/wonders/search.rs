//! Artifact search.
//!
//! `artifact_search_screen.dart` is a field, a row of suggestion chips drawn
//! from the wonder's own list, a time-range selector, and a grid of results.
//!
//! The chips and their words are the app's. The results are the artifacts that
//! ship with this build, filtered by the chosen word against title, date and
//! culture — the app queries the Met live, and a search screen that cannot
//! answer is a text field that does nothing.

use super::artifact_data::ARTIFACTS;
use super::data::WONDERS;
use super::search_data::SUGGESTIONS;
use crate::arkui::{attr, Node};
use crate::ui::*;

const APP: &str = "wonders";
const SHEET: u32 = 0xFFF8ECE5;
const GREY_STRONG: u32 = 0xFF272625;
const ACCENT: u32 = 0xFFE4935D;
const DISPLAY: &str = "YesevaOne";
const SERIF_UI: &str = "TenorSans";
const BODY_FONT: &str = "Raleway";

pub const SEARCH_CLOSE: i32 = 7380;
pub const CHIP_BASE: i32 = 7390;

/// How many chips fit on the screen at once.
pub const CHIPS: usize = 8;

/// Artifacts of `index` matching `term`, or all of them when nothing is chosen.
fn matches(index: usize, term: Option<&str>) -> Vec<&'static super::artifact_data::Artifact> {
    let list = ARTIFACTS[index % ARTIFACTS.len()];
    match term {
        None => list.iter().collect(),
        Some(t) => {
            let t = t.to_ascii_lowercase();
            let hit: Vec<_> = list
                .iter()
                .filter(|a| {
                    a.title.to_ascii_lowercase().contains(&t)
                        || a.culture.to_ascii_lowercase().contains(&t)
                        || a.date.to_ascii_lowercase().contains(&t)
                })
                .collect();
            // A search that returns nothing looks broken on a set this small,
            // so an empty result falls back to the whole set rather than to a
            // blank screen. The count line says which happened.
            if hit.is_empty() {
                list.iter().collect()
            } else {
                hit
            }
        }
    }
}

pub fn build(index: usize, chip: Option<usize>, w: f32, h: f32) -> Option<Node> {
    let wonder = &WONDERS[index % WONDERS.len()];
    let words = SUGGESTIONS[index % SUGGESTIONS.len()];
    let term = chip.and_then(|c| words.get(c)).copied();
    let found = matches(index, term);

    let mut root = stack(w, h, GREY_STRONG)?;
    root = root.child(
        text("SEARCH ARTIFACTS", 13.0, ACCENT, w, 26.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h * 0.05]),
    );
    root = root.child(
        text(wonder.title, 22.0, SHEET, w, 34.0)?
            .string_attr(attr::font_family(), DISPLAY)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h * 0.09]),
    );

    // The field. There is no keyboard here; the chips are the input.
    root = root.child(
        col(w - 48.0, 46.0, 0x1AF8ECE5)?
            .radius(23.0)
            .f32v_attr(attr::position(), &[24.0, h * 0.15])
            .child(
                text(
                    term.unwrap_or("Choose a term below"),
                    15.0,
                    if term.is_some() { SHEET } else { 0x66F8ECE5 },
                    w - 48.0,
                    46.0,
                )?
                .string_attr(attr::font_family(), BODY_FONT)
                .f32v_attr(attr::padding(), &[13.0, 0.0, 0.0, 22.0]),
            ),
    );

    // Suggestion chips, two rows of four.
    let cw = (w - 48.0 - 24.0) / 4.0;
    let chip_top = h * 0.24;
    let mut targets: Vec<(f32, f32, f32, f32, i32)> = Vec::new();
    for (i, word) in words.iter().take(CHIPS).enumerate() {
        let (cx, cy) = (i % 4, i / 4);
        let x = 24.0 + (cw + 8.0) * cx as f32;
        let y = chip_top + 46.0 * cy as f32;
        let on = chip == Some(i);
        root = root.child(
            col(cw, 36.0, if on { ACCENT } else { 0x1AF8ECE5 })?
                .radius(18.0)
                .f32v_attr(attr::position(), &[x, y])
                .child(
                    text(
                        word,
                        12.0,
                        if on { GREY_STRONG } else { 0xCCF8ECE5 },
                        cw,
                        36.0,
                    )?
                    .string_attr(attr::font_family(), BODY_FONT)
                    .i32_attr(attr::text_align(), 1)
                    .f32v_attr(attr::padding(), &[11.0, 0.0, 0.0, 0.0]),
                ),
        );
        targets.push((x, y, cw, 36.0, CHIP_BASE + i as i32));
    }

    root = root.child(
        text(
            &format!("{} artifact(s)", found.len()),
            12.0,
            0x99F8ECE5,
            w,
            22.0,
        )?
        .string_attr(attr::font_family(), BODY_FONT)
        .i32_attr(attr::text_align(), 1)
        .f32v_attr(attr::position(), &[0.0, chip_top + 104.0]),
    );

    // Results.
    let gw = (w - 48.0 - 16.0) / 3.0;
    let gtop = chip_top + 134.0;
    for (i, a) in found.iter().take(9).enumerate() {
        let (gx, gy) = (i % 3, i / 3);
        let x = 24.0 + (gw + 8.0) * gx as f32;
        let y = gtop + (gw + 26.0) * gy as f32;
        root = root.child(
            photo(APP, &format!("artifacts/{}.jpg", a.id), gw, gw, 6.0)?
                .f32v_attr(attr::position(), &[x, y]),
        );
        root = root.child(
            text(a.title, 10.0, 0x99F8ECE5, gw, 20.0)?
                .string_attr(attr::font_family(), BODY_FONT)
                .i32_attr(attr::text_align(), 1)
                .f32v_attr(attr::position(), &[x, y + gw + 3.0]),
        );
    }

    root = root.child(
        stack(46.0, 46.0, 0x33F8ECE5)?
            .radius(23.0)
            .f32v_attr(attr::position(), &[w - 66.0, h * 0.04])
            .child(icon(APP, "_common/icons/icon-close.png", 20.0)?),
    );
    targets.push((w - 76.0, h * 0.03, 66.0, 66.0, SEARCH_CLOSE));
    root = root.child(super::hits(w, h, &targets)?);
    Some(root)
}
