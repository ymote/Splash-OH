//! TikTok, ported from
//! [project-robius/makepad_tiktok](https://github.com/project-robius/makepad_tiktok).
//!
//! The useful thing about this one for the comparison is that it is the
//! opposite of a list: a full-screen media surface with a handful of overlay
//! widgets. Where the WeChat chat list builds ~150 nodes, a reel builds ~30.
//! If the Rust-vs-ArkTS gap is a per-node constant, this app should show the
//! smallest absolute difference of the four, and that is worth checking rather
//! than assuming.
//!
//! # The video
//!
//! The reference app uses makepad's `Video` widget with mp4 sources. The ArkUI
//! NDK has no video node — `ARKUI_NODE_VIDEO` does not exist, the same gap as
//! `ARKUI_NODE_WEB` — so both implementations render a poster image where the
//! video goes. That is the same substitution on both sides, so the comparison
//! stays fair; it just means neither number includes decode.
//!
//! `ReelActions` in the reference app is a column of four buttons, each a
//! `View` wrapping a `Button` and a `Label`, so three nodes per action.

use crate::arkui::Node;
use crate::ui::*;

const APP: &str = "tiktok";

const BLACK: u32 = 0xFF000000;
const WHITE: u32 = 0xFFFFFFFF;
const DIM: u32 = 0xFFBFBFBF;
const PINK: u32 = 0xFFFE2C55;

pub const TAB_BASE: i32 = 300;
pub const REEL_BASE: i32 = 3000;
pub const BACK: i32 = 310;

/// The reference app's five reels: (poster, author, caption, likes, comments).
pub const REELS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "poster1.jpg",
        "@seagulls",
        "Seagulls at the pier 🌊 #ocean",
        "1234",
        "234",
    ),
    (
        "poster2.jpg",
        "@dancing",
        "Friday night moves 💃 #dance",
        "2234",
        "512",
    ),
    (
        "poster3.jpg",
        "@cat",
        "He does this every morning 🐱",
        "3234",
        "890",
    ),
    (
        "poster4.jpg",
        "@cat2",
        "Round two #catsoftiktok",
        "4234",
        "1203",
    ),
    (
        "poster5.jpg",
        "@seagulls",
        "Golden hour 🌅 #sunset",
        "5234",
        "2201",
    ),
];

/// Top bar: Following / For You, plus search.
fn header(active: usize) -> Option<Node> {
    let mut h = row(W, 46.0, BLACK)?;
    h = h.child(spacer(60.0, 46.0)?);
    for (i, label) in ["Following", "For You"].iter().enumerate() {
        let c = if i == active { WHITE } else { DIM };
        let mut t = tap_col(110.0, 46.0, BLACK, TAB_BASE + i as i32)?;
        t = t.child(text_w(
            label,
            15.0,
            c,
            106.0,
            22.0,
            if i == active { 7 } else { 4 },
        )?);
        if i == active {
            t = t.child(col(30.0, 2.0, WHITE)?);
        }
        bar_pad(&mut t);
        h = h.child(t);
    }
    h = h.child(icon(APP, "search_icon.svg", 22.0)?);
    Some(h)
}

/// No-op that documents intent: the reference app pads these tabs, and the
/// padding is a layout property rather than a node, so it costs nothing here.
fn bar_pad(_t: &mut Node) {}

/// One action button: icon plus count. Three nodes, as the reference app has.
fn action(file: &str, count: &str, tap: i32) -> Option<Node> {
    let mut a = tap_col(60.0, 62.0, 0, tap)?;
    a = a.child(icon(APP, file, 30.0)?);
    a = a.child(text(count, 11.0, WHITE, 56.0, 16.0)?);
    Some(a)
}

/// The right-hand action column: avatar plus four actions.
fn actions(idx: usize) -> Option<Node> {
    let (_, _, _, likes, comments) = REELS[idx];
    let mut c = col(70.0, 340.0, 0)?;
    c = c.child(photo(APP, "default_avatar.png", 46.0, 46.0, 23.0)?);
    c = c.child(action("heart.png", likes, REEL_BASE + 1)?);
    c = c.child(action("chat_icon.svg", comments, REEL_BASE + 2)?);
    c = c.child(action("star_icon.svg", "Save", REEL_BASE + 3)?);
    c = c.child(action("share_icon.svg", "Share", REEL_BASE + 4)?);
    Some(c)
}

/// The caption block along the bottom left.
fn caption(idx: usize) -> Option<Node> {
    let (_, author, cap, _, _) = REELS[idx];
    let mut c = col(W - 90.0, 80.0, 0)?;
    c = c.child(text_w(author, 15.0, WHITE, W - 100.0, 22.0, 7)?);
    c = c.child(text(cap, 13.0, WHITE, W - 100.0, 36.0)?);
    let mut music = row(W - 100.0, 20.0, 0)?;
    music = music.child(icon(APP, "at_sign.png", 14.0)?);
    music = music.child(text("original sound", 11.0, DIM, 160.0, 16.0)?);
    c = c.child(music);
    Some(c)
}

/// One full-screen reel: media, then the overlay on top of it.
fn reel(idx: usize) -> Option<Node> {
    let media_h = PAGE_H - 46.0 - 56.0;
    let mut s = stack(W, media_h, BLACK)?;
    // The video stand-in. Same substitution on both implementations.
    s = s.child(photo(APP, REELS[idx].0, W, media_h, 0.0)?);
    // Overlay: caption bottom-left, actions bottom-right.
    let mut overlay = col(W, media_h, 0)?;
    overlay = overlay.child(spacer(W, media_h - 130.0)?);
    let mut bottom = row(W, 120.0, 0)?;
    bottom = bottom.child(caption(idx)?);
    bottom = bottom.child(actions(idx)?);
    overlay = overlay.child(bottom);
    s = s.child(overlay);
    Some(s)
}

/// The bottom bar. TikTok's has a distinctive centre "+" button.
fn tab_bar() -> Option<Node> {
    let mut bar = row(W, 56.0, BLACK)?;
    for (i, label) in ["Home", "Discover", "", "Inbox", "Me"].iter().enumerate() {
        if label.is_empty() {
            let mut plus = row(56.0, 34.0, WHITE)?.radius(8.0);
            plus = plus.child(icon(APP, "plus.png", 22.0)?);
            bar = bar.child(plus);
            continue;
        }
        let c = if i == 0 { WHITE } else { DIM };
        let mut t = tap_col(W / 5.0, 56.0, BLACK, TAB_BASE + 10 + i as i32)?;
        t = t.child(text(label, 10.0, c, W / 5.0 - 2.0, 14.0)?);
        bar = bar.child(t);
    }
    Some(bar)
}

/// The comment sheet, which the reference app opens over the reel.
fn comments(idx: usize) -> Option<Node> {
    let mut root = col(W, PAGE_H, 0xFF161616)?;
    let mut h = row(W, 46.0, 0xFF161616)?;
    h = h.child(tap_row(56.0, 46.0, 0, BACK)?.child(text("‹", 24.0, WHITE, 44.0, 30.0)?));
    h = h.child(text_w(
        &format!("{} comments", REELS[idx].4),
        15.0,
        WHITE,
        W - 120.0,
        22.0,
        7,
    )?);
    root = root.child(h);
    let mut body = col(W, 0.0, 0xFF161616)?;
    for i in 0..14 {
        let mut r = row(W, 62.0, 0xFF161616)?;
        r = r.child(photo(APP, "default_avatar.png", 36.0, 36.0, 18.0)?);
        let mut c = col(W - 130.0, 52.0, 0)?;
        c = c.child(text_w(
            REELS[i % REELS.len()].1,
            12.0,
            DIM,
            W - 140.0,
            18.0,
            5,
        )?);
        c = c.child(text(
            REELS[i % REELS.len()].2,
            13.0,
            WHITE,
            W - 140.0,
            20.0,
        )?);
        r = r.child(c);
        let mut like = col(50.0, 44.0, 0)?;
        like = like.child(icon(APP, "heart.png", 16.0)?);
        like = like.child(text("12", 10.0, DIM, 44.0, 14.0)?);
        r = r.child(like);
        body = body.child(r);
    }
    root = root.child(scroll(PAGE_H - 46.0)?.child(body));
    Some(root)
}

/// `tab` selects Following / For You; `sheet` opens the comment sheet.
pub fn build(tab: usize, reel_idx: usize, sheet: bool) -> Option<Node> {
    if sheet {
        return comments(reel_idx);
    }
    let mut root = col(W, PAGE_H, BLACK)?;
    root = root.child(header(tab)?);
    root = root.child(reel(reel_idx.min(REELS.len() - 1))?);
    root = root.child(tab_bar()?);
    Some(root)
}

/// The whole feed built at once, for the memory arm — a real session scrolls
/// through reels rather than holding one.
pub fn build_feed() -> Option<Node> {
    let mut root = col(W, PAGE_H, BLACK)?;
    root = root.child(header(1)?);
    let mut body = col(W, 0.0, BLACK)?;
    for i in 0..REELS.len() {
        body = body.child(reel(i)?);
    }
    root = root.child(scroll(PAGE_H - 46.0 - 56.0)?.child(body));
    root = root.child(tab_bar()?);
    Some(root)
}
