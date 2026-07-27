import { NodeContent, FrameNode } from '@kit.ArkUI';
import { systemDateTime } from '@kit.BasicServicesKit';
import splash from 'libsplash_oh.so';
import { WeChatArkTs, LoadGenerator, RouteKind } from './WeChatArkTs';
import { Taobao, TikTok, Wonderous } from './AppsArkTs';

/**
 * The whole ArkTS surface of the app.
 *
 * It owns one `NodeContent` slot and decides which of two implementations of
 * the same WeChat demo is mounted into it:
 *
 *   - **Rust → ArkUI NDK.** ArkTS calls `wechatRender()` once; after that the
 *     tree is built, mounted and event-wired by native code, and this file is
 *     not on the path for anything.
 *   - **ArkTS → typeNode.** This file builds the identical tree itself and
 *     mounts it with `NodeContent.addFrameNode`.
 *
 * Both produce the same native nodes from the same data, so tapping the header
 * button swaps the implementation under a running app and the screen should not
 * visibly change. That makes the comparison checkable by eye, not only by
 * numbers.
 *
 * Module scope for the constants below, not `static` members of the struct: an
 * ArkTS `@Component` struct is not a class, and statics declared inside one read
 * back as `undefined` at runtime — which made an earlier version of the
 * benchmark's state machine fall straight through without an error.
 */

/** The four ported apps, toured in turn. Four different widget mixes, because
 *  one app cannot tell you whether the gap is a per-node constant or something
 *  that varies with what the nodes are. */
const APPS: Array<string> = ['wechat', 'taobao', 'tiktok', 'wonderous'];
/** Screens toured per app, as "tab|route". Filled from Rust so both sides
 *  walk exactly the same routes. */
let TOURS: Array<Array<string>> = new Array<Array<string>>();
/** Tours per condition, after warm-up. */
const WC_TOURS: number = 8;
/** Discarded warm-up tours. ArkTS needs several: the first measured run came
 *  in ~40% slower than the same code later, purely from JIT. */
const WC_WARMUP: number = 4;
/** Condition per tour, an ABBA design. Running idle-then-loaded put every
 *  loaded tour later than its idle partner, so continuing JIT warm-up made
 *  "under load" look FASTER than idle — which load does not do. */
const WC_COND: Array<boolean> = [false, true, true, false, true, false, false, true];

/** Skip the benchmark entirely, for looking at the apps.
 *
 *  The suite saturates the JS thread for the better part of a minute, which
 *  starves the toggle poll and makes the app feel dead while it runs. */
const SKIP_BENCH: boolean = false;

/** Memory mode: skip the timing suite and measure RAM only.
 *
 *  One implementation per process launch, because the contamination is
 *  within-process: measuring both in one run showed whoever went second
 *  refilling pages the first freed but the allocator retained, which made it
 *  look nearly free and in one case negative. A fresh process each is the only
 *  way to get a number that means anything. */
const MEM_MODE: boolean = false;
/** Which implementation this build measures. Flip and rebuild for the other. */
const MEM_RUST: boolean = true;
/** Copies of the whole app held at once, for the marginal figure. */
const MEM_COPIES: number = 12;
/** Which app the memory arm measures. Index into APPS. */
const MEM_APP: number = 0;

@Entry
@Component
struct Index {
  private content: NodeContent = new NodeContent();
  private wc: WeChatArkTs | null = null;
  private load: LoadGenerator = new LoadGenerator();
  private useRust: boolean = true;

  aboutToAppear(): void {
    splash.mount(this.content);
    this.wc = new WeChatArkTs(this.getUIContext());
    this.wc.remount = () => { this.mountArkTs(); };
    this.wc.onToggle = () => { this.toggle(); };
    // Rust drives the app first.
    splash.wechatRender();
    // A tap on the header toggle inside the native tree cannot call back into
    // ArkTS directly, so Rust raises a flag and this polls it. Cheap, and it
    // sidesteps the native -> JS path that does not currently work here.
    setInterval(() => {
      if ((splash.wechatTakeToggle() as number) === 1) {
        this.toggle();
      }
    }, 300);
  }

  onPageShow(): void {
    // Let the first frame settle, then run the comparison. Every trial is
    // driven one per event-loop tick: doing the suite inline blocks the JS
    // thread long enough that its timer queue stops being serviced and the
    // rest of the benchmark silently never runs.
    if (MEM_MODE) {
      setTimeout(() => { this.memStep(0); }, 3000);
      return;
    }
    if (SKIP_BENCH) {
      // Still need the ArkTS twins constructed so the toggle can mount them.
      const ctx = this.getUIContext();
      this.tb = new Taobao(ctx);
      this.tk = new TikTok(ctx);
      this.wo = new Wonderous(ctx);
      this.tb.onTap = (t: number) => { this.arkTap(t); };
      this.tk.onTap = (t: number) => { this.arkTap(t); };
      this.wo.onTap = (t: number) => { this.arkTap(t); };
      TOURS = new Array<Array<string>>();
      for (let a = 0; a < APPS.length; a++) {
        TOURS.push(splash.appTour(APPS[a]) as Array<string>);
      }
      // Auto-cycle. Only WeChat's header carries a toggle button; adding one to
      // the other three would change their node counts, and those counts are
      // what proves the two implementations build the same tree. So the cycle
      // is driven by a timer instead of by the UI.
      setInterval(() => { this.toggle(); }, 6000);
      return;
    }
    setTimeout(() => { this.wcStep(0); }, 2500);
  }

  /** Screens held to measure marginal cost. */
  private heldArk: Array<Object> = new Array<Object>();

  /**
   * RAM, measured in two ways.
   *
   * 1. **Steady state** — a fresh process with one implementation mounted and
   *    settled on the chat list. This is what "how much RAM does the app use"
   *    actually means.
   * 2. **Marginal** — then stack `MEM_COPIES` more copies of every screen and
   *    watch RSS climb, which gives the per-screen cost without the allocator
   *    reuse that ruined the earlier two-paths-in-one-process attempt.
   */
  private memStep(i: number): void {
    const which: string = MEM_RUST ? 'rust' : 'arkts';

    if (i === 0) {
      const ctx = this.getUIContext();
      this.tb = new Taobao(ctx);
      this.tk = new TikTok(ctx);
      this.wo = new Wonderous(ctx);
      TOURS = new Array<Array<string>>();
      for (let a = 0; a < APPS.length; a++) {
        TOURS.push(splash.appTour(APPS[a]) as Array<string>);
      }
      if (MEM_RUST) {
        splash.appRender(APPS[MEM_APP]);
      } else {
        this.mountArkTs();
      }
      splash.wechatLog('RAM ' + APPS[MEM_APP] + ' ' + which + ' mounted, settling');
    } else if (i <= 3) {
      splash.wechatLog('RAM ' + APPS[MEM_APP] + ' ' + which + ' steady rss=' + splash.rssKb()
        + 'kB peak=' + splash.peakRssKb() + 'kB');
    } else if (i === 4) {
      splash.wechatLog('RAM ' + APPS[MEM_APP] + ' ' + which + ' stacking ' + MEM_COPIES + ' copies');
    } else if (i <= 4 + MEM_COPIES) {
      // One whole app (all six screens) per tick.
      if (MEM_RUST) {
        splash.appKeepAll(APPS[MEM_APP]);
      } else {
        this.keepAllArkTs();
      }
      if ((i - 4) % 4 === 0) {
        splash.wechatLog('RAM ' + APPS[MEM_APP] + ' ' + which + ' held=' + (i - 4)
          + ' rss=' + splash.rssKb() + 'kB');
      }
    } else if (i === 5 + MEM_COPIES) {
      splash.wechatLog('RAM ' + APPS[MEM_APP] + ' ' + which + ' held=' + MEM_COPIES
        + ' rss=' + splash.rssKb() + 'kB peak=' + splash.peakRssKb() + 'kB');
    } else {
      return;
    }
    setTimeout(() => { this.memStep(i + 1); }, 500);
  }

  /**
   * Build every screen of the measured app once and keep them all alive.
   *
   * The nodes must stay referenced or the cost does not show up in RSS, which
   * is the whole point — an earlier version held only the roots and ArkTS's
   * child wrappers were collected out from under the measurement.
   */
  private keepAllArkTs(): void {
    const tour: Array<string> = TOURS[MEM_APP];
    for (let k = 0; k < tour.length; k++) {
      const parts: Array<string> = tour[k].split('|');
      this.buildArkTs(MEM_APP, parseInt(parts[0]), parts[1]);
      const n = this.arkNode(MEM_APP);
      if (n !== null) {
        this.heldArk.push(n);
      }
      this.clearArkTs(MEM_APP);
    }
  }

  /** The tree the last ArkTS build produced, for the memory arm. */
  private arkNode(app: number): FrameNode | null {
    if (app === 0) {
      return this.wc!.node();
    }
    if (app === 1) {
      return this.tb!.node();
    }
    if (app === 2) {
      return this.tk!.node();
    }
    return this.wo!.node();
  }

  /** Which of the four apps is on screen. */
  private shown: number = 0;

  /**
   * Cycle: rust(app) -> arkts(app) -> rust(next app) -> ...
   *
   * Tapping through it walks all four apps in both implementations, and the
   * screen should not change appearance when it flips between the two
   * implementations of the same app. That is what makes the comparison
   * checkable by eye rather than only by stopwatch.
   */
  private toggle(): void {
    try {
      this.useRust = !this.useRust;
      if (this.useRust) {
        this.detachArkTs();
        this.shown = (this.shown + 1) % APPS.length;
        splash.appRender(APPS[this.shown]);
      } else {
        this.mountArkTs();
      }
    } catch (e) {
      console.info('SPLASHERR toggle ' + e);
    }
  }

  /** Build the ArkTS tree and put it in the slot, replacing whatever is there. */
  /** The ArkTS tree currently in the slot, so it can be taken out again. */
  private mounted: FrameNode | null = null;

  /** Take whichever tree is in the slot out of it.
   *
   *  Both sides add to the same NodeContent, and nothing was removing the
   *  ArkTS node when switching back to Rust — so the two trees stacked and the
   *  screen went blank. Rust detaches its own; this detaches ArkTS's. */
  private detachArkTs(): void {
    if (this.mounted !== null) {
      this.content.removeFrameNode(this.mounted);
      this.mounted = null;
    }
  }

  private mountArkTs(): number {
    splash.wechatDetach();
    this.detachArkTs();
    if (this.tb === null) {
      const ctx = this.getUIContext();
      this.tb = new Taobao(ctx);
      this.tk = new TikTok(ctx);
      this.wo = new Wonderous(ctx);
      this.tb.onTap = (t: number) => { this.arkTap(t); };
      this.tk.onTap = (t: number) => { this.arkTap(t); };
      this.wo.onTap = (t: number) => { this.arkTap(t); };
    }
    const n = this.buildArkTs(this.shown, 0, 'root');
    const node = this.arkNode(this.shown);
    if (node !== null) {
      this.content.addFrameNode(node);
      this.mounted = node;
    }
    return n;
  }

  /** A tap inside an ArkTS tree: update that app's state and re-mount. */
  private arkTap(t: number): void {
    if (t === 0) {
      return;
    }
    try {
    if (this.shown === 1) {
      if (t === 210) {
        this.tb!.detail = -1;
      } else if (t >= 200 && t < 205) {
        this.tb!.tab = t - 200;
        this.tb!.detail = -1;
      } else if (t >= 2000) {
        this.tb!.detail = t - 2000;
      }
    } else if (this.shown === 2) {
      if (t === 310) {
        this.tk!.sheet = false;
      } else if (t >= 300 && t < 302) {
        this.tk!.tab = t - 300;
      } else if (t === 3002) {
        this.tk!.sheet = true;
      }
    } else if (this.shown === 3) {
      if (t >= 400 && t < 404) {
        this.wo!.tab = t - 400;
      } else if (t >= 4000) {
        this.wo!.wonder = (t - 4000) % 4;
      }
    }
    splash.wechatDetach();
    this.detachArkTs();
    this.buildArkTs(this.shown, this.shown === 1 ? this.tb!.tab
      : this.shown === 2 ? this.tk!.tab : this.wo!.tab, 'root');
    const node = this.arkNode(this.shown);
    if (node !== null) {
      this.content.addFrameNode(node);
      this.mounted = node;
    }
    } catch (e) {
      console.info('SPLASHERR arkTap ' + e);
    }
  }

  // --- benchmark ------------------------------------------------------------

  private tb: Taobao | null = null;
  private tk: TikTok | null = null;
  private wo: Wonderous | null = null;

  /** Flat accumulators indexed by app * SCREENS + screen. */
  private rustN: Array<number> = new Array<number>();
  private arkN: Array<number> = new Array<number>();
  private rustL: Array<number> = new Array<number>();
  private arkL: Array<number> = new Array<number>();
  private nodesR: Array<number> = new Array<number>();
  private nodesA: Array<number> = new Array<number>();

  private nowUs(): number {
    return systemDateTime.getTime(true) / 1000;
  }

  /** Screens per app tour. Every app's tour has this many entries. */
  private static readonly SCREENS: number = 6;

  /**
   * Tour every screen of every app on both paths, idle and under load.
   *
   * One path per tick, never both in the same one: building both inside a
   * single callback made whichever went first pay ~10 ms for the previous
   * tick's teardown, which showed up as per-screen ratios ranging 1.06x to
   * 8.60x purely by build order.
   */
  private wcStep(i: number): void {
    const S = 6;
    const A = APPS.length;
    // Two ticks per screen (one per path), all screens, all apps.
    const perTour = A * S * 2;
    const warm = WC_WARMUP * perTour;
    const total = WC_TOURS * perTour;

    if (i === 0) {
      const ctx = this.getUIContext();
      this.tb = new Taobao(ctx);
      this.tk = new TikTok(ctx);
      this.wo = new Wonderous(ctx);
      TOURS = new Array<Array<string>>();
      for (let a = 0; a < A; a++) {
        TOURS.push(splash.appTour(APPS[a]) as Array<string>);
      }
      for (let k = 0; k < A * S; k++) {
        this.rustN.push(0);
        this.arkN.push(0);
        this.rustL.push(0);
        this.arkL.push(0);
        this.nodesR.push(0);
        this.nodesA.push(0);
      }
      splash.wechatLog('warming up (' + WC_WARMUP + ' tours over ' + A + ' apps, discarded)');
    } else if (i <= warm) {
      const n = i - 1;
      if (n === Math.floor(warm / 2)) {
        this.load.start(8);
      }
      const w = n % perTour;
      this.half(Math.floor(w / 2 / S), Math.floor(w / 2) % S, (w % 2) === 0, false, false);
      if (i === warm) {
        this.load.stop();
      }
    } else if (i <= warm + total) {
      const n = i - warm - 1;
      const tour = Math.floor(n / perTour);
      const w = n % perTour;
      const loaded = WC_COND[tour % WC_COND.length];
      // Which path goes first flips every tour, so the "first pays" penalty
      // lands on both sides equally instead of tracking the screen index.
      const rustFirst = (tour % 2) === 0;
      const isRust = ((w % 2) === 0) === rustFirst;
      if (w === 0) {
        if (loaded) {
          this.load.start(8);
        } else {
          this.load.stop();
        }
      }
      this.half(Math.floor(w / 2 / S), Math.floor(w / 2) % S, isRust, loaded, true);
    } else if (i === warm + total + 1) {
      this.load.stop();
      this.report();
      this.detachArkTs();
      this.shown = 0;
      this.useRust = true;
      splash.appRender('wechat');
      return;
    }
    setTimeout(() => { this.wcStep(i + 1); }, 40);
  }

  /** One path building one screen of one app, alone in its tick. */
  private half(app: number, k: number, isRust: boolean, loaded: boolean, record: boolean): void {
    const id = APPS[app];
    const entry: string = TOURS[app][k];
    const parts: Array<string> = entry.split('|');
    const tab: number = parseInt(parts[0]);
    const route: string = parts[1];
    const slot = app * 6 + k;

    let us = 0;
    let nodes = 0;
    if (isRust) {
      const r: Array<number> = splash.appTime(id, tab, route) as Array<number>;
      nodes = r[0];
      us = r[1];
    } else {
      const t0 = this.nowUs();
      nodes = this.buildArkTs(app, tab, route);
      us = this.nowUs() - t0;
      this.clearArkTs(app);
    }
    if (!record) {
      return;
    }
    if (isRust) {
      this.nodesR[slot] = nodes;
      if (loaded) {
        this.rustL[slot] = this.rustL[slot] + us;
      } else {
        this.rustN[slot] = this.rustN[slot] + us;
      }
    } else {
      this.nodesA[slot] = nodes;
      if (loaded) {
        this.arkL[slot] = this.arkL[slot] + us;
      } else {
        this.arkN[slot] = this.arkN[slot] + us;
      }
    }
  }

  /**
   * Build one ArkTS screen and return its node count.
   *
   * Does NOT drop the tree — the caller decides. The benchmark discards it
   * immediately; the toggle needs it to mount. An earlier version cleared here,
   * so the toggle asked for a tree that had just been thrown away and got a
   * blank screen with no error.
   */
  private buildArkTs(app: number, tab: number, route: string): number {
    if (app === 0) {
      this.wc!.tab = tab;
      this.wc!.route = Index.routeOf(route);
      this.wc!.chatId = 1;
      return this.wc!.build();
    }
    if (app === 1) {
      this.tb!.tab = tab;
      this.tb!.detail = route === 'detail' ? 0 : -1;
      this.tb!.build();
      return this.tb!.nodes();
    }
    if (app === 2) {
      this.tk!.tab = tab;
      this.tk!.sheet = route === 'sheet';
      this.tk!.feed = route === 'feed';
      this.tk!.reel = route.startsWith('reel') ? parseInt(route.substring(4)) : 0;
      this.tk!.build();
      return this.tk!.nodes();
    }
    this.wo!.tab = tab;
    this.wo!.wonder = route.length > 1 && route.charAt(0) === 'w'
      ? parseInt(route.substring(1)) % 4 : 0;
    this.wo!.build();
    return this.wo!.nodes();
  }

  /** Drop the tree the last ArkTS build produced. */
  private clearArkTs(app: number): void {
    if (app === 0) {
      this.wc!.clear();
    } else if (app === 1) {
      this.tb!.clear();
    } else if (app === 2) {
      this.tk!.clear();
    } else {
      this.wo!.clear();
    }
  }

  private static routeOf(s: string): RouteKind {
    if (s === 'chat') {
      return RouteKind.Chat;
    }
    if (s === 'moments') {
      return RouteKind.Moments;
    }
    return RouteKind.Root;
  }

  private report(): void {
    const S = 6;
    const T = WC_TOURS / 2;
    for (let a = 0; a < APPS.length; a++) {
      let trN = 0;
      let taN = 0;
      let trL = 0;
      let taL = 0;
      let nodes = 0;
      let mismatch = 0;
      for (let k = 0; k < S; k++) {
        const slot = a * S + k;
        trN += this.rustN[slot] / T / 1000;
        taN += this.arkN[slot] / T / 1000;
        trL += this.rustL[slot] / T / 1000;
        taL += this.arkL[slot] / T / 1000;
        nodes += this.nodesR[slot];
        if (this.nodesR[slot] !== this.nodesA[slot]) {
          mismatch++;
        }
      }
      splash.wechatLog(APPS[a] + ' nodes=' + nodes
        + (mismatch > 0 ? ' MISMATCH=' + mismatch : '')
        + ' | idle rust=' + trN.toFixed(1) + ' arkts=' + taN.toFixed(1)
        + ' (' + (taN / Math.max(trN, 0.001)).toFixed(2) + 'x)'
        + ' | loaded rust=' + trL.toFixed(1) + ' arkts=' + taL.toFixed(1)
        + ' (' + (taL / Math.max(trL, 0.001)).toFixed(2) + 'x)');
    }
  }

  build() {
    Column() {
      ContentSlot(this.content)
    }
    .width('100%')
    .height('100%')
  }
}
