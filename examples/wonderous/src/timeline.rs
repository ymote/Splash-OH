//! The global timeline: all eight wonders on one scale.
//!
//! `wonders_timeline_screen.dart` draws every wonder as a bar spanning its
//! construction years, from the earliest start to the latest end, so the reader
//! can see that Giza is finished two thousand years before Petra begins.

use super::data::WONDERS;
use super::timeline_data::TIMELINES;
use splash_oh_native::arkui::{attr, Node};
use splash_oh_native::ui::*;

const APP: &str = "wonders";
const SHEET: u32 = 0xFFF8ECE5;
const GREY_STRONG: u32 = 0xFF272625;
const ACCENT: u32 = 0xFFE4935D;
const DISPLAY: &str = "YesevaOne";
const SERIF_UI: &str = "TenorSans";
const BODY_FONT: &str = "Raleway";

pub const TIMELINE_CLOSE: i32 = 7360;

/// A year as the app writes it: negative years are BCE.
fn year_label(y: i32) -> String {
    if y < 0 {
        format!("{} BCE", -y)
    } else {
        format!("{y} CE")
    }
}

pub fn build(current: usize, w: f32, h: f32) -> Option<Node> {
    let lo = TIMELINES.iter().map(|t| t.start_yr).min().unwrap_or(-2600);
    let hi = TIMELINES.iter().map(|t| t.end_yr).max().unwrap_or(2000);
    let span = (hi - lo).max(1) as f32;

    let mut root = stack(w, h, GREY_STRONG)?;
    root = root.child(
        text("TIMELINE", 13.0, ACCENT, w, 26.0)?
            .string_attr(attr::font_family(), SERIF_UI)
            .i32_attr(attr::text_align(), 1)
            .f32v_attr(attr::position(), &[0.0, h * 0.055]),
    );
    root = root.child(
        text(
            &format!("{} — {}", year_label(lo), year_label(hi)),
            13.0,
            0x99F8ECE5,
            w,
            24.0,
        )?
        .string_attr(attr::font_family(), BODY_FONT)
        .i32_attr(attr::text_align(), 1)
        .f32v_attr(attr::position(), &[0.0, h * 0.095]),
    );

    // One row per wonder: portrait, name, and a bar across the shared scale.
    let top = h * 0.16;
    let row_h = 68.0;
    let track_x = 96.0;
    let track_w = w - track_x - 24.0;

    for (i, wo) in WONDERS.iter().enumerate() {
        let t = &TIMELINES[i];
        let y = top + row_h * i as f32;
        let x0 = track_x + (t.start_yr - lo) as f32 / span * track_w;
        let x1 = track_x + (t.end_yr - lo) as f32 / span * track_w;
        let bar_w = (x1 - x0).max(3.0);
        let on = i == current % WONDERS.len();

        root = root.child(
            photo(APP, &format!("{}/button.png", wo.dir), 34.0, 34.0, 17.0)?
                .f32v_attr(attr::position(), &[20.0, y + 6.0]),
        );
        root = root.child(
            text(
                wo.title,
                11.0,
                if on { SHEET } else { 0x99F8ECE5 },
                track_x - 26.0,
                18.0,
            )?
            .string_attr(attr::font_family(), BODY_FONT)
            .f32v_attr(attr::position(), &[20.0, y + 44.0]),
        );
        // The track, then this wonder's span on it.
        root = root.child(
            col(track_w, 2.0, 0x22F8ECE5)?.f32v_attr(attr::position(), &[track_x, y + 22.0]),
        );
        root = root.child(
            col(bar_w, 10.0, if on { ACCENT } else { wo.fg })?
                .radius(5.0)
                .f32v_attr(attr::position(), &[x0, y + 18.0]),
        );
        root = root.child(
            text(&year_label(t.start_yr), 10.0, 0x8CF8ECE5, 90.0, 16.0)?
                .string_attr(attr::font_family(), BODY_FONT)
                .f32v_attr(attr::position(), &[x0.min(w - 96.0), y + 34.0]),
        );
    }

    root = root.child(
        stack(46.0, 46.0, 0x33F8ECE5)?
            .radius(23.0)
            .f32v_attr(attr::position(), &[w - 66.0, h * 0.04])
            .child(icon(APP, "_common/icons/icon-close.png", 20.0)?),
    );
    root = root.child(super::hits(
        w,
        h,
        &[(w - 76.0, h * 0.03, 66.0, 66.0, TIMELINE_CLOSE)],
    )?);
    Some(root)
}

/// One wonder's events, as the details screen's fourth tab lists them: a year
/// and a line, down a rule.
pub fn events(index: usize, w: f32, h: f32) -> Option<Node> {
    let wo = &WONDERS[index % WONDERS.len()];
    let t = &TIMELINES[index % TIMELINES.len()];
    let pad = 24.0;
    let cw = w - pad * 2.0;

    let mut inner = col(cw, 0.0, 0x00000000)?;
    let mut total = 0.0f32;

    let head = 96.0;
    inner = inner.child(
        col(cw, head, 0x00000000)?
            .child(text(wo.title, 26.0, SHEET, cw, 40.0)?.string_attr(attr::font_family(), DISPLAY))
            .child(
                text(
                    &format!("{} — {}", year_label(t.start_yr), year_label(t.end_yr)),
                    13.0,
                    ACCENT,
                    cw,
                    24.0,
                )?
                .string_attr(attr::font_family(), SERIF_UI),
            ),
    );
    total += head;

    for ev in t.events {
        // Two lines of text at 14 pt over this width is the common case; the
        // longer entries wrap to three.
        let lines = (ev.text.chars().count() as f32 / (cw * 2.163 * 0.94 / 14.0)).ceil();
        let th = lines * 14.0 * 1.5 + 8.0;
        let block = th + 46.0;
        let mut r = row(cw, block, 0x00000000)?;
        r = r.child(col(2.0, block - 12.0, 0x44E4935D)?);
        let mut c =
            col(cw - 18.0, block, 0x00000000)?.f32v_attr(attr::padding(), &[0.0, 0.0, 0.0, 16.0]);
        c = c.child(
            text(&year_label(ev.year), 12.0, ACCENT, cw - 18.0, 22.0)?
                .string_attr(attr::font_family(), SERIF_UI),
        );
        c = c.child(
            text(ev.text, 14.0, 0xCCF8ECE5, cw - 18.0, th)?
                .string_attr(attr::font_family(), BODY_FONT),
        );
        r = r.child(c);
        inner = inner.child(r);
        total += block;
    }

    let inner = inner.height(total);
    let mut s = Node::new(splash_oh_native::arkui::ty::scroll())?
        .width(w)
        .height(h)
        .i32_attr(attr::alignment(), 1);
    s = s.child(
        col(w, total + 76.0, wo.bg)?
            .f32v_attr(attr::padding(), &[36.0, pad, 40.0, pad])
            .child(inner),
    );
    Some(s)
}
