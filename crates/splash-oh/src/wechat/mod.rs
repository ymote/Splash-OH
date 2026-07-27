//! The WeChat demo, rebuilt on native ArkUI, driven entirely from Rust.
//!
//! This is a port of
//! [project-robius/makepad_wechat](https://github.com/project-robius/makepad_wechat)
//! — the same app, the same data, the same navigation — with makepad's renderer
//! replaced by real OpenHarmony widgets created through the ArkUI NDK. An ArkTS
//! implementation of the identical app lives in
//! `deveco/entry/src/main/ets/pages/WeChatArkTs.ets`, and the two can be
//! switched between at runtime so they can be compared on the same device, in
//! the same process, against the same data.
//!
//! # Navigation
//!
//! The reference app uses makepad's `StackNavigation`: a root view holding four
//! tab pages, plus four views that get pushed on top (chat, moments, add
//! contact, my profile). That is modelled here as a tab index plus a route,
//! which is all `StackNavigation` amounts to for an app of this shape.
//!
//! # Event ids
//!
//! ArkUI hands back only an `i32` per click, so ids are allocated in ranges.
//! Chat rows need to carry which chat they opened, hence `CHAT_BASE + id`.

pub mod db;

use crate::arkui::{attr, event, ty, Node};
use db::*;
use std::cell::RefCell;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Palette, taken from the reference app's styles.rs
// ---------------------------------------------------------------------------
const BG: u32 = 0xFFEDEDED;
const SURFACE: u32 = 0xFFFFFFFF;
const NAV_BG: u32 = 0xFFF7F7F7;
const TEXT: u32 = 0xFF191919;
const SUBTLE: u32 = 0xFF9A9A9A;
const DIVIDER: u32 = 0xFFE5E5E5;
const GREEN: u32 = 0xFF07C160;
const BUBBLE_OUT: u32 = 0xFF95EC69;
const AVATAR_BG: u32 = 0xFFC8C8C8;

const W: f32 = 402.0;
const PAGE_H: f32 = 780.0;

// ---------------------------------------------------------------------------
// Event ids
// ---------------------------------------------------------------------------
pub const TAB_BASE: i32 = 100; // +0..3
pub const BACK: i32 = 110;
pub const CHAT_BASE: i32 = 1000; // + chat id
pub const LINK_MOMENTS: i32 = 120;
pub const LINK_MY_PROFILE: i32 = 121;
pub const LINK_ADD_CONTACT: i32 = 122;
pub const TOGGLE_IMPL: i32 = 130;

/// Which page is on top of the navigation stack.
#[derive(Clone, Copy, PartialEq)]
pub enum Route {
    Root,
    Chat(u64),
    Moments,
    AddContact,
    MyProfile,
}

pub struct Nav {
    pub tab: usize,
    pub route: Route,
}

thread_local! {
    static NAV: RefCell<Nav> = const {
        RefCell::new(Nav { tab: 0, route: Route::Root })
    };
    static COUNT: RefCell<usize> = const { RefCell::new(0) };
    /// Set when the header toggle is tapped in the native tree. ArkTS polls it,
    /// because the native -> JS direction is the one that does not currently
    /// work here (see measurement C in CONCLUSION.md), and a 300 ms poll is
    /// plenty for a button.
    static WANTS_TOGGLE: RefCell<bool> = const { RefCell::new(false) };
}

/// True once, if the toggle was tapped since the last call.
pub fn take_toggle() -> bool {
    WANTS_TOGGLE.with(|w| {
        let v = *w.borrow();
        *w.borrow_mut() = false;
        v
    })
}

pub fn nav_tab() -> usize {
    NAV.with(|n| n.borrow().tab)
}
pub fn nav_route() -> Route {
    NAV.with(|n| n.borrow().route)
}

/// Apply a click. Returns true if it changed anything and a rebuild is needed.
pub fn handle(target: i32) -> bool {
    NAV.with(|n| {
        let mut nav = n.borrow_mut();
        match target {
            TOGGLE_IMPL => {
                WANTS_TOGGLE.with(|w| *w.borrow_mut() = true);
                false
            }
            BACK => {
                if nav.route == Route::Root {
                    false
                } else {
                    nav.route = Route::Root;
                    true
                }
            }
            LINK_MOMENTS => {
                nav.route = Route::Moments;
                true
            }
            LINK_MY_PROFILE => {
                nav.route = Route::MyProfile;
                true
            }
            LINK_ADD_CONTACT => {
                nav.route = Route::AddContact;
                true
            }
            t if (TAB_BASE..TAB_BASE + 4).contains(&t) => {
                let tab = (t - TAB_BASE) as usize;
                let changed = nav.tab != tab || nav.route != Route::Root;
                nav.tab = tab;
                nav.route = Route::Root;
                changed
            }
            t if t >= CHAT_BASE => {
                nav.route = Route::Chat((t - CHAT_BASE) as u64);
                true
            }
            _ => false,
        }
    })
}

// ---------------------------------------------------------------------------
// Leaves
// ---------------------------------------------------------------------------
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

/// A tappable row. `tap` is the event id ArkUI will hand back.
fn tap_row(w: f32, h: f32, bg: u32, tap: i32) -> Option<Node> {
    Some(row(w, h, bg)?.on_event(event::click(), tap))
}

/// Avatar: an Image node with a solid fill, since the reference app's PNGs are
/// not shipped here. Same node type and cost on both implementations.
fn avatar(size: f32) -> Option<Node> {
    bump();
    Some(
        Node::new(ty::image())?
            .width(size)
            .height(size)
            .bg(AVATAR_BG)
            .radius(size * 0.12),
    )
}

fn divider() -> Option<Node> {
    col(W, 1.0, DIVIDER)
}

// ---------------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------------

/// Header. On a pushed route it carries a back button, matching the reference
/// app's `WeChatNavigationView`.
fn header(title: &str, with_back: bool) -> Option<Node> {
    let mut h = row(W, 46.0, NAV_BG)?;
    if with_back {
        h = h.child(
            tap_row(56.0, 46.0, NAV_BG, BACK)?.child(text("‹", 24.0, TEXT, 44.0, 30.0)?),
        );
    } else {
        h = h.child(col(56.0, 46.0, NAV_BG)?);
    }
    h = h.child(text(title, 17.0, TEXT, W - 170.0, 26.0)?);
    // Right-hand action: "+" on the root, an implementation toggle elsewhere.
    h = h.child(
        tap_row(90.0, 46.0, NAV_BG, TOGGLE_IMPL)?.child(text("Rust ⇄", 12.0, GREEN, 84.0, 20.0)?),
    );
    Some(h)
}

/// The four-tab bar. Two nodes per tab plus the container, so 9.
fn tab_bar(active: usize) -> Option<Node> {
    let mut bar = row(W, 56.0, NAV_BG)?;
    for (i, label) in ["Chats", "Contacts", "Discover", "Me"].iter().enumerate() {
        let c = if i == active { GREEN } else { SUBTLE };
        let mut t = tap_row(W / 4.0, 56.0, NAV_BG, TAB_BASE + i as i32)?;
        t = t.child(text(label, 11.0, c, W / 4.0 - 4.0, 16.0)?);
        bar = bar.child(t);
    }
    Some(bar)
}

fn search_bar() -> Option<Node> {
    let mut s = row(W, 44.0, BG)?;
    let mut inner = row(W - 24.0, 32.0, SURFACE)?.radius(6.0);
    inner = inner.child(text("🔍  Search", 13.0, SUBTLE, 200.0, 20.0)?);
    s = s.child(inner);
    Some(s)
}

// ---------------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------------

/// `ChatPreview` from the reference app: avatar, name, message preview,
/// timestamp — and tapping it opens that chat.
fn chat_row(c: &Chat) -> Option<Node> {
    let mut r = tap_row(W, 66.0, SURFACE, CHAT_BASE + c.id as i32)?;
    r = r.child(avatar(44.0)?);
    let mut mid = col(W - 150.0, 60.0, SURFACE)?;
    mid = mid.child(text(c.username, 16.0, TEXT, W - 160.0, 22.0)?);
    mid = mid.child(text(c.preview.text(), 13.0, SUBTLE, W - 160.0, 20.0)?);
    r = r.child(mid);
    r = r.child(text(c.timestamp, 11.0, SUBTLE, 70.0, 18.0)?);
    Some(r)
}

/// A chat bubble. Outgoing sits right and green, incoming left and white,
/// as in the reference app's chat screen.
fn message_row(m: &Message) -> Option<Node> {
    let outgoing = m.direction == Direction::Outgoing;
    let mut r = row(W, 58.0, BG)?;
    if outgoing {
        r = r.child(col(W - 250.0, 50.0, BG)?);
    } else {
        r = r.child(avatar(38.0)?);
    }
    let mut bubble = col(200.0, 44.0, if outgoing { BUBBLE_OUT } else { SURFACE })?.radius(6.0);
    bubble = bubble.child(text(m.text, 15.0, TEXT, 188.0, 24.0)?);
    r = r.child(bubble);
    if outgoing {
        r = r.child(avatar(38.0)?);
    }
    Some(r)
}

/// A plain list row: icon, label, chevron.
fn menu_row(label: &str, tap: i32) -> Option<Node> {
    let mut r = tap_row(W, 52.0, SURFACE, tap)?;
    r = r.child(avatar(26.0)?);
    r = r.child(text(label, 15.0, TEXT, W - 110.0, 22.0)?);
    r = r.child(text("›", 16.0, SUBTLE, 24.0, 22.0)?);
    Some(r)
}

fn contact_row(name: &str) -> Option<Node> {
    let mut r = tap_row(W, 50.0, SURFACE, 0)?;
    r = r.child(avatar(34.0)?);
    r = r.child(text(name, 15.0, TEXT, W - 90.0, 22.0)?);
    Some(r)
}

fn section_label(s: &str) -> Option<Node> {
    let mut r = row(W, 26.0, BG)?;
    r = r.child(text(s, 12.0, SUBTLE, W - 20.0, 18.0)?);
    Some(r)
}

fn moment_row(author: &str, body: &str, photo: bool) -> Option<Node> {
    let mut m = row(W, if photo { 150.0 } else { 82.0 }, SURFACE)?;
    m = m.child(avatar(40.0)?);
    let mut c = col(W - 80.0, if photo { 140.0 } else { 72.0 }, SURFACE)?;
    c = c.child(text(author, 14.0, 0xFF576B95, W - 90.0, 20.0)?);
    c = c.child(text(body, 14.0, TEXT, W - 90.0, 22.0)?);
    if photo {
        c = c.child(avatar(80.0)?);
    }
    m = m.child(c);
    Some(m)
}

// ---------------------------------------------------------------------------
// Screens
// ---------------------------------------------------------------------------

fn screen_chats(body: Node) -> Option<Node> {
    let mut b = body;
    b = b.child(search_bar()?);
    for c in CHATS {
        b = b.child(chat_row(c)?);
        b = b.child(divider()?);
    }
    Some(b)
}

fn screen_contacts(body: Node) -> Option<Node> {
    let mut b = body;
    b = b.child(search_bar()?);
    for a in CONTACT_ACTIONS {
        b = b.child(menu_row(a, LINK_ADD_CONTACT)?);
    }
    for (initial, names) in CONTACT_GROUPS {
        b = b.child(section_label(initial)?);
        for n in *names {
            b = b.child(contact_row(n)?);
        }
    }
    Some(b)
}

fn screen_discover(body: Node) -> Option<Node> {
    let mut b = body;
    for group in DISCOVER_GROUPS {
        for entry in *group {
            let tap = if *entry == "Moments" { LINK_MOMENTS } else { 0 };
            b = b.child(menu_row(entry, tap)?);
        }
        b = b.child(col(W, 10.0, BG)?);
    }
    Some(b)
}

fn screen_me(body: Node) -> Option<Node> {
    let mut b = body;
    // Profile header, tappable into My Profile.
    let mut head = tap_row(W, 116.0, SURFACE, LINK_MY_PROFILE)?;
    head = head.child(avatar(64.0)?);
    let mut hc = col(W - 160.0, 80.0, SURFACE)?;
    hc = hc.child(text("Rik Arends", 20.0, TEXT, W - 170.0, 28.0)?);
    hc = hc.child(text("WeChat ID: rikarends", 12.0, SUBTLE, W - 170.0, 20.0)?);
    hc = hc.child(text("＋ Status", 12.0, SUBTLE, W - 170.0, 20.0)?);
    head = head.child(hc);
    head = head.child(text("›", 16.0, SUBTLE, 24.0, 22.0)?);
    b = b.child(head);
    b = b.child(col(W, 10.0, BG)?);
    for group in PROFILE_GROUPS {
        for entry in *group {
            let tap = if *entry == "Moments" { LINK_MOMENTS } else { 0 };
            b = b.child(menu_row(entry, tap)?);
        }
        b = b.child(col(W, 10.0, BG)?);
    }
    Some(b)
}

fn screen_chat(body: Node, chat_id: u64) -> Option<Node> {
    let mut b = body;
    for i in 0..MESSAGES_PER_CHAT {
        b = b.child(message_row(&message(chat_id, i))?);
    }
    // Composer, as in the reference app.
    let mut c = row(W, 52.0, NAV_BG)?;
    c = c.child(avatar(28.0)?);
    c = c.child(col(W - 130.0, 34.0, SURFACE)?.radius(5.0));
    c = c.child(text("😊", 18.0, TEXT, 30.0, 26.0)?);
    c = c.child(text("＋", 18.0, TEXT, 30.0, 26.0)?);
    b = b.child(c);
    Some(b)
}

fn screen_moments(body: Node) -> Option<Node> {
    let mut b = body;
    b = b.child(avatar(120.0)?); // hero banner
    for (author, text_, photo) in MOMENTS {
        b = b.child(moment_row(author, text_, *photo)?);
        b = b.child(divider()?);
    }
    Some(b)
}

fn screen_add_contact(body: Node) -> Option<Node> {
    let mut b = body;
    b = b.child(search_bar()?);
    b = b.child(section_label("My WeChat ID: rikarends")?);
    for entry in ["Scan", "Mobile Contacts", "Official Accounts", "WeCom Contacts"] {
        b = b.child(menu_row(entry, 0)?);
    }
    Some(b)
}

fn screen_my_profile(body: Node) -> Option<Node> {
    let mut b = body;
    for entry in ["Profile Photo", "Name", "WeChat ID", "My QR Code", "More"] {
        b = b.child(menu_row(entry, 0)?);
    }
    Some(b)
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

/// Build the whole app for the current navigation state.
/// Returns (root, nodes built, µs).
pub fn build() -> (Option<Node>, usize, f64) {
    COUNT.with(|c| *c.borrow_mut() = 0);
    let t0 = Instant::now();
    let (tab, route) = NAV.with(|n| {
        let n = n.borrow();
        (n.tab, n.route)
    });

    let built = (|| -> Option<Node> {
        let mut root = col(W, PAGE_H, BG)?;

        let (title, with_back) = match route {
            Route::Root => (["WeChat", "Contacts", "Discover", "Me"][tab], false),
            Route::Chat(id) => (chat(id).map(|c| c.username).unwrap_or("Chat"), true),
            Route::Moments => ("Moments", true),
            Route::AddContact => ("Add Contact", true),
            Route::MyProfile => ("My Profile", true),
        };
        root = root.child(header(title, with_back)?);

        // Scrolling body.
        // ARKUI_ALIGNMENT_TOP. A Scroll defaults to centring content that is
        // shorter than itself, which drops the page half way down the screen.
        let scroll = Node::new(ty::scroll())?
            .width(W)
            .height(PAGE_H - 46.0 - 56.0)
            .i32_attr(attr::alignment(), 1);
        bump();
        // Height is left to the content: a fixed height here either clips the
        // list or leaves the scroll with nothing to scroll.
        let body = col(W, 0.0, BG)?;

        let body = match route {
            Route::Root => match tab {
                0 => screen_chats(body)?,
                1 => screen_contacts(body)?,
                2 => screen_discover(body)?,
                _ => screen_me(body)?,
            },
            Route::Chat(id) => screen_chat(body, id)?,
            Route::Moments => screen_moments(body)?,
            Route::AddContact => screen_add_contact(body)?,
            Route::MyProfile => screen_my_profile(body)?,
        };
        root = root.child(scroll.child(body));

        // The reference app hides the tab bar on pushed routes.
        if route == Route::Root {
            root = root.child(tab_bar(tab)?);
        } else {
            root = root.child(col(W, 56.0, NAV_BG)?);
        }
        Some(root)
    })();

    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    let n = COUNT.with(|c| *c.borrow());
    (built, n, us)
}

/// Build without keeping the result — for timing only.
pub fn build_timed(tab: usize, route: Route) -> (usize, f64) {
    let saved = NAV.with(|n| {
        let old = *n.borrow();
        *n.borrow_mut() = Nav { tab, route };
        old
    });
    let (node, n, us) = build();
    drop(node);
    NAV.with(|n| *n.borrow_mut() = saved);
    (n, us)
}

impl Clone for Nav {
    fn clone(&self) -> Self {
        Nav {
            tab: self.tab,
            route: self.route,
        }
    }
}
impl Copy for Nav {}
