//! A browser card: native Splash chrome around an ArkTS `Web` surface.
//!
//! This exists to exercise `webslot` rather than to be a browser. The point it
//! proves is that a Splash tree can contain a web view *at a position the DSL
//! chooses*, with a URL the DSL chooses, alongside native widgets — rather than
//! the previous arrangement, where a build-time flag swapped the whole page for
//! a hardcoded `Web` component and the DSL only got a 56 px strip above it.
//!
//! The tab bar and address bar are real ArkUI nodes built by Rust. The page
//! body is a hole the ArkTS overlay fills. Tapping a tab changes which URL the
//! hole loads, which is the part that was not previously possible.

use crate::webslot::{web, web_html};
use splash_oh_arkui::arkui::Node;
use splash_oh_arkui::ui::*;

const CHROME: u32 = 0xFFF7F7F7;
const TEXT: u32 = 0xFF1A1A1A;
const SUBTLE: u32 = 0xFF8A8A8E;
const ACCENT: u32 = 0xFF0A84FF;
const BG: u32 = 0xFFFFFFFF;

pub const TAB_BASE: i32 = 500;
pub const RELOAD: i32 = 510;

/// The sites the card can show. Chosen to be light and to render sensibly in a
/// webview at phone width.
pub const TABS: &[(&str, &str)] = &[
    ("Wikipedia", "https://en.m.wikipedia.org/wiki/OpenHarmony"),
    ("Hacker News", "https://news.ycombinator.com/"),
    (
        "YouTube",
        "https://www.youtube.com/embed/jNQXAC9IVRw?autoplay=1&mute=1&playsinline=1",
    ),
    ("Example", "https://example.com/"),
    // A page served from the HAP, loaded into a URL-kind slot so the gate
    // treats it as untrusted. It reports whether an untrusted surface can
    // reach splash_native -- turning "verified by construction" into
    // "verified on device".
    ("Gate probe", "resource://rawfile/probe/bridge-probe.html"),
];

/// Chrome height above the web surface: status strip + address bar + tabs.
const ADDR_H: f32 = 44.0;
const TABS_H: f32 = 40.0;

fn address_bar(url: &str) -> Option<Node> {
    let mut r = row(W, ADDR_H, CHROME)?;
    let mut field = row(W - 80.0, 30.0, 0xFFE8E8ED)?.radius(8.0);
    // Show the host rather than the whole URL, as a phone browser does.
    let host = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url);
    field = field.child(text(host, 12.0, TEXT, W - 100.0, 18.0)?);
    r = r.child(field);
    r = r.child(tap_row(56.0, ADDR_H, CHROME, RELOAD)?.child(text("↻", 17.0, ACCENT, 44.0, 24.0)?));
    Some(r)
}

fn tab_strip(active: usize) -> Option<Node> {
    let mut bar = row(W, TABS_H, CHROME)?;
    for (i, (label, _)) in TABS.iter().enumerate() {
        let c = if i == active { ACCENT } else { SUBTLE };
        let mut t = tap_row(W / TABS.len() as f32, TABS_H, CHROME, TAB_BASE + i as i32)?;
        t = t.child(text(label, 11.0, c, W / TABS.len() as f32 - 4.0, 16.0)?);
        bar = bar.child(t);
    }
    Some(bar)
}

/// `tab` selects which site the web surface loads.
pub fn build(tab: usize) -> Option<Node> {
    let tab = tab.min(TABS.len() - 1);
    let (_, url) = TABS[tab];

    let mut root = col(W, PAGE_H, BG)?;
    root = root.child(address_bar(url)?);
    root = root.child(tab_strip(tab)?);

    // The hole. Its y is the chrome above it, which the DSL knows because it
    // sized that chrome itself.
    let top = ADDR_H + TABS_H;
    root = root.child(web(url, 0.0, top, W, PAGE_H - top)?);

    Some(root)
}
