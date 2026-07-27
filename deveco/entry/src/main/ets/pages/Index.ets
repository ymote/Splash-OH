import { NodeContent, typeNode } from '@kit.ArkUI';
import { systemDateTime } from '@kit.BasicServicesKit';
import splash from 'libsplash_oh.so';

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

  onPageShow(): void {
    // Let the first frame settle before measuring, so we are not timing
    // against the app's own startup layout.
    setTimeout(() => {
      this.benchArkTs();
      splash.runBridgeBench();
    }, 3000);
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

  /** Measurement B: same shape as Rust's — warm up, five trials, report all. */
  private benchArkTs(): void {
    const N = 2000;
    this.trial(128); // warm up: the first loop pays JIT
    const trialsMs: Array<number> = new Array<number>();
    for (let t = 0; t < 5; t++) {
      trialsMs.push(this.trial(N) / 1000);
    }
    splash.reportArkts(N, trialsMs);
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
