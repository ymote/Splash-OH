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
use splash_oh_arkui::arkui::{attr, Node};
use splash_oh_arkui::ui::*;

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
/// The year pill at the bottom, and the two handles it opens.
pub const RANGE_TOGGLE: i32 = 7382;
pub const RANGE_START: i32 = 7383;
pub const RANGE_END: i32 = 7384;

/// Whether the time-range panel is open, and where its handles are.
///
/// `ExpandingTimeRangeSelector` sits at the bottom of the search screen as a
/// pill showing the range, and opens into two handles over a year axis. The
/// years start at the wonder's own `artifactStartYr`/`artifactEndYr`.
static RANGE_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static RANGE: std::sync::Mutex<Option<(i32, i32)>> = std::sync::Mutex::new(None);
/// The two slider nodes, so a drag can be read back off them.
static HANDLES: std::sync::Mutex<(usize, usize)> = std::sync::Mutex::new((0, 0));
/// The pill's text and the two row labels, which say the years and so have to
/// follow the handles. Nothing rebuilds during a drag, so they are written to.
static RANGE_LABELS: std::sync::Mutex<(usize, usize, usize)> = std::sync::Mutex::new((0, 0, 0));

pub fn range_open() -> bool {
    RANGE_OPEN.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn toggle_range() -> bool {
    RANGE_OPEN.fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
    true
}

/// The range for `index`, defaulting to the wonder's own span.
fn range_for(index: usize) -> (i32, i32) {
    let d = super::data::ARTIFACT_YEARS[index % super::data::ARTIFACT_YEARS.len()];
    RANGE.lock().ok().and_then(|g| *g).unwrap_or(d)
}

/// Read both handles back and re-filter. Returns false: the grid is updated in
/// place, exactly as typing does, so dragging is not interrupted by a rebuild.
pub fn read_range() -> bool {
    let (a, b) = HANDLES.lock().map(|g| *g).unwrap_or((0, 0));
    if a == 0 || b == 0 {
        return false;
    }
    let get = |n: usize| unsafe {
        Node::get_f32(
            n as splash_oh_arkui::arkui::NodeHandle,
            attr::slider_value(),
            0,
        )
    };
    let (Some(s), Some(e)) = (get(a), get(b)) else {
        return false;
    };
    // Either handle may be dragged past the other; the range is what lies
    // between them, which is what the app's two-thumb selector gives you.
    let (lo, hi) = if s <= e { (s, e) } else { (e, s) };
    let (lo, hi) = (lo.round() as i32, hi.round() as i32);
    if let Ok(mut g) = RANGE.lock() {
        *g = Some((lo, hi));
    }
    if let Ok(g) = RANGE_LABELS.lock() {
        let (pill, from, to) = *g;
        let put = |n: usize, t: &str| {
            if n != 0 {
                unsafe {
                    Node::set_string_raw(
                        n as splash_oh_arkui::arkui::NodeHandle,
                        attr::text_content(),
                        t,
                    )
                };
            }
        };
        put(pill, &format!("{} - {}", year_label(lo), year_label(hi)));
        put(from, &year_label(lo));
        put(to, &year_label(hi));
    }
    apply_filter();
    false
}

/// A year as the app writes it — `StringUtils.formatYr`.
fn year_label(y: i32) -> String {
    let y = if y == 0 { 1 } else { y };
    if y < 0 {
        format!("{} BCE", -y)
    } else {
        format!("{y} CE")
    }
}

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
        let vis = if i < hits.len().min(GRID_MAX) { 0 } else { 2 };
        unsafe {
            Node::set_i32_raw(
                tile as splash_oh_arkui::arkui::NodeHandle,
                attr::visibility(),
                vis,
            );
            Node::set_i32_raw(
                label as splash_oh_arkui::arkui::NodeHandle,
                attr::visibility(),
                vis,
            );
        }
        let Some(found) = hits.get(i) else { continue };
        let (gx, gy) = (i % 3, i / 3);
        let x = grid.origin.0 + grid.step.0 * gx as f32;
        let y = grid.origin.1 + grid.step.1 * gy as f32;
        unsafe {
            Node::set_string_raw(
                tile as splash_oh_arkui::arkui::NodeHandle,
                attr::image_src(),
                &super::corpus::thumb_url(found.id),
            );
            Node::set_f32v_raw(
                tile as splash_oh_arkui::arkui::NodeHandle,
                attr::position(),
                &[x, y],
            );
            Node::set_string_raw(
                label as splash_oh_arkui::arkui::NodeHandle,
                attr::text_content(),
                found.title,
            );
            Node::set_f32v_raw(
                label as splash_oh_arkui::arkui::NodeHandle,
                attr::position(),
                &[x, y + grid.cell + 3.0],
            );
        }
    }
    if grid.count != 0 {
        unsafe {
            Node::set_string_raw(
                grid.count as splash_oh_arkui::arkui::NodeHandle,
                attr::text_content(),
                &format!("{} artifact(s)", hits.len()),
            )
        };
    }
}

/// Adopt a term chosen some other way than typing — a suggestion chip.
///
/// The chip used to live only in the route, so it filtered the build and then
/// nothing else knew about it: opening the year panel and dragging a handle
/// re-filtered on an empty field and threw the chip's results away.
pub fn set_term(t: &str) {
    if let Ok(mut g) = TYPED.lock() {
        *g = t.to_string();
    }
}

/// What is in the field now. Returns true if it changed.
pub fn read_typed() -> bool {
    let node = FIELD.lock().map(|g| *g).unwrap_or(0);
    if node == 0 {
        return false;
    }
    let Some(v) = (unsafe {
        splash_oh_arkui::arkui::Node::get_string(
            node as splash_oh_arkui::arkui::NodeHandle,
            attr::input_text(),
        )
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
/// Forget what was searched for, when the screen is left for good.
///
/// Separate from `forget_nodes`: the handles go stale on every rebuild, but a
/// chip tap rebuilds too and the term has to survive that.
pub fn reset_query() {
    if let Ok(mut g) = TYPED.lock() {
        g.clear();
    }
    if let Ok(mut g) = RANGE.lock() {
        *g = None;
    }
    RANGE_OPEN.store(false, std::sync::atomic::Ordering::Relaxed);
}

/// Drop every handle this module holds. See `details::forget_nodes`.
pub fn forget_nodes() {
    if let Ok(mut g) = FIELD.lock() {
        *g = 0;
    }
    if let Ok(mut g) = GRID.lock() {
        *g = None;
    }
    if let Ok(mut g) = HANDLES.lock() {
        *g = (0, 0);
    }
    if let Ok(mut g) = RANGE_LABELS.lock() {
        *g = (0, 0, 0);
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
    let (lo, hi) = range_for(index);
    let word = term.map(|t| t.to_ascii_lowercase());
    // No `take` here. Capping inside the filter made the count line report the
    // number of tiles rather than the number of matches -- "9 artifact(s)" for
    // a term that matches two hundred. The caller takes what it can show.
    list.iter()
        .filter(|a| a.year >= lo && a.year <= hi)
        .filter(|a| match word.as_deref() {
            None => true,
            Some(t) => a.title.to_ascii_lowercase().contains(t) || a.keywords.contains(t),
        })
        .collect()
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
    let field = Node::new(splash_oh_arkui::arkui::ty::input())?
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
        .on_event(splash_oh_arkui::arkui::event::input_change(), SEARCH_TYPED);
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
        let tile = Node::new(splash_oh_arkui::arkui::ty::image())?
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

    // `ExpandingTimeRangeSelector`, pinned to the bottom: a pill showing the
    // range, which opens into two handles over the wonder's own span.
    let (lo, hi) = range_for(index);
    let (dlo, dhi) = super::data::ARTIFACT_YEARS[index % super::data::ARTIFACT_YEARS.len()];
    let open = range_open();
    let panel_h: f32 = if open { 150.0 } else { 52.0 };
    let panel_w = w - 48.0;
    let panel_y = h - panel_h - 24.0;
    let mut panel = stack(panel_w, panel_h, 0xD91E1B18)?
        .radius(12.0)
        .f32v_attr(attr::position(), &[24.0, panel_y]);
    // The closed row: the range, then the calendar-edit glyph in accent1.
    let pill = text(
        &format!("{} - {}", year_label(lo), year_label(hi)),
        14.0,
        SHEET,
        panel_w - 60.0,
        22.0,
    )?
    .string_attr(attr::font_family(), SERIF_UI)
    .f32v_attr(attr::position(), &[20.0, 15.0]);
    let pill_node = pill.raw() as usize;
    let mut row_labels = (0usize, 0usize);
    panel = panel.child(pill);
    panel = panel.child(
        icon(APP, "_common/icons/icon-timeline.png", 18.0)?
            .f32v_attr(attr::position(), &[panel_w - 40.0, 17.0]),
    );
    targets.push((24.0, panel_y, panel_w, 52.0, RANGE_TOGGLE));

    if open {
        // One row per end of the range.
        //
        // The app draws a single axis with two thumbs on it. ArkUI has no
        // range slider, and two full-width ones stacked on the same line means
        // the upper one takes every touch -- the lower thumb could be seen and
        // never grabbed. A row each keeps both reachable and filters the same.
        let mut handles = (0usize, 0usize);
        for (k, (label, v, id)) in [("From", lo, RANGE_START), ("To", hi, RANGE_END)]
            .iter()
            .enumerate()
        {
            let row_y = 58.0 + k as f32 * 44.0;
            panel = panel.child(
                text(label, 10.0, 0x8CF8ECE5, 34.0, 16.0)?
                    .string_attr(attr::font_family(), SERIF_UI)
                    .f32v_attr(attr::position(), &[20.0, row_y + 8.0]),
            );
            let sl = Node::new(splash_oh_arkui::arkui::ty::slider())?
                .width(panel_w - 150.0)
                .height(32.0)
                .f32_attr(attr::slider_min(), dlo as f32)
                .f32_attr(attr::slider_max(), dhi as f32)
                .f32_attr(attr::slider_value(), *v as f32)
                .u32_attr(attr::slider_selected(), ACCENT)
                .u32_attr(attr::slider_block(), SHEET)
                .u32_attr(attr::slider_track(), 0x33F8ECE5)
                .f32v_attr(attr::position(), &[58.0, row_y])
                .on_event(splash_oh_arkui::arkui::event::slider_change(), *id);
            if k == 0 {
                handles.0 = sl.raw() as usize;
            } else {
                handles.1 = sl.raw() as usize;
            }
            panel = panel.child(sl);
            let yl = text(&year_label(*v), 10.0, SHEET, 80.0, 16.0)?
                .string_attr(attr::font_family(), BODY_FONT)
                .i32_attr(attr::text_align(), 2)
                .f32v_attr(attr::position(), &[panel_w - 92.0, row_y + 8.0]);
            if k == 0 {
                row_labels.0 = yl.raw() as usize;
            } else {
                row_labels.1 = yl.raw() as usize;
            }
            panel = panel.child(yl);
        }
        if let Ok(mut g) = HANDLES.lock() {
            *g = handles;
        }
    } else if let Ok(mut g) = HANDLES.lock() {
        *g = (0, 0);
    }
    if let Ok(mut g) = RANGE_LABELS.lock() {
        *g = (pill_node, row_labels.0, row_labels.1);
    }
    root = root.child(panel);

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
