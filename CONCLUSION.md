# Rust vs ArkTS: Widget Construction on OpenHarmony

Measured on a SUP-AL90 (HarmonyOS 6.1), release build. Reproduce it by opening
**Performance** in the app, or by reading `hilog`. Full method in
[`crates/splash-oh/src/bench.rs`](crates/splash-oh/src/bench.rs).

[中文版本在下面](#rust-和-arkts-建控件谁快)

---

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

---

# Rust 和 ArkTS 建控件谁快

在 SUP-AL90（HarmonyOS 6.1）上实测，release 构建。想复现的话，在 app 里打开
**Performance** 页，或者直接看 `hilog`。方法都写在
[`crates/splash-oh/src/bench.rs`](crates/splash-oh/src/bench.rs) 里。

## 结果

Rust 走 ArkUI NDK 比 ArkTS 快 2.6 到 3.2 倍，不是之前说的 45 倍。之前那个数是错的：
只实测了 Rust 这边，ArkTS 那边是拿另一个项目的常数推算出来的。

```
A  Rust → ArkUI NDK    ~17 µs/节点
B  ArkTS → typeNode    ~52 µs/节点
```

两边建的是同一个原生节点。`typeNode.createNode(ctx,'Text')` 和 NDK 的
`createNode(ARKUI_NODE_TEXT)` 最后都走到 libace 里同一个 `TextPattern`，属性也一样是
四个。2000 个节点，每次跑 10 轮，一共测了 6 次。

这个"同一个节点"很关键：后面的布局、测量、绘制两边是同一份原生代码，可以约掉，
所以只有"建"这一步有差别，这次测的也就是这一步。

## 差在哪

| 每次调用 | Rust | ArkTS | 倍数 |
|---|---|---|---|
| `createNode` | 14.0 – 17.3 µs | 34.6 – 41.2 µs | 2.1 – 2.6× |
| `setAttribute` | **0.025 µs** | **1.15 µs** | **46×** |
| 一次 JS → native napi 调用 | — | **0.058 µs** | — |
| 一次空 JS 循环迭代 | — | 0.017 µs | — |

桥本身只有 58 纳秒，不是瓶颈。ArkTS 设一个属性要 1.15 µs，其中过桥只占 5%，
剩下 95% 都在 JS 这边：`attribute` 那个 modifier 对象、参数装箱、分发、校验。
Rust 那 25 纳秒就是一次函数指针调用加填一个小结构体。

真正费时间的是 **JS 对象**。`typeNode.createNode` 除了建原生节点，还要建包装对象、
注册 finalizer、维护跨语言引用，之后 GC 还得把这些收掉。波动也是这么来的：
ArkTS 一轮一轮之间在 43 到 85 µs 之间跳，而 Rust 拆开单独测的那几个数，
三位有效数字都不带变的。

下面这两行在几次运行之间几乎不动（属性是 1.146 / 1.149 / 1.152 / 1.154 / 1.158 /
1.162 µs，过桥是 0.058 / 0.059 µs），所以结论靠它们撑着，上面那些抖得厉害的整节点
数字不用背这个锅。

## 不是预热

按顺序排的十轮：

```
A  Rust    13.4 12.9 15.5 17.9 16.0 15.9 20.9 24.6 20.5 16.7
B  ArkTS   47.1 81.5 85.3 47.3 47.5 55.5 44.5 50.4 50.0 81.5
```

预热的话曲线应该一路往下。B 没往下，就是来回跳，看不出趋势。

但要是不给事件循环留空隙、连着跑完，B 就变成一路往上：**37.9 → 87.4**。
这就是堆里每轮攒 2000 个包装对象的样子。中间给事件循环留出空隙，曲线就从"爬坡"
变成"抖动"。是 GC，不是 JIT。

（A 也在慢慢往上飘，但它这条路上根本没有分配器，那是设备在发热。两边是在同一个
tick 里交替跑的，就是为了让这种影响平摊到双方头上。）

## 那为什么还要用 NDK

其实不是图快。是因为 napi 的延迟跟 JS 线程忙不忙直接相关，而建控件树本身就会把
JS 线程占满。

octos-one 那边测出来一次往返 1.05 ms，其中 730 µs 纯粹是在排队等。同样一次跨越，
线程空着的时候只要 31 µs 左右。两个数都没测错，是我把线程争用当成桥的开销了。

所以：建控件快 3 倍是一方面，更要紧的是它不排队 —— UI 线程越忙，这个差距只会越大。
这个说法比这个仓库一开始的说法弱得多，但它是数据能支撑的那个。

## 局限

- **测量 C（napi 往返）在现在这版跑不起来。** 把整套测试改成由 ArkTS 一个 tick 跑
  一轮之后，JS 那边就不再确认 worker 发过去的消息了，原因还没查出来。README 里标了
  那几个数是旧版测的。worker 现在会超时报错，不会像以前那样一直挂着。
- **28 个页面只看了 6 个**，剩下 22 个没逐个验证。用的是同一套 DSL 辅助函数和同一个
  遍历器，但确实没看。
- **没在负载下测过。** 而"线程争用"这个说法到底成不成立，恰恰得在负载下才能验证，
  空闲状态的 microbenchmark 说明不了问题。
- ArkTS 的**声明式**那套完全没测。`typeNode` 是它的命令式口子，选它就是因为只有它
  能跟 NDK 做等价对比。

## 三个测量上的坑

每一个都给出了看着挺像回事、其实是错的数字：

1. **拿常数推算。** ArkTS 那边压根没测过，是 `1051 µs × 控件数` 算出来的 ——
   那个 1051 还是从另一个 app 里测的往返时间，而且默认了"每个控件过一次桥"这个从没
   验证过的前提。
2. **相减的结果比噪声还小。** 用（完整试验 − 只建节点）去算每个属性的开销，
   Rust 那边算出来是**负数**。信号本身不到 1 µs，而两次 2000 节点试验之差的噪声底
   是它的好几倍。现在属性是单独直接测的。
3. **事件循环被卡住。** 在 `mount()` 里跑 benchmark，把 JS 线程占得太久，定时器队列
   直接不转了，结果整个 ArkTS 那半套测试静默地根本没跑 —— 不报错、不输出，就是没结果。
   现在每一轮都由 ArkTS 驱动，一个 tick 跑一轮。

还有个小的：ArkTS 的 `@Component` struct 里声明的 `static` 成员，运行时读出来是
`undefined`，于是状态机一路穿下去也不报错。改成模块级 `const` 就好了。
