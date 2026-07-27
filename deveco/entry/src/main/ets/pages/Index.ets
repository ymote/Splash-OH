import { NodeContent, typeNode } from '@kit.ArkUI';
import { systemDateTime } from '@kit.BasicServicesKit';
import splash from 'libsplash_oh.so';

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
    setTimeout(() => { this.memStep(0); }, 2000);
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
      // Timing suite second, so a stall in it cannot hide the memory numbers.
      setTimeout(() => { this.step(0); }, 1000);
      return;
    }
    setTimeout(() => { this.memStep(i + 1); }, 400);
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
