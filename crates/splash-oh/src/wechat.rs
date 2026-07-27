//! A WeChat-shaped app built through the ArkUI NDK, for comparison against the
//! same app built in ArkTS.
//!
//! # Why this exists
//!
//! The microbenchmark in `bench.rs` says Rust builds a widget ~3× faster than
//! ArkTS. On one screen built once that is a few milliseconds and nobody would
//! notice. The objection to that framing is fair though: a super-app is not one
//! screen built once. It builds and tears down view hierarchies continuously,
//! and its JS thread is never idle, which is exactly the condition under which
//! the napi round trip went from 31 µs to 1051 µs in the octos-one port.
//!
//! So this rebuilds the screens from
//! [project-robius/makepad_wechat](https://github.com/project-robius/makepad_wechat)
//! twice — once here through the NDK, once in ArkTS through `typeNode` — and
//! measures both, idle and under load.
//!
//! # Keeping the two honest
//!
//! Both sides build the same tree: same node types, same counts, same
//! attributes. `SCREENS` below is the contract, and the node count each side
//! reports is checked against it at runtime, because "the ArkTS version is
//! faster because it quietly builds less" is the obvious way for a comparison
//! like this to be wrong.
//!
//! Row shapes are taken from the reference app:
//!
//! | row | nodes | from |
//! |---|---|---|
//! | chat preview | 6 | `ChatPreview` in `home/chat_list.rs` |
//! | message bubble | 4 | `home/chat_screen.rs` |
//! | contact | 3 | `contacts/contacts_list.rs` |
//! | discover entry | 4 | `discover/discover_screen.rs` |
//! | moment | 5 | `discover/moment_list.rs` |
//!
//! Avatars are real `ARKUI_NODE_IMAGE` nodes with no source set, on both sides.
//! That counts the image node's construction cost, which is the part under
//! test, without dragging in decode time, which is neither side's doing and
//! would swamp everything else.

use crate::arkui::{attr, ty, Node};
use std::cell::RefCell;
use std::time::Instant;

/// Material-ish WeChat palette, as ARGB.
const BG: u32 = 0xFFEDEDED;
const SURFACE: u32 = 0xFFFFFFFF;
const NAV: u32 = 0xFFF7F7F7;
const TEXT: u32 = 0xFF191919;
const SUBTLE: u32 = 0xFF9A9A9A;
const GREEN: u32 = 0xFF07C160;
const AVATAR: u32 = 0xFFD8D8D8;

/// One screen of the app: id, display name, and how many nodes it must build.
///
/// The count is not decoration — both implementations are checked against it,
/// so a version that silently builds a smaller tree fails loudly instead of
/// winning the benchmark.
pub struct Screen {
    pub id: &'static str,
    pub title: &'static str,
    pub rows: usize,
    pub nodes: usize,
}

/// Chrome common to every screen: nav bar (3) + tab bar (10).
const CHROME: usize = 13;

pub const SCREENS: &[Screen] = &[
    // nav 3 + search 2 + 20 × 6 + tabs 10
    Screen { id: "chats",    title: "WeChat",   rows: 20, nodes: CHROME + 2 + 20 * 6 },
    // nav 3 + 25 × 4 + composer 3 + tabs 10
    Screen { id: "chat",     title: "Chat",     rows: 25, nodes: CHROME + 3 + 25 * 4 },
    // nav 3 + search 2 + 40 × 3 + tabs 10
    Screen { id: "contacts", title: "Contacts", rows: 40, nodes: CHROME + 2 + 40 * 3 },
    // nav 3 + 8 × 4 + tabs 10
    Screen { id: "discover", title: "Discover", rows: 8,  nodes: CHROME + 8 * 4 },
    // nav 3 + 15 × 5 + tabs 10
    Screen { id: "moments",  title: "Moments",  rows: 15, nodes: CHROME + 15 * 5 },
    // nav 3 + header 8 + 6 × 4 + tabs 10
    Screen { id: "me",       title: "Me",       rows: 6,  nodes: CHROME + 8 + 6 * 4 },
];

/// Nodes created during the current build, so both sides can be checked.
thread_local! {
    static COUNT: RefCell<usize> = const { RefCell::new(0) };
    /// The built screen, held so its cost shows up in RSS.
    static SCREEN: RefCell<Option<Node>> = const { RefCell::new(None) };
    /// Screens accumulated for the memory arm — a super-app keeps many pages
    /// alive at once, so the question is what a stack of them costs, not one.
    static KEPT: RefCell<Vec<Node>> = const { RefCell::new(Vec::new()) };
}

fn bump() {
    COUNT.with(|c| *c.borrow_mut() += 1);
}

fn text(s: &str, size: f32, color: u32, w: f32, h: f32) -> Option<Node> {
    bump();
    Some(
        Node::new(ty::text())?
            .text(s)
            .font_size(size)
            .font_color(color)
            .width(w)
            .height(h),
    )
}

fn col(w: f32, h: f32, bg: u32) -> Option<Node> {
    bump();
    Some(Node::new(ty::column())?.width(w).height(h).bg(bg))
}

fn row(w: f32, h: f32, bg: u32) -> Option<Node> {
    bump();
    Some(Node::new(ty::row())?.width(w).height(h).bg(bg))
}

/// An avatar: a real Image node with no source, same as the ArkTS side.
fn avatar(size: f32) -> Option<Node> {
    bump();
    Some(
        Node::new(ty::image())?
            .width(size)
            .height(size)
            .bg(AVATAR)
            .radius(6.0),
    )
}

const W: f32 = 402.0;

/// Nav bar: container + title + action. 3 nodes.
fn nav(title: &str) -> Option<Node> {
    let mut n = row(W, 44.0, NAV)?;
    n = n.child(text(title, 17.0, TEXT, 300.0, 24.0)?);
    n = n.child(text("+", 20.0, TEXT, 40.0, 24.0)?);
    Some(n)
}

/// Tab bar: container + 4 tabs × (icon + label + 1 spacer). 10 nodes.
fn tabs(active: usize) -> Option<Node> {
    let mut bar = row(W, 52.0, NAV)?;
    for (i, label) in ["Chats", "Contacts", "Discover", "Me"].iter().enumerate() {
        let c = if i == active { GREEN } else { SUBTLE };
        bar = bar.child(col(20.0, 20.0, c)?);
        bar = bar.child(text(label, 10.0, c, 70.0, 14.0)?);
    }
    Some(bar)
}

/// Search bar: container + placeholder. 2 nodes.
fn search() -> Option<Node> {
    let mut s = row(W - 16.0, 32.0, SURFACE)?;
    s = s.child(text("Search", 13.0, SUBTLE, 200.0, 18.0)?);
    Some(s)
}

/// `ChatPreview` from the reference app: row + avatar + column + 3 labels.
fn chat_row(i: usize) -> Option<Node> {
    let mut r = row(W, 64.0, SURFACE)?;
    r = r.child(avatar(40.0)?);
    let mut mid = col(W - 130.0, 56.0, SURFACE)?;
    mid = mid.child(text(NAMES[i % NAMES.len()], 15.0, TEXT, W - 140.0, 20.0)?);
    mid = mid.child(text(
        PREVIEWS[i % PREVIEWS.len()],
        12.0,
        SUBTLE,
        W - 140.0,
        18.0,
    )?);
    r = r.child(mid);
    r = r.child(text("yesterday", 10.0, SUBTLE, 60.0, 16.0)?);
    Some(r)
}

/// A chat bubble: row + avatar + column + label.
fn message_row(i: usize) -> Option<Node> {
    let mut r = row(W, 52.0, BG)?;
    r = r.child(avatar(34.0)?);
    let mut b = col(W - 120.0, 40.0, if i % 2 == 0 { SURFACE } else { GREEN })?;
    b = b.child(text(
        PREVIEWS[i % PREVIEWS.len()],
        14.0,
        if i % 2 == 0 { TEXT } else { SURFACE },
        W - 130.0,
        20.0,
    )?);
    r = r.child(b);
    Some(r)
}

/// A contact: row + icon + label.
fn contact_row(i: usize) -> Option<Node> {
    let mut r = row(W, 48.0, SURFACE)?;
    r = r.child(avatar(32.0)?);
    r = r.child(text(NAMES[i % NAMES.len()], 15.0, TEXT, W - 80.0, 20.0)?);
    Some(r)
}

/// A discover entry: row + icon + label + chevron.
fn discover_row(i: usize) -> Option<Node> {
    let mut r = row(W, 50.0, SURFACE)?;
    r = r.child(avatar(24.0)?);
    r = r.child(text(DISCOVER[i % DISCOVER.len()], 15.0, TEXT, W - 100.0, 20.0)?);
    r = r.child(text("›", 15.0, SUBTLE, 20.0, 20.0)?);
    Some(r)
}

/// A moment: column + avatar + name + body + banner.
fn moment_row(i: usize) -> Option<Node> {
    let mut m = col(W, 150.0, SURFACE)?;
    m = m.child(avatar(36.0)?);
    m = m.child(text(NAMES[i % NAMES.len()], 14.0, GREEN, W - 20.0, 20.0)?);
    m = m.child(text(PREVIEWS[i % PREVIEWS.len()], 13.0, TEXT, W - 20.0, 20.0)?);
    m = m.child(avatar(64.0)?);
    Some(m)
}

const NAMES: &[&str] = &[
    "Rik Arends", "Eddy Bruel", "Ken Wu", "Julian Montes de Oca", "Edward Tan",
    "Alex Zhang", "Jorge Bejar", "Sandra Li", "Wei Chen", "Tom Xu",
];
const PREVIEWS: &[&str] = &[
    "Hi there! I'm using WeChat",
    "Let's meet at 3pm tomorrow",
    "Sent a sticker",
    "Did you see the new build?",
    "[Photo]",
];
const DISCOVER: &[&str] = &[
    "Moments", "Channels", "Scan", "Shake", "Search", "Top Stories",
    "Mini Programs", "Nearby",
];

/// Build one screen. Returns (nodes built, µs).
pub fn build(screen: &str) -> (usize, f64) {
    COUNT.with(|c| *c.borrow_mut() = 0);
    let t0 = Instant::now();

    let spec = SCREENS.iter().find(|s| s.id == screen);
    let Some(spec) = spec else {
        return (0, 0.0);
    };

    let built = (|| -> Option<Node> {
        let mut root = col(W, 780.0, BG)?;
        root = root.child(nav(spec.title)?);

        match spec.id {
            "chats" => {
                root = root.child(search()?);
                for i in 0..spec.rows {
                    root = root.child(chat_row(i)?);
                }
            }
            "chat" => {
                for i in 0..spec.rows {
                    root = root.child(message_row(i)?);
                }
                // composer: row + input + send
                let mut c = row(W, 48.0, NAV)?;
                c = c.child(text("Message", 14.0, SUBTLE, W - 100.0, 20.0)?);
                c = c.child(text("Send", 14.0, GREEN, 50.0, 20.0)?);
                root = root.child(c);
            }
            "contacts" => {
                root = root.child(search()?);
                for i in 0..spec.rows {
                    root = root.child(contact_row(i)?);
                }
            }
            "discover" => {
                for i in 0..spec.rows {
                    root = root.child(discover_row(i)?);
                }
            }
            "moments" => {
                for i in 0..spec.rows {
                    root = root.child(moment_row(i)?);
                }
            }
            "me" => {
                // header: 8 nodes
                let mut h = row(W, 110.0, SURFACE)?;
                h = h.child(avatar(64.0)?);
                let mut hc = col(W - 120.0, 70.0, SURFACE)?;
                hc = hc.child(text("Rik Arends", 20.0, TEXT, 200.0, 26.0)?);
                hc = hc.child(text("WeChat ID: rik", 12.0, SUBTLE, 200.0, 18.0)?);
                hc = hc.child(text("＋ Status", 12.0, SUBTLE, 200.0, 18.0)?);
                h = h.child(hc);
                h = h.child(avatar(20.0)?);
                h = h.child(text("›", 15.0, SUBTLE, 20.0, 20.0)?);
                root = root.child(h);
                for i in 0..spec.rows {
                    root = root.child(discover_row(i)?);
                }
            }
            _ => {}
        }

        root = root.child(tabs(0)?);
        Some(root)
    })();

    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    let n = COUNT.with(|c| *c.borrow());

    // Hold it so the memory cost is visible in RSS; the previous screen goes
    // now rather than lingering into the next measurement.
    SCREEN.with(|s| *s.borrow_mut() = built);

    (n, us)
}

/// Drop the held screen.
pub fn clear() {
    SCREEN.with(|s| *s.borrow_mut() = None);
}

/// Build a screen and keep it, on top of everything kept so far.
/// Returns how many screens are now held.
pub fn keep(screen: &str) -> usize {
    let (_, _) = build(screen);
    let node = SCREEN.with(|s| s.borrow_mut().take());
    KEPT.with(|k| {
        let mut v = k.borrow_mut();
        if let Some(n) = node {
            v.push(n);
        }
        v.len()
    })
}

/// Drop every kept screen.
pub fn drop_kept() -> usize {
    KEPT.with(|k| {
        let v = std::mem::take(&mut *k.borrow_mut());
        let n = v.len();
        drop(v);
        n
    })
}

/// What `screen` is contractually required to build.
pub fn expected(screen: &str) -> usize {
    SCREENS
        .iter()
        .find(|s| s.id == screen)
        .map(|s| s.nodes)
        .unwrap_or(0)
}
