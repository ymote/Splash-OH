import { NodeContent, typeNode } from '@kit.ArkUI';
import { systemDateTime } from '@kit.BasicServicesKit';
import splash from 'libsplash_oh.so';
import { WeChatArkTs, LoadGenerator } from './WeChatArkTs';

// Module scope, not `static` members of the struct: an ArkTS @Component struct
// is not a class, and statics declared inside one read back as undefined at
// runtime — which made the benchmark's state machine fall straight through
// without an error.
const N: number = 2000;
const TRIALS: number = 10;

// Memory ramp. Built in small chunks rather than big jumps: creating 30k nodes
// in one callback blocked the UI thread past THREAD_BLOCK_6S and the system
// killed the app mid-measurement.
const MEM_CHUNK: number = 1000;
const MEM_TARGET: number = 8000;
// How many samples to take after releasing, on each side. Both sides get the
// same number over the same interval — the first version gave Rust two samples
// and ArkTS five, which is not a comparison.
const MEM_SETTLE: number = 10;

// Screen ids and row counts, mirroring SCREENS in crates/splash-oh/src/wechat.rs.
const WC_SCREENS: Array<string> = ['chats', 'chat', 'contacts', 'discover', 'moments', 'me'];
const WC_ROWS: Array<number> = [20, 25, 40, 8, 15, 6];
/** Tours per condition. One tour = all six screens, both paths. */
const WC_TOURS: number = 8;
/** Discarded warm-up tours. ArkTS needs a lot: the first measured run came in
 *  40% slower than the same code later, purely from JIT. */
const WC_WARMUP: number = 4;
/** Condition per tour, an ABBA design. Running I,L,I,L put every loaded tour
 *  later than its idle partner, so continuing JIT warm-up made "under load"
 *  look FASTER than idle — which load does not do. ABBA gives both conditions
 *  the same mean position in the run. */
const WC_COND: Array<boolean> = [false, true, true, false, true, false, false, true];
/** Screens stacked in the memory arm, half on each path. */
const WC_MEM: number = 60;
/** Which path stacks first. Flipped between runs to test whether the memory
 *  numbers are measuring memory or measuring build order — whoever goes second
 *  refills pages the first one freed but the allocator kept. */
const WC_MEM_ARKTS_FIRST: boolean = true;

/**
 * The entire ArkTS surface of this app.
 *
 * It declares one empty slot and hands it to Rust exactly once. Every widget on
 * screen — text, buttons, toggles, sliders, inputs, pickers, containers — is
 * created, configured, laid out and event-wired by native code through the
 * ArkUI NDK. There is no other ArkTS in the app.
 *
 * The rest of this file is the benchmark's ArkTS half, and it is deliberately
 * the same work as the Rust half: `typeNode.createNode(ctx, 'Text')` produces
 * the SAME native node that `createNode(ARKUI_NODE_TEXT)` produces through the
 * NDK. Same node, same four attributes, same count. Only the caller differs,
 * which is the point — everything downstream (measure, layout, paint) is
 * identical native code and cancels out of the comparison.
 */
@Entry
@Component
struct Index {
  private content: NodeContent = new NodeContent();

  aboutToAppear(): void {
    // The one and only ArkTS -> native call the app itself needs.
    splash.mount(this.content);

    // Give Rust a JS function so it can time the native -> JS direction.
    splash.setBridge((n: number) => this.bridgeWork(n));
  }

  private rustFull: Array<number> = new Array<number>();
  private rustCreate: Array<number> = new Array<number>();
  private arktsFull: Array<number> = new Array<number>();
  private arktsCreate: Array<number> = new Array<number>();

  onPageShow(): void {
    // Block body, not `() => this.step(0)`: the concise form silently never
    // fires here.
    setTimeout(() => { this.wcStep(0); }, 2000);
  }

  /**
   * The suite as a state machine, one trial per timer tick.
   *
   * Both sides are driven from here, alternating, so neither monopolises the
   * JS thread. That is not cosmetic: running the suite inline blocks the event
   * loop for seconds, the system watchdog fires at THREAD_BLOCK_6S, and — more
   * insidiously — the timer queue stops being serviced, so the rest of the
   * benchmark silently never runs. Interleaving also means both sides see the
   * same heap and thermal conditions rather than one of them running first on
   * a cold device.
   */
  private step(i: number): void {
    if (i === 0) {
      // Two full-size warm-ups per side: 128 nodes was not enough, and the
      // first 2000-node trial came in at twice the steady-state rate.
      splash.rustWarmup();
      splash.rustTrial(0);
      splash.rustTrial(1);
      this.trial(N);
      this.trialCreateOnly(N);
      this.trialNoop(1000);
      this.trialEmptyLoop(1000);
    } else if (i <= TRIALS) {
      // All four in the same tick, so the create-vs-full subtraction is a
      // paired comparison. Taking it between medians of separately-run batches
      // crossed different heap and thermal states and produced a negative
      // per-attribute cost for Rust — an impossible number, and the tell.
      this.rustFull.push(splash.rustTrial(0));
      this.rustCreate.push(splash.rustTrial(1));
      this.arktsFull.push(this.trial(N) / N);
      this.arktsCreate.push(this.trialCreateOnly(N) / N);
    } else if (i === TRIALS + 1) {
      // The two floors underneath everything: one JS -> native napi call, and
      // one iteration of the same loop without a call in it.
      const M = 20000;
      const noopUs: Array<number> = new Array<number>();
      const emptyUs: Array<number> = new Array<number>();
      for (let t = 0; t < 5; t++) {
        noopUs.push(this.trialNoop(M) / M);
        emptyUs.push(this.trialEmptyLoop(M) / M);
      }
      const empty = this.median(emptyUs);

      // Attributes, measured directly on both sides.
      const K = 50000;
      this.trialAttrOnly(1000);
      const rustAttr: Array<number> = new Array<number>();
      const arktsAttr: Array<number> = new Array<number>();
      for (let t = 0; t < 5; t++) {
        rustAttr.push(splash.rustTrial(2));
        arktsAttr.push(this.trialAttrOnly(K));
      }

      splash.reportAll(
        this.rustFull, this.rustCreate,
        this.arktsFull, this.arktsCreate,
        this.median(rustAttr), this.median(arktsAttr),
        this.median(noopUs) - empty, empty);
      // Measurement C, started here rather than from another tick: the chain
      // did not survive one more hop after reportAll.
      splash.runBridgeBench();
      return;
    } else {
      return;
    }
    setTimeout(() => { this.step(i + 1); }, 80);
  }

  private memNodes: Array<typeNode.Text> = new Array<typeNode.Text>();

  /**
   * Memory ramp, `MEM_CHUNK` widgets per tick.
   *
   * Rust first: build up to MEM_TARGET, sampling RSS as it goes, then drop it
   * all and sample again. Then the identical shape in ArkTS. Both read the same
   * `/proc/self/status` counter in the same process, so there is nothing to
   * normalise between them.
   *
   * The nodes are held between ticks deliberately — a widget's cost only shows
   * up in RSS while something still references it.
   *
   * Chunked for the same reason the timing suite is: one big allocation loop
   * blocks the UI thread long enough for the watchdog to kill the app, and a
   * measurement that gets the process killed measures nothing.
   */
  private memStep(i: number): void {
    const steps = MEM_TARGET / MEM_CHUNK;

    if (i === 0) {
      splash.memLog('baseline', 0);
    } else if (i <= steps) {
      const held: number = splash.memHold(MEM_CHUNK) as number;
      splash.memLog('rust', held);
    } else if (i === steps + 1) {
      splash.memRelease();
      splash.memLog('rust-released', 0);
    } else if (i <= steps + 1 + MEM_SETTLE) {
      // The allocator does not hand pages back at once. Sample repeatedly, and
      // allocate nothing while doing it — the first version churned garbage
      // here to provoke the GC, which inflated the RSS it was trying to read.
      splash.memLog('rust-settled', 0);
    } else if (i <= steps + 1 + MEM_SETTLE + steps) {
      const ctx = this.getUIContext();
      for (let k = 0; k < MEM_CHUNK; k++) {
        const node: typeNode.Text = typeNode.createNode(ctx, 'Text');
        node.initialize('benchmark');
        node.attribute.fontSize(14).fontColor(0xFF1C1B1F).width(200).height(24);
        this.memNodes.push(node);
      }
      splash.memLog('arkts', this.memNodes.length);
    } else if (i === steps + 2 + MEM_SETTLE + steps) {
      this.memNodes.length = 0;
      splash.memLog('arkts-released', 0);
    } else if (i <= steps + 2 + MEM_SETTLE + steps + MEM_SETTLE) {
      // Dropping the references only makes them collectable; when the collector
      // acts is its own business. Same sample count and interval as Rust got,
      // and nothing allocated here either.
      splash.memLog('arkts-settled', 0);
    } else {
      splash.memLog('final', 0);
      return;
    }
    setTimeout(() => { this.memStep(i + 1); }, 400);
  }

  private wc: WeChatArkTs | null = null;
  private load: LoadGenerator = new LoadGenerator();
  /** Accumulated per-screen totals: [rustUs, arktsUs] indexed by screen. */
  private wcRust: Array<number> = new Array<number>();
  private wcArk: Array<number> = new Array<number>();
  /** N = idle, L = under ArkTS-thread load. */
  private wcRustN: Array<number> = new Array<number>();
  private wcArkN: Array<number> = new Array<number>();
  private wcRustL: Array<number> = new Array<number>();
  private wcArkL: Array<number> = new Array<number>();

  /**
   * The WeChat comparison: tour all six screens on both paths, first with the
   * ArkTS thread idle, then with it under a super-app's normal load.
   *
   * The two conditions are the point. On an idle thread the microbenchmark
   * already told us Rust builds widgets ~3x faster, which on one screen is a
   * few milliseconds. The open question was what happens when the JS thread is
   * busy — because that is a super-app's steady state, and it is the condition
   * under which a napi round trip went from 31 us to 1051 us in octos-one.
   *
   * One screen per tick, alternating paths within the tick so both see the same
   * heap and thermal state.
   */
  private wcStep(i: number): void {
    const S = WC_SCREENS.length;
    // Two ticks per screen, one per path. Building both inside one tick made
    // whichever went first pay for the previous tick's teardown — a fixed cost
    // of ~10 ms that swamped the thing being measured, and that showed up as
    // wildly different per-screen ratios depending only on build order.
    const perTour = S * 2;
    const warm = WC_WARMUP * perTour;
    const total = WC_TOURS * perTour;

    if (i === 0) {
      this.wc = new WeChatArkTs(this.getUIContext());
      for (let k = 0; k < S; k++) {
        this.wcRustN.push(0);
        this.wcArkN.push(0);
        this.wcRustL.push(0);
        this.wcArkL.push(0);
      }
      splash.wechatLog('warming up (' + WC_WARMUP + ' tours, discarded)');
    } else if (i <= warm) {
      const n = i - 1;
      if (n === Math.floor(warm / 2)) {
        this.load.start(8);
      }
      this.wcHalf(Math.floor((n % perTour) / 2), (n % 2) === 0, false, false);
      if (i === warm) {
        this.load.stop();
      }
    } else if (i <= warm + total) {
      const n = i - warm - 1;
      const tour = Math.floor(n / perTour);
      const within = n % perTour;
      const loaded = WC_COND[tour % WC_COND.length];
      // Which path goes first flips every tour, so the "first pays" penalty
      // lands on each side equally instead of tracking the screen index.
      const rustFirst = (tour % 2) === 0;
      const isRust = ((within % 2) === 0) === rustFirst;

      if (within === 0) {
        if (loaded) {
          this.load.start(8);
        } else {
          this.load.stop();
        }
      }
      this.wcHalf(Math.floor(within / 2), isRust, loaded, true);
    } else if (i === warm + total + 1) {
      this.load.stop();
      this.wcReport();
    } else if (i <= warm + total + 1 + WC_MEM) {
      // Memory arm: stack up whole screens on one path, then the other, and
      // watch RSS. One screen's delta was pure noise at this granularity; a
      // stack of 30 is not.
      const n = i - warm - total - 2;
      const half = WC_MEM / 2;
      const firstHalf = n < half;
      const doRust = firstHalf !== WC_MEM_ARKTS_FIRST;
      if (n === 0 || n === half) {
        splash.wechatLog('MEM ' + (doRust ? 'rust' : 'arkts')
          + '-baseline rss=' + splash.rssKb() + 'kB');
      }
      if (doRust) {
        const idx = firstHalf ? n : n - half;
        const held: number = splash.wechatKeep(WC_SCREENS[idx % WC_SCREENS.length]) as number;
        if (idx === half - 1) {
          splash.wechatLog('MEM rust ' + held + ' screens rss=' + splash.rssKb() + 'kB');
          splash.wechatDropKept();
          splash.wechatLog('MEM rust-after-drop rss=' + splash.rssKb() + 'kB');
        }
      } else {
        const m = firstHalf ? n : n - half;
        const held: number = this.wc!.keep(WC_SCREENS[m % WC_SCREENS.length],
          WC_ROWS[m % WC_ROWS.length]);
        if (m === half - 1) {
          splash.wechatLog('MEM arkts ' + held + ' screens rss=' + splash.rssKb() + 'kB');
          this.wc!.dropKept();
        }
      }
    } else {
      splash.wechatLog('MEM after-arkts-drop rss=' + splash.rssKb() + 'kB');
      // Widget microbenchmark and the generic memory ramp afterwards — they
      // leave hundreds of MB resident, so nothing that needs a clean heap can
      // run after them.
      setTimeout(() => { this.step(0); }, 500);
      return;
    }
    setTimeout(() => { this.wcStep(i + 1); }, 60);
  }

  /** One path building one screen, alone in its own tick. */
  private wcHalf(k: number, isRust: boolean, loaded: boolean, record: boolean): void {
    const id = WC_SCREENS[k];

    if (isRust) {
      const r: Array<number> = splash.wechatBuild(id) as Array<number>;
      splash.wechatClear();
      if (r[0] !== r[2]) {
        splash.wechatLog('MISMATCH rust ' + id + ': ' + r[0] + ' vs ' + r[2]);
      }
      if (record) {
        if (loaded) {
          this.wcRustL[k] = this.wcRustL[k] + r[1];
        } else {
          this.wcRustN[k] = this.wcRustN[k] + r[1];
        }
      }
    } else {
      const t0 = this.nowUs();
      const n: number = this.wc!.build(id, WC_ROWS[k]);
      const us = this.nowUs() - t0;
      this.wc!.clear();
      if (n !== (splash.wechatBuild(id) as Array<number>)[2]) {
        splash.wechatLog('MISMATCH arkts ' + id + ': ' + n);
      }
      splash.wechatClear();
      if (record) {
        if (loaded) {
          this.wcArkL[k] = this.wcArkL[k] + us;
        } else {
          this.wcArkN[k] = this.wcArkN[k] + us;
        }
      }
    }
  }

  /**
   * Build one screen both ways and time each.
   *
   * Which path goes first alternates with the screen index — whoever builds
   * first pays for whatever the other one left on the heap, and that should not
   * be the same side every time.
   *
   * RSS is deliberately not sampled here. Around a single ~10 ms build the
   * deltas came back dominated by allocator behaviour and the load generator's
   * own garbage, negative as often as not. Memory is measured separately, by
   * holding many screens at once.
   */
  private wcOneScreen(k: number, loaded: boolean): void {
    const id = WC_SCREENS[k];
    const rows = WC_ROWS[k];
    const rustFirst = (k % 2) === 0;

    let rustUs = 0;
    let arkUs = 0;
    let n = 0;
    let r: Array<number> = [0, 0, 0];

    if (rustFirst) {
      r = splash.wechatBuild(id) as Array<number>;
      rustUs = r[1];
      splash.wechatClear();
      const t0 = this.nowUs();
      n = this.wc!.build(id, rows);
      arkUs = this.nowUs() - t0;
      this.wc!.clear();
    } else {
      const t0 = this.nowUs();
      n = this.wc!.build(id, rows);
      arkUs = this.nowUs() - t0;
      this.wc!.clear();
      r = splash.wechatBuild(id) as Array<number>;
      rustUs = r[1];
      splash.wechatClear();
    }

    // r = [built, us, expected]. Both sides must build the same tree, or the
    // comparison is between two different amounts of work.
    if (r[0] !== r[2] || n !== r[2]) {
      splash.wechatLog('MISMATCH ' + id + ': rust=' + r[0] + ' arkts=' + n + ' expected=' + r[2]);
    }

    if (loaded) {
      this.wcRustL[k] = this.wcRustL[k] + rustUs;
      this.wcArkL[k] = this.wcArkL[k] + arkUs;
    } else {
      this.wcRustN[k] = this.wcRustN[k] + rustUs;
      this.wcArkN[k] = this.wcArkN[k] + arkUs;
    }
  }

  private wcReport(): void {
    const S = WC_SCREENS.length;
    const T = WC_TOURS / 2;
    let trN = 0;
    let taN = 0;
    let trL = 0;
    let taL = 0;
    for (let k = 0; k < S; k++) {
      const rn = this.wcRustN[k] / T / 1000;
      const an = this.wcArkN[k] / T / 1000;
      const rl = this.wcRustL[k] / T / 1000;
      const al = this.wcArkL[k] / T / 1000;
      trN += rn;
      taN += an;
      trL += rl;
      taL += al;
      splash.wechatLog(WC_SCREENS[k]
        + ' | idle rust=' + rn.toFixed(2) + ' arkts=' + an.toFixed(2)
        + ' (' + (an / Math.max(rn, 0.001)).toFixed(2) + 'x)'
        + ' | loaded rust=' + rl.toFixed(2) + ' arkts=' + al.toFixed(2)
        + ' (' + (al / Math.max(rl, 0.001)).toFixed(2) + 'x)');
    }
    splash.wechatLog('TOUR idle rust=' + trN.toFixed(1) + 'ms arkts=' + taN.toFixed(1)
      + 'ms (' + (taN / Math.max(trN, 0.001)).toFixed(2) + 'x)'
      + ' | loaded rust=' + trL.toFixed(1) + 'ms arkts=' + taL.toFixed(1)
      + 'ms (' + (taL / Math.max(trL, 0.001)).toFixed(2) + 'x)');
    splash.wechatLog('load penalty: rust ' + (trL / Math.max(trN, 0.001)).toFixed(2)
      + 'x, arkts ' + (taL / Math.max(taN, 0.001)).toFixed(2) + 'x');
  }

  /**
   * Microseconds, monotonic. `Date.now()` is millisecond-resolution wall clock
   * and can step; this is the uptime clock, in nanoseconds.
   */
  private nowUs(): number {
    return systemDateTime.getTime(true) / 1000;
  }

  /**
   * One trial: build `n` Text nodes with four attributes, keeping them all
   * alive so the GC cannot reclaim them mid-loop and flatter the result.
   */
  private trial(n: number): number {
    const ctx = this.getUIContext();
    const sink: Array<typeNode.Text> = new Array<typeNode.Text>();
    const t0 = this.nowUs();
    for (let i = 0; i < n; i++) {
      const node: typeNode.Text = typeNode.createNode(ctx, 'Text');
      node.initialize('benchmark');
      node.attribute
        .fontSize(14)
        .fontColor(0xFF1C1B1F)
        .width(200)
        .height(24);
      sink.push(node);
    }
    const us = this.nowUs() - t0;
    sink.length = 0;
    return us;
  }

  /** Node with no attributes touched, so attribute cost can be subtracted. */
  private trialCreateOnly(n: number): number {
    const ctx = this.getUIContext();
    const sink: Array<typeNode.Text> = new Array<typeNode.Text>();
    const t0 = this.nowUs();
    for (let i = 0; i < n; i++) {
      sink.push(typeNode.createNode(ctx, 'Text'));
    }
    const us = this.nowUs() - t0;
    sink.length = 0;
    return us;
  }

  /**
   * One attribute set `n` times on a single already-created node. Measured
   * directly, not as (full − create-only): the difference of two 2000-node
   * trials has a noise floor of several µs and the signal is under 1 µs, so
   * subtraction produced a negative cost for Rust — impossible, and the tell.
   */
  private trialAttrOnly(n: number): number {
    const node: typeNode.Text = typeNode.createNode(this.getUIContext(), 'Text');
    const attr = node.attribute;
    const t0 = this.nowUs();
    for (let i = 0; i < n; i++) {
      attr.fontSize(14);
    }
    return (this.nowUs() - t0) / n;
  }

  /** One trivial JS -> native napi call, `n` times. The cheap direction. */
  private trialNoop(n: number): number {
    const t0 = this.nowUs();
    for (let i = 0; i < n; i++) {
      splash.noop();
    }
    return this.nowUs() - t0;
  }

  /** The same loop with no crossing at all — the interpreter's own floor. */
  private trialEmptyLoop(n: number): number {
    let acc = 0;
    const t0 = this.nowUs();
    for (let i = 0; i < n; i++) {
      acc = acc + i;
    }
    const us = this.nowUs() - t0;
    if (acc < 0) {
      console.log('unreachable'); // keep the loop from being optimised away
    }
    return us;
  }

  private median(v: Array<number>): number {
    const c = v.slice();
    c.sort((a: number, b: number) => a - b);
    return c[Math.floor(c.length / 2)];
  }

  /**
   * Measurement C's JS half. Rust posts here from a worker thread and blocks
   * until `bridgeDone` is called, so this measures a true round trip and not a
   * fire-and-forget post.
   *
   * `n === 0` means "no work, just redraw" — Rust's way of hopping back onto
   * the JS thread after the worker has finished, since only this thread may
   * touch the tree.
   */
  private bridgeWork(n: number): void {
    // 0xFFFFFFFF: hop onto this thread purely to redraw.
    if (n === 4294967295) {
      splash.benchRefresh();
      return;
    }
    // 0xFFFFFFFE: the control — acknowledge without doing any work, so Rust
    // can measure the bridge on its own and subtract it from the rest.
    if (n === 4294967294) {
      splash.bridgeDone();
      return;
    }
    const ctx = this.getUIContext();
    // Same retention policy as `trial`, so C's JS-side work is exactly B's.
    const sink: Array<typeNode.Text> = new Array<typeNode.Text>();
    for (let i = 0; i < n; i++) {
      const node: typeNode.Text = typeNode.createNode(ctx, 'Text');
      node.initialize('benchmark');
      node.attribute.fontSize(14).fontColor(0xFF1C1B1F).width(200).height(24);
      sink.push(node);
    }
    sink.length = 0;
    splash.bridgeDone();
  }

  build() {
    Column() {
      ContentSlot(this.content)
    }
    .width('100%')
    .height('100%')
  }
}
