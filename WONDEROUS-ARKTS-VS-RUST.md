# Wonderous: ArkTS vs Rust, measured

Both arms build the same ten screens and mount them into the same ArkUI tree.
`crates/splash-oh-native/src/wonders/` constructs from Rust through the NDK;
`deveco/entry/src/main/ets/pages/WonderousArkTs.ets` constructs from ArkTS
through `typeNode`. Both land on the same C++ patterns inside libace, so
measure, layout and paint are identical native code in both cases and cancel
out. Construction is the only stage that differs, and it is what this measures.

Data comes from tables generated out of the Rust arm's own, and the assets are
the same rawfiles, so neither side is rendering different content.

HUAWEI Mate 70 Air (SUP-AL90), 406×805 vp page at ratio 3.25. Two warm-up
passes discarded — the first pass cost 10 ms a screen against 2 ms warm, which
is the interpreter starting up rather than the work.

| screen | ArkTS ms | ArkTS nodes | Rust ms | Rust nodes |
|---|---|---|---|---|
| intro | 0.73 | 7 | 0.33 | 17 |
| home | 5.46 | 75 | 4.65 | 129 |
| editorial | 3.16 | 51 | 1.70 | 61 |
| photos | 3.33 | 43 | 1.57 | 59 |
| artifacts | 1.79 | 27 | 2.28 | 66 |
| events | 2.76 | 48 | 1.22 | 56 |
| menu | 3.05 | 48 | 1.79 | 77 |
| collection | 3.65 | 51 | 1.28 | 58 |
| timeline | 2.56 | 37 | 1.17 | 50 |
| search | 2.80 | 40 | 1.68 | 63 |
| artifact | 0.86 | 12 | 0.32 | 13 |
| **total** | **30.14** | **439** | **17.98** | **649** |

## What can and cannot be concluded

**The totals are not a ratio.** The two arms do not yet build the same number
of nodes — 439 against 649 — so 30.14 against 17.98 compares different amounts
of work. The ArkTS arm is the thinner of the two: its home screen mounts eight
illustrations where the Rust one mounts eight plus their backgrounds, and
several screens elide detail the Rust arm draws.

**Per node is defensible**, as long as the node mix is broadly similar, and it
is — both are mostly Text, Image, Column, Row and Stack:

* ArkTS: 30.14 ms / 439 = **68.7 µs per node**
* Rust: 17.98 ms / 649 = **27.7 µs per node**
* **2.5×**

That figure agrees with this repo's own microbenchmark, which builds one node
type N times through both paths and has long reported ~2.5× on construction.
Two independent measurements landing on the same number is worth more than
either alone.

## What has not been done

The ArkTS screens have been built and disposed, not put on screen. They are
measured, not seen. Node counts still differ, so before quoting a total ratio
the ArkTS arm needs bringing up to the Rust arm's tree — the home screen's
per-wonder backgrounds first, since that is most of the 210-node gap.
