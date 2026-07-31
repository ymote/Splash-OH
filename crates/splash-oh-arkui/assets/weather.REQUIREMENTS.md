# Weather card — generation spec (Splash-OH)

`weather.splash` is **LLM-generated from this spec**, not hand-authored. To
regenerate or restyle it, hand this document (plus the framework references
below) to an LLM and have it write `crates/splash-oh-webview/assets/weather.splash`.
The card is then evaluated by the `makepad-script` VM and walked into native
OpenHarmony ArkUI widgets by `src/dsl.rs`.

## Framework references the generator should read
- `src/dsl.rs` — the renderer walk: the authoritative list of supported node
  types and attribute names, and where the `fetch_*` / image capabilities are wired.
- `src/net.rs` — exact behaviour of `fetch_num` / `fetch_fmt` / `fetch_weekday`.
- `assets/catalog.splash` — a large example of DSL style (helpers, loops, node literals).

## The DSL (makepad-script)
- A UI node is a plain object: `{t: "<type>", <attrs...>, c: [<children>]}`.
- Language: `fn`, `let`, `if/else`, `while`, `for x in arr`, arrays with `.push()`, `return`.
- **String `+` concatenates** and casts numbers: `"H " + 90 + "%"` → `"H 90%"`.
- **Colours:** hex literals evaluate to 0 — always compose ARGB with
  `fn argb(a,r,g,b){ return ((a*256+r)*256+g)*256+b }`. Alpha < 255 = translucent.

## Supported node types (ONLY these; anything else silently drops)
column, row, stack, scroll, list, grid, text, image, button, toggle, checkbox,
radio, slider, progress, loading, input, textarea, datepicker, timepicker,
textpicker, swiper, waterflow, refresh

## Supported attributes
`t`, `text`, `size`, `weight` (1–9), `color` (argb), `bg` (argb), `w`, `h`,
`radius`, `pad`, `margin`, `align` (a `scroll` page wants `align: 1` = TOP),
`src` (image source), `fit` (0=CONTAIN, 1=COVER), `on`, `tap`.

## Hard layout rules
- Native ArkUI never auto-sizes. **Every `text` node needs an explicit `h`**
  (one line ≈ `size*1.3 + 8`). Containers may omit `h` to hug their content.
- A `column` centres children horizontally → give hero lines a snug `w`.
- No `margin` for rhythm; use spacer nodes `{t:"column", w:6, h:<gap>}`.
- A row's child widths should sum to the row's content width.

## Live data — NOTHING baked; fetch at render time
Injected DSL globals (responses cached per-URL → ~2 HTTP requests total):
- `fetch_num(url, path, i)` → number at a dotted JSON path; `i>=0` appends one
  array index (per-day forecast), else `-1`. nil if missing.
- `fetch_fmt(url, path, i, suffix)` → number rounded + suffix, e.g. `"22°"`, `"89%"`.
- `fetch_weekday(url, path, i)` → `"Tue"` for the ISO date at that path.

Sources (Open-Meteo, keyless, Tokyo lat 35.68 lon 139.76):
```
WX = https://api.open-meteo.com/v1/forecast?...current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,dew_point_2m,surface_pressure&daily=weather_code,temperature_2m_max,temperature_2m_min,uv_index_max&timezone=Asia%2FTokyo&forecast_days=7
AQ = https://air-quality-api.open-meteo.com/v1/air-quality?...current=us_aqi,pm2_5,pm10,ozone,nitrogen_dioxide&timezone=Asia%2FTokyo
```
Paths: `current.<field>`; daily arrays `daily.time|weather_code|temperature_2m_max|temperature_2m_min|uv_index_max` indexed 0..6; AQ `current.us_aqi|pm2_5|pm10|ozone|nitrogen_dioxide`.

## Background — live satellite map from the internet
ArkUI `image` loads network `https://` URLs. Full-bleed satellite image as a fixed
background inside a `stack` root:
```
SAT = https://server.arcgisonline.com/arcgis/rest/services/World_Imagery/MapServer/export?bbox=139.60,35.53,139.90,35.85&bboxSR=4326&size=760,1360&format=jpg&f=image
```

## Weather icons — bundled SVGs via `image` `src`
`resource://RAWFILE/weather/{sunny,mostly,partly,cloudy,fog,drizzle,rain,storm}.svg`.
Provide `wmo_icon(code)` + `wmo_word(code)`. WMO buckets: 0 clear(sunny) · 1
mainly-clear(mostly) · 2 partly · 3 overcast(cloudy) · 45/48 fog · 51–57 drizzle
· 61–67/80–82 rain · 71–77/85–86 snow(cloudy) · 95/96/99 storm.

## The card
Root `stack` (≈402×900): satellite image (fit COVER) → dark scrim column
(bg alpha ~120) → scroll (align 1) whose content column holds:
1. **Hero** (centred over the map): "Tokyo"; big temp (~size 74); condition word;
   a `+`-built summary "Feels …° · H …% · Wind … km/h".
2. **7-DAY FORECAST** panel (translucent, radius ~22): section label + a `while i<7`
   loop of rows — weekday · 40×40 icon · word · max · min; hairline dividers.
3. **AIR QUALITY** panel: big US AQI + `aqi_word(a)` category (≤50 Good, ≤100
   Moderate, ≤150 Unhealthy for sensitive, ≤200 Unhealthy, ≤300 Very unhealthy,
   else Hazardous) + 2×2 tiles PM2.5/PM10/Ozone/NO₂ (` µg/m³`).
4. **DETAILS** panel: tiles Humidity, Wind, Feels like, Dew point, UV index
   (`daily.uv_index_max` i=0), Pressure (` hPa`).
Panels translucent (bg alpha ~165) so the map reads through; text white/light.

## Self-check before finishing
Every `t:` is in the supported set; every `text` has an explicit `h`; every value
comes from a `fetch_*` call; braces/brackets balance; the file's final expression
is the root `stack` (no trailing `let`).
