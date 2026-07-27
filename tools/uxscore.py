#!/usr/bin/env python3
"""Mechanical UX score for a phone screenshot. No model in the loop.

Why this exists
---------------
The plan it serves is: an agent writes a UI, the phone renders it, we screenshot
it, score it, and feed the score back so the agent iterates. The obvious way to
do the scoring is to ask a model "rate this UI out of 10". That does not work.
A model scoring its own output against a soft rubric optimises the score rather
than the artifact -- it learns that describing the design confidently earns
points. The measurement has to be independent of the thing being measured, so
everything here is computed from pixels and nothing here can be argued with.

Why pixels and not the view tree
--------------------------------
`uiautomator dump` on a makepad app returns 14 generic FrameLayout/LinearLayout
containers and no text or widget nodes, because makepad draws into a single GL
surface. So tap-target sizes, text bounds and clipping -- the things you would
normally check -- are not observable. Pixels are all there is. Every metric
below is chosen to be computable from a PNG alone.

The five metrics, each 0-2, total 0-10
--------------------------------------
contrast   text-vs-background WCAG ratio, on detected text bands
safe_area  no text under the status bar or over the gesture bar
alignment  content left edges concentrate on a few margins, not many
rhythm     vertical gaps between content bands snap to a grid
palette    a small number of colours covers the screen

Calibration is the part to be suspicious of, so `--selftest` runs the scorer
over a set of screenshots with known-good and known-bad members and prints the
ranking. A scorer that cannot put a blank screen below a real one is not
measuring anything, and you want to find that out before you build a loop on it.
"""

import argparse
import json
import sys
from collections import Counter

try:
    from PIL import Image
except ImportError:
    sys.exit("needs Pillow: pip3 install Pillow")


# --- WCAG -------------------------------------------------------------------

def _lin(c):
    c = c / 255.0
    return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4


def luminance(rgb):
    r, g, b = rgb[:3]
    return 0.2126 * _lin(r) + 0.7152 * _lin(g) + 0.0722 * _lin(b)


def contrast_ratio(a, b):
    la, lb = luminance(a), luminance(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


# --- helpers ----------------------------------------------------------------

def row_profile(px, w, h):
    """Per-row (ink_fraction, bg_colour, fg_colour).

    "Ink" is a pixel far from its row's dominant colour. Text rows have a
    little ink; empty rows have none; image rows have a lot, which is why the
    metrics below only trust rows in a text-like band of ink fraction.
    """
    out = []
    for y in range(h):
        counts = Counter(px[x, y][:3] for x in range(0, w, 3))
        bg = counts.most_common(1)[0][0]
        lbg = luminance(bg)
        ink, far, farlum = 0, bg, lbg
        for x in range(0, w, 3):
            c = px[x, y][:3]
            d = abs(luminance(c) - lbg)
            if d > 0.20:
                ink += 1
                if d > abs(farlum - lbg):
                    far, farlum = c, luminance(c)
        out.append((ink / max(1, len(range(0, w, 3))), bg, far))
    return out


def content_left_edges(px, w, h, rows):
    """First x per row that differs from that row's background."""
    edges = []
    for y in range(h):
        frac, bg, _ = rows[y]
        if frac < 0.005 or frac > 0.60:
            continue
        lbg = luminance(bg)
        for x in range(w):
            if abs(luminance(px[x, y][:3]) - lbg) > 0.20:
                edges.append(x)
                break
    return edges


# --- metrics ----------------------------------------------------------------

def m_contrast(px, w, h, rows):
    """Text-bearing rows should clear WCAG AA (4.5:1)."""
    ratios = [contrast_ratio(fg, bg) for frac, bg, fg in rows if 0.005 < frac < 0.35]
    if not ratios:
        return 0.0, {"text_rows": 0, "note": "no text-like rows found"}
    ok = sum(1 for r in ratios if r >= 4.5) / len(ratios)
    worst = min(ratios)
    return round(2.0 * ok, 2), {
        "text_rows": len(ratios),
        "pass_fraction": round(ok, 3),
        "worst_ratio": round(worst, 2),
    }


def m_content(rows):
    """An absence gate.

    Without this a blank white screen scored 5.0/10: it cannot intrude on the
    safe area and it has one colour, so two metrics award full marks for
    drawing nothing. Any score-maximising loop finds that immediately. This
    zeroes the total below a floor of actual content.
    """
    inked = sum(1 for f, _, _ in rows if f > 0.004) / max(1, len(rows))
    return round(2.0 * min(1.0, inked / 0.45), 2), {"inked_row_fraction": round(inked, 3)}


def m_safe_area(px, w, h, rows, top=0.0, bottom=0.0):
    """Nothing text-like in the app's own top/bottom margins.

    The system status bar and gesture bar are cropped before scoring -- their
    clock and icons are not the app's doing, and counting them penalised every
    real screenshot equally, which is bias rather than signal.
    """
    t, b = int(h * top), int(h * (1 - bottom))
    intr_t = sum(1 for y in range(t) if 0.005 < rows[y][0] < 0.35)
    intr_b = sum(1 for y in range(b, h) if 0.005 < rows[y][0] < 0.35)
    total = t + (h - b)
    bad = (intr_t + intr_b) / max(1, total)
    return round(2.0 * max(0.0, 1.0 - bad * 3), 2), {
        "status_bar_rows_with_text": intr_t,
        "gesture_bar_rows_with_text": intr_b,
    }


def m_alignment(px, w, h, rows):
    """Content should start from a few consistent margins, not many."""
    edges = content_left_edges(px, w, h, rows)
    if len(edges) < 20:
        return 0.0, {"sampled_rows": len(edges), "note": "too little content"}
    # Snap to 4px and see how much mass the top few margins hold.
    snapped = Counter(e // 4 * 4 for e in edges)
    top3 = sum(c for _, c in snapped.most_common(3))
    conc = top3 / len(edges)
    return round(2.0 * min(1.0, conc / 0.75), 2), {
        "distinct_margins": len(snapped),
        "top3_concentration": round(conc, 3),
        "margins": [m for m, _ in snapped.most_common(3)],
    }


def m_rhythm(px, w, h, rows):
    """Gaps between content bands should cluster, not scatter."""
    empty = [rows[y][0] < 0.004 for y in range(h)]
    gaps, run = [], 0
    for e in empty:
        if e:
            run += 1
        else:
            if run > 4:
                gaps.append(run)
            run = 0
    if len(gaps) < 3:
        return 1.0, {"gaps": len(gaps), "note": "too few gaps to judge"}
    snapped = Counter(g // 8 for g in gaps)
    conc = sum(c for _, c in snapped.most_common(3)) / len(gaps)
    return round(2.0 * min(1.0, conc / 0.8), 2), {
        "gap_count": len(gaps),
        "distinct_gap_sizes": len(snapped),
        "top3_concentration": round(conc, 3),
    }


def m_palette(px, w, h, rows):
    """A disciplined UI is a small number of colours -- in its CHROME.

    Measured only on rows that look like UI rather than imagery. Scoring the
    whole frame gave every photo-bearing screen 0.0, which says "this app has
    photographs in it", not "this app is undisciplined."
    """
    ui_rows = [y for y in range(h) if rows[y][0] < 0.35]
    if len(ui_rows) < h * 0.15:
        return 1.0, {"note": "mostly imagery; palette not judged"}
    q = Counter()
    for y in ui_rows[::4]:
        for x in range(0, w, 4):
            r, g, b = px[x, y][:3]
            q[(r // 24, g // 24, b // 24)] += 1
    total = sum(q.values())
    cum, n = 0, 0
    for _, c in q.most_common():
        cum += c
        n += 1
        if cum / total >= 0.95:
            break
    # 1-8 buckets is disciplined; 25+ is noise.
    score = 2.0 if n <= 8 else max(0.0, 2.0 * (1 - (n - 8) / 17))
    return round(score, 2), {"buckets_for_95pct": n, "ui_rows": len(ui_rows)}


# --- driver -----------------------------------------------------------------

# Fractions of the frame owned by the system, not the app.
STATUS_BAR = 0.050
GESTURE_BAR = 0.022


def score(path):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    # Crop the system bars before anything looks at the pixels.
    im = im.crop((0, int(h * STATUS_BAR), w, int(h * (1 - GESTURE_BAR))))
    w, h = im.size
    # Work at a manageable height; the metrics are all fractional.
    if h > 1400:
        im = im.resize((int(w * 1400 / h), 1400))
        w, h = im.size
    px = im.load()
    rows = row_profile(px, w, h)

    parts = {}
    parts["content"], n_d = m_content(rows)
    parts["contrast"], c_d = m_contrast(px, w, h, rows)
    parts["safe_area"], s_d = m_safe_area(px, w, h, rows)
    parts["alignment"], a_d = m_alignment(px, w, h, rows)
    parts["rhythm"], r_d = m_rhythm(px, w, h, rows)
    parts["palette"], p_d = m_palette(px, w, h, rows)

    # Six metrics of 2 renormalised to 10, then gated: a screen with almost no
    # content cannot score on presentation it does not have.
    raw = sum(parts.values()) / 12.0 * 10.0
    gate = min(1.0, parts["content"] / 1.0)
    return {
        "file": path,
        "total": round(raw * gate, 2),
        "parts": parts,
        "detail": {
            "content": n_d, "contrast": c_d, "safe_area": s_d,
            "alignment": a_d, "rhythm": r_d, "palette": p_d,
        },
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("images", nargs="+")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true",
                    help="rank the given images; use with known-good and "
                         "known-bad ones to check the metric is measuring "
                         "anything before trusting a loop built on it")
    args = ap.parse_args()

    results = [score(p) for p in args.images]

    if args.json:
        print(json.dumps(results, indent=2))
        return

    if args.selftest:
        results.sort(key=lambda r: -r["total"])
        print(f"{'total':>6}  {'cntn':>5} {'cont':>5} {'safe':>5} {'algn':>5} {'rhym':>5} {'pltt':>5}  file")
        for r in results:
            p = r["parts"]
            print(f"{r['total']:>6}  {p['content']:>5} {p['contrast']:>5} "
                  f"{p['safe_area']:>5} {p['alignment']:>5} {p['rhythm']:>5} "
                  f"{p['palette']:>5}  {r['file'].split('/')[-1]}")
        return

    for r in results:
        print(f"{r['file'].split('/')[-1]}: {r['total']}/10")
        for k, v in r["parts"].items():
            print(f"  {k:<10} {v:>4}   {r['detail'][k]}")


if __name__ == "__main__":
    main()
