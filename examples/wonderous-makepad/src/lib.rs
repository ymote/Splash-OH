//! Wonderous, ported from
//! [project-robius/makepad_wonderous](https://github.com/project-robius/makepad_wonderous).
//!
//! The photo-heavy end of the four apps: a full-bleed parallax header, an
//! editorial body, a two-column artifact grid and a gallery. Where TikTok has
//! few nodes and one big image, and WeChat has many nodes and small ones, this
//! has a moderate node count with large images — a third point on the curve.
//!
//! The reference app's structure is preserved: a wonder screen with
//! `before_content_header` / `content_header` / `content_sections`, an
//! artifacts screen whose `GridImage` is a `RoundedView` wrapping an `Image`
//! (two nodes each), and a gallery.
//!
//! What is deliberately **not** ported is the parallax itself. The reference
//! app animates the header against scroll offset with a shader; there is no
//! equivalent native node, and faking it would measure the fake. The static
//! composition is built instead, and the tab is honest about that.

use splash_oh_arkui::arkui::Node;
use splash_oh_arkui::ui::*;

const APP: &str = "wonderous";

// The reference app's palette: dark editorial with a bronze accent.
const BG: u32 = 0xFF1B1B1B;
const SURFACE: u32 = 0xFF262626;
const CREAM: u32 = 0xFFEDE9E1;
const BRONZE: u32 = 0xFFC0A062;
const SUBTLE: u32 = 0xFF9A948A;

pub const TAB_BASE: i32 = 400;
pub const ART_BASE: i32 = 4000;
pub const BACK: i32 = 410;

/// The wonders the reference app ships flattened images for.
pub const WONDERS: &[(&str, &str, &str)] = &[
    (
        "Great Wall of China",
        "great-wall-flattened.jpg",
        "China · 700 BCE",
    ),
    ("Taj Mahal", "taj-mahal-flattened.jpg", "India · 1632 CE"),
    (
        "Machu Picchu",
        "machu-picchu-flattened.jpg",
        "Peru · 1450 CE",
    ),
    ("Petra", "petra-flattened.jpg", "Jordan · 312 BCE"),
];

pub const TABS: &[(&str, &str)] = &[
    ("Editorial", "tab-editorial-active.png"),
    ("Photos", "tab-photos-active.png"),
    ("Artifacts", "tab-artifacts-active.png"),
    ("Timeline", "tab-timeline-active.png"),
];

/// Editorial body, as the reference app's `content_sections`.
const SECTIONS: &[(&str, &str)] = &[
    ("History", "Constructed over two millennia, the wall was never a single continuous structure but a network of walls and fortifications built by successive dynasties."),
    ("Construction", "Rammed earth gave way to brick and stone under the Ming, whose sections are the ones most visitors see today."),
    ("Location", "It runs roughly east to west across the historical northern borders of China, from Liaoning to Gansu."),
    ("Legacy", "Inscribed as a UNESCO World Heritage Site in 1987, and among the most recognised structures ever built."),
];

/// Artifact grid entries. `GridImage` in the reference app is a rounded
/// container wrapping an image — two nodes each.
const ARTIFACTS: &[&str] = &[
    "great-wall-1.jpg",
    "great-wall-2.jpg",
    "great-wall-3.jpg",
    "great-wall-4.jpg",
    "great-wall-5.jpg",
    "great-wall-1.jpg",
    "great-wall-2.jpg",
    "great-wall-3.jpg",
];

/// The full-bleed header the reference app parallaxes. Static here — see the
/// module note.
fn hero(wonder: usize) -> Option<Node> {
    let (title, img, place) = WONDERS[wonder];
    let mut s = stack(W, 320.0, BG)?;
    s = s.child(photo(APP, img, W, 320.0, 0.0)?);
    let mut overlay = col(W, 320.0, 0)?;
    overlay = overlay.child(spacer(W, 200.0)?);
    overlay = overlay.child(text_w(title, 26.0, CREAM, W - 40.0, 36.0, 7)?);
    overlay = overlay.child(text(place, 12.0, BRONZE, W - 40.0, 20.0)?);
    s = s.child(overlay);
    Some(s)
}

fn tab_bar(active: usize) -> Option<Node> {
    let mut bar = row(W, 56.0, SURFACE)?;
    for (i, (label, file)) in TABS.iter().enumerate() {
        let c = if i == active { BRONZE } else { SUBTLE };
        let mut t = tap_col(W / 4.0, 56.0, SURFACE, TAB_BASE + i as i32)?;
        t = t.child(icon(APP, file, 20.0)?);
        t = t.child(text(label, 10.0, c, W / 4.0 - 2.0, 14.0)?);
        bar = bar.child(t);
    }
    Some(bar)
}

/// A section separator: the reference app draws a small ornament between them.
fn separator() -> Option<Node> {
    let mut r = row(W, 30.0, BG)?;
    r = r.child(divider(120.0, BRONZE)?);
    Some(r)
}

fn editorial(body: Node, wonder: usize) -> Option<Node> {
    let mut b = body;
    b = b.child(hero(wonder)?);
    for (heading, para) in SECTIONS {
        b = b.child(separator()?);
        b = b.child(text_w(heading, 15.0, BRONZE, W - 40.0, 24.0, 7)?);
        // ~46 chars a line at this width and size.
        let lines = (para.len() as f32 / 46.0).ceil().max(1.0);
        b = b.child(text(para, 13.0, CREAM, W - 40.0, lines * 20.0 + 8.0)?);
    }
    b = b.child(spacer(W, 20.0)?);
    Some(b)
}

/// The photo tab: a stack of large images, the heaviest screen of the four apps.
fn photos(body: Node) -> Option<Node> {
    let mut b = body;
    for (_, img, caption) in WONDERS {
        b = b.child(photo(APP, img, W, 220.0, 6.0)?);
        b = b.child(text(caption, 11.0, SUBTLE, W - 40.0, 18.0)?);
        b = b.child(spacer(W, 10.0)?);
    }
    Some(b)
}

/// The artifacts grid: two columns of `GridImage`.
fn artifacts(body: Node) -> Option<Node> {
    let mut b = body;
    let cw = (W - 24.0) / 2.0;
    let mut i = 0usize;
    while i < ARTIFACTS.len() {
        let mut r = row(W, 150.0, BG)?;
        for k in 0..2 {
            if i + k >= ARTIFACTS.len() {
                break;
            }
            // RoundedView wrapping an Image, as GridImage is.
            let mut cell = tap_col(cw, 142.0, SURFACE, ART_BASE + (i + k) as i32)?.radius(8.0);
            cell = cell.child(photo(APP, ARTIFACTS[i + k], cw, 142.0, 8.0)?);
            r = r.child(cell);
        }
        b = b.child(r);
        i += 2;
    }
    Some(b)
}

/// The timeline tab: dated entries down a rule.
const TIMELINE: &[(&str, &str)] = &[
    ("700 BCE", "First walls raised by the state of Chu"),
    ("221 BCE", "Qin Shi Huang links the northern walls"),
    ("130 BCE", "Han dynasty extends westward"),
    ("1368 CE", "Ming dynasty begins the brick sections"),
    ("1644 CE", "Construction ends with the Qing"),
    ("1987 CE", "Inscribed by UNESCO"),
];

fn timeline(body: Node) -> Option<Node> {
    let mut b = body;
    for (year, event) in TIMELINE {
        let mut r = row(W, 60.0, BG)?;
        r = r.child(text_w(year, 12.0, BRONZE, 76.0, 20.0, 7)?);
        r = r.child(divider(1.0, SUBTLE)?);
        r = r.child(text(event, 13.0, CREAM, W - 110.0, 40.0)?);
        b = b.child(r);
    }
    Some(b)
}

/// Exposed so the ArkTS twin renders the same copy.
pub fn sections() -> &'static [(&'static str, &'static str)] {
    SECTIONS
}
pub fn artifacts_list() -> &'static [&'static str] {
    ARTIFACTS
}
pub fn timeline_list() -> &'static [(&'static str, &'static str)] {
    TIMELINE
}

pub fn build(tab: usize, wonder: usize) -> Option<Node> {
    let mut root = col(W, PAGE_H, BG)?;
    let mut h = row(W, 46.0, BG)?;
    h = h.child(tap_row(56.0, 46.0, BG, BACK)?.child(text("‹", 22.0, CREAM, 44.0, 28.0)?));
    h = h.child(text_w(WONDERS[wonder].0, 15.0, CREAM, W - 120.0, 22.0, 5)?);
    root = root.child(h);

    let body = col(W, 0.0, BG)?;
    let body = match tab {
        0 => editorial(body, wonder)?,
        1 => photos(body)?,
        2 => artifacts(body)?,
        _ => timeline(body)?,
    };
    root = root.child(scroll(PAGE_H - 46.0 - 56.0)?.child(body));
    root = root.child(tab_bar(tab)?);
    Some(root)
}
