//! The widget catalog — an OpenHarmony analogue of the Android "catalog"
//! sample: one screen per widget family, every one of them a real ArkUI
//! component created from Rust.
//!
//! This is deliberately written against the same vocabulary the Splash DSL
//! uses (containers, text, controls), so the same tree shape can later be
//! produced by the VM instead of by hand — see README "Where the DSL plugs in".

include!(concat!(env!("OUT_DIR"), "/catalog_screens.rs"));

use splash_oh_arkui::arkui::{attr, event, ty, Node};

// Apple-ish neutral palette, ARGB.
const INK: u32 = 0xFF11_1418;
const DIM: u32 = 0xFF6E_6E73;
const LINE: u32 = 0xFFE5_E5EA;
const CARD: u32 = 0xFFFF_FFFF;
const PAGE: u32 = 0xFFF2_F2F7;
const TINT: u32 = 0xFF0A_7CFF;

/// ArkUI takes vp, not a "match parent" sentinel. The device is ~406vp wide;
/// this keeps cards full-bleed without needing percentage support in the shim.
const FULL: f32 = 402.0;
const PAGE_H: f32 = 3000.0;

/// Event ids, so the receiver can tell which control fired.
pub mod ev {
    pub const BUTTON_TAP: i32 = 1;
    pub const TOGGLE: i32 = 2;
    pub const SLIDER: i32 = 3;
    pub const CHECKBOX: i32 = 4;
    pub const TEXT_CHANGED: i32 = 5;
}

fn title(s: &str) -> Option<Node> {
    Some(
        Node::new(ty::text())?
            .text(s)
            .font_size(28.0)
            .font_weight(7)
            .font_color(INK)
            .margin(0.0)
            .f32v_attr(attr::padding(), &[18.0, 18.0, 4.0, 18.0]),
    )
}

fn section(s: &str) -> Option<Node> {
    Some(
        Node::new(ty::text())?
            .text(s)
            .font_size(13.0)
            .font_weight(6)
            .font_color(DIM)
            .f32v_attr(attr::padding(), &[18.0, 18.0, 6.0, 18.0]),
    )
}

/// A rounded white card that groups one widget family.
fn card(children: Vec<Node>) -> Option<Node> {
    let mut n = Node::new(ty::column())?
        .width(FULL)
        .bg(CARD)
        .radius(14.0)
        .padding(14.0)
        .f32v_attr(attr::margin(), &[10.0, 14.0, 6.0, 14.0])
        .f32v_attr(attr::border_width(), &[1.0, 1.0, 1.0, 1.0])
        .u32_attr(attr::border_color(), LINE);
    for c in children {
        n = n.child(c);
    }
    Some(n)
}

fn caption(s: &str) -> Option<Node> {
    Some(
        Node::new(ty::text())?
            .text(s)
            .font_size(12.0)
            .font_color(DIM)
            .f32v_attr(attr::padding(), &[0.0, 0.0, 8.0, 0.0]),
    )
}

/// Build the whole catalog tree. Every node here is a native ArkUI component.
pub fn build() -> Option<Node> {
    let mut root = Node::new(ty::scroll())?.width(FULL).bg(PAGE);

    let mut col = Node::new(ty::column())?.width(FULL);

    col = col.child(title("Splash-OH")?);
    col = col.child(
        Node::new(ty::text())?
            .text("Every widget below is a native ArkUI component created from Rust. No ArkTS.")
            .font_size(13.0)
            .font_color(DIM)
            .f32v_attr(attr::padding(), &[0.0, 18.0, 10.0, 18.0]),
    );

    // ---- text ----
    col = col.child(section("TEXT")?);
    col = col.child(card(vec![
        caption("ARKUI_NODE_TEXT — size, weight, colour")?,
        Node::new(ty::text())?
            .text("The quick brown fox")
            .font_size(22.0)
            .font_weight(7)
            .font_color(INK),
        Node::new(ty::text())?
            .text("Regular body text at 15sp")
            .font_size(15.0)
            .font_color(INK),
        Node::new(ty::text())?
            .text("Tinted caption")
            .font_size(13.0)
            .font_color(TINT),
    ])?);

    // ---- buttons ----
    col = col.child(section("BUTTON")?);
    col = col.child(card(vec![
        caption("ARKUI_NODE_BUTTON — tap is delivered to Rust")?,
        Node::new(ty::button())?
            .label("Tap me")
            .height(44.0)
            .bg(TINT)
            .radius(10.0)
            .font_color(0xFFFF_FFFF)
            .on_event(event::click(), ev::BUTTON_TAP),
    ])?);

    // ---- selection controls ----
    col = col.child(section("SELECTION")?);
    col = col.child(card(vec![
        caption("ARKUI_NODE_TOGGLE / CHECKBOX / RADIO")?,
        Node::new(ty::toggle())?
            .height(32.0)
            .on_event(event::click(), ev::TOGGLE),
        Node::new(ty::checkbox())?
            .height(32.0)
            .on_event(event::click(), ev::CHECKBOX),
        Node::new(ty::radio())?.height(32.0),
    ])?);

    // ---- value controls ----
    col = col.child(section("VALUE")?);
    col = col.child(card(vec![
        caption("ARKUI_NODE_SLIDER / PROGRESS / LOADING_PROGRESS")?,
        Node::new(ty::slider())?
            .height(40.0)
            .on_event(event::click(), ev::SLIDER),
        Node::new(ty::progress())?
            .height(24.0)
            .f32_attr(attr::progress_value(), 62.0)
            .f32_attr(attr::progress_total(), 100.0),
        Node::new(ty::loading())?.height(36.0),
    ])?);

    // ---- input ----
    col = col.child(section("INPUT")?);
    col = col.child(card(vec![
        caption("ARKUI_NODE_TEXT_INPUT / TEXT_AREA — real IME, natively wired")?,
        Node::new(ty::input())?
            .string_attr(attr::input_placeholder(), "Type here…")
            .height(44.0)
            .bg(PAGE)
            .radius(10.0)
            .padding(10.0)
            .on_event(event::click(), ev::TEXT_CHANGED),
        Node::new(ty::textarea())?
            .height(80.0)
            .bg(PAGE)
            .radius(10.0)
            .padding(10.0)
            .margin(8.0),
    ])?);

    // ---- layout containers ----
    col = col.child(section("LAYOUT")?);
    col = col.child(card(vec![
        caption("ARKUI_NODE_ROW — three children, evenly spaced")?,
        Node::new(ty::row())?
            .height(52.0)
            .child(swatch(0xFFFF_3B30)?)
            .child(swatch(0xFF34_C759)?)
            .child(swatch(0xFF0A_7CFF)?),
        caption("ARKUI_NODE_STACK — overlapping children")?,
        Node::new(ty::stack())?
            .height(64.0)
            .child(
                Node::new(ty::column())?
                    .width(120.0)
                    .height(56.0)
                    .bg(0xFFD1_D1D6)
                    .radius(10.0),
            )
            .child(
                Node::new(ty::text())?
                    .text("on top")
                    .font_size(13.0)
                    .font_color(INK),
            ),
    ])?);

    // ---- pickers ----
    col = col.child(section("PICKERS")?);
    col = col.child(card(vec![
        caption("ARKUI_NODE_DATE_PICKER — a full system picker")?,
        Node::new(ty::datepicker())?.height(160.0),
    ])?);

    // Breathing room under the last card.
    col = col.child(Node::new(ty::column())?.height(28.0));

    root = root.child(col);
    Some(root)
}

fn swatch(argb: u32) -> Option<Node> {
    Some(
        Node::new(ty::column())?
            .width(64.0)
            .height(40.0)
            .bg(argb)
            .radius(8.0)
            .margin(6.0),
    )
}
