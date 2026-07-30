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

pub fn build(index: usize, tab: usize, w: f32, h: f32) -> Option<Node> {
    let wonder = &WONDERS[index % WONDERS.len()];
    let bar_h = tabbar::height();
    let mut root = stack(w, h, wonder.bg)?;

    let body = match tab {
        1 => photos(wonder, w, h - bar_h),
        2 => artifacts(wonder, w, h - bar_h),
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
    // The bar draws correctly and receives nothing on its own: a target nested
    // in a positioned Row inside a Stack is not hit-tested. The cells therefore
    // live in one overlay row, laid out rather than positioned, so their layout
    // boxes are where they appear.
    let d = 52.0;
    let cell = (w - d - 20.0) / tabbar::TABS.len() as f32;
    let mut overlay = col(w, h, 0x00000000)?;
    overlay = overlay.child(spacer(w, h - bar_h)?);
    let mut line = row(w, bar_h, 0x00000000)?;
    line = line.child(tap_col(d + 20.0, bar_h, 0x00000000, tabbar::HOME_TAP)?);
    for i in 0..tabbar::TABS.len() {
        line = line.child(tap_col(
            cell,
            bar_h,
            0x00000000,
            tabbar::TAB_BASE + i as i32,
        )?);
    }
    overlay = overlay.child(line);
    root = root.child(overlay);
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
    Some(s)
}

/// The collapsing hero: a photo, the name in the display face, the region and
/// the dates beneath it.
fn hero(wonder: &Wonder, e: &super::editorial_data::Editorial, w: f32) -> Option<Node> {
    let hh = w * 1.05;
    let mut st = stack(w, hh, wonder.fg)?;
    st = st.child(photo(
        APP,
        &format!("{}/photo-1.jpg", wonder.dir),
        w,
        hh,
        0.0,
    )?);

    // A scrim, so the title holds over any photograph.
    st = st.child(col(w, hh * 0.5, 0x99000000)?.f32v_attr(attr::position(), &[0.0, hh * 0.5]));

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
    st = st.child(cap);
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
fn photos(wonder: &Wonder, w: f32, h: f32) -> Option<Node> {
    let gap = 4.0;
    let cell = (w - gap) / 2.0;
    let mut s = scroll_w(w, h)?;
    let mut grid = col(w, (cell + gap) * 4.0, wonder.bg)?;
    let files = ["photo-1.jpg", "photo-2.jpg", "photo-3.jpg", "photo-4.jpg"];
    for r in 0..4 {
        let mut line = row(w, cell, 0x00000000)?.f32v_attr(attr::margin(), &[0.0, 0.0, gap, 0.0]);
        for c in 0..2 {
            let f = files[(r * 2 + c) % files.len()];
            line = line.child(
                photo(APP, &format!("{}/{}", wonder.dir, f), cell, cell, 0.0)?
                    .f32v_attr(attr::margin(), &[0.0, gap / 2.0, 0.0, gap / 2.0]),
            );
        }
        grid = grid.child(line);
    }
    s = s.child(grid);
    Some(s)
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

/// One artifact, centred on an arch, with its name and date — the shape of
/// `artifact_carousel_screen.dart`.
fn artifacts(wonder: &Wonder, w: f32, h: f32) -> Option<Node> {
    let mut st = stack(w, h, wonder.bg)?;
    st = st.child(
        text("ARTIFACTS", 13.0, SHEET, w, 30.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, 28.0]),
    );

    // The arch: a tall rounded sheet with the piece centred on it.
    let aw = w * 0.62;
    let ah = aw * 1.45;
    let ax = (w - aw) / 2.0;
    let ay = h * 0.13;
    st = st.child(
        col(aw, ah, SHEET)?
            .radius(aw / 2.0)
            .f32v_attr(attr::position(), &[ax, ay]),
    );
    st = st.child(
        photo(
            APP,
            &format!("{}/photo-3.jpg", wonder.dir),
            aw * 0.72,
            aw * 0.72,
            aw * 0.36,
        )?
        .f32v_attr(attr::position(), &[ax + aw * 0.14, ay + ah * 0.18]),
    );

    let mut cap = col(w, 120.0, 0x00000000)?.f32v_attr(attr::position(), &[0.0, ay + ah + 26.0]);
    cap = cap.child(
        text(wonder.title, 30.0, SHEET, w, 44.0)?
            .string_attr(attr::font_family(), DISPLAY)
            .i32_attr(attr::text_align(), 1),
    );
    cap = cap.child(
        text(
            EDITORIAL[WONDERS
                .iter()
                .position(|x| x.dir == wonder.dir)
                .unwrap_or(0)]
            .region,
            15.0,
            0xCCF8ECE5,
            w,
            26.0,
        )?
        .string_attr(attr::font_family(), BODY_FONT)
        .i32_attr(attr::text_align(), 1),
    );
    st = st.child(cap);

    st = st.child(
        col(w * 0.72, 52.0, GREY_STRONG)?
            .radius(4.0)
            .f32v_attr(attr::position(), &[w * 0.14, h - 96.0])
            .child(
                text("BROWSE ALL ARTIFACTS", 13.0, SHEET, w * 0.72, 52.0)?
                    .string_attr(attr::font_family(), SERIF_UI)
                    .i32_attr(attr::text_align(), 1)
                    .f32v_attr(attr::padding(), &[18.0, 0.0, 0.0, 0.0]),
            ),
    );
    Some(st)
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// The wonder's events, as the timeline screen lists them: a year and a line of
/// prose, down a rule.
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
