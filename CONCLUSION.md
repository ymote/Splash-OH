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
