# Rust vs ArkTS: Widget Construction on OpenHarmony

*[中文版](CONCLUSION.zh-CN.md)*

Measured on a SUP-AL90 (HarmonyOS 6.1), release build. Reproduce it by opening
**Performance** in the app, or by reading `hilog`. Full method in
[`crates/splash-oh/src/bench.rs`](crates/splash-oh/src/bench.rs).

## The number

**Rust through the ArkUI NDK is ~2.6–3.2× faster than ArkTS at building
widgets.** Not 45×. The original claim in this repo was wrong because one side
was measured and the other was *projected* from a constant borrowed out of a
different app.

```
A  Rust → ArkUI NDK    ~17 µs/node
B  ArkTS → typeNode    ~52 µs/node
```

Both paths create the **same** native node — `typeNode.createNode(ctx,'Text')`
and `createNode(ARKUI_NODE_TEXT)` both land on the same `TextPattern` inside
libace — with the same four attributes. 2000 nodes, ten trials, six runs.

That equivalence is what makes the comparison fair: layout, measure and paint
are identical native code afterwards and cancel out, so construction is the only
stage where the two differ, and construction is what this isolates.

## Where the difference comes from

| per call | Rust | ArkTS | ratio |
|---|---|---|---|
| `createNode` | 14.0 – 17.3 µs | 34.6 – 41.2 µs | 2.1 – 2.6× |
| `setAttribute` | **0.025 µs** | **1.15 µs** | **46×** |
| one JS → native napi call | — | **0.058 µs** | — |
| one empty JS loop iteration | — | 0.017 µs | — |

The bridge is 58 nanoseconds. It is not the problem. Of the 1.15 µs an ArkTS
attribute set costs, about 5% is the boundary; the other 95% is JS-side — the
`attribute` modifier object, boxing the argument, dispatch, validation. Rust's
25 ns is a function-pointer call and a small struct.

The cost is **JS object churn**. `typeNode.createNode` does not just create the
native node: it builds a wrapper object, registers a finalizer, and wires up
cross-language reference tracking. The collector then has to undo all of it.
That also explains the variance — ArkTS trials swing 43–85 µs while Rust's
decomposed numbers are stable to three significant figures.

These two lower rows barely move between runs (1.146 / 1.149 / 1.152 / 1.154 /
1.158 / 1.162 µs for the attribute; 0.058 / 0.059 µs for the crossing), which is
why they carry the argument. The noisy full-node numbers do not have to.

## Not warm-up

Ten trials in order:

```
A  Rust    13.4 12.9 15.5 17.9 16.0 15.9 20.9 24.6 20.5 16.7
B  ArkTS   47.1 81.5 85.3 47.3 47.5 55.5 44.5 50.4 50.0 81.5
```

Warm-up falls monotonically. B does not fall; it oscillates with no trend.

Run back-to-back *without* yielding to the event loop, B instead climbs steadily
— **37.9 → 87.4** — which is a heap filling with 2000 wrapper objects per trial.
Giving the loop room between trials turns the ramp into a bounce. That is
collection, not JIT.

(A drifts upward too, with no allocator in it. That is the device warming; both
sides are interleaved within each tick specifically so it hits them equally.)

## The claim that actually survives

The case for the NDK path was never really raw speed. It is that **napi latency
is a function of how busy the JS thread is**, and building a widget tree is
exactly what makes it busy.

octos-one measured 1.05 ms per round trip, 730 µs of it queue wait. The same
crossing on an idle thread is ~31 µs. Both measurements were real — contention
was mislabelled as bridge cost.

So: ~3× on construction, and — more importantly — a cost that does not degrade
as the UI thread fills up, because it never queues behind it. That is a weaker
claim than this repo started with, and it is the one the evidence supports.

## Memory

Same node, same process, RSS from `/proc/self/status`, sampled around each
phase. 8000 widgets built 1000 per event-loop tick, then released, then ten
samples over four seconds with nothing else allocating.

### Bytes per widget

| | marginal cost per node |
|---|---|
| **Rust → ArkUI NDK** | **~46 KB** |
| **ArkTS → typeNode** | **~50 KB** |

Measured over the 1000→8000 range, so the first chunk's setup is excluded:
`(496 928 − 175 032) / 7000 = 46.0 KB` for Rust, `(534 584 − 187 704) / 7000 =
49.6 KB` for ArkTS.

**ArkTS costs about 8% more — roughly 3.6 KB per widget.** That is the JS
wrapper object, its finalizer and the cross-language reference tracking. Real,
and much smaller than the 2.6–3.2× it costs in *time*. The wrapper is cheap in
bytes and expensive in cycles.

### The number that actually matters

**A single `Text` widget costs ~46 KB resident**, and that is the native ArkUI
node — both paths pay it. It dwarfs everything else here.

```
    2 000 widgets  ≈   92 MB
    8 000 widgets  ≈  350 MB
   30 000 widgets  ≈  1.1 GB
```

At 30 000 the app died. Not from OOM, as it happens — `THREAD_BLOCK_6S`,
because allocating that much in one callback also blocks the UI thread — but
1.1 GB for a widget tree is not a thing you get to do on a phone regardless.

This reframes the timing result more than it supports it. Building 2000 widgets
costs ~40 ms in Rust and ~105 ms in ArkTS: a 65 ms difference, once. Both cost
92 MB, permanently. **A large widget tree is memory-bound long before
construction time is what stops you.** If the reason to pick the NDK path was
"we can afford more widgets", memory says no in both languages.

### Reclaim

| | on release | after 4 s idle |
|---|---|---|
| **Rust** | 42% returned immediately | flat |
| **ArkTS** | **nothing** — RSS rose 21 MB | flat |

Rust `drop`s; ArkTS drops a reference and hopes. Within the measurement window
ArkTS returned none of its ~350 MB, and RSS went *up* slightly on release.

One honest complication: Rust's remaining 58% did come back — but only later,
when the ArkTS phase started allocating. It sat flat at 350 MB for four idle
seconds and then fell to ~163 MB the moment there was demand. So the allocator
holds pages until something wants them, which means **"memory came back" is hard
to prove and "memory did not come back" is easy** — an asymmetry worth
remembering before reading either column as a leak. ArkTS was not given the same
later opportunity, because nothing allocated after it.

### What this does not measure

Whether a forced GC would have reclaimed the ArkTS side (`ArkTools.forceFullGC`
is not available in a release build), what happens over hours rather than
seconds, and mounted widgets — every node here is detached, so none of the
layout, text-shaping or render-node memory a visible tree would add is counted.
The real per-widget figure on screen is higher than 46 KB, on both paths.

## A real app: rebuilding makepad_wechat both ways

The microbenchmark above builds 2000 identical `Text` nodes, which is not an
app. So the six screens of
[project-robius/makepad_wechat](https://github.com/project-robius/makepad_wechat)
were rebuilt twice — once through the ArkUI NDK from Rust
([`wechat.rs`](crates/splash-oh/src/wechat.rs)), once in ArkTS through
`typeNode` ([`WeChatArkTs.ets`](deveco/entry/src/main/ets/pages/WeChatArkTs.ets))
— with the row shapes taken from the reference app: chat preview 6 nodes,
message bubble 4, contact 3, discover entry 4, moment 5.

Both sides build the same tree. Node counts are asserted against a shared
contract at runtime, because "the other version is faster because it quietly
builds less" is the obvious way for a comparison like this to be wrong.

### Result

A tour of all six screens, ~600 nodes total. Median of 4 tours per condition,
after 4 discarded warm-up tours.

| condition | Rust → NDK | ArkTS → typeNode | ratio |
|---|---|---|---|
| **JS thread idle** | 23–27 ms | 78–87 ms | **2.9 – 3.6×** |
| **JS thread under load** | 24–25 ms | 73–90 ms | **3.0 – 3.8×** |

Per screen, the densest one (`chats`, 20 rows, 135 nodes) is **6.5 ms vs
20 ms**. Five runs, ratios clustering 2.4–4.5× per screen.

This lines up with the 2.6–3.2× from the synthetic benchmark, which is
reassuring — a real tree of mixed `Row`/`Column`/`Text`/`Image` nodes behaves
like the microbenchmark said it would.

### Load did not widen the gap, and I expected it to

The reason for building this was a specific prediction: that a super-app's JS
thread is never idle, that the napi round trip degrades badly under exactly that
condition (31 µs idle → 1051 µs busy, measured in octos-one), and that the gap
would therefore blow out to something like 30× rather than 3×.

**It did not.** Under a synthetic super-app load — JSON parse/stringify of
message batches, array churn, promise resolution, every 8 ms — the ratio moved
from ~3.0× to ~3.4×. Within run-to-run noise of no change at all.

The honest reason is a limitation of the harness, not a refutation of the idea:
**both paths run on the JS thread here.** Rust is invoked through napi from
ArkTS, so it queues behind the same load ArkTS does. The real architecture does
not work that way — in Splash-OH a rebuild is driven from the ArkUI event
thread, which never touches the JS loop. Measuring *that* needs the build
triggered off-thread, which this harness does not do.

So the load prediction is still untested. What is now tested is that **the ~3×
is real on a real app's screens**, which is the part that was speculative before.

### The memory arm failed, and the failure is the finding

Stacking 30 whole screens on each path and watching RSS produced this:

| | goes first | goes second |
|---|---|---|
| Rust-first run | Rust **+3613 kB/screen** | ArkTS +691 kB/screen |
| ArkTS-first run | ArkTS **+1861 kB/screen** | Rust **negative** |

Whoever measures second refills pages the first one freed but the allocator
retained, so it looks nearly free — and in one case RSS *fell* while 30 screens
were being built. The numbers track build order, not memory. **This method
cannot measure per-widget memory with both paths in one process**, and flipping
the order was the only way to find that out; either run on its own looks like a
clean result.

The per-node memory figure that does hold is the one from the dedicated
single-path ramp above: **~46 KB per widget, ~8% more in ArkTS**. That worked
because it measured one path, monotonically, from a clean baseline.

### What a super-app should take from this

Roughly 3× on construction, on real screens, confirmed. On one screen that is
14 ms rather than 4 ms — invisible. It becomes real when construction is
continuous: a fling that rebuilds 1000 widgets is ~7 ms in Rust and ~22 ms in
ArkTS, and only one of those fits in a 120 Hz frame.

The stronger argument — that Rust never queues behind a busy JS thread — remains
unmeasured, and this harness is structurally unable to measure it.

## Caveats

- **Measurement C (the napi round trip) does not run** in the current build.
  After the suite was restructured to drive every trial from ArkTS one
  event-loop tick at a time, the JS side stopped acknowledging the worker's
  posts and I have not found why. Its figures in the README are marked as coming
  from the earlier build. The worker now times out and reports it rather than
  parking forever, which is what it used to do.
- **22 of 28 catalog screens are visually unverified.** Same DSL helpers, same
  walker, but only six were inspected on device.
- **Nothing was measured under load**, which is where the contention argument
  would actually be settled. An idle-thread microbenchmark says nothing about it.
- **Only detached widgets were measured for memory.** Nothing was mounted, so
  layout and render-node memory is not in the 46 KB figure.
- Everything about ArkTS's *declarative* path is out of scope. `typeNode` is its
  imperative escape hatch, chosen precisely because it is the apples-to-apples
  control.

## What the measurement bugs cost

Three separate defects, each of which produced a confident, plausible, wrong
number:

1. **A projected constant.** The ArkTS side was never measured — it was
   `1051 µs × widget count`, from a round trip measured in a different app, on
   the untested assumption of one napi crossing per widget.
2. **A subtraction below its own noise floor.** Deriving per-attribute cost as
   (full trial − create-only trial) returned a *negative* number for Rust. The
   signal is under 1 µs; the difference of two 2000-node trials has a noise
   floor several times that. Attributes are now timed directly.
3. **A blocked event loop.** Benchmarking inside `mount()` held the JS thread
   long enough that its timer queue stopped being serviced, so the entire ArkTS
   half of the suite silently never ran — no error, no output, just missing
   results. Every trial is now driven from ArkTS, one per tick.

A fourth, smaller one: ArkTS `static` members declared inside an `@Component`
struct read back as `undefined` at runtime, so the state machine fell straight
through without an error. Module-level `const` instead.
