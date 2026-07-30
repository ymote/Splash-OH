# Wonderous: ArkTS vs Rust, measured

Two implementations of the same app. `crates/splash-oh-native/src/wonders/`
constructs the tree from Rust through the ArkUI NDK;
`deveco/entry/src/main/ets/pages/WonderousArkTs.ets` constructs it from ArkTS
through `typeNode`. Both land on the same C++ patterns inside libace, so
measure, layout and paint are identical native code and cancel out.
Construction is the only stage that differs, and it is what this measures.

Both walk the same data — the ArkTS tables are generated from the Rust ones —
and load the same rawfiles, so neither is rendering different content.

HUAWEI Mate 70 Air (SUP-AL90), 406×805 vp at ratio 3.25. Two warm-up passes
discarded, then five timed runs, median reported.

| | ArkTS | Rust |
|---|---|---|
| median, ten screens | **32.17 ms** | **17.15 ms** |
| range over five runs | 31.20–35.33 | 16.57–19.93 |
| nodes | 574 | 649 |
| per node | **56.0 µs** | **26.4 µs** |

**Construction costs about 2.1× more from ArkTS than from Rust.**

The earlier single-sample runs suggested 2.4–2.5×. They were noise: the Rust
total wandered between 13.55 and 17.98 ms across launches. Five runs and a
median put both arms inside a ±10% band, and the ratio settles lower. The
repo's own microbenchmark — one node type, N times, both paths — has long
reported ~2.5×; this is the same effect measured on real screens, and a little
smaller, which is what you would expect once per-screen work that is not node
construction is included in both totals.

## What is the same

Ten screens, plus the fullscreen photograph, the collectible-found screen and
the two web-backed viewers. The illustration rule, the masthead, the 5×5 wall
and its cutout scrim, the carousel's collapse geometry and dots, the paragraph
height estimate, the tap overlay's banding, the swipe threshold and its
eight-way behaviour, the band fade against scroll, the live Met fetch, the
collectibles and where each is hidden.

`wonderous-arkts-vs-rust.png` is the two arms side by side.

## What is not

**Node counts differ: 574 against 649.** The remainder is content the Rust arm
draws that this one does not, screen by screen. It is not a difference in how
either builds a node, so per-node cost is the honest comparison and the total
is indicative rather than exact.

**Neither arm is the Flutter app.** Both are reproductions of it, measured
against each other. Nothing here says what Flutter would cost on the same
device.
