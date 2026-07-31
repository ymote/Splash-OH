//! The home screen: one wonder, full bleed, with its name over the artwork.
//!
//! Reproduces `home_screen.dart` and `wonder_title_text.dart`. The layout is
//! the illustration at full frame, the title block low over it, eight dots for
//! the eight wonders, a menu button top-left, and a chevron inviting a swipe up
//! into the details.

use super::data::{Wonder, WONDERS};
use super::illustration::illustration_with;
use splash_oh_native::arkui::{attr, ty, Node};
use splash_oh_native::ui::*;

/// Off-white, over every illustration. Wonderous uses one title colour for all
/// eight; each artwork is dark enough behind the name to carry it.
pub const TITLE: u32 = 0xFFF8ECE5;

/// `ARKUI_TEXT_ALIGNMENT_CENTER`.
const CENTER: i32 = 1;
/// Registered by the shell at startup from the app's own font files.
const DISPLAY_FONT: &str = "YesevaOne";
const BODY_ITALIC: &str = "RalewayItalic";

/// A title line: the display face, centred in a box of its own.
fn centred(s: &str, size: f32, color: u32, w: f32, h: f32) -> Option<Node> {
    Some(
        text(s, size, color, w, h)?
            .i32_attr(attr::text_align(), CENTER)
            .string_attr(attr::font_family(), DISPLAY_FONT),
    )
}

/// Wonderous passes `enableShadows: true` to the home title, and it matters:
/// over Petra's night sky or the Great Wall's foliage the name is otherwise
/// barely legible.
fn shadowed(node: Node) -> Node {
    // radius, type(0 = colour), offsetX, offsetY — then the colour separately.
    node.f32v_attr(attr::text_shadow(), &[10.0, 0.0, 0.0, 2.0])
}

/// Tap targets. The home screen is a horizontal pager in the original; until
/// swipe gestures are available, the two halves of the artwork page it.
pub const PREV_TAP: i32 = 7101;
pub const NEXT_TAP: i32 = 7102;
pub const MENU_TAP: i32 = 7103;
pub const DETAILS_TAP: i32 = 7104;
/// Swipe base for the pager. The app's home screen is a horizontal pager, so
/// a drag across the artwork moves to the next wonder; the two tap halves stay
/// as well, and either works.
pub const HOME_SWIPE: i32 = 7430;

const APP: &str = "wonders";

/// Every wonder's illustration and title, and the background behind them,
/// kept mounted so paging is a fade rather than a rebuild.
///
/// This is how `wonders_home_screen.dart` does it too: it maps all eight
/// wonders into the background and mid-ground layers at once and toggles
/// `isShowing`, and the illustration fades itself in or out. Rebuilding for
/// each page instead cannot cross-fade, because the wonder being left has
/// already been dropped by the time the new one is mounted.
static PAGES: std::sync::Mutex<Option<Pages>> = std::sync::Mutex::new(None);

struct Pages {
    /// One (background, illustration, title) per wonder, by index.
    layers: Vec<(usize, usize, usize)>,
}

/// `$styles.times.med`, which is what the home screen's cross-fade runs at.
const FADE_MS: i32 = 300;

/// Drop the handles this module holds. See `details::forget_nodes`.
pub fn forget_nodes() {
    if let Ok(mut g) = PAGES.lock() {
        *g = None;
    }
}

/// Fade to `index`. Nothing is rebuilt; eight opacities move.
pub fn fade_to(index: usize) {
    let Ok(g) = PAGES.lock() else { return };
    let Some(pages) = g.as_ref() else { return };
    let Some(&(anchor, _, _)) = pages.layers.first() else {
        return;
    };
    let shown = index % pages.layers.len();
    let set: Vec<(usize, f32)> = pages
        .layers
        .iter()
        .enumerate()
        .flat_map(|(i, &(bg, art, title))| {
            let a = if i == shown { 1.0 } else { 0.0 };
            [(bg, a), (art, a), (title, a)]
        })
        .collect();
    unsafe {
        splash_oh_native::arkui::animate(
            anchor as splash_oh_native::arkui::NodeHandle,
            FADE_MS,
            splash_oh_native::arkui::CURVE_EASE_OUT,
            move || {
                for (n, a) in set {
                    unsafe {
                        Node::set_f32_attr_raw(
                            n as splash_oh_native::arkui::NodeHandle,
                            attr::opacity(),
                            a,
                        )
                    };
                }
            },
        )
    };
}

pub fn build(index: usize, w: f32, h: f32) -> Option<Node> {
    let shown = index % WONDERS.len();
    let wonder = &WONDERS[shown];
    let mut root = stack(w, h, wonder.bg)?;
    // The title is its own layer above the whole illustration, foreground
    // included -- `_buildFloatingUi` in wonders_home_screen.dart. Putting it
    // between mid-ground and foreground instead loses it entirely behind an
    // opaque foreground: the Pyramids dunes cover the lower half of the frame.
    //
    // All eight are mounted; only one is opaque. The flat colour behind each
    // illustration is its own layer for the same reason -- the background is
    // per wonder, so it has to fade with the artwork rather than jump.
    let mut layers = Vec::with_capacity(WONDERS.len());
    let mut arts = Vec::with_capacity(WONDERS.len());
    let mut titles = Vec::with_capacity(WONDERS.len());
    for (i, wo) in WONDERS.iter().enumerate() {
        let a = if i == shown { 1.0 } else { 0.0 };
        let bg = col(w, h, wo.bg)?
            .f32_attr(attr::opacity(), a)
            .f32v_attr(attr::position(), &[0.0, 0.0]);
        let art = illustration_with(wo, w, h, None)?.f32_attr(attr::opacity(), a);
        let title = title_block(wo, i, w, h)?.f32_attr(attr::opacity(), a);
        layers.push((bg.raw() as usize, art.raw() as usize, title.raw() as usize));
        root = root.child(bg);
        arts.push(art);
        titles.push(title);
    }
    for art in arts {
        root = root.child(art);
    }
    for title in titles {
        root = root.child(title);
    }
    if let Ok(mut g) = PAGES.lock() {
        *g = Some(Pages { layers });
    }
    root = root.child(menu_button(h)?);
    root = root.child(chevron(w, h)?);
    // Every tap target in one overlay: the menu, the two paging halves, and
    // the strip along the bottom that opens the details. One overlay because a
    // full-frame column per target means only the last one ever fires.
    root = root.child(super::hits_swipe(
        w,
        h,
        &[
            (12.0, h * 0.045, 62.0, 62.0, MENU_TAP),
            (0.0, h * 0.14, w * 0.35, h * 0.6, PREV_TAP),
            (w * 0.65, h * 0.14, w * 0.35, h * 0.6, NEXT_TAP),
            (0.0, h * 0.80, w, h * 0.17, DETAILS_TAP),
        ],
        Some(HOME_SWIPE),
    )?);
    Some(root)
}

/// The name, the article, and the dots.
///
/// Wonderous sets the name in Yeseva One with any leading article ("the") much
/// smaller and tucked to the left of the second line rather than centred above
/// it. That one detail is most of why the title reads as a masthead.
fn title_block(wonder: &Wonder, index: usize, w: f32, h: f32) -> Option<Node> {
    // Sized from the screen, not from a constant: the title sits a fixed
    // distance off the bottom in the original, and the illustration behind it
    // is whatever height the device is.
    // Wonderous scales the display face with the screen. Cap it so the widest
    // name still fits: "Colosseum" is 5.45 em, so the size can be at most
    // width / 5.45 before it would run off the edge.
    let widest = wonder.em1.max(wonder.em2).max(1.0);
    let size = (w * 0.115).clamp(30.0, (w * 0.92 / widest).min(52.0));
    let line_h = size * 1.28;
    let one_line = wonder.line2.is_empty();
    let block_h = if one_line { line_h } else { line_h * 2.0 } + 44.0;
    // Proportions read off the app's own screenshots rather than guessed:
    // the first title line lands near 0.72 of the frame and the dots near 0.88,
    // which puts the gap below the block at about 9% of the height.
    // The dots need room below the block, and on a short screen the block
    // itself is proportionally taller, so the gap is not a flat fraction.
    let y = (h - block_h - h * 0.115).max(h * 0.5);

    let mut col = col(w, block_h, 0x00000000)?.f32v_attr(attr::position(), &[0.0, y]);
    // Line one is centred in the full width; there is no article beside it.
    col = col.child(shadowed(centred(wonder.line1, size, TITLE, w, line_h)?));

    if !one_line {
        // The article rides beside the second word rather than above it, in a
        // much smaller italic. It is the detail that makes the name read as a
        // masthead instead of two stacked words.
        // The article and the word are one centred group, not a word centred
        // in the frame with the article parked at the margin. Sizing each to
        // its content and centring the row is what puts "the" against the R.
        let art_w = if wonder.article.is_empty() {
            0.0
        } else {
            size * 0.9
        };
        // The measured width of the word, plus a little slack. Too small and
        // ArkUI wraps mid-word ("Redeeme / r"); too large and the centred row
        // pushes the article away from it.
        let word_w = wonder.em2 * size + size * 0.25;
        let mut line = row(w, line_h, 0x00000000)?
            // ARKUI_FLEX_ALIGNMENT_CENTER is 2. It is 1 that means START, and
            // using it left-aligned every second line against the frame edge.
            .i32_attr(attr::row_justify(), 2);
        if art_w > 0.0 {
            line = line.child(
                text(wonder.article, size * 0.40, TITLE, art_w, line_h)?
                    .i32_attr(attr::text_align(), 2)
                    .string_attr(attr::font_family(), BODY_ITALIC)
                    // Sits on the baseline of the big word rather than centred
                    // against its box.
                    .f32v_attr(attr::padding(), &[line_h * 0.36, 0.0, 0.0, 0.0]),
            );
        }
        line = line.child(shadowed(centred(
            wonder.line2,
            size,
            TITLE,
            word_w,
            line_h,
        )?));
        col = col.child(line);
    }

    col = col.child(dots(index, w)?);
    Some(col)
}

/// Eight dots, the current one filled.
fn dots(index: usize, w: f32) -> Option<Node> {
    let n = WONDERS.len();
    // `AppPageIndicator` at dotSize 8 with an `ExpandingDotsEffect` whose
    // expansionFactor is 2: every dot is the same colour and the current one is
    // twice as wide, a pill rather than a circle. Fading the others instead
    // gets the emphasis right and the shape wrong, and the shape is the part
    // you actually notice.
    let (d, gap) = (8.0, 10.0);
    let cur = index % n;
    let total = (n as f32 + 1.0) * d + (n as f32 - 1.0) * gap;

    let mut r =
        row(w, 36.0, 0x00000000)?.f32v_attr(attr::padding(), &[14.0, 0.0, 0.0, (w - total) / 2.0]);

    for i in 0..n {
        let dw = if i == cur { d * 2.0 } else { d };
        let dot = col(dw, d, TITLE)?
            .radius(d / 2.0)
            .f32v_attr(attr::margin(), &[0.0, gap / 2.0, 0.0, gap / 2.0]);
        r = r.child(dot);
    }
    Some(r)
}

/// The hamburger, top left, on a translucent dark disc.
///
/// The glyph is the app's own `icon-menu.png` rather than three drawn bars:
/// Wonderous's menu icon has uneven bar lengths and rounded caps that a stack
/// of rectangles gets subtly wrong.
fn menu_button(h: f32) -> Option<Node> {
    let size = 46.0;
    let mut disc = stack(size, size, 0xB3272625)?
        .radius(size / 2.0)
        .f32v_attr(attr::position(), &[20.0, h * 0.055]);
    disc = disc.child(icon(APP, "_common/icons/icon-menu.png", 22.0)?);
    Some(disc)
}

/// The swipe-up affordance at the very bottom.
/// The swipe-up affordance. `arrow-indicator.png` is the app's own asset; the
/// nearest text glyph is a different shape and a different weight.
fn chevron(w: f32, h: f32) -> Option<Node> {
    let s = 26.0;
    let mut c = stack(w, s, 0x00000000)?.f32v_attr(attr::position(), &[0.0, h * 0.94]);
    c = c.child(icon(APP, "_common/arrow-indicator.png", s)?);
    Some(c)
}
