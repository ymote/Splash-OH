//! The catalog list: every flutter/samples directory, and what it is here.
//!
//! Two sections, not one, and the split is the point. An earlier version of this
//! list drew a check beside all twenty-seven directories while three of them
//! worked — an outside review counted eighteen that were the sample transcribed
//! accurately with nothing behind it. A list that cannot say "drawn but inert"
//! will eventually claim it is finished, so the shape here makes the claim
//! explicit and the count is derived from the data rather than typed in.

use super::*;

pub fn build() -> Option<Node> {
    let responds = SCREENS.iter().filter(|s| s.responds).count();
    let notes = SCREENS.len() - responds;

    let mut page = col_fit(W(), SURFACE)?;

    // Header.
    let mut head = col_fit(W(), SURF_CONT)?.padding(16.0);
    head = head.child(text("flutter/samples", 28.0, 400, ON_SURFACE, W() - 32.0, 40.0)?);
    head = head.child(para(
        &format!(
            "All {} directories have a screen. {responds} respond — live data, \
             motion, or controls that change what you see. {notes} are notes: \
             the sample is configuration or prose, with no UI to port.",
            SCREENS.len()
        ),
        14.0,
        ON_SURF_VAR,
        W() - 32.0,
    )?);
    page = page.child(head);

    let body = scroll_rest()?;
    let mut list = col_fit(W(), SURFACE)?.padding(16.0);

    // Counters, from the table above rather than typed in — the old list said
    // "27 of 27" in text that no longer matched what the rows showed.
    let mut counts = row(W() - 32.0, 76.0, SURFACE)?;
    counts = counts.child(count_card(&responds.to_string(), "respond", PRI_CONT, ON_PRI_CONT)?);
    counts = counts.child(gap_w(8.0)?);
    counts = counts.child(count_card(&notes.to_string(), "notes", SURF_CONT, ON_SURF_VAR)?);
    list = list.child(counts);
    list = list.child(gap(16.0)?);

    list = list.child(group(
        "Responds — data, motion, or a control that works",
        true,
    )?);
    list = list.child(gap(16.0)?);
    list = list.child(group("Notes, not ports", false)?);

    Some(page.child(body.child(list)))
}

fn gap_w(w: f32) -> Option<Node> {
    col(w, 1.0, 0)
}

fn count_card(n: &str, label: &str, bg: u32, ink: u32) -> Option<Node> {
    let w = (W() - 40.0) / 2.0;
    let mut c = col(w, 76.0, bg)?.radius(12.0).padding(12.0);
    c = c.child(text(n, 26.0, 500, ink, w - 24.0, 34.0)?);
    c = c.child(text(label, 11.0, 400, ink, w - 24.0, 16.0)?);
    Some(c)
}

fn group(title: &str, responds: bool) -> Option<Node> {
    let iw = W() - 64.0;
    let mut card = col_fit(W() - 32.0, SURF_CONT)?.radius(12.0).padding(16.0);
    card = card.child(para(title, 16.0, ON_SURFACE, iw)?);
    card = card.child(gap(4.0)?);
    for s in SCREENS.iter().filter(|s| s.responds == responds) {
        card = card.child(row_entry(s)?);
    }
    Some(card)
}

fn row_entry(s: &Screen) -> Option<Node> {
    let mut r = tap_row(W() - 64.0, 64.0, SURF_CONT, s.route)?;

    // A check only where the screen does something. The badge is what makes the
    // distinction visible at a glance, which is what the old single-section list
    // could not do.
    let (tint, ink, glyph) = if s.responds {
        (PRI_CONT, ON_PRI_CONT, "\u{2713}")
    } else {
        (SURF_HIGHEST, ON_SURF_VAR, "\u{00B7}")
    };
    let mut badge = col(36.0, 36.0, tint)?.radius(10.0);
    badge = badge.child(text(glyph, 14.0, 400, ink, 36.0, 36.0)?);
    r = r.child(badge);
    r = r.child(gap_w(12.0)?);

    let mut labels = col_fit(W() - 140.0, SURF_CONT)?;
    labels = labels.child(text(s.label, 16.0, 400, ON_SURFACE, W() - 140.0, 24.0)?);
    labels = labels.child(text(s.route, 12.0, 400, ON_SURF_VAR, W() - 140.0, 18.0)?);
    r = r.child(labels);

    r = r.child(text("\u{203A}", 14.0, 400, ON_SURF_VAR, 24.0, 64.0)?);
    Some(r)
}
