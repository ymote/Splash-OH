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

    // A native render surface, created by Rust with no ArkTS involved, so the
    // page below can report whether one actually materialised. It sits above
    // the web slot: an XComponent is a real ArkUI node in the same tree, unlike
    // a Web, which has no native node type and must be overlaid by ArkTS.
    let surf_h = 120.0;
    if let Some(s) = crate::xcomp::surface(W, surf_h) {
        root = root.child(s);
    }

    let body_h = PAGE_H - BAR_H - surf_h;
    root = root.child(web_html(page(W, body_h), 0.0, BAR_H + surf_h, W, body_h)?);
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

<div class=r><span class=k>time zone</span><span class="v m" id=d_tz>&#8230;</span></div>
<div class=r><span class=k>notifications</span><span class="v m" id=d_notif>&#8230;</span></div>

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

<h2>Screen capture &mdash; libnative_display_manager</h2>
<div class=r><span class=k>screen.capture</span><span class="v m" id=c_cap>&#8230;</span></div>

<h2>Network &mdash; libnet_connection</h2>
<div class=r><span class=k>default route</span><span class="v m" id=w_net>&#8230;</span></div>
<div class=r><span class=k>capabilities</span><span class="v m" id=w_caps>&#8230;</span></div>
<div class=r><span class=k>system proxy</span><span class="v m" id=w_proxy>&#8230;</span></div>

<h2>Native surface &mdash; ARKUI_NODE_XCOMPONENT</h2>
<div class=r><span class=k>surface (no ArkTS)</span><span class="v m" id=x_surf>&#8230;</span></div>

<h2>Camera and image codecs</h2>
<div class=r><span class=k>camera.list</span><span class="v m" id=i_cams>&#8230;</span></div>
<div class=r><span class=k>pick an image &#8594;</span><span class="v m" id=i_pick>tap &#8594;</span></div>
<div class=r><span class=k>image.info</span><span class="v m" id=i_info>&#8212;</span></div>
<div class=r><span class=k>image.thumbnail</span><span class="v m" id=i_thumb>&#8212;</span></div>
<div id=i_prev></div>

<h2>Location &mdash; liblocation_ndk</h2>
<div class=r><span class=k>system switch</span><span class="v m" id=l_on>&#8230;</span></div>
<div class=r><span class=k>fix (tap to allow)</span><span class="v m" id=l_fix>&#8230;</span></div>
<div class=r><span class=k>nearest weather city</span><span class="v m" id=l_city>&#8230;</span></div>

<h2>Radio &mdash; libtelephony_radio, libwifi_ndk</h2>
<div class=r><span class=k>cellular</span><span class="v m" id=r_cell>&#8230;</span></div>
<div class=r><span class=k>wifi radio</span><span class="v m" id=r_wifi>&#8230;</span></div>

<h2>Crypto &mdash; libohcrypto</h2>
<div class=r><span class=k>sha256 of "abc"</span><span class="v m" id=x_abc>&#8230;</span></div>
<div class=r><span class=k>sha256 of prefs.json</span><span class="v m" id=x_file>&#8230;</span></div>

<h2>Clipboard &mdash; ArkTS pasteboard, via the Rust&#8594;ArkTS channel</h2>
<div class=r><span class=k>clipboard.write</span><span class="v m" id=cb_w>&#8230;</span></div>
<div class=r><span class=k>clipboard.read</span><span class="v m" id=cb_r>&#8230;</span></div>

<h2>Persistent storage</h2>
<div class=r><span class=k>prefs round trip</span><span class="v m" id=k_rt>&#8230;</span></div>
<div class=r><span class=k>launches seen</span><span class="v m" id=k_runs>&#8230;</span></div>

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
    ['d_phone','d_model','d_os','d_api','d_patch','d_tz','d_notif',
     'p_size','p_dens','p_rot',
     'b_cap','s_list','s_acc','s_light','s_vib',
     'f_rss','f_stat','f_list','c_cap','w_net','w_caps','w_proxy','x_surf','i_cams','i_pick','i_info','i_thumb','l_on','l_fix','l_city','r_cell','r_wifi','x_abc','x_file','cb_w','cb_r','k_rt','k_runs',
     'n_get','n_vm','n_echo','g_ssrf','g_unk']
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

  call('device.time', undefined, ['d_tz'], function (t) {{
    var skew = Math.abs(t.unixSeconds * 1000 - Date.now()) / 1000;
    set('d_tz', 'p', t.timeZone + ' · clock ±' + skew.toFixed(1) + 's vs JS');
  }});
  call('device.notifications', undefined, ['d_notif'], function (n) {{
    set('d_notif', n.enabled ? 'p' : 'w', n.enabled ? 'enabled' : 'disabled by user');
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

  // A refusal here is a result, not a bug -- the row says which refusal.
  splash.invoke('screen.capture').then(function (c) {{
    set('c_cap', 'p', c.width + '×' + c.height + ' ' + c.pixelFormat
        + (c.averageColor ? ' · avg ' + c.averageColor : ''));
  }}).catch(function (e) {{ set('c_cap', 'w', e.message); }});

  call('net.info', undefined, ['w_net','w_caps','w_proxy'], function (n) {{
    set('w_net', n.online ? 'p' : 'f',
        (n.online ? 'up' : 'down') + ' · ' + (n.bearers.join(', ') || '?')
        + (n.metered === true ? ' · metered' : ''));
    // "validated" is the interesting one: a link can be associated and still
    // not carry traffic, which is what the phone looks like behind a dead proxy.
    set('w_caps', n.capabilities.indexOf('validated') >= 0 ? 'p' : 'w',
        n.capabilities.join(', ') || 'none');
    set('w_proxy', n.proxy ? 'w' : 'p',
        n.proxy ? n.proxy.host + ':' + n.proxy.port : 'none');
  }});

  // The point of this row: a Web needs an ArkTS overlay because there is no
  // ARKUI_NODE_WEB, but a render surface does not -- XComponent is a real node
  // type, so Rust builds it and the producer writes frames straight in.
  call('surface.state', undefined, ['x_surf'], function (s) {{
    set('x_surf', s.created ? 'p' : 'w',
        s.created ? 'id ' + s.surfaceId + ' · ' + s.width + '×' + s.height + ' px'
                  : (s.destroyed ? 'destroyed' : 'no surface yet'));
  }});

  call('camera.list', undefined, ['i_cams'], function (cs) {{
    var back = cs.filter(function (c) {{ return c.position === 'back'; }}).length;
    var front = cs.filter(function (c) {{ return c.position === 'front'; }}).length;
    set('i_cams', 'p', cs.length + ' cameras · ' + back + ' back, ' + front + ' front');
  }});

  // The whole point of the codec work, in one gesture: pick a photo, learn
  // what it is from its header, then decode it small enough to actually show.
  // A phone photo is megabytes and every reply here crosses as one JSON string
  // evaluated into the page -- so the full image is the one thing this channel
  // cannot carry, and decoding at a reduced size is what makes it possible.
  document.getElementById('i_pick').parentNode.onclick = function () {{
    set('i_pick', 'w', 'picking\u2026');
    splash.invoke('fs.pick', {{ mode: 'file' }}).then(function (sel) {{
      if (!sel || !sel.length) {{ set('i_pick', 'w', 'nothing picked'); return; }}
      var path = sel[0].path;
      set('i_pick', 'p', sel[0].name);
      return splash.invoke('image.info', {{ path: path }}).then(function (info) {{
        set('i_info', 'p', info.width + '×' + info.height + ' · '
            + info.megapixels.toFixed(1) + ' MP · ' + Math.round(info.fileBytes / 1024) + ' KB'
            + (info.hdr ? ' · HDR' : ''));
        return splash.invoke('image.thumbnail', {{ path: path, maxEdge: 320, quality: 80 }});
      }}).then(function (t) {{
        var ratio = t.jpegBytes / Math.max(1, t.sourceWidth * t.sourceHeight * 3);
        set('i_thumb', 'p', t.width + '×' + t.height + ' · '
            + Math.round(t.jpegBytes / 1024) + ' KB · 1/'
            + Math.round(1 / Math.max(ratio, 1e-9)) + ' of raw');
        document.getElementById('i_prev').innerHTML =
          '<div style="padding:10px 14px"><img src="' + t.dataUri
          + '" style="max-width:100%;border-radius:10px"></div>';
      }});
    }}).catch(function (e) {{
      set('i_pick', 'f', e.message);
    }});
  }};

  call('location.enabled', undefined, ['l_on'], function (l) {{
    set('l_on', l.enabled ? 'p' : 'w', l.enabled ? 'on' : 'off in system settings');
  }});

  // The first capability whose availability is not a property of the build.
  // A 201 here is not a bug and not a wall -- it is a question that has not
  // been asked yet, so the row offers to ask it rather than reporting failure.
  function showFix(f) {{
    set('l_fix', 'p', f.latitude.toFixed(4) + ', ' + f.longitude.toFixed(4)
        + ' · ±' + f.accuracy.toFixed(0) + ' m');
    return splash.invoke('splash.eval', {{
      source: ['input.lat', ''].join('\n'), input: {{ lat: f.latitude }}
    }}).then(function () {{ return f; }}).catch(function () {{ return f; }});
  }}

  // Two attempts, cheap first. The daily-life scene lands on the passive
  // provider, which only forwards fixes another app asked for -- so on a quiet
  // phone it returns nothing at all. Falling back to the navigation scene asks
  // a provider that actually goes and looks.
  function locate() {{
    return splash.invoke('location.get', {{ timeoutMs: 6000, scene: 'daily' }})
      .catch(function () {{
        set('l_fix', 'w', 'passive gave nothing; asking GNSS\u2026');
        return splash.invoke('location.get', {{ timeoutMs: 20000, scene: 'navigation' }});
      }});
  }}

  function tryFix() {{
    set('l_fix', 'w', 'locating\u2026');
    return locate().then(function (f) {{
      showFix(f);
      return splash.invoke('location.nearestCity', {{ lat: f.latitude, lon: f.longitude }});
    }}).then(function (c) {{
      set('l_city', 'p', c.city + ' (' + c.km.toFixed(0) + ' km)');
    }});
  }}

  tryFix().catch(function (e) {{
    var needsAsk = /permission/i.test(e.message || '');
    // A timeout here is almost always the sky, not the code. The system log
    // says so plainly ("satellite num: 1 < 4") but an app cannot read that, so
    // the row states the likely cause rather than repeating the bare timeout.
    var noSignal = /no fix within/.test(e.message || '');
    set('l_fix', 'w', needsAsk ? 'tap to allow \u2192'
        : noSignal ? 'no fix \u2014 needs sky, or wait longer' : e.message);
    set('l_city', 'm', needsAsk ? 'waiting on location' : '\u2014');
    if (!needsAsk) {{ return; }}
    document.getElementById('l_fix').parentNode.onclick = function () {{
      set('l_fix', 'w', 'asking\u2026');
      splash.invoke('permission.request',
          ['ohos.permission.APPROXIMATELY_LOCATION', 'ohos.permission.LOCATION'])
        .then(function (r) {{
          if (!r.granted.length) {{ set('l_fix', 'w', 'refused by user'); return; }}
          return tryFix();
        }})
        .catch(function (e2) {{ set('l_fix', 'f', e2.message); }});
    }};
  }});

  call('radio.cellular', undefined, ['r_cell'], function (c) {{
    set('r_cell', c.registration === 'in-service' ? 'p' : 'w',
        (c.operator || 'no operator') + ' · ' + c.technology + ' · ' + c.registration
        + (c.roaming ? ' · roaming' : ''));
  }});
  call('radio.wifi', undefined, ['r_wifi'], function (w) {{
    set('r_wifi', w.enabled ? 'p' : 'w', w.enabled ? 'on' : 'off');
  }});

  // A known-answer test, not just "a hash came back". SHA-256("abc") is one of
  // the most widely published digests there is, so a wrong one is visible.
  var ABC = 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad';
  splash.invoke('crypto.sha256', {{ text: 'abc' }})
    .then(function (h) {{
      set('x_abc', h.hex === ABC ? 'p' : 'f',
          h.hex === ABC ? h.hex.slice(0, 16) + '\u2026 \u2713 known answer' : h.hex);
    }})
    .catch(function (e) {{ set('x_abc', 'f', e.message); }});

  splash.invoke('crypto.sha256', {{ path: '/data/storage/el2/base/files/prefs.json' }})
    .then(function (h) {{ set('x_file', 'p', h.hex.slice(0, 16) + '\u2026 (' + h.bytes + ' B)'); }})
    .catch(function (e) {{ set('x_file', 'f', e.message); }});

  // Write and read are reported separately because they are not the same
  // capability here. Writing needs no permission; reading is gated by the
  // pasteboard service unless the app holds READ_PASTEBOARD (user_grant) or
  // the call is a genuine user paste. An empty read is the gate, not a bug,
  // so the row says so rather than showing a mismatch.
  var mark = 'splash-' + Math.floor(Date.now() / 1000 % 100000);
  splash.invoke('clipboard.write', mark)
    .then(function (w) {{ set('cb_w', 'p', 'wrote ' + w.wrote + ' chars \u2713'); }})
    .catch(function (e) {{ set('cb_w', 'f', e.message); }});

  splash.invoke('clipboard.read')
    .then(function (c) {{
      if (c.text === mark) {{ set('cb_r', 'p', mark + ' \u2713 (' + c.mimeType + ')'); }}
      else if (!c.text) {{ set('cb_r', 'w', 'empty \u2014 needs READ_PASTEBOARD or a real paste'); }}
      else {{ set('cb_r', 'p', 'read "' + c.text + '"'); }}
    }})
    .catch(function (e) {{ set('cb_r', 'w', e.message); }});

  // Written, read back, and counted across launches -- so the row proves the
  // value survived the process rather than just this page.
  splash.invoke('prefs.set', {{ key: 'probe', value: 'v1' }})
    .then(function () {{ return splash.invoke('prefs.get', 'probe'); }})
    .then(function (v) {{ set('k_rt', v === 'v1' ? 'p' : 'f', 'stored and read "' + v + '"'); }})
    .catch(function (e) {{ set('k_rt', 'f', e.message); }});

  splash.invoke('prefs.get', 'launches').then(function (v) {{
    var n = (parseInt(v, 10) || 0) + 1;
    return splash.invoke('prefs.set', {{ key: 'launches', value: String(n) }})
      .then(function () {{ set('k_runs', 'p', n + (n === 1 ? ' (first run)' : '')); }});
  }}).catch(function (e) {{ set('k_runs', 'f', e.message); }});

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
