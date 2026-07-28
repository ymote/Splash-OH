//! A file browser inside a web surface.
//!
//! The question this card answers is not "can a page render a list" — it is
//! **how far into the phone an app's web surface can actually see**. So tab 0
//! is a probe that walks a set of well-known OHOS paths and reports, for each,
//! whether it listed, was denied, or does not exist. The remaining tabs drop
//! you into a browsable root.
//!
//! Everything after the first render happens inside the page: tapping a folder
//! calls `fs.list` again and redraws in JS. Rust is not rebuilt and the slot
//! does not change, which is the arrangement a real embedded UI wants — the
//! native side hands over a capability, not a screen.
//!
//! # What this deliberately does not do
//!
//! There is no `fs.read`. Listing tells a page that a file exists; handing it
//! the bytes is a strictly larger capability, and nothing here needs it. There
//! is also no path allowlist on `fs.list`, because the OS sandbox is the real
//! boundary and pre-empting it from the host side would mean the probe was
//! measuring my guesses rather than the system.

use super::ui::*;
use crate::arkui::Node;

const CHROME: u32 = 0xFF1B1B26;
const SUBTLE: u32 = 0xFF8A8A9E;
const ACCENT: u32 = 0xFF64B5F6;

pub const TAB_BASE: i32 = 700;

/// Starting points. An empty path means the boundary probe rather than a
/// listing.
///
/// The paths are OHOS's app-sandbox layout: an application sees `el2/base` as
/// its own private world, and everything outside it is the question.
pub const ROOTS: &[(&str, &str)] = &[
    ("Probe", ""),
    ("Files", "/data/storage/el2/base/files"),
    ("Base", "/data/storage/el2/base"),
    ("Bundle", "/data/storage/el1/bundle"),
    ("/", "/"),
];

/// Paths the probe tries. Chosen to straddle the sandbox: the first few should
/// be readable, the rest are the interesting ones.
///
/// The Android-shaped guesses (`.../haps/entry/files`) are kept deliberately.
/// They come back `absent`, which is the finding: OHOS puts an app's own
/// directories directly under `el2/base`, and `haps/<module>` holds a second,
/// per-module tree beside them.
const PROBE_PATHS: &[&str] = &[
    "/data/storage/el2/base/files",
    "/data/storage/el2/base/cache",
    "/data/storage/el2/base/haps/entry/files",
    "/data/storage/el2/base/haps",
    "/data/storage/el2/base",
    "/data/storage/el1/bundle",
    "/data/storage",
    "/storage/media/100/local/files",
    "/storage",
    "/data",
    "/system/lib64",
    "/system",
    "/proc/self",
    "/",
];

const BAR_H: f32 = 44.0;

fn tab_strip(active: usize) -> Option<Node> {
    let mut bar = row(W, BAR_H, CHROME)?;
    let cw = W / ROOTS.len() as f32;
    for (i, (label, _)) in ROOTS.iter().enumerate() {
        let c = if i == active { ACCENT } else { SUBTLE };
        let mut t = tap_row(cw, BAR_H, CHROME, TAB_BASE + i as i32)?;
        t = t.child(text(label, 11.0, c, cw - 4.0, 16.0)?);
        bar = bar.child(t);
    }
    Some(bar)
}

pub fn build(tab: usize) -> Option<Node> {
    let tab = tab.min(ROOTS.len() - 1);
    let (_, start) = ROOTS[tab];

    let mut root = col(W, PAGE_H, CHROME)?;
    root = root.child(tab_strip(tab)?);

    let body_h = PAGE_H - BAR_H;
    let html = page(start, W, body_h);
    root = root.child(web_html(html, 0.0, BAR_H, W, body_h)?);
    Some(root)
}

fn page(start: &str, w: f32, h: f32) -> String {
    let probes: String = PROBE_PATHS
        .iter()
        .map(|p| format!("'{p}'"))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        r#"<!doctype html><html><head><meta charset=utf-8>
<meta name=viewport content="width={w:.0}, initial-scale=1">
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{width:{w:.0}px;height:{h:.0}px;overflow:hidden;background:#12121b;color:#e9e9f2;
 font:400 14px/1.4 -apple-system,'HarmonyOS Sans','Helvetica Neue',sans-serif}}
#bar{{padding:9px 14px;background:#1b1b26;border-bottom:1px solid #2a2a3a;
 display:flex;align-items:center;gap:10px}}
#up{{padding:3px 9px;border-radius:7px;background:#2c2c3d;color:#9fd0ff;font-size:12px;
 flex:none}}
#up.off{{opacity:.3}}
#path{{font-size:11px;color:#9a9ab0;word-break:break-all;line-height:1.25}}
#list{{height:{list_h:.0}px;overflow-y:auto}}
.e{{display:flex;align-items:center;gap:10px;padding:9px 14px;
 border-bottom:1px solid #1e1e2c}}
.e.d{{color:#cfe6ff}}
.ic{{width:18px;text-align:center;flex:none}}
.nm{{flex:1;word-break:break-all}}
.sz{{font-size:11px;color:#71718a;flex:none}}
.msg{{padding:16px 14px;color:#ff8a8a;font-size:13px}}
.hint{{padding:10px 14px;color:#71718a;font-size:12px}}
/* Probe view */
.p{{display:flex;justify-content:space-between;gap:10px;padding:7px 14px;
 border-bottom:1px solid #1e1e2c;font-size:12px}}
.pp{{color:#c2c2d4;word-break:break-all}}
.pv{{flex:none;font-weight:600}}
.yes{{color:#5ad469}} .no{{color:#ff7676}} .miss{{color:#8a8a9e}}
</style></head><body>
<div id=bar><span id=up class=off>&#8593; up</span><span id=path>&#8230;</span></div>
<div id=list></div>
<script>
(function () {{
  var listEl = document.getElementById('list');
  var pathEl = document.getElementById('path');
  var upEl = document.getElementById('up');
  var cur = '', parent = '';

  if (!window.splash || !splash.available()) {{
    listEl.innerHTML = '<div class=msg>no bridge — this surface cannot reach fs.list</div>';
    return;
  }}

  function esc(s) {{
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;');
  }}

  function human(n) {{
    if (n < 1024) {{ return n + ' B'; }}
    if (n < 1048576) {{ return (n / 1024).toFixed(1) + ' K'; }}
    return (n / 1048576).toFixed(1) + ' M';
  }}

  // The boundary map. Each path is asked for independently and reported with
  // the OS's own answer, because "denied" and "does not exist" are different
  // facts about where the sandbox ends.
  function probe() {{
    cur = ''; parent = '';
    pathEl.textContent = 'what this app can reach';
    upEl.className = 'off';
    listEl.innerHTML = '';
    var paths = [{probes}];
    paths.forEach(function (p) {{
      var row = document.createElement('div');
      row.className = 'p';
      row.innerHTML = '<span class=pp>' + esc(p) + '</span><span class=pv>&#8230;</span>';
      listEl.appendChild(row);
      var vEl = row.querySelector('.pv');
      splash.invoke('fs.list', p).then(function (r) {{
        vEl.className = 'pv yes';
        vEl.textContent = r.entries.length + (r.truncated ? '+ items' : ' items');
        row.onclick = function () {{ go(p); }};
      }}).catch(function (e) {{
        var m = String(e.message || '');
        // Distinguish the two denials rather than reporting both as failure.
        var missing = m.indexOf('No such file') >= 0 || m.indexOf('os error 2') >= 0;
        vEl.className = 'pv ' + (missing ? 'miss' : 'no');
        vEl.textContent = missing ? 'absent' : 'denied';
      }});
    }});
  }}

  function go(p) {{
    listEl.innerHTML = '<div class=hint>reading&#8230;</div>';
    splash.invoke('fs.list', p).then(function (r) {{
      cur = r.path; parent = r.parent;
      pathEl.textContent = r.path;
      upEl.className = parent && parent !== r.path ? '' : 'off';
      if (!r.entries.length) {{
        listEl.innerHTML = '<div class=hint>empty</div>';
        return;
      }}
      listEl.innerHTML = r.truncated
        ? '<div class=hint>showing the first 500 entries</div>' : '';
      r.entries.forEach(function (e) {{
        var d = document.createElement('div');
        d.className = 'e' + (e.dir ? ' d' : '');
        d.innerHTML = '<span class=ic>' + (e.dir ? '📁' : '📄') + '</span>'
          + '<span class=nm>' + esc(e.name) + '</span>'
          + '<span class=sz>' + (e.dir ? '' : human(e.size)) + '</span>';
        if (e.dir) {{
          d.onclick = function () {{
            go(cur.replace(/\/$/, '') + '/' + e.name);
          }};
        }}
        listEl.appendChild(d);
      }});
    }}).catch(function (e) {{
      pathEl.textContent = p;
      listEl.innerHTML = '<div class=msg>' + esc(e.message) + '</div>';
    }});
  }}

  upEl.onclick = function () {{
    if (parent && parent !== cur) {{ go(parent); }}
  }};

  var start = '{start}';
  if (start) {{ go(start); }} else {{ probe(); }}
}})();
</script></body></html>"#,
        w = w,
        h = h,
        list_h = h - 40.0,
        probes = probes,
        start = start,
    )
}
