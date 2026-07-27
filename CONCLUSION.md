# Rust vs ArkTS: Widget Construction on OpenHarmony

*[中文版](CONCLUSION.zh-CN.md)*

Everything below was measured on a SUP-AL90 (HarmonyOS 6.1), release build.
Reproduce with `./build.sh` and read `hilog`; memory needs `./memrun.sh <app> <path>`,
one implementation per process.

---

## The answer

**Building a widget tree from Rust through the ArkUI NDK is ~2.5–3× faster than
building the identical tree from ArkTS through `typeNode`, and costs ~9–18% less
memory.**

That is a smaller claim than this repo originally made, and it is the one the
evidence supports. Two figures have been retracted along the way; both are
documented below rather than quietly corrected.

### What it rests on

Four makepad reference apps, each implemented twice — once in Rust against the
NDK, once in ArkTS against `typeNode` — and toured screen by screen on device:

| app | shape | nodes/tour | idle | under load |
|---|---|---|---|---|
| [WeChat](https://github.com/project-robius/makepad_wechat) | text-heavy lists | 592 | 2.83× / 2.56× | 2.53× / 1.89× |
| [Taobao](https://github.com/project-robius/makepad_taobao) | two-column product grid | 502 | 2.47× / 2.75× | 1.93× / 2.39× |
| [TikTok](https://github.com/project-robius/makepad_tiktok) | full-screen media | 446 | 2.32× / 2.41× | 2.70× / 2.78× |
| [Wonderous](https://github.com/project-robius/makepad_wonderous) | editorial and photos | 234 | 3.05× / 2.49× | 2.34× / 2.59× |

Two independent runs per cell. Four quite different widget mixes — a text list,
a photo grid, a video surface, an editorial page — and they all land in
**2.3–3.1×**.

That band is the most useful thing here. It says **the gap behaves like a
per-node constant**, not something that varies with what the nodes are. One app
could not have told you that.

![taobao](app-taobao.jpeg)
![tiktok](app-tiktok.jpeg)
![wonderous](app-wonderous.jpeg)

### Why the comparison is trustworthy

`typeNode.createNode(ctx, 'Text')` and the NDK's `createNode(ARKUI_NODE_TEXT)`
land on the same C++ `TextPattern` inside libace. Not equivalent nodes — the
same node. So measure, layout and paint are identical native code afterwards
and cancel out; construction is the only stage that differs.

Both implementations read their content from one place (Rust, crossing to ArkTS
over napi) and use the reference apps' own assets. **Node counts are asserted
at runtime**, every screen, every run — no MISMATCH in any run reported here.
"The other one is faster because it quietly builds less" is the obvious way for
a comparison like this to be wrong, so it is checked rather than assumed.

Both also mount into the same `NodeContent`, and a header button swaps which one
drives the app mid-flight. The screen does not visibly change when it does.

---

## Where the difference comes from

Each row measured directly, not derived by subtracting one noisy trial from
another:

| per call | Rust | ArkTS | ratio |
|---|---|---|---|
| `createNode` | 14.0 – 17.3 µs | 34.6 – 41.2 µs | 2.1 – 2.6× |
| `setAttribute` | **0.025 µs** | **1.15 µs** | **46×** |
| one JS → native napi call | — | **0.058 µs** | — |
| one empty JS loop iteration | — | 0.017 µs | — |

### It is not the bridge

A JS → native napi call costs **58 nanoseconds**. Of the 1.15 µs an ArkTS
attribute set costs, the boundary is about 5%; the other 95% is JS-side — the
`attribute` modifier object, boxing, dispatch, validation. Rust's 25 ns is a
function-pointer call and a small struct.

### It is not warm-up

Ten trials in order:

```
A  Rust    12.9 12.3 14.1 16.4 20.2 20.4 20.9 25.4 16.5 21.1
B  ArkTS   53.2 78.0 73.7 50.5 52.7 57.3 43.9 44.9 49.2 83.2
```

Warm-up falls monotonically. B does not fall; it oscillates with no trend. Run
back-to-back *without* yielding to the event loop it instead climbs steadily —
**37.9 → 87.4** — which is a heap filling with 2000 wrapper objects per trial.
Giving the loop room between trials turns the ramp into a bounce. That is
collection, not JIT.

### It is object churn

`typeNode.createNode` does not just create the native node: it builds a JS
wrapper, registers a finalizer, and wires up cross-language reference tracking.
The collector then has to undo all of it. That is the ~21 µs gap on creation,
and the rest of the gap between the decomposition (37 + 5×1.15 ≈ 43 µs) and the
measured full node (~52 µs).

---

## Memory

Fresh process per app per implementation — the contamination is within-process,
and getting this wrong produced one of the two retractions below.

**Steady state** — the app mounted and settled:

Every app, both paths: **~165–170 MB**. Essentially all of it is the ArkUI
runtime floor. **Which language builds the widgets does not change what the
running app costs.**

**Marginal** — the slope of stacking 12 more copies of the whole app:

| app | Rust | ArkTS | overhead |
|---|---|---|---|
| WeChat | 21.5 MB | 24.8 MB | +15% |
| Taobao | 19.5 MB | 21.2 MB | +9% |
| TikTok | 16.8 MB | 19.6 MB | +17% |
| Wonderous | 7.3 MB | 8.6 MB | +18% |

**+9% to +18%** — the JS wrapper and its finalizer, per node. Real, and far
smaller than the 2.5–3× the same wrapper costs in *time*. Cheap in bytes,
expensive in cycles.

### The number that matters more than the ratio

A single `Text` widget costs **~46 KB resident** (29–39 KB in the mixed trees
above, which include cheaper containers). That is the native ArkUI node — both
paths pay it.

```
  2 000 widgets ≈  92 MB
  8 000 widgets ≈ 350 MB
 30 000 widgets ≈ 1.1 GB   ← the app died here
```

Building 2000 widgets is a ~65 ms difference between the two paths, **once**.
Both cost 92 MB, **permanently**. A widget tree hits the memory wall long before
construction time is what stops you, so "we can afford more widgets" is not
something the NDK path buys.

### Reclaim

On release, Rust returned 42% immediately and the rest only later, when
something else started allocating — it sat flat for four idle seconds first.
ArkTS returned nothing inside the same window.

The asymmetry matters before reading that as a leak: the allocator holds freed
pages until something wants them, which makes **"memory came back" hard to prove
and "memory did not come back" easy**. ArkTS was never given the same later
opportunity, because nothing allocated after it.

---

## Two retractions

### 45× → 2.5×

The original claim compared a *measured* Rust cost against a *projected* ArkTS
cost: `1051 µs × widget count`, where 1051 µs was a round trip measured in a
different app, on the untested assumption of one napi crossing per widget.

Measured properly, ArkTS builds the same node in ~60 µs against Rust's ~24 µs.

**And the 1051 µs was itself mislabelled.** The same crossing here is ~31 µs on
an idle thread, and 58 ns in the JS → native direction. In octos-one the JS
thread was busy, and 730 µs of that 1051 was the post sitting in a queue. So:

> **napi is not slow. Waiting behind a busy JS thread is slow.**

That reframes the whole argument. Bridge latency is a function of load, and
building a widget tree *is* load.

### TikTok +50% → +17%

TikTok's memory came out at +50% against +9–18% for the other three, and an
earlier version of this document reported it as an unexplained outlier.

It was a harness bug. `build_route`, used by the timing arm, maps TikTok's
`feed` route to `build_feed()` — all five reels. `set_route`, used by the
*memory* arm, had no `feed` case and fell through to a single reel. Rust held
one reel where the ArkTS twin held five, and the gap was the difference in work,
not in cost.

Worth recording how it surfaced: not from the benchmark, which reported a clean
number, but from asking why one app disagreed with the other three. **An
unexplained result is a defect until proven otherwise.**

---

## What this means in practice

**For an ordinary screen, nothing.** ~2.5× of 8 ms is 20 ms — perceptible if you
go looking, invisible if you do not. Memory is a rounding error next to the
157 MB framework floor.

**Where it becomes real is continuous construction.** A fling that rebuilds a
long list pays the difference every frame, and 20 ms does not fit in a 120 Hz
budget while 8 ms does. A super-app that builds and tears down view hierarchies
all session pays it repeatedly rather than once.

**The strongest argument is still unmeasured.** Rust never queues behind a busy
JS thread — but this harness cannot show that, for a structural reason given
below.

---

## Caveats

- **The load column proves nothing.** The benchmark invokes Rust *through napi
  from ArkTS*, so both paths queue behind the same load generator. The real
  architecture does not work that way — a tap arrives on the ArkUI event thread
  and Rust rebuilds there without touching the JS loop — but measuring that needs
  the build driven from the event thread, which this does not do. The ~3× under
  load is therefore the same measurement as idle, not a second one.
- **Measurement C (the napi round trip) does not run** in the current build. Its
  figures come from an earlier one. The worker now times out and says so rather
  than parking forever.
- **No video.** `ARKUI_NODE_VIDEO` does not exist in the NDK (the same gap as
  `ARKUI_NODE_WEB`), so TikTok renders a poster frame pulled from its own mp4.
  Same substitution on both sides; neither number includes decode.
- **No parallax.** Wonderous's shader-driven header is not ported. Faking it
  would measure the fake.
- **Memory measured on detached trees** for the per-node figure; mounted widgets
  add layout and render-node memory the 46 KB does not include.
- **One device.** SUP-AL90, HarmonyOS 6.1, one SoC.

---

## The part most likely to be reusable

Six measurement defects, each of which produced a confident, plausible, wrong
number:

1. **A projected constant** standing in for a measurement (the 45×).
2. **A subtraction below its own noise floor** — deriving per-attribute cost from
   (full − create-only) returned a *negative* number for Rust.
3. **A blocked event loop** — benchmarking inside `mount()` held the JS thread
   long enough that its timer queue stopped being serviced, so half the suite
   silently never ran. No error, no output, just missing results.
4. **A GC-provoking loop inside the sample window**, inflating the RSS it was
   trying to read.
5. **Asymmetric sample counts** between the two arms — 2 against 5.
6. **Two arms building different things** (the TikTok `feed` route).

Plus three order effects that had to be designed out: conditions running
idle-then-loaded made ArkTS look *faster* under load (JIT warm-up tracking the
condition); both paths building in one tick made whichever went first eat ~10 ms
of the previous tick's teardown, producing per-screen ratios from 1.06× to 8.60×
purely by build order; and in the memory arm, whichever implementation measured
second refilled pages the first had freed, which made it look nearly free and in
one ordering negative.

**Every one was caught by a sanity check, not by the benchmark.** A negative
number that cannot exist. A missing log line. One arm with fewer samples than the
other. One app disagreeing with three others. The benchmark reported all of them
cleanly.

Build the sanity check. The number will look fine either way.
