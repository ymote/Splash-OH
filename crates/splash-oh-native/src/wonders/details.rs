//! The wonder-details screens: editorial, photos, artifacts, events.
//!
//! One scaffold with four bodies, matching `wonder_details_screen.dart`: the
//! wonder's own `bgColor` behind everything, the tab bar pinned at the bottom,
//! and the selected tab's content scrolling above it.

use super::data::{Wonder, WONDERS};
use super::editorial_data::EDITORIAL;
use super::tabbar;
use crate::arkui::{attr, ty, Node};
use crate::ui::*;

/// The hero band and its title block, kept so a scroll can move them without
/// rebuilding the tree.
///
/// `editorial_screen.dart` fades the band out over the first 700 of scroll and
/// slides the title down at .3 of it. Neither is a parallax: the band is a
/// fixed-height box behind the article, not a layer moving at its own rate, and
/// `_TopIllustration`'s only translate is a constant horizontal one. An earlier
/// version of this screen used a photograph with a half-rate parallax, which
/// was this port's invention rather than the app's.
///
/// Both are per-frame, so rebuilding the screen for each event is not an
/// option; these handles get their attributes set directly.
static HERO_IMAGE: AtomicUsize = AtomicUsize::new(0);
static HERO_TITLE: AtomicUsize = AtomicUsize::new(0);
static HERO_SCROLL: AtomicUsize = AtomicUsize::new(0);
static HERO_H: std::sync::Mutex<f32> = std::sync::Mutex::new(0.0);

/// How far the article scrolls before the illustration band is gone —
/// `opacity = (1 - value / 700)` in `editorial_screen.dart`.
const BAND_FADE_OVER: f32 = 700.0;
/// And how far the title slides before it has faded out: the app slides it at
/// .3 of the scroll and fades over 150 of that slide.
const TITLE_FADE_OVER: f32 = 150.0;

/// Apply the current scroll offset to the hero. Called from the scroll event.
///
/// Every handle here is cleared by `forget_nodes` before any screen is built,
/// and the dispatcher only routes a scroll tick while a details screen is up,
/// so a late event cannot write through a handle whose node has been dropped.
pub fn apply_scroll() {
    let band = HERO_H.lock().map(|h| *h).unwrap_or(0.0);
    let scroll = HERO_SCROLL.load(Ordering::Relaxed);
    if band <= 0.0 || scroll == 0 {
        return;
    }
    // Ask the Scroll where it is. Accumulating the deltas the event carries
    // drifts: the sum ran several times ahead of the real offset within one
    // drag, which pinned the hero at the end of its travel immediately.
    let Some(y) =
        (unsafe { Node::get_f32(scroll as crate::arkui::NodeHandle, attr::scroll_offset(), 1) })
    else {
        return;
    };
    let img = HERO_IMAGE.load(Ordering::Relaxed);
    let title = HERO_TITLE.load(Ordering::Relaxed);

    if img != 0 {
        // `editorial_screen.dart`: the band's opacity is 1 - scroll / 700.
        unsafe {
            Node::set_f32_attr_raw(
                img as crate::arkui::NodeHandle,
                attr::opacity(),
                (1.0 - y / BAND_FADE_OVER).clamp(0.0, 1.0),
            );
        }
    }
    if title != 0 {
        // And the title slides down at .3 of the scroll while fading over 150
        // of that slide, so it settles behind the bar rather than under it.
        let slide = (y * 0.3).max(0.0);
        unsafe {
            Node::set_f32v_raw(
                title as crate::arkui::NodeHandle,
                attr::translate(),
                &[0.0, slide, 0.0],
            );
            Node::set_f32_attr_raw(
                title as crate::arkui::NodeHandle,
                attr::opacity(),
                (1.0 - slide / TITLE_FADE_OVER).clamp(0.0, 1.0),
            );
        }
    }
}

/// A scroll sized to the caller's width.
///
/// `ui::scroll` is hardwired to the benchmark page width, which on the Pura X
/// leaves a strip of background down both edges of a full-bleed sheet.
fn scroll_w(w: f32, h: f32) -> Option<Node> {
    Some(
        Node::new(ty::scroll())?
            .width(w)
            .height(h)
            // ARKUI_ALIGNMENT_TOP: a Scroll centres content shorter than itself.
            .i32_attr(attr::alignment(), 1),
    )
}

const APP: &str = "wonders";

/// `$styles.colors.offWhite` — the editorial sheet.
const SHEET: u32 = 0xFFF8ECE5;
const BODY: u32 = 0xFF514F4D;
const CAPTION: u32 = 0xFF7D7873;
const ACCENT: u32 = 0xFFC47642;
const GREY_STRONG: u32 = 0xFF272625;
/// `$styles.colors.black` — used for text that sits on the pale ground.
const INK: u32 = 0xFF1E1B18;
/// `$styles.colors.accent1` — what `AppHeader` tints its title.
const ACCENT1: u32 = 0xFFE4935D;

const DISPLAY: &str = "YesevaOne";
const SERIF_UI: &str = "TenorSans";
const BODY_FONT: &str = "Raleway";
const BODY_ITALIC: &str = "RalewayItalic";

/// How tall a paragraph will be once wrapped.
///
/// ArkUI will not report a laid-out text height and a Column needs one up
/// front, so it has to be predicted. Raleway's average advance is 0.4624 em —
/// measured from the font, not guessed — which puts 2.163 characters on each
/// point of width. Guessing 2.05 left a visible gap under every paragraph.
///
/// Wrapping is per word, so a line breaks early on average; the 0.94 accounts
/// for that. The result is close enough that paragraph spacing stays even, and
/// erring long only ever adds white space rather than clipping the text.
fn text_height(s: &str, size: f32, w: f32) -> f32 {
    let per_line = (w * 2.163 * 0.94 / size).max(1.0);
    let lines = (s.chars().count() as f32 / per_line).ceil().max(1.0);
    lines * size * 1.5 + size * 0.35
}

fn para(s: &str, size: f32, colour: u32, w: f32) -> Option<Node> {
    Some(
        text(s, size, colour, w, text_height(s, size, w))?
            .string_attr(attr::font_family(), BODY_FONT),
    )
}

/// A column that is told how tall it is.
///
/// Native ArkUI nodes do not auto-size — a Column left at height 0 renders
/// nothing and a Scroll over it has nothing to scroll. Every container here
/// therefore carries a height its caller has added up.
struct Stack1 {
    node: Node,
    h: f32,
}

impl Stack1 {
    fn new(node: Node) -> Self {
        Stack1 { node, h: 0.0 }
    }
    fn push(mut self, child: Option<Node>, h: f32) -> Self {
        if let Some(c) = child {
            self.node = self.node.child(c);
            self.h += h;
        }
        self
    }
    fn done(self, w: f32) -> (Node, f32) {
        (self.node.width(w).height(self.h), self.h)
    }
}

use std::sync::atomic::{AtomicUsize, Ordering};
static ARTIFACT_SEL: AtomicUsize = AtomicUsize::new(0);

pub fn artifact_sel() -> usize {
    ARTIFACT_SEL.load(Ordering::Relaxed)
}
pub fn set_artifact_sel(i: usize) {
    ARTIFACT_SEL.store(i, Ordering::Relaxed);
}

/// Drop every handle this module holds.
///
/// Called before *any* screen is built, not just a details screen. Clearing
/// them only when another details screen was built left them pointing into a
/// tree that navigating away had already dropped, and a scroll or swipe event
/// still in flight would then write through them.
pub fn forget_nodes() {
    HERO_IMAGE.store(0, Ordering::Relaxed);
    HERO_TITLE.store(0, Ordering::Relaxed);
    HERO_SCROLL.store(0, Ordering::Relaxed);
    if let Ok(mut g) = HERO_H.lock() {
        *g = 0.0;
    }
    if let Ok(mut g) = WALL.lock() {
        *g = None;
    }
    if let Ok(mut g) = CAROUSEL.lock() {
        *g = None;
    }
}

/// Put the gallery and the carousel back to where a fresh route starts them:
/// the middle cell, and the first piece.
pub fn reset_selection() {
    PHOTO_SEL.store(13, Ordering::Relaxed);
    ARTIFACT_SEL.store(0, Ordering::Relaxed);
}

pub fn build(index: usize, tab: usize, w: f32, h: f32) -> Option<Node> {
    let wonder = &WONDERS[index % WONDERS.len()];
    let bar_h = tabbar::height();
    let mut root = stack(w, h, wonder.bg)?;

    let body = match tab {
        1 => photos(wonder, w, h - bar_h),
        2 => artifacts(wonder, index, artifact_sel(), w, h - bar_h),
        3 => super::timeline::events(index, w, h - bar_h),
        _ => editorial(wonder, index, w, h - bar_h),
    };
    if let Some(b) = body {
        // Pinned to the top. A Stack centres its children, so a body one bar
        // shorter than the screen sat 36 vp down and its lower edge overlapped
        // the tab bar -- where the Scroll then swallowed every tap meant for a
        // tab. Positioning it removes the overlap rather than fighting it.
        root = root.child(b.f32v_attr(attr::position(), &[0.0, 0.0]));
    }
    root =
        root.child(tabbar::build(wonder, tab, w)?.f32v_attr(attr::position(), &[0.0, h - bar_h]));
    // One overlay for the whole screen: the bar's cells and, on the artifacts
    // tab, the carousel's paging halves. Two overlays means the later one wins
    // and the earlier is dead — which is how the bar ate every carousel tap.
    let d = 52.0;
    let cell = (w - d - 20.0) / tabbar::TABS.len() as f32;
    let mut targets: Vec<(f32, f32, f32, f32, i32)> =
        vec![(0.0, h - bar_h, d + 20.0, bar_h, tabbar::HOME_TAP)];
    // One band, cells edge to edge.
    //
    // The middle cell used to be dead. It was not layout: the system's
    // navigation gesture bar takes touches in a strip across the bottom centre
    // before the app sees them, and that cell sat under it. The page now stops
    // above the gesture inset — see `page()` — so the whole bar is reachable.
    for i in 0..tabbar::TABS.len() {
        targets.push((
            d + 20.0 + cell * i as f32,
            h - bar_h,
            cell,
            bar_h,
            tabbar::TAB_BASE + i as i32,
        ));
    }
    if tab == 1 {
        // The gallery's four peeking edges. The middle band holds the left and
        // right strips, which share a y; the strips above and below are their
        // own bands, and the lower one stops short of the tab bar's so the two
        // are never grouped together.
        let body_h = h - bar_h;
        let (iw, ih) = (w * 0.66, body_h * 0.5);
        let (hx, hy) = ((w - iw) / 2.0, (body_h - ih) / 2.0);
        targets.push((0.0, 0.0, w, hy, PHOTO_UP));
        targets.push((0.0, hy, hx, ih, PHOTO_LEFT));
        targets.push((hx + iw, hy, w - hx - iw, ih, PHOTO_RIGHT));
        targets.push((0.0, hy + ih, w, hy - 8.0, PHOTO_DOWN));
    }
    if tab == 2 {
        // BROWSE ALL ARTIFACTS opens the search screen, as it does in the app.
        //
        // Its band has to end above the tab bar's. Overlapping, the two merged
        // into one row, and `hits` dropped whichever target started to the left
        // of the cursor -- which was this one, silently.
        let browse_h = 52.0;
        let browse_y = h - bar_h - browse_h - 14.0;
        targets.push((w * 0.14, browse_y, w * 0.72, browse_h, BROWSE_TAP));
        let (ay, ah) = artifact_arch(w, h - bar_h);
        // Three across the arch: the selected piece in the middle opens, and
        // the space either side of it pages, which is what tapping a neighbour
        // does in the app.
        let body_h = h - bar_h;
        let iw = (body_h - 200.0 - body_h / 2.75).clamp(250.0, 400.0) * 0.666;
        let cx = (w - iw) / 2.0;
        targets.push((0.0, ay, cx, ah, ARTIFACT_PREV));
        targets.push((cx, ay, iw, ah, ARTIFACT_OPEN));
        targets.push((cx + iw, ay, w - cx - iw, ah, ARTIFACT_NEXT));
    }
    // The wall and the carousel both swipe in the app; which base is live
    // depends on which tab is up, and only one of them is ever on screen.
    //
    // The wall is the exception: its tap targets are four thin edge strips and
    // the whole middle of the screen is bare, so the drag is measured on the
    // wall itself instead. Registering it in both places double-counted --
    // ArkUI walks a touch up the tree, so a swipe that started on a strip was
    // reported by the strip and again by the wall, and the wall moved two.
    let swipe = match tab {
        2 => Some(ARTIFACT_SWIPE),
        _ => None,
    };
    root = root.child(super::hits_swipe(w, h, &targets, swipe)?);

    Some(root)
}

// ---------------------------------------------------------------------------
// Editorial
// ---------------------------------------------------------------------------

/// The long read. Section order is `_scrolling_content.dart`:
/// history1 → callout1 → history2 → CONSTRUCTION → construction1 → callout2 →
/// construction2 → LOCATION → location1 → quote → location2.
fn editorial(wonder: &Wonder, index: usize, w: f32, h: f32) -> Option<Node> {
    let e = &EDITORIAL[index % EDITORIAL.len()];
    let pad = 24.0;
    let cw = w - pad * 2.0;

    let hero_h = super::short::HERO_H;
    let sec_h = w * 0.62 * 0.16 + 70.0 + 62.0;

    let mut b = Stack1::new(col(cw, 0.0, 0x00000000)?);
    b = b.push(
        para(e.history1, 15.0, BODY, cw),
        text_height(e.history1, 15.0, cw),
    );
    if !e.callout1.is_empty() {
        b = b.push(
            callout(e.callout1, cw),
            text_height(e.callout1, 19.0, cw - 22.0) + 56.0,
        );
    }
    b = b.push(
        para(e.history2, 15.0, BODY, cw),
        text_height(e.history2, 15.0, cw),
    );
    b = b.push(
        section(cw, "arc-construction.png", "construction.png"),
        sec_h,
    );
    b = b.push(
        para(e.construction1, 15.0, BODY, cw),
        text_height(e.construction1, 15.0, cw),
    );
    if !e.video_caption.is_empty() {
        b = b.push(video(wonder, e, cw), cw * 0.56 + 102.0);
    }
    if !e.callout2.is_empty() {
        b = b.push(
            callout(e.callout2, cw),
            text_height(e.callout2, 19.0, cw - 22.0) + 56.0,
        );
    }
    b = b.push(
        para(e.construction2, 15.0, BODY, cw),
        text_height(e.construction2, 15.0, cw),
    );
    b = b.push(section(cw, "arc-location.png", "geography.png"), sec_h);
    b = b.push(
        para(e.location1, 15.0, BODY, cw),
        text_height(e.location1, 15.0, cw),
    );
    b = b.push(
        para(e.location2, 15.0, BODY, cw),
        text_height(e.location2, 15.0, cw),
    );
    let (body, body_h) = b.done(cw);

    let mut sheet = Stack1::new(col(w, 0.0, SHEET)?);
    sheet = sheet.push(hero(wonder, index, e, w, h), hero_h);
    let title = title_text(wonder, e, w);
    if let Some(t) = title {
        HERO_TITLE.store(t.raw() as usize, Ordering::Relaxed);
        sheet = sheet.push(Some(t), 150.0);
    }
    sheet = sheet.push(
        Some(
            col(w, body_h + 68.0, 0x00000000)?
                .f32v_attr(attr::padding(), &[28.0, pad, 40.0, pad])
                .child(body),
        ),
        body_h + 68.0,
    );
    let (sheet, _) = sheet.done(w);

    let mut s = scroll_w(w, h)?;
    s = s.child(sheet);
    // The scroll drives the hero. The event is only a tick — the handler reads
    // the offset back off this node.
    s = s.on_event(crate::arkui::event::did_scroll(), SCROLL_TICK);
    HERO_SCROLL.store(s.raw() as usize, Ordering::Relaxed);
    Some(s)
}

/// The id the editorial's Scroll reports under.
pub const SCROLL_TICK: i32 = 7399;

/// The band across the top of an article, and the title under it.
///
/// Not a photograph: `_TopIllustration` draws the wonder's own illustration in
/// `shortMode` — background and mid-ground, no foreground — and the title
/// follows underneath as part of the scrolling content rather than sitting over
/// the picture. Both move as the article scrolls: the band fades out over the
/// first 700 of scroll, the title slides down at .3 and fades over 150.
fn hero(
    wonder: &Wonder,
    index: usize,
    e: &super::editorial_data::Editorial,
    w: f32,
    screen_h: f32,
) -> Option<Node> {
    let band = super::short::HERO_H;
    let mut st = stack(w, band, wonder.fg)?.i32_attr(attr::clip(), 1);

    let art = super::short::hero(index, w, screen_h)?;
    HERO_IMAGE.store(art.raw() as usize, Ordering::Relaxed);
    if let Ok(mut g) = HERO_H.lock() {
        *g = band;
    }
    st = st.child(art);
    Some(st)
}

/// `_TitleText`: a rule, the sub-title in caps, a rule, then the name and the
/// region. On the sheet, under the band.
fn title_text(wonder: &Wonder, e: &super::editorial_data::Editorial, w: f32) -> Option<Node> {
    let size = (w * 0.10).clamp(26.0, 40.0);
    let mut cap = col(w, 150.0, 0x00000000)?;
    // The sub-title between two rules, which is the app's masthead rule.
    let label_w = (e.sub_title.chars().count() as f32 * 7.0 + 24.0).min(w - 96.0);
    let rule_w = ((w - 48.0 - label_w) / 2.0).max(8.0);
    let mut row_ = row(w, 30.0, 0x00000000)?.f32v_attr(attr::padding(), &[10.0, 24.0, 0.0, 24.0]);
    row_ = row_.child(col(rule_w, 1.0, wonder.fg)?);
    row_ = row_.child(
        text(
            &e.sub_title.to_uppercase(),
            11.0,
            GREY_STRONG,
            label_w,
            20.0,
        )?
        .string_attr(attr::font_family(), SERIF_UI)
        .i32_attr(attr::text_align(), 1),
    );
    row_ = row_.child(col(rule_w, 1.0, wonder.fg)?);
    cap = cap.child(row_);
    cap = cap.child(
        text(wonder.title, size, GREY_STRONG, w - 48.0, size * 1.5)?
            .string_attr(attr::font_family(), DISPLAY)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::padding(), &[6.0, 24.0, 0.0, 24.0]),
    );
    cap = cap.child(
        text(e.region, 12.0, CAPTION, w - 48.0, 22.0)?
            .string_attr(attr::font_family(), BODY_FONT)
            .i32_attr(attr::text_align(), 1),
    );
    Some(cap)
}

fn section(w: f32, arc: &str, glyph: &str) -> Option<Node> {
    let mut c = col(w, 150.0, 0x00000000)?.f32v_attr(attr::margin(), &[44.0, 0.0, 18.0, 0.0]);
    c = c.child(
        Node::new(ty::image())?
            .width(w * 0.62)
            .height(w * 0.62 * 0.16)
            .string_attr(
                attr::image_src(),
                &format!("resource://RAWFILE/{APP}/_common/{arc}"),
            )
            .i32_attr(attr::image_fit(), 0)
            .f32v_attr(attr::position(), &[w * 0.19, 0.0]),
    );
    c = c.child(icon(APP, &format!("_common/{glyph}"), 46.0)?.f32v_attr(
        attr::position(),
        &[(w - 46.0) / 2.0, w * 0.62 * 0.16 + 12.0],
    ));
    Some(c)
}

/// A pulled-out sentence, set larger with a rule down its left edge.
fn callout(s: &str, w: f32) -> Option<Node> {
    let inner = w - 22.0;
    let th = text_height(s, 19.0, inner);
    let mut r = row(w, th + 12.0, 0x00000000)?.f32v_attr(attr::margin(), &[22.0, 0.0, 22.0, 0.0]);
    r = r.child(col(3.0, th, ACCENT)?);
    r = r.child(
        text(s, 19.0, GREY_STRONG, inner - 16.0, th)?
            .string_attr(attr::font_family(), BODY_ITALIC)
            .f32v_attr(attr::padding(), &[0.0, 0.0, 0.0, 16.0]),
    );
    Some(r)
}

/// The video still and its caption. The app embeds YouTube; there is no player
/// here, so this is the thumbnail and the credit the app prints under it.
fn video(wonder: &Wonder, e: &super::editorial_data::Editorial, w: f32) -> Option<Node> {
    let vh = w * 0.56;
    let mut c = col(w, vh + 70.0, 0x00000000)?.f32v_attr(attr::margin(), &[20.0, 0.0, 12.0, 0.0]);
    let mut frame = stack(w, vh, 0xFF000000)?;
    frame = frame.child(photo(
        APP,
        &format!("{}/photo-2.jpg", wonder.dir),
        w,
        vh,
        0.0,
    )?);
    // The play button: a translucent disc with a triangle, as the app draws it.
    frame = frame.child(
        stack(54.0, 54.0, 0x66000000)?
            .radius(27.0)
            .f32v_attr(attr::position(), &[(w - 54.0) / 2.0, (vh - 54.0) / 2.0])
            .child(text("\u{25B6}", 20.0, SHEET, 54.0, 54.0)?.i32_attr(attr::text_align(), 1)),
    );
    c = c.child(frame);
    c = c.child(
        text(e.video_caption, 13.0, CAPTION, w, 60.0)?
            .string_attr(attr::font_family(), BODY_ITALIC)
            .f32v_attr(attr::padding(), &[10.0, 0.0, 0.0, 0.0]),
    );
    Some(c)
}

// ---------------------------------------------------------------------------
// Photos
// ---------------------------------------------------------------------------

/// The gallery. The app pulls an Unsplash collection; the four photographs that
/// ship with each wonder stand in, tiled the way the grid lays them out.
/// The gallery is 5×5 and does not scroll — `photo_gallery.dart`.
pub const GRID: usize = 5;
/// The 24 photographs each wonder ships, repeated to fill the 25th cell,
/// exactly as `_initPhotoIds` does.
const GALLERY_PHOTOS: usize = 24;
/// `_index` starts in the middle: `round(25 / 2)` is 13.
static PHOTO_SEL: AtomicUsize = AtomicUsize::new(13);
/// The 25 cells and the step between them, kept so a pan can move them where
/// they are instead of rebuilding the screen.
///
/// The scrim never moves: the selected cell is always the middle of the frame,
/// whichever cell it is, so the hole it leaves is in the same place every time.
/// Only the wall slides under it.
static WALL: std::sync::Mutex<Option<Wall>> = std::sync::Mutex::new(None);

struct Wall {
    cells: Vec<usize>,
    origin: (f32, f32),
    step: (f32, f32),
}

pub fn photo_sel() -> usize {
    PHOTO_SEL.load(Ordering::Relaxed)
}

/// Move the selection one cell, refusing the moves the app refuses: off the
/// grid, and wrapping round a row edge.
///
/// Returns false either way: the wall is slid to its new place here rather
/// than rebuilt, which is what lets the move be animated.
pub fn move_photo_sel(dx: i32, dy: i32) -> bool {
    let cur = photo_sel() as i32;
    let next = cur + dx + dy * GRID as i32;
    if !(0..(GRID * GRID) as i32).contains(&next) {
        return false;
    }
    if dx != 0 && (cur / GRID as i32) != (next / GRID as i32) {
        return false;
    }
    PHOTO_SEL.store(next as usize, Ordering::Relaxed);
    slide_wall(next as usize);
    false
}

/// Put every cell where the new selection wants it, tweened.
///
/// `$styles.times.med * .4` is the app's swipe duration and `Curves.easeOut`
/// its curve.
fn slide_wall(sel: usize) {
    let Ok(g) = WALL.lock() else { return };
    let Some(wall) = g.as_ref() else { return };
    let anchor = match wall.cells.first() {
        Some(&c) if c != 0 => c,
        _ => return,
    };
    let (ox, oy) = wall.origin;
    let ((sx, sy), (col, row)) = (wall.step, (sel % GRID, sel / GRID));
    let places: Vec<(usize, f32, f32)> = wall
        .cells
        .iter()
        .enumerate()
        .filter(|(_, &c)| c != 0)
        .map(|(i, &c)| {
            (
                c,
                ox + (i % GRID) as f32 * sx - col as f32 * sx,
                oy + (i / GRID) as f32 * sy - row as f32 * sy,
            )
        })
        .collect();
    unsafe {
        crate::arkui::animate(
            anchor as crate::arkui::NodeHandle,
            120,
            crate::arkui::CURVE_EASE_OUT,
            move || {
                for (c, x, y) in places {
                    unsafe {
                        Node::set_f32v_raw(c as crate::arkui::NodeHandle, attr::position(), &[x, y])
                    };
                }
            },
        )
    };
}

/// The photo gallery: a 5×5 wall of the wonder's own Unsplash collection,
/// panned one cell at a time, with everything but the centre cell dimmed.
///
/// `photo_gallery.dart` sizes each cell at 66% of the screen width and half its
/// height and translates the whole grid so the selected cell lands in the
/// middle, so four neighbours always peek in from the edges. It swipes; this
/// taps those peeking edges, which is the same move.
fn photos(wonder: &Wonder, w: f32, h: f32) -> Option<Node> {
    let sel = photo_sel().min(GRID * GRID - 1);
    let (iw, ih) = (w * 0.66, h * 0.5);
    let pad = 24.0; // $styles.insets.md
    let (sx, sy) = (iw + pad, ih + pad);

    // Cell (0,0)'s place when cell (0,0) is the selected one. Every other
    // arrangement is this minus the selected column and row, which is what
    // `slide_wall` applies -- and what makes the selected cell the middle one.
    let ox = (w - iw) / 2.0;
    let oy = (h - ih) / 2.0;

    let mut root = stack(w, h, wonder.bg)?
        .i32_attr(attr::clip(), 1)
        .on_event(crate::arkui::event::touch(), PHOTO_SWIPE);
    // All 25 are built, not just the nine that can be seen: a cell has to
    // exist to be slid into view.
    let mut cells = Vec::with_capacity(GRID * GRID);
    for i in 0..GRID * GRID {
        let (x, y) = (
            ox + (i % GRID) as f32 * sx - (sel % GRID) as f32 * sx,
            oy + (i / GRID) as f32 * sy - (sel / GRID) as f32 * sy,
        );
        let cell = photo(
            APP,
            &format!("{}/gallery/{:02}.jpg", wonder.dir, i % GALLERY_PHOTOS),
            iw,
            ih,
            0.0,
        )?
        .f32v_attr(attr::position(), &[x, y]);
        cells.push(cell.raw() as usize);
        root = root.child(cell);
    }
    if let Ok(mut g) = WALL.lock() {
        *g = Some(Wall {
            cells,
            origin: (ox, oy),
            step: (sx, sy),
        });
    }

    // `_AnimatedCutoutOverlay`: a 70% scrim over everything except the selected
    // cell. Four rectangles around the hole — ArkUI has no cut-out shape, and
    // four solid edges are exactly what the cut-out leaves behind.
    let (hx, hy) = (ox, oy);
    let scrim = 0xB3000000;
    for (x, y, rw, rh) in [
        (0.0, 0.0, w, hy),
        (0.0, hy + ih, w, (h - hy - ih).max(0.0)),
        (0.0, hy, hx.max(0.0), ih),
        (hx + iw, hy, (w - hx - iw).max(0.0), ih),
    ] {
        if rw > 0.0 && rh > 0.0 {
            root = root.child(col(rw, rh, scrim)?.f32v_attr(attr::position(), &[x, y]));
        }
    }
    Some(root)
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

/// One artifact, centred on an arch, with its name and date — the shape of
/// `artifact_carousel_screen.dart`.
/// Where the arch sits, so `build` can put the carousel's paging halves over it
/// in the screen's single overlay. Two overlays means the later one wins and
/// the earlier one is dead — which is how the tab bar silently ate every tap
/// meant for the carousel.
/// The gallery's four peeking edges.
///
/// 7400 and up: 7340..7342 are the collection's and the menu's, and sharing
/// them meant a tap on the collection's close button was read as a pan of a
/// photo wall that was not even on screen. `ids_are_unique` now says so.
pub const PHOTO_UP: i32 = 7400;
pub const PHOTO_DOWN: i32 = 7401;
pub const PHOTO_LEFT: i32 = 7402;
pub const PHOTO_RIGHT: i32 = 7403;

/// Swipe bases. A swipe on one of these is reported as base + 1..4, in the
/// order left, right, up, down.
pub const PHOTO_SWIPE: i32 = 7410;
pub const ARTIFACT_SWIPE: i32 = 7420;

/// The carousel's two halves, and the button under it.
pub const ARTIFACT_PREV: i32 = 7370;
pub const ARTIFACT_NEXT: i32 = 7371;
pub const BROWSE_TAP: i32 = 7372;
/// Tapping the selected piece opens its details; tapping past it pages.
/// `_handleArtifactTap` makes the same distinction.
pub const ARTIFACT_OPEN: i32 = 7373;

/// Where the centre artifact sits, so the tap targets and the arch agree.
///
/// `artifact_carousel_screen.dart`: the bottom text block takes `h / 2.75`,
/// the item is what is left after that and a 200 clamp, and it is 0.666 as
/// wide as it is tall. The centre item is a portrait capsule of `width × 1.5`,
/// centred and then lifted by a quarter of its own height.
fn artifact_arch(w: f32, h: f32) -> (f32, f32) {
    let bottom_h = h / 2.75;
    let item_h = (h - 200.0 - bottom_h).clamp(250.0, 400.0);
    let iw = item_h * 0.666;
    let tall = iw * 1.5;
    let _ = w;
    ((h - tall) / 2.0 - tall * 0.25, tall)
}

/// Every carousel item's nodes, so paging is a collapse rather than a rebuild.
///
/// `_CollapsingCarouselItem` gives each item a geometry that depends only on
/// how far it is from the selected one, so the whole move is a set of sizes and
/// positions -- exactly what `animateTo` tweens. The pieces either side of the
/// selection are square, the selected one is a portrait capsule, and anything
/// three away is invisible.
static CAROUSEL: std::sync::Mutex<Option<Carousel>> = std::sync::Mutex::new(None);

struct Carousel {
    /// (blurred background, frame, photo, title block, dot) per artifact.
    items: Vec<(usize, usize, usize, usize, usize)>,
    /// Frame width, the tall height, and where a centred item sits.
    iw: f32,
    tall: f32,
    centre: (f32, f32),
    /// The dot's unexpanded width, so the tween knows what twice as wide means.
    dot_d: f32,
}

/// `$styles.times.fast`, which is what the carousel animates at.
const CAROUSEL_MS: i32 = 200;

/// One item's place, given how many steps it is from the selected one.
///
/// The vertical offsets are the app's: half a width for the neighbour, .825
/// for the one beyond it, a full width past that.
fn carousel_place(k: i32, iw: f32, tall: f32, centre: (f32, f32)) -> (f32, f32, f32, f32, f32) {
    let drop = match k.abs() {
        0 => 0.0,
        1 => iw * 0.5,
        2 => iw * 0.825,
        _ => iw,
    };
    let ih = if k == 0 { tall } else { iw };
    let x = centre.0 + k as f32 * iw;
    // `Center` inside the page slot, then translated up by a quarter of the
    // tall height and back down by the drop.
    let y = centre.1 + (tall - ih) / 2.0 + drop;
    let opacity = if k.abs() <= 2 { 1.0 } else { 0.0 };
    (x, y, iw, ih, opacity)
}

/// Collapse the carousel around `sel`.
pub fn collapse_carousel(sel: usize) {
    let Ok(g) = CAROUSEL.lock() else { return };
    let Some(c) = g.as_ref() else { return };
    let n = c.items.len();
    if n == 0 {
        return;
    }
    let Some(&(anchor, _, _, _, _)) = c.items.first() else {
        return;
    };
    // Steps, not raw index difference: the carousel wraps, so the item before
    // the first is the last one and it should come in from the left.
    let step = |i: usize| -> i32 {
        let d = (i as i32 - sel as i32).rem_euclid(n as i32);
        if d * 2 > n as i32 {
            d - n as i32
        } else {
            d
        }
    };
    let moves: Vec<(usize, usize, usize, usize, usize, i32)> = c
        .items
        .iter()
        .enumerate()
        .map(|(i, &(bg, frame, pic, title, dot))| (bg, frame, pic, title, dot, step(i)))
        .collect();
    let (iw, tall, centre) = (c.iw, c.tall, c.centre);
    let dot_d = c.dot_d;
    unsafe {
        crate::arkui::animate(
            anchor as crate::arkui::NodeHandle,
            CAROUSEL_MS,
            crate::arkui::CURVE_EASE_OUT,
            move || {
                for (bg, frame, pic, title, dot, k) in moves {
                    let (x, y, fw, fh, op) = carousel_place(k, iw, tall, centre);
                    let inset = if k == 0 { 8.0 } else { iw * 0.1 };
                    unsafe {
                        set_box(frame, x, y, fw, fh, op);
                        set_box(
                            pic,
                            x + inset,
                            y + inset,
                            fw - inset * 2.0,
                            fh - inset * 2.0,
                            op,
                        );
                        // Only the selected piece's backdrop, name and dot are
                        // lit; the rest go dark under it.
                        let on = if k == 0 { 1.0 } else { 0.0 };
                        Node::set_f32_attr_raw(bg as crate::arkui::NodeHandle, attr::opacity(), on);
                        Node::set_f32_attr_raw(
                            title as crate::arkui::NodeHandle,
                            attr::opacity(),
                            on,
                        );
                        Node::set_f32_attr_raw(
                            dot as crate::arkui::NodeHandle,
                            attr::width(),
                            if k == 0 { dot_d * 2.0 } else { dot_d },
                        );
                    }
                }
            },
        )
    };
}

/// Move a mounted node's box: position, size, radius and opacity together.
///
/// # Safety
/// `raw` must be a live node handle.
unsafe fn set_box(raw: usize, x: f32, y: f32, w: f32, h: f32, opacity: f32) {
    let n = raw as crate::arkui::NodeHandle;
    unsafe {
        Node::set_f32_attr_raw(n, attr::width(), w);
        Node::set_f32_attr_raw(n, attr::height(), h);
        // radius 999 in the app: always a capsule, whatever the box.
        Node::set_f32v_raw(
            n,
            attr::border_radius(),
            &[w / 2.0, w / 2.0, w / 2.0, w / 2.0],
        );
        Node::set_f32v_raw(n, attr::position(), &[x, y]);
        Node::set_f32_attr_raw(n, attr::opacity(), opacity);
    }
}

fn artifacts(wonder: &Wonder, index: usize, sel: usize, w: f32, h: f32) -> Option<Node> {
    let list = super::artifact_data::ARTIFACTS[index % 8];
    if list.is_empty() {
        return stack(w, h, wonder.bg);
    }
    let n = list.len();
    let sel = sel % n;
    let mut st = stack(w, h, wonder.bg)?.i32_attr(attr::clip(), 1);

    let bottom_h = h / 2.75;
    let item_h = (h - 200.0 - bottom_h).clamp(250.0, 400.0);
    let iw = item_h * 0.666;
    let (ay, tall) = artifact_arch(w, h);
    let centre = ((w - iw) / 2.0, ay);

    // `_BlurredImageBg`: the current piece, scaled up, blurred, under a black
    // wash. The screen takes its colour from the object on it rather than from
    // the wonder -- so every piece's backdrop is mounted and they cross-fade.
    let mut bgs = Vec::with_capacity(n);
    for (i, a) in list.iter().enumerate() {
        let bg = photo(
            APP,
            &format!("artifacts/{}.jpg", a.id),
            w * 1.25,
            h * 1.25,
            0.0,
        )?
        .f32_attr(attr::blur(), 6.0)
        .f32_attr(attr::opacity(), if i == sel { 1.0 } else { 0.0 })
        .f32v_attr(attr::position(), &[-w * 0.125, -h * 0.05]);
        bgs.push(bg.raw() as usize);
        st = st.child(bg);
    }
    st = st.child(col(w, h, 0x99000000)?.f32v_attr(attr::position(), &[0.0, 0.0]));

    // `_buildBgCircle`: a 2000-wide disc pushed down by half its size, so what
    // shows is a very shallow arc across the middle of the screen. Rounding
    // only the top corners of a tall box is the same shape.
    let dome = 2000.0f32;
    st = st.child(
        col(dome, dome, 0xCCF8ECE5)?
            .f32v_attr(attr::border_radius(), &[dome / 2.0, dome / 2.0, 0.0, 0.0])
            .f32v_attr(attr::position(), &[(w - dome) / 2.0, h * 0.5]),
    );

    // Every piece, placed by its distance from the selection. Furthest first,
    // so the selected one ends up on top of its neighbours.
    let step = |i: usize| -> i32 {
        let d = (i as i32 - sel as i32).rem_euclid(n as i32);
        if d * 2 > n as i32 {
            d - n as i32
        } else {
            d
        }
    };
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| -step(i).abs());
    let mut frames = vec![0usize; n];
    let mut pics = vec![0usize; n];
    for &i in &order {
        let k = step(i);
        let (x, y, fw, fh, op) = carousel_place(k, iw, tall, centre);
        let inset = if k == 0 { 8.0 } else { iw * 0.1 };
        // `_DoubleBorderImage`: a capsule outlined in off-white with the piece
        // inset inside it, clipped to the same capsule.
        let frame = col(fw, fh, 0x00000000)?
            .f32v_attr(
                attr::border_radius(),
                &[fw / 2.0, fw / 2.0, fw / 2.0, fw / 2.0],
            )
            .f32_attr(attr::border_width(), 1.0)
            .u32_attr(attr::border_color(), SHEET)
            .f32_attr(attr::opacity(), op)
            .f32v_attr(attr::position(), &[x, y]);
        let pic = photo(
            APP,
            &format!("artifacts/{}.jpg", list[i].id),
            fw - inset * 2.0,
            fh - inset * 2.0,
            (fw - inset * 2.0) / 2.0,
        )?
        .f32_attr(attr::opacity(), op)
        .f32v_attr(attr::position(), &[x + inset, y + inset]);
        frames[i] = frame.raw() as usize;
        pics[i] = pic.raw() as usize;
        st = st.child(frame);
        st = st.child(pic);
    }

    // The header: the title, and the search button that opens the search
    // screen, which the app puts in the top right corner.
    st = st.child(
        text("ARTIFACTS", 13.0, ACCENT1, w, 30.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, 22.0]),
    );
    st = st.child(
        stack(44.0, 44.0, 0x33F8ECE5)?
            .radius(22.0)
            .f32v_attr(attr::position(), &[w - 60.0, 16.0])
            .child(icon(APP, "_common/icons/icon-search.png", 18.0)?),
    );

    // Below the arc the ground is pale, so the name and date are dark here --
    // `$styles.colors.black` on `offWhite`, not the off-white used above it.
    // One block per piece, cross-faded with the pieces themselves.
    let text_top = h - bottom_h + 24.0;
    let mut titles = Vec::with_capacity(n);
    for (i, a) in list.iter().enumerate() {
        let title_lines = if a.title.chars().count() > 22 {
            2.0
        } else {
            1.0
        };
        let title_h = 36.0 * title_lines;
        let mut cap = col(w, title_h + 40.0, 0x00000000)?
            .f32_attr(attr::opacity(), if i == sel { 1.0 } else { 0.0 })
            .f32v_attr(attr::position(), &[0.0, text_top]);
        cap = cap.child(
            text(a.title, 30.0, INK, w - 48.0, title_h)?
                .string_attr(attr::font_family(), DISPLAY)
                .i32_attr(attr::text_align(), 1)
                .f32v_attr(attr::padding(), &[0.0, 24.0, 0.0, 24.0]),
        );
        cap = cap.child(
            text(a.date, 14.0, 0xB31E1B18, w, 24.0)?
                .string_attr(attr::font_family(), BODY_FONT)
                .i32_attr(attr::text_align(), 1),
        );
        titles.push(cap.raw() as usize);
        st = st.child(cap);
    }

    // `AppPageIndicator`: every dot the same colour, the current one twice as
    // wide. The width is what the tween carries, and the row keeps a fixed
    // pitch so the dots do not shuffle sideways as it expands.
    let (d, gap) = (6.0, 10.0);
    let total = (n as f32 + 1.0) * d + (n as f32 - 1.0) * gap;
    let dot_y = h - 96.0 + 8.0;
    let dot_x = |i: usize| (w - total) / 2.0 + i as f32 * (d + gap);
    let mut dots = Vec::with_capacity(n);
    for i in 0..n {
        let dot = col(if i == sel { d * 2.0 } else { d }, d, ACCENT)?
            .radius(d / 2.0)
            .f32v_attr(attr::position(), &[dot_x(i), dot_y]);
        dots.push(dot.raw() as usize);
        st = st.child(dot);
    }

    st = st.child(
        col(w * 0.72, 52.0, GREY_STRONG)?
            .radius(4.0)
            .f32v_attr(attr::position(), &[w * 0.14, h - 66.0])
            .child(
                text("BROWSE ALL ARTIFACTS", 13.0, SHEET, w * 0.72, 52.0)?
                    .string_attr(attr::font_family(), SERIF_UI)
                    .i32_attr(attr::text_align(), 1)
                    .f32v_attr(attr::padding(), &[18.0, 0.0, 0.0, 0.0]),
            ),
    );

    if let Ok(mut g) = CAROUSEL.lock() {
        *g = Some(Carousel {
            items: (0..n)
                .map(|i| (bgs[i], frames[i], pics[i], titles[i], dots[i]))
                .collect(),
            iw,
            tall,
            centre,
            dot_d: d,
        });
    }
    Some(st)
}
fn events(wonder: &Wonder, index: usize, w: f32, h: f32) -> Option<Node> {
    let e = &EDITORIAL[index % EDITORIAL.len()];
    let pad = 24.0;
    let cwid = w - pad * 2.0;
    let mut b = Stack1::new(col(cwid, 0.0, 0x00000000)?);
    b = b.push(
        Some(
            text(wonder.title, 26.0, SHEET, cwid, 40.0)?.string_attr(attr::font_family(), DISPLAY),
        ),
        40.0,
    );
    b = b.push(
        Some(
            text(e.region, 13.0, 0xB3F8ECE5, cwid, 26.0)?
                .string_attr(attr::font_family(), SERIF_UI),
        ),
        30.0,
    );

    for (year, line) in [
        ("Construction", e.construction1),
        ("History", e.history1),
        ("Location", e.location1),
    ] {
        let cw = cwid - 16.0;
        let th = text_height(line, 14.0, cw);
        let mut rowbox =
            row(cwid, th + 46.0, 0x00000000)?.f32v_attr(attr::margin(), &[26.0, 0.0, 0.0, 0.0]);
        rowbox = rowbox.child(col(2.0, th + 30.0, 0x66E4935D)?);
        let mut txt =
            col(cw, th + 40.0, 0x00000000)?.f32v_attr(attr::padding(), &[0.0, 0.0, 0.0, 14.0]);
        txt = txt.child(
            text(year, 13.0, 0xFFE4935D, cw, 24.0)?.string_attr(attr::font_family(), SERIF_UI),
        );
        txt = txt.child(para(line, 14.0, 0xCCF8ECE5, cw)?);
        rowbox = rowbox.child(txt);
        b = b.push(Some(rowbox), th + 72.0);
    }
    let (inner, ih) = b.done(cwid);
    let mut s = scroll_w(w, h)?;
    s = s.child(
        col(w, ih + 76.0, wonder.bg)?
            .f32v_attr(attr::padding(), &[36.0, pad, 40.0, pad])
            .child(inner),
    );
    Some(s)
}
