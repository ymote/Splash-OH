//! Every bridge tool on one screen, with live values.
//!
//! The bridge grew tool by tool and each one was proved by a panel bolted onto
//! whichever card happened to be nearby — `http.get` on the weather card,
//! `fs.list` on the files card, the capability gate in two places. That made
//! each addition harder to verify than the last.
//!
//! This is the fixture instead: one page, one row per tool, every value fetched
//! live. Adding a capability means adding a row, and a regression in any of
//! them is visible without hunting for the card that happened to exercise it.
//!
//! Note what several of these rows mean for a *web* surface. A page in a
//! browser gets a user-agent string and a rounded `devicePixelRatio`; these
//! rows carry the phone's actual brand and model, the panel's real density and
//! refresh rate, the battery's charge, and the process's own resident set —
//! because Rust asks the system and hands the answer back across the bridge.

use super::ui::*;
use crate::arkui::Node;

const CHROME: u32 = 0xFF14141C;

const BAR_H: f32 = 38.0;

pub fn build() -> Option<Node> {
    let mut root = col(W, PAGE_H, CHROME)?;

    let mut bar = row(W, BAR_H, CHROME)?;
    bar = bar.child(text(
        "NATIVE CAPABILITIES · JS → RUST",
        11.0,
        0xFF6E6E88,
        W - 20.0,
        16.0,
    )?);
    root = root.child(bar);

    let body_h = PAGE_H - BAR_H;
    root = root.child(web_html(page(W, body_h), 0.0, BAR_H, W, body_h)?);
    Some(root)
}

fn page(w: f32, h: f32) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset=utf-8>
<meta name=viewport content="width={w:.0}, initial-scale=1">
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{width:{w:.0}px;height:{h:.0}px;overflow-y:auto;background:#0e0e15;color:#e9e9f2;
 font:400 13px/1.4 -apple-system,'HarmonyOS Sans','Helvetica Neue',sans-serif}}
h2{{font-size:10px;letter-spacing:1.2px;text-transform:uppercase;color:#5f5f7a;
 padding:16px 14px 6px}}
.r{{display:flex;justify-content:space-between;gap:12px;padding:8px 14px;
 border-bottom:1px solid #1a1a26}}
.k{{color:#8f8fa8;flex:none}}
.v{{text-align:right;word-break:break-word}}
.p{{color:#5ad469}} .f{{color:#ff7676}} .w{{color:#e8c25a}} .m{{color:#8f8fa8}}
.note{{padding:12px 14px 24px;color:#5f5f7a;font-size:11px;line-height:1.5}}
</style></head><body>
<h2>Device &mdash; libdeviceinfo_ndk</h2>
<div class=r><span class=k>phone</span><span class="v m" id=d_phone>&#8230;</span></div>
<div class=r><span class=k>model</span><span class="v m" id=d_model>&#8230;</span></div>
<div class=r><span class=k>os</span><span class="v m" id=d_os>&#8230;</span></div>
<div class=r><span class=k>api / abi</span><span class="v m" id=d_api>&#8230;</span></div>
<div class=r><span class=k>security patch</span><span class="v m" id=d_patch>&#8230;</span></div>

<h2>Display &mdash; libnative_display_manager</h2>
<div class=r><span class=k>panel</span><span class="v m" id=p_size>&#8230;</span></div>
<div class=r><span class=k>density</span><span class="v m" id=p_dens>&#8230;</span></div>
<div class=r><span class=k>orientation</span><span class="v m" id=p_rot>&#8230;</span></div>

<h2>Battery &mdash; libohbattery_info</h2>
<div class=r><span class=k>charge</span><span class="v m" id=b_cap>&#8230;</span></div>

<h2>Sensors and haptics &mdash; libohsensor, libohvibrator</h2>
<div class=r><span class=k>sensor.list</span><span class="v m" id=s_list>&#8230;</span></div>
<div class=r><span class=k>accelerometer</span><span class="v m" id=s_acc>&#8230;</span></div>
<div class=r><span class=k>ambient light</span><span class="v m" id=s_light>&#8230;</span></div>
<div class=r><span class=k>vibrate (tap me)</span><span class="v m" id=s_vib>tap &#8594;</span></div>

<h2>Filesystem</h2>
<div class=r><span class=k>fs.read /proc/self/status</span><span class="v m" id=f_rss>&#8230;</span></div>
<div class=r><span class=k>fs.stat sandbox</span><span class="v m" id=f_stat>&#8230;</span></div>
<div class=r><span class=k>fs.list sandbox</span><span class="v m" id=f_list>&#8230;</span></div>

<h2>Network and VM</h2>
<div class=r><span class=k>http.get</span><span class="v m" id=n_get>&#8230;</span></div>
<div class=r><span class=k>splash.eval</span><span class="v m" id=n_vm>&#8230;</span></div>
<div class=r><span class=k>echo</span><span class="v m" id=n_echo>&#8230;</span></div>

<h2>Capability gate</h2>
<div class=r><span class=k>ssrf probes</span><span class="v m" id=g_ssrf>&#8230;</span></div>
<div class=r><span class=k>unknown tool</span><span class="v m" id=g_unk>&#8230;</span></div>

<div class=note>Every value above was fetched live through the bridge. A page in
a browser can see none of them: it gets a user-agent string and a rounded
devicePixelRatio.</div>
<script>
(function () {{
  function set(id, cls, txt) {{
    var el = document.getElementById(id);
    el.className = 'v ' + cls;
    el.textContent = txt;
  }}

  if (!window.splash || !splash.available()) {{
    ['d_phone','d_model','d_os','d_api','d_patch','p_size','p_dens','p_rot',
     'b_cap','s_list','s_acc','s_light','s_vib',
     'f_rss','f_stat','f_list','n_get','n_vm','n_echo','g_ssrf','g_unk']
      .forEach(function (id) {{ set(id, 'f', 'no bridge'); }});
    return;
  }}

  // Run each tool and report its own failure in its own row, so one broken
  // capability does not blank the rest of the page.
  function call(tool, args, ids, onOk) {{
    return splash.invoke(tool, args).then(onOk).catch(function (e) {{
      ids.forEach(function (id) {{ set(id, 'f', e.message); }});
    }});
  }}

  call('device.info', undefined,
    ['d_phone','d_model','d_os','d_api','d_patch'], function (i) {{
      // marketName often already carries the brand ("HUAWEI Mate 70 Air"), so
      // prefixing it unconditionally produced "HUAWEI HUAWEI Mate 70 Air".
      var brand = i.brand || '', market = i.marketName || '';
      set('d_phone', 'p', market.indexOf(brand) === 0 ? market : (brand + ' ' + market).trim());
      set('d_model', 'p', i.productModel + ' / ' + i.deviceType);
      set('d_os', 'p', (i.distroName || i.osFullName) + ' ' + (i.distroVersion || i.displayVersion));
      set('d_api', 'p', 'API ' + i.sdkApiVersion + ' · ' + i.abiList);
      set('d_patch', 'p', i.securityPatch || 'n/a');
    }});

  call('device.display', undefined, ['p_size','p_dens','p_rot'], function (d) {{
    set('p_size', 'p', d.width + ' × ' + d.height + ' px @ ' + d.refreshRate + ' Hz');
    set('p_dens', 'p', d.densityDpi + ' dpi · ratio ' + d.pixelRatio);
    set('p_rot', 'p', d.orientation + ' · ' + d.rotation);
  }});

  call('device.battery', undefined, ['b_cap'], function (b) {{
    set('b_cap', 'p', b.capacity + '%' + (b.charging ? ' · charging (' + b.pluggedType + ')' : ''));
  }});

  // The process's own resident set, read out of procfs. A web page has no way
  // to learn how much memory the app it is inside is using.
  call('fs.read', '/proc/self/status', ['f_rss'], function (r) {{
    var m = /VmRSS:\s*(\d+)\s*kB/.exec(r.content || '');
    set('f_rss', 'p', m ? 'VmRSS ' + Math.round(m[1] / 1024) + ' MB (' + r.encoding + ')'
                        : 'read ' + r.size + ' bytes');
  }});

  call('sensor.list', undefined, ['s_list'], function (l) {{
    set('s_list', 'p', l.length + ' sensors');
  }});

  // A single reading means subscribe, wait for a tick, unsubscribe -- there is
  // no getter in the NDK. Rust does that on a worker; the page just awaits.
  call('sensor.read', 'accelerometer', ['s_acc'], function (r) {{
    var v = r.values.map(function (x) {{ return x.toFixed(2); }});
    set('s_acc', 'p', 'x ' + v[0] + '  y ' + v[1] + '  z ' + v[2] + ' m/s²');
  }});
  call('sensor.read', 'ambient light', ['s_light'], function (r) {{
    set('s_light', 'p', r.values[0].toFixed(1) + ' lux');
  }});

  // The one capability with a side effect you can feel rather than read.
  document.getElementById('s_vib').parentNode.onclick = function () {{
    set('s_vib', 'w', 'buzzing\u2026');
    splash.invoke('vibrate', {{ ms: 120 }})
      .then(function (r) {{ set('s_vib', 'p', r.ms + ' ms \u2713'); }})
      .catch(function (e) {{ set('s_vib', 'f', e.message); }});
  }};

  var SANDBOX = '/data/storage/el2/base/files';
  call('fs.stat', SANDBOX, ['f_stat'], function (s) {{
    set('f_stat', 'p', (s.dir ? 'directory' : 'file') + (s.readonly ? ', read-only' : ', writable'));
  }});
  call('fs.list', SANDBOX, ['f_list'], function (l) {{
    set('f_list', 'p', l.entries.length + ' entries');
  }});

  call('http.get', 'https://api.open-meteo.com/v1/forecast?latitude=35.7&longitude=139.7'
       + '&current=temperature_2m&timezone=auto', ['n_get'], function (body) {{
    set('n_get', 'p', 'Tokyo ' + Math.round(JSON.parse(body).current.temperature_2m) + '°');
  }});

  call('splash.eval', {{ source: ['let n = 0', 'for v in input.xs {{', '  n = n + v', '}}', 'n'].join('\n'),
                        input: {{ xs: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] }} }},
       ['n_vm'], function (r) {{
    set('n_vm', r === 55 ? 'p' : 'f', 'sum 1..10 = ' + r);
  }});

  call('echo', 'round-trip', ['n_echo'], function (r) {{
    set('n_echo', r === 'round-trip' ? 'p' : 'f', r);
  }});

  // The gate should refuse these three even from a trusted page.
  var probes = ['https://example.com/', 'https://127.0.0.1/', 'http://api.open-meteo.com/'];
  Promise.all(probes.map(function (u) {{
    return splash.invoke('http.get', u).then(function () {{ return 1; }})
      .catch(function () {{ return 0; }});
  }})).then(function (rs) {{
    var leaked = rs.reduce(function (a, b) {{ return a + b; }}, 0);
    set('g_ssrf', leaked ? 'f' : 'p', leaked ? leaked + ' LEAKED' : 'all 3 blocked');
  }});

  // An unknown tool must be refused rather than silently ignored -- otherwise a
  // typo in a tool name looks like a hung call.
  splash.invoke('does.not.exist')
    .then(function () {{ set('g_unk', 'f', 'ACCEPTED'); }})
    .catch(function (e) {{ set('g_unk', 'p', e.message); }});
}})();
</script></body></html>"#,
        w = w,
        h = h,
    )
}
