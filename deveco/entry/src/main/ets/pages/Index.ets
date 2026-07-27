import { NodeContent } from '@kit.ArkUI';
import { systemDateTime } from '@kit.BasicServicesKit';
import splash from 'libsplash_oh.so';
import { WeChatArkTs, LoadGenerator, RouteKind } from './WeChatArkTs';

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

/** Screens toured by the benchmark, as "tab|route". */
const TOUR: Array<string> = ['0|root', '1|root', '2|root', '3|root', '0|chat', '0|moments'];
/** Tours per condition, after warm-up. */
const WC_TOURS: number = 8;
/** Discarded warm-up tours. ArkTS needs several: the first measured run came
 *  in ~40% slower than the same code later, purely from JIT. */
const WC_WARMUP: number = 4;
/** Condition per tour, an ABBA design. Running idle-then-loaded put every
 *  loaded tour later than its idle partner, so continuing JIT warm-up made
 *  "under load" look FASTER than idle — which load does not do. */
const WC_COND: Array<boolean> = [false, true, true, false, true, false, false, true];

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
    setTimeout(() => { this.wcStep(0); }, 2500);
  }

  /** Swap which implementation owns the surface, mid-flight. */
  private toggle(): void {
    this.useRust = !this.useRust;
    if (this.useRust) {
      splash.wechatRender();
    } else {
      this.mountArkTs();
    }
  }

  /** Build the ArkTS tree and put it in the slot, replacing whatever is there. */
  private mountArkTs(): number {
    splash.wechatDetach();
    const n = this.wc!.build();
    const node = this.wc!.node();
    if (node !== null) {
      this.content.addFrameNode(node);
    }
    return n;
  }

  // --- benchmark ------------------------------------------------------------

  private rustN: Array<number> = new Array<number>();
  private arkN: Array<number> = new Array<number>();
  private rustL: Array<number> = new Array<number>();
  private arkL: Array<number> = new Array<number>();

  private nowUs(): number {
    return systemDateTime.getTime(true) / 1000;
  }

  /**
   * Tour every screen on both paths, idle and under load.
   *
   * One path per tick, never both in the same one: building both inside a
   * single callback made whichever went first pay ~10 ms for the previous
   * tick's teardown, which showed up as per-screen ratios ranging 1.06x to
   * 8.60x purely by build order.
   */
  private wcStep(i: number): void {
    const S = TOUR.length;
    const perTour = S * 2;
    const warm = WC_WARMUP * perTour;
    const total = WC_TOURS * perTour;

    if (i === 0) {
      for (let k = 0; k < S; k++) {
        this.rustN.push(0);
        this.arkN.push(0);
        this.rustL.push(0);
        this.arkL.push(0);
      }
      splash.wechatLog('warming up (' + WC_WARMUP + ' tours, discarded)');
    } else if (i <= warm) {
      const n = i - 1;
      if (n === Math.floor(warm / 2)) {
        this.load.start(8);
      }
      this.half(Math.floor((n % perTour) / 2), (n % 2) === 0, false, false);
      if (i === warm) {
        this.load.stop();
      }
    } else if (i <= warm + total) {
      const n = i - warm - 1;
      const tour = Math.floor(n / perTour);
      const within = n % perTour;
      const loaded = WC_COND[tour % WC_COND.length];
      // Which path goes first flips every tour, so the "first pays" penalty
      // lands on both sides equally instead of tracking the screen index.
      const rustFirst = (tour % 2) === 0;
      const isRust = ((within % 2) === 0) === rustFirst;
      if (within === 0) {
        if (loaded) {
          this.load.start(8);
        } else {
          this.load.stop();
        }
      }
      this.half(Math.floor(within / 2), isRust, loaded, true);
    } else if (i === warm + total + 1) {
      this.load.stop();
      this.report();
      // Hand the app back to Rust, on the chats screen.
      splash.wechatRender();
      return;
    }
    setTimeout(() => { this.wcStep(i + 1); }, 60);
  }

  /** One path building one screen, alone in its tick. */
  private half(k: number, isRust: boolean, loaded: boolean, record: boolean): void {
    const parts: Array<string> = TOUR[k].split('|');
    const tab: number = parseInt(parts[0]);
    const route: string = parts[1];

    let us = 0;
    if (isRust) {
      const r: Array<number> = splash.wechatTime(tab, route, 1) as Array<number>;
      us = r[1];
    } else {
      this.wc!.tab = tab;
      this.wc!.route = Index.routeOf(route);
      this.wc!.chatId = 1;
      const t0 = this.nowUs();
      this.wc!.build();
      us = this.nowUs() - t0;
      this.wc!.clear();
    }
    if (!record) {
      return;
    }
    if (isRust) {
      if (loaded) {
        this.rustL[k] = this.rustL[k] + us;
      } else {
        this.rustN[k] = this.rustN[k] + us;
      }
    } else {
      if (loaded) {
        this.arkL[k] = this.arkL[k] + us;
      } else {
        this.arkN[k] = this.arkN[k] + us;
      }
    }
  }

  private static routeOf(s: string): RouteKind {
    if (s === 'chat') {
      return RouteKind.Chat;
    }
    if (s === 'moments') {
      return RouteKind.Moments;
    }
    if (s === 'addcontact') {
      return RouteKind.AddContact;
    }
    if (s === 'myprofile') {
      return RouteKind.MyProfile;
    }
    return RouteKind.Root;
  }

  private report(): void {
    const T = WC_TOURS / 2;
    let trN = 0;
    let taN = 0;
    let trL = 0;
    let taL = 0;
    for (let k = 0; k < TOUR.length; k++) {
      const rn = this.rustN[k] / T / 1000;
      const an = this.arkN[k] / T / 1000;
      const rl = this.rustL[k] / T / 1000;
      const al = this.arkL[k] / T / 1000;
      trN += rn;
      taN += an;
      trL += rl;
      taL += al;
      splash.wechatLog(TOUR[k]
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

  build() {
    Column() {
      ContentSlot(this.content)
    }
    .width('100%')
    .height('100%')
  }
}
