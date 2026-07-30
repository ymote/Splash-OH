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

/// The hero's photograph and its title block, kept so a scroll can move them
/// without rebuilding the tree.
///
/// Wonderous collapses the hero as the article scrolls: the image drifts up at
/// half the scroll rate — parallax — and the title fades toward the top. That
/// is per-frame, so rebuilding the whole screen for each event is not an
/// option; these two handles get their attributes set directly.
static HERO_IMAGE: AtomicUsize = AtomicUsize::new(0);
static HERO_TITLE: AtomicUsize = AtomicUsize::new(0);
static HERO_SCROLL: AtomicUsize = AtomicUsize::new(0);
static HERO_H: std::sync::Mutex<f32> = std::sync::Mutex::new(0.0);

/// The photograph is drawn half again as tall as the frame that clips it, so
/// there is something to slide. At rest it is centred, a quarter of the excess
/// hidden above and a quarter below; at the end of the travel those swap. Any
/// less and the bottom edge of the frame would run off the picture.
const PARALLAX_OVERDRAW: f32 = 1.5;
/// How much of the hero the title survives — it is gone well before the
/// picture is, which is what makes the two read as separate planes.
const TITLE_FADE: f32 = 1.6;

/// Apply the current scroll offset to the hero. Called from the scroll event.
///
/// Both handles are cleared whenever the screen is rebuilt, so a stale one is
/// never written to.
pub fn apply_parallax() {
    let hero_h = HERO_H.lock().map(|h| *h).unwrap_or(0.0);
    let scroll = HERO_SCROLL.load(Ordering::Relaxed);
    if hero_h <= 0.0 || scroll == 0 {
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
    let t = (y / hero_h).clamp(0.0, 1.0);
    let img = HERO_IMAGE.load(Ordering::Relaxed);
    let title = HERO_TITLE.load(Ordering::Relaxed);

    if img != 0 {
        // The Stack has already been carried up by the whole scroll, so pushing
        // the photograph back *down* by half of it leaves the photograph
        // climbing the screen at half the rate of the article over it.
        let travel = hero_h * (PARALLAX_OVERDRAW - 1.0) / 2.0;
        unsafe {
            Node::set_f32v_raw(
                img as crate::arkui::NodeHandle,
                attr::position(),
                &[0.0, travel * (2.0 * t - 1.0)],
            );
        }
    }
    if title != 0 {
        unsafe {
            Node::set_f32_attr_raw(
                title as crate::arkui::NodeHandle,
                attr::opacity(),
                (1.0 - t * TITLE_FADE).clamp(0.0, 1.0),
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

pub fn build(index: usize, tab: usize, w: f32, h: f32) -> Option<Node> {
    // The old tree is about to be dropped. Forget its hero before anything can
    // write to those handles; only the editorial tab puts them back.
    HERO_IMAGE.store(0, Ordering::Relaxed);
    HERO_TITLE.store(0, Ordering::Relaxed);
    HERO_SCROLL.store(0, Ordering::Relaxed);
    if let Ok(mut g) = HERO_H.lock() {
        *g = 0.0;
    }
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
        targets.push((0.0, ay, w * 0.5, ah, ARTIFACT_PREV));
        targets.push((w * 0.5, ay, w * 0.5, ah, ARTIFACT_NEXT));
    }
    root = root.child(super::hits(w, h, &targets)?);

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

    let hero_h = w * 1.05;
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
    sheet = sheet.push(hero(wonder, e, w), hero_h);
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

/// The collapsing hero: a photo, the name in the display face, the region and
/// the dates beneath it.
fn hero(wonder: &Wonder, e: &super::editorial_data::Editorial, w: f32) -> Option<Node> {
    let hh = w * 1.05;
    // Clipped, because the photograph inside is deliberately taller than the
    // frame it sits in and would otherwise paint over the article.
    let mut st = stack(w, hh, wonder.fg)?.i32_attr(attr::clip(), 1);
    let img = photo(
        APP,
        &format!("{}/photo-1.jpg", wonder.dir),
        w,
        hh * PARALLAX_OVERDRAW,
        0.0,
    )?
    .f32v_attr(
        attr::position(),
        &[0.0, -hh * (PARALLAX_OVERDRAW - 1.0) / 2.0],
    );
    HERO_IMAGE.store(img.raw() as usize, Ordering::Relaxed);
    if let Ok(mut g) = HERO_H.lock() {
        *g = hh;
    }
    st = st.child(img);

    // The scrim and the title are one layer, because they fade together: the
    // scrim exists only to hold the title over the photograph.
    //
    // ARKUI_LINEAR_GRADIENT_DIRECTION_BOTTOM. A flat rectangle of black left a
    // hard horizontal seam straight across the pyramids once the hero started
    // moving under it.
    let mut veil = stack(w, hh, 0x00000000)?;
    veil = veil.child(
        col(w, hh * 0.62, 0x00000000)?
            .gradient(3, &[0x00000000, 0x66000000, 0xB3000000], &[0.0, 0.55, 1.0])
            .f32v_attr(attr::position(), &[0.0, hh * 0.38]),
    );

    let size = (w * 0.10).clamp(26.0, 40.0);
    let mut cap = col(w, 130.0, 0x00000000)?.f32v_attr(attr::position(), &[0.0, hh - 140.0]);
    cap = cap.child(
        text(wonder.title, size, SHEET, w - 48.0, size * 1.4)?
            .string_attr(attr::font_family(), DISPLAY)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::padding(), &[0.0, 24.0, 0.0, 24.0]),
    );
    cap = cap.child(
        text(e.sub_title, 14.0, 0xCCF8ECE5, w - 48.0, 22.0)?
            .string_attr(attr::font_family(), BODY_ITALIC)
            .i32_attr(attr::text_align(), 1),
    );
    cap = cap.child(
        text(e.region, 12.0, 0xB3F8ECE5, w - 48.0, 20.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1),
    );
    veil = veil.child(cap);
    HERO_TITLE.store(veil.raw() as usize, Ordering::Relaxed);
    st = st.child(veil);
    Some(st)
}

/// A section break: the arc label with its icon under it.
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

pub fn photo_sel() -> usize {
    PHOTO_SEL.load(Ordering::Relaxed)
}

/// Move the selection one cell, refusing the moves the app refuses: off the
/// grid, and wrapping round a row edge.
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
    true
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

    // Where the grid's own top-left corner ends up: centre it, then shift by
    // the selected cell so that cell is the one in the middle.
    let gw = GRID as f32 * iw + (GRID - 1) as f32 * pad;
    let gh = GRID as f32 * ih + (GRID - 1) as f32 * pad;
    let ox = (w - gw) / 2.0 + (2.0 - (sel % GRID) as f32) * sx;
    let oy = (h - gh) / 2.0 + (2.0 - (sel / GRID) as f32) * sy;

    let mut root = stack(w, h, wonder.bg)?.i32_attr(attr::clip(), 1);
    for i in 0..GRID * GRID {
        let (c, r) = (i % GRID, i / GRID);
        let (x, y) = (ox + c as f32 * sx, oy + r as f32 * sy);
        // Only the nine cells around the selection can be on screen.
        if x > w || y > h || x + iw < 0.0 || y + ih < 0.0 {
            continue;
        }
        root = root.child(
            photo(
                APP,
                &format!("{}/gallery/{:02}.jpg", wonder.dir, i % GALLERY_PHOTOS),
                iw,
                ih,
                0.0,
            )?
            .f32v_attr(attr::position(), &[x, y]),
        );
    }

    // `_AnimatedCutoutOverlay`: a 70% scrim over everything except the selected
    // cell. Four rectangles around the hole — ArkUI has no cut-out shape, and
    // four solid edges are exactly what the cut-out leaves behind.
    let (hx, hy) = (ox + (sel % GRID) as f32 * sx, oy + (sel / GRID) as f32 * sy);
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
pub const PHOTO_UP: i32 = 7340;
pub const PHOTO_DOWN: i32 = 7341;
pub const PHOTO_LEFT: i32 = 7342;
pub const PHOTO_RIGHT: i32 = 7343;

/// The carousel's two halves, and the button under it.
pub const ARTIFACT_PREV: i32 = 7370;
pub const ARTIFACT_NEXT: i32 = 7371;
pub const BROWSE_TAP: i32 = 7372;

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

fn artifacts(wonder: &Wonder, index: usize, sel: usize, w: f32, h: f32) -> Option<Node> {
    let list = super::artifact_data::ARTIFACTS[index % 8];
    if list.is_empty() {
        return stack(w, h, wonder.bg);
    }
    let n = list.len();
    let art = &list[sel % n];
    let mut st = stack(w, h, wonder.bg)?.i32_attr(attr::clip(), 1);

    // `_BlurredImageBg`: the current piece, scaled up, blurred, under a black
    // wash. The screen takes its colour from the object on it rather than from
    // the wonder.
    st = st.child(
        photo(
            APP,
            &format!("artifacts/{}.jpg", art.id),
            w * 1.25,
            h * 1.25,
            0.0,
        )?
        .f32_attr(attr::blur(), 6.0)
        .f32v_attr(attr::position(), &[-w * 0.125, -h * 0.05]),
    );
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

    let bottom_h = h / 2.75;
    let item_h = (h - 200.0 - bottom_h).clamp(250.0, 400.0);
    let iw = item_h * 0.666;
    let (ay, tall) = artifact_arch(w, h);

    // The neighbours either side, square and dropped by half a width, at the
    // page width the carousel's viewport fraction puts them at.
    for d in [-1i32, 1] {
        let other = &list[((sel + n) as i32 + d) as usize % n];
        let x = (w - iw) / 2.0 + d as f32 * iw;
        let y = (h - iw) / 2.0 - tall * 0.25 + iw * 0.5;
        st = st.child(
            photo(
                APP,
                &format!("artifacts/{}.jpg", other.id),
                iw * 0.8,
                iw * 0.8,
                iw * 0.4,
            )?
            .f32v_attr(attr::position(), &[x + iw * 0.1, y + iw * 0.1]),
        );
    }

    // `_DoubleBorderImage`: a capsule outlined in off-white with the piece
    // inset by 8, clipped to the same capsule.
    let ax = (w - iw) / 2.0;
    st = st.child(
        col(iw, tall, 0x00000000)?
            .f32v_attr(
                attr::border_radius(),
                &[iw / 2.0, iw / 2.0, iw / 2.0, iw / 2.0],
            )
            .f32_attr(attr::border_width(), 1.0)
            .u32_attr(attr::border_color(), SHEET)
            .f32v_attr(attr::position(), &[ax, ay]),
    );
    st = st.child(
        photo(
            APP,
            &format!("artifacts/{}.jpg", art.id),
            iw - 16.0,
            tall - 16.0,
            (iw - 16.0) / 2.0,
        )?
        .f32v_attr(attr::position(), &[ax + 8.0, ay + 8.0]),
    );

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
    let title_lines = if art.title.chars().count() > 22 {
        2.0
    } else {
        1.0
    };
    let title_h = 36.0 * title_lines;
    let text_top = h - bottom_h + 24.0;
    let mut cap = col(w, title_h + 40.0, 0x00000000)?.f32v_attr(attr::position(), &[0.0, text_top]);
    cap = cap.child(
        text(art.title, 30.0, INK, w - 48.0, title_h)?
            .string_attr(attr::font_family(), DISPLAY)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::padding(), &[0.0, 24.0, 0.0, 24.0]),
    );
    cap = cap.child(
        text(art.date, 14.0, 0xB31E1B18, w, 24.0)?
            .string_attr(attr::font_family(), BODY_FONT)
            .i32_attr(attr::text_align(), 1),
    );
    st = st.child(cap);

    // `AppPageIndicator`, one dot per highlight.
    let (d, gap) = (7.0, 10.0);
    let total = n as f32 * d + (n as f32 - 1.0) * gap;
    let mut dots = row(w, 24.0, 0x00000000)?
        .f32v_attr(attr::position(), &[0.0, h - 96.0])
        .f32v_attr(attr::padding(), &[8.0, 0.0, 0.0, (w - total) / 2.0]);
    for i in 0..n {
        dots = dots.child(
            col(d, d, if i == sel % n { ACCENT } else { 0x4D1E1B18 })?
                .radius(d / 2.0)
                .f32v_attr(attr::margin(), &[0.0, gap / 2.0, 0.0, gap / 2.0]),
        );
    }
    st = st.child(dots);

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
