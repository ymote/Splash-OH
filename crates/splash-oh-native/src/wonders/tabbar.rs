//! The bottom bar of the wonder-details screens.
//!
//! A circular portrait of the current wonder on the left, then four tabs —
//! editorial, photos, artifacts, timeline. The portrait doubles as the way back
//! out to the home screen.
//!
//! Reproduces `wonder_details_tab_menu.dart`. The bar sits on the wonder's own
//! `bgColor`, and the selected tab is drawn in `accent1` where the rest are the
//! same colour at half strength.

use super::data::Wonder;
use crate::arkui::{attr, Node};
use crate::ui::*;

const APP: &str = "wonders";
/// `$styles.colors.accent1`.
const ACCENT: u32 = 0xFFE4935D;
const IDLE: u32 = 0x99E4935D;

pub const TAB_BASE: i32 = 7200;
pub const HOME_TAP: i32 = 7210;

/// The four tabs, in the order the app lists them, with their icons.
pub const TABS: &[(&str, &str)] = &[
    ("Information", "icon-info.png"),
    ("Photos", "icon-wallpaper.png"),
    ("Artifacts", "icon-collection.png"),
    ("Events", "icon-timeline.png"),
];

pub fn height() -> f32 {
    72.0
}

pub fn build(wonder: &Wonder, active: usize, w: f32) -> Option<Node> {
    let h = height();
    let mut bar = row(w, h, wonder.bg)?;

    // The portrait button. `wonder-button.png` is the app's own circular crop;
    // scaling the full illustration down to 48 px gives a different image.
    let d = 52.0;
    let mut portrait = stack(d + 20.0, h, 0x00000000)?;
    portrait = portrait.child(
        photo(APP, &format!("{}/button.png", wonder.dir), d, d, d / 2.0)?
            .f32v_attr(attr::position(), &[14.0, (h - d) / 2.0]),
    );
    // No click here. The screen's single overlay owns every tap; registering
    // them twice put two live targets over the same pixels, and which one won
    // varied by cell -- three tabs responded and photos did not.
    bar = bar.child(portrait);

    let cell = (w - d - 20.0) / TABS.len() as f32;
    for (i, (_, glyph)) in TABS.iter().enumerate() {
        let mut c = col(cell, h, 0x00000000)?;
        let on = i == active;
        c = c.child(
            icon(APP, &format!("_common/icons/{glyph}"), 26.0)?
                .f32v_attr(attr::position(), &[(cell - 26.0) / 2.0, h * 0.28]),
        );
        if on {
            // The selected tab carries a short underline in accent1.
            c = c.child(
                col(22.0, 2.0, ACCENT)?
                    .radius(1.0)
                    .f32v_attr(attr::position(), &[(cell - 22.0) / 2.0, h * 0.72]),
            );
        }
        let _ = if on { ACCENT } else { IDLE };
        bar = bar.child(c);
    }
    Some(bar)
}
