//! A weather app rendered as a web page inside a Splash tree.
//!
//! The companion to `assets/weather.splash`, which draws the same data with
//! native ArkUI widgets. Same source, same numbers, two renderers — which makes
//! it a fair place to see what each is good for.
//!
//! * The **native** card gets real widgets, real scrolling, and the ~2.5×
//!   construction advantage measured in `CONCLUSION.md`. It is bounded by what
//!   the DSL can express: no gradients, no blur, no arbitrary typography.
//! * This **web** card gets CSS. Gradients, backdrop blur, web fonts, grid — all
//!   free — at the cost of a whole browser engine behind it.
//!
//! Data is Open-Meteo, which needs no API key. Fetched with `net::http_get`
//! through OpenHarmony's native HTTP stack, parsed in Rust, and interpolated
//! into the markup here, so the page has no JavaScript and makes no network
//! requests of its own. That matters: a webview that fetches its own data is a
//! second, invisible network path with its own failure modes.

use crate::webslot::{web, web_html};
use splash_oh_native::arkui::Node;
use splash_oh_native::net;
use splash_oh_native::ui::*;
use std::sync::Mutex;

const CHROME: u32 = 0xFF1C1C2E;
const CREAM: u32 = 0xFFF2F2F7;
const SUBTLE: u32 = 0xFF9A9AB0;
const ACCENT: u32 = 0xFF64B5F6;

pub const CITY_BASE: i32 = 600;

/// Cities the card can show: (name, lat, lon).
pub const CITIES: &[(&str, f64, f64)] = &[
    ("Tokyo", 35.68, 139.76),
    ("San Jose", 37.34, -121.89),
    ("Shenzhen", 22.54, 114.06),
    ("London", 51.51, -0.13),
];

/// WMO weather code → (label, emoji). The codes Open-Meteo returns.
fn wmo(code: i64) -> (&'static str, &'static str) {
    match code {
        0 => ("Clear", "☀️"),
        1 | 2 => ("Partly cloudy", "⛅"),
        3 => ("Overcast", "☁️"),
        45 | 48 => ("Fog", "🌫️"),
        51 | 53 | 55 | 56 | 57 => ("Drizzle", "🌦️"),
        61 | 63 | 65 | 66 | 67 => ("Rain", "🌧️"),
        71 | 73 | 75 | 77 => ("Snow", "🌨️"),
        80 | 81 | 82 => ("Showers", "🌦️"),
        85 | 86 => ("Snow showers", "🌨️"),
        95 | 96 | 99 => ("Thunderstorm", "⛈️"),
        _ => ("—", "🌡️"),
    }
}

/// Everything the page needs, in one fetch.
#[derive(Clone)]
struct Forecast {
    temp: String,
    feels: String,
    code: i64,
    hi: String,
    lo: String,
    humidity: String,
    wind: String,
    days: Vec<(String, String, String, i64)>,
    ok: bool,
}

/// Forecasts already fetched, by city index.
///
/// The build path must never touch the network. `fetch` issues blocking HTTP
/// through OpenHarmony's stack, and `build()` runs on whichever thread asked
/// for a render -- the JS thread at startup, the ArkUI event thread after a
/// tap. Blocking either is wrong, and doing it on the event thread also writes
/// the resulting web slot into the wrong thread's storage, so the JS-side poll
/// reads an empty list and tears the surface down again. That is what made this
/// card render white while the identical markup with static data rendered fine.
///
/// So: build reads this cache and returns immediately, a worker fills it, and
/// the next render picks it up.
/// One entry per city, not one entry total. A single slot meant switching city
/// evicted the previous one, so going back re-fetched something already had —
/// and while that fetch was in flight the card showed placeholders for a city
/// it already knew the answer for. There are four cities; keep all of them.
static CACHE: Mutex<Vec<(usize, Forecast)>> = Mutex::new(Vec::new());
/// Cities with a worker already running. A tap produced two builds and so two
/// identical fetches; the second was pure waste and its later arrival
/// re-flagged DIRTY for nothing.
static INFLIGHT: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// Set when a worker lands new data, so the surface can be redrawn. Without it
/// the page renders once with placeholders and never updates -- the poll on the
/// ArkTS side has nothing to notice, because the slot geometry is unchanged.
static DIRTY: Mutex<bool> = Mutex::new(false);

/// True once, if a forecast arrived since the last call.
pub fn take_dirty() -> bool {
    DIRTY
        .lock()
        .map(|mut d| std::mem::replace(&mut *d, false))
        .unwrap_or(false)
}

fn cached(city: usize) -> Option<Forecast> {
    CACHE
        .lock()
        .ok()
        .and_then(|c| c.iter().find(|(i, _)| *i == city).map(|(_, f)| f.clone()))
}

/// Fetch `city` on a worker thread and cache it. Cheap to call repeatedly.
pub fn prefetch(city: usize) {
    if cached(city).is_some() {
        return;
    }
    // Claim the city before spawning, so a rebuild while the fetch is in
    // flight does not start a second one.
    match INFLIGHT.lock() {
        Ok(mut f) if !f.contains(&city) => f.push(city),
        _ => return,
    }
    std::thread::spawn(move || {
        let f = fetch(city);
        if let Ok(mut c) = CACHE.lock() {
            c.retain(|(i, _)| *i != city);
            c.push((city, f));
        }
        if let Ok(mut inf) = INFLIGHT.lock() {
            inf.retain(|i| *i != city);
        }
        if let Ok(mut d) = DIRTY.lock() {
            *d = true;
        }
        crate::log(&format!("weather: forecast ready for {}", CITIES[city].0));
    });
}

/// Shown until the worker lands. Not an error state -- just "not yet".
fn pending() -> Forecast {
    Forecast {
        temp: "--".into(),
        feels: "--".into(),
        code: -1,
        hi: "--".into(),
        lo: "--".into(),
        humidity: "--".into(),
        wind: "--".into(),
        days: (0..6)
            .map(|_| ("—".to_string(), "--".into(), "--".into(), -1))
            .collect(),
        ok: true,
    }
}

/// Static stand-in, for isolating the renderer from the network.
fn fetch_static() -> Forecast {
    Forecast {
        temp: "23°".into(),
        feels: "24°".into(),
        code: 2,
        hi: "27°".into(),
        lo: "19°".into(),
        humidity: "68%".into(),
        wind: "9 km/h".into(),
        days: (0..6)
            .map(|i| {
                (
                    ["Today", "Tue", "Wed", "Thu", "Fri", "Sat"][i].to_string(),
                    "27°".to_string(),
                    "19°".to_string(),
                    if i % 2 == 0 { 2 } else { 61 },
                )
            })
            .collect(),
        ok: true,
    }
}

fn fetch(city: usize) -> Forecast {
    let (_, lat, lon) = CITIES[city.min(CITIES.len() - 1)];
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m\
         &daily=weather_code,temperature_2m_max,temperature_2m_min\
         &timezone=auto&forecast_days=6"
    );

    let f = |path: &str, idx: i32, suffix: &str| net::fetch_fmt(&url, path, idx, suffix);
    let n = |path: &str, idx: i32| net::fetch_num(&url, path, idx);

    let mut days = Vec::new();
    for i in 0..6 {
        let name = if i == 0 {
            "Today".to_string()
        } else {
            net::fetch_weekday(&url, "daily.time", i).unwrap_or_else(|| "—".into())
        };
        days.push((
            name,
            f("daily.temperature_2m_max", i, "°"),
            f("daily.temperature_2m_min", i, "°"),
            n("daily.weather_code", i).unwrap_or(-1.0) as i64,
        ));
    }

    let code = n("current.weather_code", -1).unwrap_or(-1.0) as i64;
    Forecast {
        temp: f("current.temperature_2m", -1, "°"),
        feels: f("current.apparent_temperature", -1, "°"),
        code,
        hi: f("daily.temperature_2m_max", 0, "°"),
        lo: f("daily.temperature_2m_min", 0, "°"),
        humidity: f("current.relative_humidity_2m", -1, "%"),
        wind: f("current.wind_speed_10m", -1, " km/h"),
        days,
        // `fetch_fmt` yields "--" when a field is missing, which is how a
        // failed fetch shows up. Say so rather than presenting dashes as data.
        ok: !f("current.temperature_2m", -1, "").starts_with("--"),
    }
}

/// The page. No script, no external assets — everything it needs is inline, so
/// it cannot make a second network request behind the app's back.
fn page(city: usize, w: f32, h: f32) -> String {
    page_with(fetch(city), city, w, h)
}

fn page_with(fc: Forecast, city: usize, w: f32, h: f32) -> String {
    let (label, icon) = wmo(fc.code);
    let (name, lat, lon) = CITIES[city.min(CITIES.len() - 1)];

    let rows: String = fc
        .days
        .iter()
        .map(|(day, hi, lo, code)| {
            let (_, ic) = wmo(*code);
            format!(
                "<div class=r><span class=d>{day}</span><span class=i>{ic}</span>\
                 <span class=lo>{lo}</span><span class=bar></span><span class=hi>{hi}</span></div>"
            )
        })
        .collect();

    let banner = if fc.ok {
        String::new()
    } else {
        "<div class=err>No data — the forecast request did not return</div>".to_string()
    };

    // What the Splash VM computes over. It used to be a hardcoded literal,
    // which exercised the bridge but proved nothing with it: the answer was the
    // same 29° for every city, including ones whose forecast peaked at 37°. Fed
    // the card's own highs, the number has to agree with the right-hand column
    // of the list above it, so a wrong answer is visible rather than plausible.
    let highs: String = fc
        .days
        .iter()
        .filter_map(|(_, hi, _, _)| hi.trim_end_matches('°').parse::<i64>().ok())
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");

    format!(
        r#"<!doctype html><html><head><meta charset=utf-8>
<meta name=viewport content="width={w:.0}, initial-scale=1">
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{width:{w:.0}px;height:{h:.0}px;overflow:hidden;
 font:400 16px/1.3 -apple-system,'HarmonyOS Sans','Helvetica Neue',sans-serif;
 color:#fff;background:linear-gradient(180deg,#2b5876 0%,#4e4376 55%,#1c1c2e 100%)}}
.wrap{{height:100%;display:flex;flex-direction:column;padding:18px 20px}}
.city{{font-size:15px;letter-spacing:.4px;opacity:.85;text-align:center}}
.now{{font-size:76px;font-weight:200;text-align:center;line-height:1;margin:6px 0 2px}}
.cond{{text-align:center;font-size:14px;opacity:.85}}
.hilo{{text-align:center;font-size:13px;opacity:.7;margin-top:2px}}
.card{{margin-top:14px;padding:12px 14px;border-radius:16px;
 background:rgba(255,255,255,.12);backdrop-filter:blur(18px);
 border:1px solid rgba(255,255,255,.14)}}
.r{{display:flex;align-items:center;gap:10px;padding:7px 0;font-size:14px}}
.r+.r{{border-top:1px solid rgba(255,255,255,.10)}}
.d{{width:64px;opacity:.9}}
.i{{width:24px;text-align:center}}
.lo{{width:38px;text-align:right;opacity:.6}}
.hi{{width:38px;text-align:right}}
.bar{{flex:1;height:4px;border-radius:2px;
 background:linear-gradient(90deg,#7ec8ff,#ffd479)}}
.grid{{display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-top:12px}}
.tile{{padding:10px 12px;border-radius:14px;background:rgba(255,255,255,.10);
 border:1px solid rgba(255,255,255,.12)}}
.k{{font-size:10px;letter-spacing:1px;opacity:.6;text-transform:uppercase}}
.v{{font-size:20px;font-weight:300;margin-top:2px}}
.err{{margin-top:10px;padding:8px 12px;border-radius:10px;font-size:12px;
 background:rgba(255,90,90,.25);border:1px solid rgba(255,120,120,.4)}}
/* The bridge diagnostics, as compact rows rather than one tile each: five
   full-size tiles do not fit a fixed-height page, and the first thing to slide
   off the bottom would have been the one reporting the capability gate. */
.br{{display:flex;justify-content:space-between;gap:10px;padding:5px 0;
 font-size:13px;line-height:1.35}}
.br+.br{{border-top:1px solid rgba(255,255,255,.10)}}
.bk{{opacity:.55;white-space:nowrap}}
.bv{{text-align:right}}
</style></head><body><div class=wrap>
<div class=city>{name}</div>
<div class=now>{temp}</div>
<div class=cond>{icon} {label}</div>
<div class=hilo>H:{hi}  L:{lo}</div>
{banner}
<div class=card>{rows}</div>
<div class=grid>
 <div class=tile><div class=k>Feels like</div><div class=v>{feels}</div></div>
 <div class=tile><div class=k>Humidity</div><div class=v>{humidity}</div></div>
 <div class=tile><div class=k>Wind</div><div class=v>{wind}</div></div>
 <div class=tile><div class=k>Source</div><div class=v style="font-size:13px">Open-Meteo</div></div>
</div>
<div class=tile style="margin-top:12px">
 <div class=k>JS &#8594; Rust bridge</div>
 <div class=br><span class=bk>device.info</span><span class="bv" id=bridge>&#8230;</span></div>
 <div class=br><span class=bk>splash.eval</span><span class="bv" id=vm>&#8230;</span></div>
 <div class=br><span class=bk>http.get</span><span class="bv" id=net>&#8230;</span></div>
 <div class=br><span class=bk>echo</span><span class="bv" id=echo>&#8230;</span></div>
 <div class=br><span class=bk>gate</span><span class="bv" id=gate>&#8230;</span></div>
</div>
<script>
// Not decoration: this is the page proving on-device that it can call into
// Rust and get an answer back. If the bridge regresses, the card says so.
(function () {{
  var el = document.getElementById('bridge');
  if (!window.splash || !splash.available()) {{
    // Say it on every row. Leaving four of them on the ellipsis would read as
    // "still running" when the answer is already known.
    ['bridge', 'vm', 'net', 'echo', 'gate'].forEach(function (id) {{
      document.getElementById(id).textContent = 'no bridge';
    }});
    return;
  }}
  splash.invoke('device.info').then(function (i) {{
    el.textContent = i.platform + ' \u00b7 slot ' + i.slot;
  }}).catch(function (e) {{ el.textContent = 'failed: ' + e.message; }});

  // The page runs Splash. `input` is JSON this page supplied; the script
  // computes over it and the result comes back as JSON. Same language that
  // describes the native widgets around this webview.
  var vmEl = document.getElementById('vm');
  // The card's own six highs, so the answer must match the right-hand column
  // of the list above. Empty only while the forecast is still in flight.
  var temps = [{highs}];
  if (!temps.length) {{
    vmEl.textContent = 'waiting for forecast';
  }} else {{
  splash.invoke('splash.eval', {{
    // Canonical Splash wants one statement per line -- the one-liner form
    // of this was rejected at line 2 col 43, which is exactly the located
    // error splash-core gives that the bare VM did not.
    source: [
      'let hi = 0',
      'for t in input.temps {{',
      '  if t > hi {{',
      '    hi = t',
      '  }}',
      '}}',
      'hi'
    ].join('\n'),
    input: {{ temps: temps }}
  }}).then(function (r) {{
    vmEl.textContent = 'peak ' + r + '\u00b0 of ' + temps.length;
  }}).catch(function (e) {{ vmEl.textContent = 'failed: ' + e.message; }});
  }}

  // http.get's SUCCESS path. Everything else here only ever watched the gate
  // say no, and a deny-all bug is indistinguishable from a working allowlist
  // when every test expects a refusal -- so this asks for a host that is ON
  // the allowlist and requires it to come back.
  //
  // The page fetches the same current temperature the card already shows,
  // which Rust obtained separately through its own net path. Agreement means
  // the whole chain ran: page JS -> splash_native -> Rust -> network -> reply
  // queue -> runJavaScript -> promise. A number that merely arrives would only
  // prove something came back; a number that MATCHES proves it is the right
  // something.
  var netEl = document.getElementById('net');
  var shown = '{temp}';
  splash.invoke('http.get',
    'https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}'
    + '&current=temperature_2m&timezone=auto')
    .then(function (body) {{
      var t = JSON.parse(body).current.temperature_2m;
      var mine = Math.round(t) + '°';
      netEl.textContent = mine + (mine === shown ? ' ✓ matches card' : ' ≠ card ' + shown);
    }})
    .catch(function (e) {{ netEl.textContent = 'FAILED: ' + e.message; }});

  // The cheapest possible round trip, so a bridge that is up but returning
  // nothing useful is still distinguishable from one that is down.
  //
  // No JSON.parse here: the shim passes a string arg through raw rather than
  // stringifying it, so `echo` gets `ping-0` and not `"ping-0"`, and the reply
  // is already the decoded string. Parsing it threw on the first run.
  var echoEl = document.getElementById('echo');
  splash.invoke('echo', 'ping-{city}')
    .then(function (r) {{
      echoEl.textContent = r === 'ping-{city}' ? 'ping-{city} ✓' : 'MISMATCH: ' + r;
    }})
    .catch(function (e) {{ echoEl.textContent = 'FAILED: ' + e.message; }});

  // `log` has no visible result by design -- its entire effect is a line in
  // hilog, so a row on the card could only ever report that the promise
  // settled, not that the logging happened. Called here so the last tool has a
  // passing test at all; the check is on the host:
  //
  //   hdc shell hilog | grep 'page: card ready'
  //
  // which verifies the side effect rather than the acknowledgement.
  splash.invoke('log', 'card ready: {name}');

  // The gate should refuse these three even from a trusted page. If any of
  // them succeeds, the allowlist or the SSRF guard has regressed.
  var gEl = document.getElementById('gate');
  var probes = [
    ['off-allowlist', 'https://example.com/'],
    ['loopback', 'https://127.0.0.1/'],
    ['plain http', 'http://api.open-meteo.com/']
  ];
  Promise.all(probes.map(function (p) {{
    return splash.invoke('http.get', p[1])
      .then(function () {{ return p[0] + ':ALLOWED'; }})
      .catch(function () {{ return p[0] + ':blocked'; }});
  }})).then(function (rs) {{
    var bad = rs.filter(function (r) {{ return r.indexOf('ALLOWED') >= 0; }});
    gEl.textContent = bad.length ? 'LEAK ' + bad.join(' ') : 'all 3 blocked';
  }});
}})();
</script>
</div></body></html>"#,
        w = w,
        h = h,
        name = name,
        temp = fc.temp,
        icon = icon,
        label = label,
        hi = fc.hi,
        lo = fc.lo,
        banner = banner,
        rows = rows,
        highs = highs,
        lat = lat,
        lon = lon,
        city = city,
        feels = fc.feels,
        humidity = fc.humidity,
        wind = fc.wind,
    )
}

/// Native chrome: a city switcher built from real ArkUI widgets, above a web
/// surface that renders the forecast.
pub fn build(city: usize) -> Option<Node> {
    let city = city.min(CITIES.len() - 1);
    let mut root = col(W, PAGE_H, CHROME)?;

    let bar_h = 46.0;
    let mut bar = row(W, bar_h, CHROME)?;
    for (i, (name, _, _)) in CITIES.iter().enumerate() {
        let c = if i == city { ACCENT } else { SUBTLE };
        let mut t = tap_row(W / CITIES.len() as f32, bar_h, CHROME, CITY_BASE + i as i32)?;
        t = t.child(text(name, 12.0, c, W / CITIES.len() as f32 - 4.0, 18.0)?);
        bar = bar.child(t);
    }
    root = root.child(bar);

    let body_h = PAGE_H - bar_h;
    // ISOLATION: smallest possible page. If this shows, the delivery mechanism
    // works and the real page is at fault; if it stays white, delivery is.
    // Never fetch here -- see CACHE. Ask for the data, draw whatever is ready.
    prefetch(city);
    let fc = cached(city).unwrap_or_else(pending);
    let html = page_with(fc, city, W, body_h);
    root = root.child(web_html(html, 0.0, bar_h, W, body_h)?);
    Some(root)
}

/// Unused today, but the reason the palette constants are here: the native
/// twin of this card lives in `assets/weather.splash`.
#[allow(dead_code)]
const _NATIVE_TWIN: (&str, u32) = ("assets/weather.splash", CREAM);
