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
/// The field reports here on every keystroke; the handler reads the text back
/// off the node, which is where it lives.
pub const SEARCH_TYPED: i32 = 7381;

/// The field's node, and what has been typed into it.
static FIELD: std::sync::Mutex<usize> = std::sync::Mutex::new(0);
static TYPED: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
/// The result grid, kept mounted so a keystroke never rebuilds the screen.
///
/// It has to be this way round: rebuilding drops the TextInput and builds a new
/// one, which loses focus, so the keyboard closed after the first character.
static GRID: std::sync::Mutex<Option<Grid>> = std::sync::Mutex::new(None);

struct Grid {
    /// (tile, label) per slot. Nine slots whose contents change, rather than
    /// one node per corpus entry -- there are hundreds of those per wonder.
    tiles: Vec<(usize, usize)>,
    /// The "n artifact(s)" line.
    count: usize,
    /// Where tile `i` goes once `i` visible tiles are before it.
    origin: (f32, f32),
    step: (f32, f32),
    cell: f32,
    index: usize,
}

/// Re-run the filter over the mounted grid: show what matches, hide what does
/// not, close the gaps, and correct the count.
fn apply_filter() {
    let Ok(g) = GRID.lock() else { return };
    let Some(grid) = g.as_ref() else { return };
    let term = typed();
    let term = if term.is_empty() { None } else { Some(term) };
    let hits = matches(grid.index, term.as_deref());
    for (i, &(tile, label)) in grid.tiles.iter().enumerate() {
        // ARKUI_VISIBILITY_VISIBLE / _NONE.
        let vis = if i < hits.len() { 0 } else { 2 };
        unsafe {
            Node::set_i32_raw(tile as crate::arkui::NodeHandle, attr::visibility(), vis);
            Node::set_i32_raw(label as crate::arkui::NodeHandle, attr::visibility(), vis);
        }
        let Some(found) = hits.get(i) else { continue };
        let (gx, gy) = (i % 3, i / 3);
        let x = grid.origin.0 + grid.step.0 * gx as f32;
        let y = grid.origin.1 + grid.step.1 * gy as f32;
        unsafe {
            Node::set_string_raw(
                tile as crate::arkui::NodeHandle,
                attr::image_src(),
                &super::corpus::thumb_url(found.id),
            );
            Node::set_f32v_raw(tile as crate::arkui::NodeHandle, attr::position(), &[x, y]);
            Node::set_string_raw(
                label as crate::arkui::NodeHandle,
                attr::text_content(),
                found.title,
            );
            Node::set_f32v_raw(
                label as crate::arkui::NodeHandle,
                attr::position(),
                &[x, y + grid.cell + 3.0],
            );
        }
    }
    if grid.count != 0 {
        unsafe {
            Node::set_string_raw(
                grid.count as crate::arkui::NodeHandle,
                attr::text_content(),
                &format!("{} artifact(s)", hits.len()),
            )
        };
    }
}

/// What is in the field now. Returns true if it changed.
pub fn read_typed() -> bool {
    let node = FIELD.lock().map(|g| *g).unwrap_or(0);
    if node == 0 {
        return false;
    }
    let Some(v) = (unsafe {
        crate::arkui::Node::get_string(node as crate::arkui::NodeHandle, attr::input_text())
    }) else {
        return false;
    };
    let changed = match TYPED.lock() {
        Ok(mut g) if *g != v => {
            *g = v;
            true
        }
        _ => false,
    };
    if changed {
        apply_filter();
    }
    // Never a rebuild: the grid was just updated in place, and rebuilding
    // would take the keyboard away mid-word.
    false
}

pub fn typed() -> String {
    TYPED.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Forget the field when the screen goes away, so a stale handle is never read.
pub fn clear_field() {
    if let Ok(mut g) = FIELD.lock() {
        *g = 0;
    }
    if let Ok(mut g) = TYPED.lock() {
        g.clear();
    }
    if let Ok(mut g) = GRID.lock() {
        *g = None;
    }
}
pub const CHIP_BASE: i32 = 7390;

/// How many chips fit on the screen at once.
pub const CHIPS: usize = 8;

/// Artifacts of `index` matching `term`, or all of them when nothing is chosen.
/// The wonder's search corpus, filtered as the app filters it: a term is
/// matched against the title and the keyword list.
///
/// The app searches this, not the handful of highlight artifacts, which is why
/// its suggestion chips always return something.
fn matches(index: usize, term: Option<&str>) -> Vec<&'static super::corpus::Found> {
    let list = super::corpus::CORPUS[index % super::corpus::CORPUS.len()];
    match term {
        None => list.iter().take(GRID_MAX).collect(),
        Some(t) => {
            let t = t.to_ascii_lowercase();
            list.iter()
                .filter(|a| a.title.to_ascii_lowercase().contains(&t) || a.keywords.contains(&t))
                .take(GRID_MAX)
                .collect()
        }
    }
}

/// How many tiles the grid holds. The corpus runs to hundreds per wonder and
/// the screen shows three across; mounting them all to hide most of them would
/// be thousands of nodes for nine visible.
const GRID_MAX: usize = 9;

pub fn build(index: usize, chip: Option<usize>, w: f32, h: f32) -> Option<Node> {
    let wonder = &WONDERS[index % WONDERS.len()];
    let words = SUGGESTIONS[index % SUGGESTIONS.len()];
    let term = chip.and_then(|c| words.get(c)).copied();
    // A chip sets the term; anything typed narrows it further, which is what
    // the app's field does over its own suggestion list.
    let live = typed();
    let effective = term.or(if live.is_empty() {
        None
    } else {
        Some(live.as_str())
    });
    let found = matches(index, effective);

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

    // The field, as `_SearchInput` has it: a real one. Tapping a chip fills it
    // in, and typing filters as you go.
    let field = Node::new(crate::arkui::ty::input())?
        .width(w - 48.0)
        .height(46.0)
        .bg(0x1AF8ECE5)
        .radius(23.0)
        .font_size(15.0)
        .font_color(SHEET)
        .string_attr(attr::font_family(), BODY_FONT)
        .string_attr(attr::input_placeholder(), "Search the collection")
        .string_attr(attr::input_text(), term.unwrap_or(&typed()))
        .f32v_attr(attr::padding(), &[0.0, 22.0, 0.0, 22.0])
        .f32v_attr(attr::position(), &[24.0, h * 0.15])
        .on_event(crate::arkui::event::input_change(), SEARCH_TYPED);
    if let Ok(mut g) = FIELD.lock() {
        *g = field.raw() as usize;
    }
    root = root.child(field);

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

    let count = text(
        &format!("{} artifact(s)", found.len()),
        12.0,
        0x99F8ECE5,
        w,
        22.0,
    )?
    .string_attr(attr::font_family(), BODY_FONT)
    .i32_attr(attr::text_align(), 1)
    .f32v_attr(attr::position(), &[0.0, chip_top + 104.0]);
    let count_node = count.raw() as usize;
    root = root.child(count);

    // Results. Every artifact the wonder has is mounted; the filter hides the
    // ones that do not match and closes the gaps, so typing never rebuilds.
    let gw = (w - 48.0 - 16.0) / 3.0;
    let gtop = chip_top + 134.0;
    let step = (gw + 8.0, gw + 26.0);
    // Nine slots. The thumbnails come off the app's own host by object id, as
    // `getSelfHostedImageUrlSmall` does -- the corpus is far too large to ship
    // as images, and the app does not ship them either.
    let mut tiles = Vec::with_capacity(GRID_MAX);
    for i in 0..GRID_MAX {
        let (gx, gy) = (i % 3, i / 3);
        let x = 24.0 + step.0 * gx as f32;
        let y = gtop + step.1 * gy as f32;
        let hit = found.get(i);
        let vis = if hit.is_some() { 0 } else { 2 };
        let tile = Node::new(crate::arkui::ty::image())?
            .width(gw)
            .height(gw)
            .radius(6.0)
            .bg(0x14F8ECE5)
            .string_attr(
                attr::image_src(),
                &hit.map(|f| super::corpus::thumb_url(f.id))
                    .unwrap_or_default(),
            )
            // COVER, as the app's grid tiles are.
            .i32_attr(attr::image_fit(), 1)
            .i32_attr(attr::visibility(), vis)
            .f32v_attr(attr::position(), &[x, y]);
        let label = text(
            hit.map(|f| f.title).unwrap_or(""),
            10.0,
            0x99F8ECE5,
            gw,
            20.0,
        )?
        .string_attr(attr::font_family(), BODY_FONT)
        .i32_attr(attr::text_align(), 1)
        .i32_attr(attr::visibility(), vis)
        .f32v_attr(attr::position(), &[x, y + gw + 3.0]);
        tiles.push((tile.raw() as usize, label.raw() as usize));
        root = root.child(tile);
        root = root.child(label);
    }
    if let Ok(mut g) = GRID.lock() {
        *g = Some(Grid {
            tiles,
            count: count_node,
            origin: (24.0, gtop),
            step,
            cell: gw,
            index,
        });
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
