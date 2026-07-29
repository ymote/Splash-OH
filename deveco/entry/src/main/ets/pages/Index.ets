import { NodeContent, FrameNode } from '@kit.ArkUI';
import { systemDateTime } from '@kit.BasicServicesKit';
import { webview } from '@kit.ArkWeb';
import { WebNetErrorList } from '@ohos.web.netErrorList';
import { BusinessError } from '@kit.BasicServicesKit';
import { picker, fileUri } from '@kit.CoreFileKit';
import { pasteboard } from '@kit.BasicServicesKit';
import { abilityAccessCtrl, Permissions, PermissionRequestResult } from '@kit.AbilityKit';
import { ble } from '@kit.ConnectivityKit';
import { window } from '@kit.ArkUI';
import splash from 'libsplash_oh.so';
import { WeChatArkTs, LoadGenerator, RouteKind } from './WeChatArkTs';
import { Taobao, TikTok, Wonderous } from './AppsArkTs';

/**
 * When true the app is the YouTube demo: Splash-OH renders the native chrome
 * bar (from assets/youtube.splash) and an ArkTS `Web` runs the YouTube web app
 * below it. OpenHarmony's WebView is ArkTS-only, so this hybrid is how a
 * Splash-OH app hosts a WebView. Set false for the original WeChat benchmark.
 */
/** Start in the browser card, which is the app that exercises the web overlay.
 *  Empty string starts in the benchmark harness instead. */
const START_APP: string = 'catalog';
/**
 * Milliseconds per screen for the catalog screenshot sweep, or 0 to leave the
 * catalog interactive.
 *
 * Set to a non-zero value (7000 works — a snapshot plus an hdc pull takes about
 * five seconds) and the app steps through every catalog screen on its own, so a
 * capture script can photograph all 28 without simulating taps. That sweep is
 * how the badge and text-picker defects were found, and it is left here so the
 * check is repeatable rather than a thing that has to be rebuilt each time.
 */
const CATALOG_WALK_MS: number = 0;
/** The sweep's order: the index, then every screen the catalog defines. */
const CATALOG_WALK: Array<string> = [];
/** Origin given to generated pages. Content loaded with no baseUrl gets an
 *  opaque origin in ArkWeb and never paints; it needs a real https one. */
/**
 * Force a native rerender this often, or 0 for normal behaviour.
 *
 * Exists to make one specific claim measurable: that a web slot now survives a
 * rebuild. Before slot ids were reset per build, every rerender destroyed the
 * `Web` component and reloaded its document. Proving the fix needs rerenders to
 * happen on demand rather than whenever background data lands — set this, watch
 * the frontend bundle's uptime row, and compare it against the wall clock.
 */
const RERENDER_EVERY_MS: number = 0;
/**
 * Navigate slot 1 to this origin after it loads, or '' to leave it alone.
 *
 * Answers one question: trust is recorded per *slot*, but `javaScriptProxy` is
 * attached to the *component*. If a trusted `splash://` page navigates itself
 * somewhere else, does the bridge go with it? Rust's `is_trusted` reads the
 * slot's Source, which is still App — so if ArkTS re-injects the proxy into the
 * new document, both gates say yes and a remote page holds all 48 tools.
 */
const NAV_PROBE_URL: string = '';
const NAV_PROBE_AFTER_MS: number = 12000;
const WEB_BASE: string = 'https://localhost/';
/** Which app the single-app mode is showing, so a background-data redraw
 *  rebuilds the right one. */
let APPS_SHOWN: string = START_APP;

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
const MEM_MODE: boolean = true;
/** Which implementation this build measures. Flip and rebuild for the other. */
const MEM_RUST: boolean = false;
/** Copies of the whole app held at once, for the marginal figure. */
const MEM_COPIES: number = 12;
/** Which app the memory arm measures. Index into APPS. */
const MEM_APP: number = 2;

/**
 * The object ArkWeb injects into a page as `splash_native`.
 *
 * `javaScriptProxy` can only expose synchronous, void methods -- it cannot
 * return a promise or a value asynchronously. So a call is split: the page
 * hands over a callId here, and Rust's answer is evaluated back into the page
 * later as `splash._resolve(callId, payload)`. The shim in `bridge.rs` wraps
 * that pair into the promise API a page actually wants.
 */
class SplashNativeBridge {
  private slot: number = 0;

  constructor(slot: number) {
    this.slot = slot;
  }

  invoke(callId: string, tool: string, args: string): void {
    splash.webBridgeInvoke(this.slot, String(callId), tool, args);
  }
}

@Entry
@Component
struct Index {
  private content: NodeContent = new NodeContent();

  /**
   * System back gesture: pop one level of the kit's route instead of leaving
   * the app. Returning false at the index lets the platform close it, which is
   * what a back gesture should do there.
   */
  onBackPress(): boolean {
    return splash.goBack() as boolean;
  }

  private wc: WeChatArkTs | null = null;
  private load: LoadGenerator = new LoadGenerator();
  private useRust: boolean = true;
  /**
   * Web surfaces the current Splash tree asked for, as "id|url|x|y|w|h".
   *
   * There is no ARKUI_NODE_WEB, so Rust cannot build a web view; it builds a
   * transparent placeholder and records where it went. This puts a real `Web`
   * component at those coordinates, above the ContentSlot.
   */
  @State webSlots: Array<string> = [];
  /** One controller per slot id, kept OUT of @State.
   *
   *  A controller reached through an @Observed/@State proxy is inert -- calls
   *  like loadUrl() silently do nothing, no exception, no log. That cost a day
   *  in octos-one. Plain Map, never decorated. */
  private controllers: Map<string, webview.WebviewController> = new Map();
  /** What each controller was last told to load, so a rebuild that did not
   *  change the URL does not reload the page out from under the user. */
  private loaded: Map<string, string> = new Map();
  /** One scheme handler per slot id, installed once when its controller
   *  attaches. Kept out of @State for the same reason the controllers are. */
  private handlers: Map<string, webview.WebSchemeHandler> = new Map();
  private lastForcedRerender: number = 0;
  private forcedRerenders: number = 0;
  private navProbeState: number = 0;
  private startedAt: number = Date.now();
  /** What each slot has actually finished rendering, set from onPageEnd.
   *
   *  Separate from `loaded` because they are different facts and conflating
   *  them is a bug this had: loadData returning without throwing does not mean
   *  the page painted -- the comments in loadSlot say so about baseUrl and mime
   *  -- yet `loaded` was set on the call returning, which disarmed the retry
   *  loop. A slot that silently failed to paint then stayed white forever. */
  private confirmed: Map<string, string> = new Map();
  /** Ticks a slot has been loaded-but-not-confirmed, so a stuck one retries. */
  private waiting: Map<string, number> = new Map();

  /** One bridge object per slot, kept out of @State for the same reason the
   *  controllers are: a proxy-wrapped object handed to javaScriptProxy is
   *  inert. */
  private bridges: Map<string, SplashNativeBridge> = new Map();

  /** The app's own window, needed to launch the system picker over it.
   *  Resolved once at startup; a pick before it lands is reported rather
   *  than silently doing nothing. */
  private mainWindow: window.Window | undefined = undefined;

  private bridgeFor(id: string): SplashNativeBridge {
    let b = this.bridges.get(id);
    if (b === undefined) {
      b = new SplashNativeBridge(Number(id));
      this.bridges.set(id, b);
    }
    return b;
  }

  /**
   * Evaluate finished bridge calls back into their pages.
   *
   * Driven from the same 250 ms poll as everything else. A reply is
   * `slot|callId|payloadJson`, split on the first two separators only because
   * the payload is JSON and may contain more.
   */
  /**
   * Deliver one line the moment Rust produces it.
   *
   * Registered as a threadsafe-function callback, so this runs on the JS thread
   * but is triggered from whichever worker finished the work — no poll, and no
   * quarter-second of latency added to every call.
   *
   * A line is `slot|callId|payload`. A callId beginning with `@` is an event
   * rather than a reply, which is how one channel carries both directions
   * without a second registration.
   */
  private deliver(line: string): void {
    const a = line.indexOf('|');
    const b = line.indexOf('|', a + 1);
    if (a < 0 || b < 0) {
      return;
    }
    const slot: string = line.substring(0, a);
    const id: string = line.substring(a + 1, b);
    const payload: string = line.substring(b + 1);
    try {
      const js: string = id.charAt(0) === '@'
        ? 'window.splash&&window.splash._event&&window.splash._event('
          + JSON.stringify(id.substring(1)) + ',' + payload + ')'
        : 'window.splash&&window.splash._resolve&&window.splash._resolve('
          + JSON.stringify(id) + ',' + payload + ')';
      this.controllerFor(slot).runJavaScript(js);
    } catch (e) {
      console.error('deliver to slot ' + slot + ' failed: ' + JSON.stringify(e));
    }
  }

  private pumpBridge(): void {
    const replies: Array<string> = splash.webBridgeDrain() as Array<string>;
    for (let i = 0; i < replies.length; i++) {
      const r = replies[i];
      const a = r.indexOf('|');
      const b = r.indexOf('|', a + 1);
      if (a < 0 || b < 0) {
        continue;
      }
      const slot = r.substring(0, a);
      const callId = r.substring(a + 1, b);
      const payload = r.substring(b + 1);
      try {
        this.controllerFor(slot).runJavaScript(
          'window.splash&&window.splash._resolve&&window.splash._resolve('
          + JSON.stringify(callId) + ',' + payload + ')');
      } catch (e) {
        console.error('bridge reply to slot ' + slot + ' failed: ' + JSON.stringify(e));
      }
    }
  }

  /**
   * Show the system picker for whatever a page asked for, and tell Rust.
   *
   * The only tool that runs Rust -> ArkTS -> Rust. `fs.list` established that
   * user storage is *absent* from the app's mount namespace rather than merely
   * denied, so no permission makes /storage/media/... appear by path; the
   * picker is the only route to a user's own directories, and it is ArkTS-only
   * because it needs a UIAbilityContext.
   *
   * The picker rejects when the user backs out, which is a normal outcome and
   * is reported as a failed call rather than left to hang the page's promise.
   */
  private pumpPicks(): void {
    const reqs: Array<string> = splash.pickDrain() as Array<string>;
    for (let i = 0; i < reqs.length; i++) {
      // reqId|op|args — args last because it may itself contain a separator.
      const a = reqs[i].indexOf('|');
      const b = reqs[i].indexOf('|', a + 1);
      if (a < 0 || b < 0) {
        continue;
      }
      const reqId = reqs[i].substring(0, a);
      const op = reqs[i].substring(a + 1, b);
      const args = reqs[i].substring(b + 1);

      if (op === 'bluetooth.scan') {
        this.bleScan(reqId);
        continue;
      }
      if (op === 'permission.request') {
        this.requestPermissions(reqId, args);
        continue;
      }
      if (op === 'clipboard.read') {
        this.clipboardRead(reqId);
        continue;
      }
      if (op === 'clipboard.write') {
        this.clipboardWrite(reqId, args);
        continue;
      }

      const folder = args === 'folder';
      if (this.mainWindow === undefined) {
        splash.pickResolve(reqId, false, 'window not ready');
        continue;
      }
      try {
        const opts = new picker.DocumentSelectOptions();
        // 1 = DocumentSelectMode.FOLDER, 0 = FILE.
        opts.selectMode = folder ? 1 : 0;
        opts.maxSelectNumber = 1;
        // Constructed with an explicit window, not just the context.
        //
        // The context-only constructor launched the picker's UIExtension and
        // then resolved with an empty array straight away. The log said why:
        //
        //   filePicker/AceOverlay: Can not get overlayManager
        //   RSSurfaceRenderNode ... FilePickerUIExtAbility ... on tree: 0
        //
        // The extension's surface never reached the render tree, because the
        // picker could not find an ArkUI overlay to mount itself into from the
        // context alone. Handing it the window resolves that, and the SDK
        // documents this constructor for exactly the case where the app owns
        // the window it wants the picker over.
        const p = new picker.DocumentViewPicker(getContext(this), this.mainWindow);
        p.select(opts).then((uris: Array<string>) => {
          // Hand back the URI and the path it maps to. Whether Rust's std::fs
          // can then read that path is the open question this exists to
          // answer, so give the page both and let it try.
          const out: Array<Record<string, string>> = [];
          for (let k = 0; k < uris.length; k++) {
            const fu = new fileUri.FileUri(uris[k]);
            out.push({ 'uri': uris[k], 'path': fu.path, 'name': fu.name });
          }
          splash.pickResolve(reqId, true, JSON.stringify(out));
        }).catch((e: Error) => {
          splash.pickResolve(reqId, false, 'picker: ' + e.message);
        });
      } catch (e) {
        splash.pickResolve(reqId, false, 'picker unavailable: ' + JSON.stringify(e));
      }
    }
  }

  /**
   * The system clipboard. There is no pasteboard NDK, so this takes the same
   * Rust -> ArkTS -> Rust road the picker does.
   *
   * getPrimaryText() returns '' both for an empty clipboard and for one holding
   * something that is not text (an image, a URI), so the MIME type is reported
   * alongside rather than letting a page read those two as the same thing.
   */
  /**
   * Ask the user for runtime permissions.
   *
   * The first thing here that puts a dialog in front of a person. It has to go
   * through ArkTS: requestPermissionsFromUser needs a UIAbilityContext, and
   * there is no NDK equivalent — the same reason the file picker does.
   *
   * The result is per-permission, not a single boolean, because a user can
   * grant approximate location and refuse precise. Reporting a partial grant as
   * failure would be wrong: coarse is all the weather card needs.
   */
  private requestPermissions(reqId: string, args: string): void {
    try {
      const names: Array<Permissions> = JSON.parse(args) as Array<Permissions>;
      abilityAccessCtrl.createAtManager()
        .requestPermissionsFromUser(getContext(this), names)
        .then((r: PermissionRequestResult) => {
          let out: string = '{"granted":[';
          let first: boolean = true;
          for (let i = 0; i < r.permissions.length; i++) {
            // 0 is PERMISSION_GRANTED; anything else is a refusal of some kind.
            if (r.authResults[i] === 0) {
              out += (first ? '' : ',') + JSON.stringify(r.permissions[i]);
              first = false;
            }
          }
          out += '],"asked":' + r.permissions.length + '}';
          splash.pickResolve(reqId, true, out);
        }).catch((e: Error) => {
          splash.pickResolve(reqId, false, 'permission request: ' + e.message);
        });
    } catch (e) {
      splash.pickResolve(reqId, false, 'permission request failed: ' + JSON.stringify(e));
    }
  }

  /**
   * A BLE scan, because the native Bluetooth surface cannot do one.
   *
   * OH_Bluetooth_GetBluetoothSwitchState is the entire NDK — discovery, pairing
   * and GATT live only in ArkTS, so this takes the same road the file picker
   * and the clipboard do.
   *
   * Scanning is a stream, not a call. It runs for a fixed window, collects
   * whatever advertises in that time, and reports the set — a page asking
   * "what is nearby" wants an answer, not a subscription it has to manage
   * across the bridge.
   */
  private bleScan(reqId: string): void {
    try {
      const seen: Map<string, number> = new Map();
      const onFind = (results: Array<ble.ScanResult>) => {
        for (let i = 0; i < results.length; i++) {
          // Keep the strongest sighting: a device seen repeatedly should be
          // reported at its best signal, not its most recent.
          const id: string = results[i].deviceId;
          const prev: number | undefined = seen.get(id);
          if (prev === undefined || results[i].rssi > prev) {
            seen.set(id, results[i].rssi);
          }
        }
      };
      ble.on('BLEDeviceFind', onFind);
      // null, not []. An empty filter array is rejected with 401 (invalid
      // parameter); null is what the API takes to mean "no filter, report
      // everything". The .d.ts types the parameter as non-nullable, so this
      // needs the cast -- the runtime contract and the declaration disagree.
      ble.startBLEScan(null as ESObject as Array<ble.ScanFilter>);
      setTimeout(() => {
        try {
          ble.stopBLEScan();
          ble.off('BLEDeviceFind', onFind);
        } catch (e) {
          // Stopping a scan that already failed to start is not worth
          // reporting over the result we do have.
        }
        let out: string = '{"devices":[';
        let first: boolean = true;
        seen.forEach((rssi: number, id: string) => {
          // The address is truncated on purpose. A page asking what is nearby
          // does not need a stable identifier for every device around the
          // user, and a full BLE address is exactly that.
          out += (first ? '' : ',') + '{"id":' + JSON.stringify(id.substring(0, 8) + '…')
               + ',"rssi":' + rssi + '}';
          first = false;
        });
        out += '],"count":' + seen.size + '}';
        splash.pickResolve(reqId, true, out);
      }, 4000);
    } catch (e) {
      splash.pickResolve(reqId, false, 'ble scan: ' + JSON.stringify(e));
    }
  }

  private clipboardRead(reqId: string): void {
    try {
      pasteboard.getSystemPasteboard().getData().then((data: pasteboard.PasteData) => {
        // Built by hand rather than JSON.stringify of an object literal.
        // ArkTS is not TypeScript: a literal mixing string and number values
        // stringified to something the page read back as undefined, with no
        // error on either side. Two JSON.stringify calls on the individual
        // strings do the escaping, and the shape is then unambiguous.
        const text: string = data.getPrimaryText();
        const mime: string = data.getPrimaryMimeType();
        splash.pickResolve(reqId, true,
          '{"text":' + JSON.stringify(text === undefined ? '' : text)
          + ',"mimeType":' + JSON.stringify(mime === undefined ? '' : mime)
          + ',"records":' + data.getRecordCount() + '}');
      }).catch((e: Error) => {
        splash.pickResolve(reqId, false, 'clipboard: ' + e.message);
      });
    } catch (e) {
      splash.pickResolve(reqId, false, 'clipboard unavailable: ' + JSON.stringify(e));
    }
  }

  private clipboardWrite(reqId: string, text: string): void {
    try {
      const data = pasteboard.createData(pasteboard.MIMETYPE_TEXT_PLAIN, text);
      pasteboard.getSystemPasteboard().setData(data).then(() => {
        splash.pickResolve(reqId, true, '{"wrote":' + text.length + '}');
      }).catch((e: Error) => {
        splash.pickResolve(reqId, false, 'clipboard: ' + e.message);
      });
    } catch (e) {
      splash.pickResolve(reqId, false, 'clipboard unavailable: ' + JSON.stringify(e));
    }
  }

  /**
   * Mark slots whose page reported its script running.
   *
   * The shim pings `slot.ready` as soon as it executes. That is the only signal
   * that cannot be wrong: loadData returns without throwing when it has
   * silently rendered nothing, and onPageEnd fires for the blank origin the
   * slot starts on, so both of those look identical whether the page is there
   * or the surface is white.
   */
  private pumpPainted(): void {
    const ids: Array<string> = splash.webSlotsPainted() as Array<string>;
    for (let i = 0; i < ids.length; i++) {
      const key: string | undefined = this.loaded.get(ids[i]);
      if (key !== undefined) {
        this.confirmed.set(ids[i], key);
        this.waiting.delete(ids[i]);
      }
    }
  }

  /**
   * Install the `splash://` handler on a slot's controller.
   *
   * Called from `onControllerAttached`, which is the only window that works.
   * The scheme is registered before the engine starts (EntryAbility.onCreate),
   * but a *handler* binds to a controller, and a controller has none until a
   * `Web` attaches it — while the component's `src` starts navigating as soon
   * as it exists. `onControllerAttached` is the documented point that is after
   * the first and before the second, so it is what catches the very first
   * main-frame request instead of letting it through unanswered.
   *
   * Everything served comes from Rust. This function decides nothing about
   * content; it moves bytes and reports failures.
   */
  private installSchemeHandler(id: string): void {
    if (this.handlers.has(id)) {
      return;
    }
    try {
      const handler: webview.WebSchemeHandler = new webview.WebSchemeHandler();
      handler.onRequestStart((req: webview.WebSchemeHandlerRequest,
                             res: webview.WebResourceHandler): boolean => {
        const url: string = req.getRequestUrl();
        try {
          const a: ESObject = splash.assetGet(url);
          console.info('SPLASHASSET probe ' + url + ' typeof=' + typeof a
            + ' mime=' + (a === undefined ? 'n/a' : String(a.mime))
            + ' status=' + (a === undefined ? 'n/a' : String(a.status))
            + ' hasBody=' + (a !== undefined && a.body !== undefined)
            + ' len=' + (a === undefined || a.body === undefined ? -1 : (a.body as Uint8Array).byteLength));
          const mime: string = a.mime as string;
          const status: number = a.status as number;
          // A napi Buffer arrives here as an ArrayBuffer, NOT as a Uint8Array
          // view over one: it reports a byteLength and has no `.buffer`. An
          // earlier version sliced `.buffer` to the view and every asset failed
          // with "cannot read property slice of undefined" -- which reads like
          // a missing body, but the body was there the whole time.
          const body: ArrayBuffer = a.body as ArrayBuffer;

          const resp: webview.WebSchemeHandlerResponse = new webview.WebSchemeHandlerResponse();
          resp.setStatus(status);
          resp.setStatusText(status === 200 ? 'OK' : 'Not Found');
          resp.setMimeType(mime);
          resp.setEncoding('utf-8');
          // The bundle is compiled into the binary, so it cannot change under
          // a running app; but a stale cache across an install would look
          // exactly like a fix that did not take.
          resp.setHeaderByName('Cache-Control', 'no-store', true);
          res.didReceiveResponse(resp);
          res.didReceiveResponseBody(body);
          res.didFinish();
          console.info('SPLASHASSET ' + status + ' ' + url + ' (' + body.byteLength + 'B ' + mime + ')');
        } catch (e) {
          // Failing loudly matters more here than anywhere else in this file:
          // a custom scheme has no default resolver, so a request that is
          // neither answered nor failed is a hang with no console error.
          const err = e as BusinessError;
          console.error('SPLASHASSET error ' + url + ': code=' + err.code
            + ' msg=' + err.message + ' raw=' + String(e));
          res.didFail(WebNetErrorList.ERR_FAILED);
        }
        return true;
      });
      this.controllerFor(id).setWebSchemeHandler('splash', handler);
      this.handlers.set(id, handler);
      console.info('SPLASHSCHEME handler installed on slot ' + id);
    } catch (e) {
      console.error('SPLASHSCHEME handler install failed on slot ' + id + ': ' + JSON.stringify(e));
    }
  }

  private controllerFor(id: string): webview.WebviewController {
    let c = this.controllers.get(id);
    if (c === undefined) {
      // Named, so Rust can address this webview directly through
      // OH_NativeArkWeb_RunJavaScript and evaluate into the page without ArkTS
      // on the path. The tag is built in one place on each side —
      // arkweb::web_tag — because a mismatch is silent: the native call simply
      // does nothing.
      c = new webview.WebviewController('slot' + id);
      this.controllers.set(id, c);
    }
    return c;
  }

  /** Pull the declared slots from Rust; re-render only if they changed. */
  private syncWebSlots(): void {
    const next: Array<string> = splash.webSlots() as Array<string>;
    if (next.length === this.webSlots.length) {
      let same = true;
      for (let i = 0; i < next.length; i++) {
        if (next[i] !== this.webSlots[i]) {
          same = false;
          break;
        }
      }
      if (same) {
        // Geometry unchanged, but a generated page may still be waiting for a
        // controller that was not ready last time. Retrying is cheap and the
        // `loaded` map makes it a no-op once it has worked.
        this.flushPending();
        return;
      }
    }
    this.webSlots = next;
    this.flushPending();
  }

  /**
   * Try to load every generated slot.
   *
   * Driven from the poll rather than from `onPageEnd`, because a slot whose
   * `src` is about:blank never fires a page event at all -- so a load hung on
   * that event simply never happened, and the surface stayed white with no
   * error anywhere. Retrying until it sticks is self-healing and costs one map
   * lookup per tick once it has.
   */
  private flushPending(): void {
    for (let i = 0; i < this.webSlots.length; i++) {
      const parts: Array<string> = this.webSlots[i].split('|');
      if (parts[1] !== 'html') {
        continue;
      }
      const id: string = parts[0];
      const want: string | undefined = this.loaded.get(id);
      if (want !== undefined && this.confirmed.get(id) !== want) {
        // Issued but never painted. Give it a second, then forget the load so
        // the line below re-issues it. Two seconds of retrying beats a surface
        // that is white for the life of the page.
        const n: number = (this.waiting.get(id) ?? 0) + 1;
        this.waiting.set(id, n);
        if (n > 8) {
          this.loaded.delete(id);
          this.waiting.delete(id);
          console.info('web slot ' + id + ' never painted; reloading');
        }
      }
      this.loadSlot(id, parts[1], parts[6]);
    }
  }

  /** Load a slot's URL, once the Web component has bound its controller.
   *
   *  Deferred to onPageEnd rather than onControllerAttached: attached only
   *  means the object is bound, not that the engine will accept a load. */
  private loadSlot(id: string, kind: string, url: string): void {
    // Key generated pages on their content, not their (empty) url, or a
    // rebuild with new data would be treated as already-loaded.
    let key = url;
    let html = '';
    if (kind === 'html') {
      html = splash.webSlotHtml(Number(id)) as string;
      key = 'html:' + html.length + ':' + html.charCodeAt(0);
    }
    if (this.loaded.get(id) === key) {
      return;
    }
    try {
      if (kind === 'html') {
        // All four arguments matter.
        //   - mime and encoding: passing undefined for either fails with code
        //     401 and renders blank, with no exception.
        //   - baseUrl: without it the content gets an opaque origin and never
        //     paints -- loadData reports success and the surface stays white.
        this.controllerFor(id).loadData(html, 'text/html', 'UTF-8', WEB_BASE);
      } else {
        this.controllerFor(id).loadUrl(url);
      }
      this.loaded.set(id, key);
      this.waiting.set(id, 0);
    } catch (e) {
      console.error('web slot ' + id + ' load failed: ' + JSON.stringify(e));
    }
  }

  aboutToAppear(): void {
    splash.mount(this.content);
    // Animation frames. The DSL evaluates a tree once per mount, so a screen can
    // only move if it is re-mounted against a changing clock; `animTick` does
    // exactly that and returns false (doing nothing) on every static screen, so
    // this costs nothing anywhere but the animation demos.
    setInterval(() => { splash.animTick(); }, 33);
    if (START_APP !== '') {
      // Prove the VM works on this build. The ported apps build in Rust and
      // never evaluate a script, so a broken VM would stay hidden until a
      // .splash card rendered blank.
      splash.dslSelfTest();
      splash.appRender(START_APP);
      // Off by default; set CATALOG_WALK_MS to re-run the screenshot sweep.
      if (START_APP === 'catalog' && CATALOG_WALK_MS > 0) {
        let k: number = 0;
        setInterval(() => {
          if (k < CATALOG_WALK.length) {
            splash.catalogScreen(CATALOG_WALK[k]);
            k++;
          }
        }, CATALOG_WALK_MS);
      }
      this.syncWebSlots();
      // Hand Rust a direct line in. Everything below this point that Rust
      // wants to say arrives immediately instead of on the next 250 ms tick.
      splash.setPush((line: string) => { this.deliver(line); });
      window.getLastWindow(getContext(this)).then((w: window.Window) => {
        this.mainWindow = w;
      }).catch((e: Error) => {
        console.error('no window for the picker: ' + e.message);
      });
      // A tap inside the native tree cannot call back into ArkTS, so Rust
      // rebuilds on its own and this polls for the geometry it declared.
      setInterval(() => {
        // Background data (a forecast, a resolved stream) lands on a worker
        // thread. Rust flags it; re-render so the page picks it up.
        //
        // appRerender, NOT appRender: the latter resets navigation, so the
        // arrival of the very data a tap asked for would throw the user back
        // to the first tab -- making the tap look like it had done nothing.
        if ((splash.appTakeDirty() as number) === 1) {
          splash.appRerender();
        }
        if (NAV_PROBE_URL !== '' && this.navProbeState < 3) {
          const t: number = Date.now() - this.startedAt;
          if (this.navProbeState === 0 && t > NAV_PROBE_AFTER_MS) {
            this.navProbeState = 1;
            console.info('NAVPROBE navigating slot 1 away to ' + NAV_PROBE_URL);
            try {
              this.controllerFor('1').loadUrl(NAV_PROBE_URL);
            } catch (e) {
              console.error('NAVPROBE navigation threw: ' + String(e));
            }
          } else if (this.navProbeState === 1 && t > NAV_PROBE_AFTER_MS + 8000) {
            this.navProbeState = 2;
            try {
              // Gate 1: did ArkTS put the proxy into the new document?
              this.controllerFor('1').runJavaScript(
                "(function(){return location.origin+' proxy='+(typeof window.splash_native)})()",
                (err, r) => { console.info('NAVPROBE gate1 ' + (err ? 'err ' + String(err) : String(r))); });
              // Gate 2: if it is there, does Rust answer a call from it?
              this.controllerFor('1').runJavaScript(
                "(function(){if(!window.splash_native)return 'no-proxy';"
                // `log` writes page-controlled text into hilog, so a line
                // appearing there is proof Rust *served* the call rather than
                // merely proof that invoke() did not throw. Absence of a
                // refusal log would not have shown that.
                + "try{window.splash_native.invoke('navprobe','log',"
                + "JSON.stringify('SPOKE FROM '+location.origin));return 'invoked'}"
                + "catch(e){return 'threw '+e}})()",
                (err, r) => { console.info('NAVPROBE gate2 ' + (err ? 'err ' + String(err) : String(r))); });
            } catch (e) {
              console.error('NAVPROBE probe threw: ' + String(e));
            }
            this.navProbeState = 3;
          }
        }
        if (RERENDER_EVERY_MS > 0) {
          const now: number = Date.now();
          if (now - this.lastForcedRerender >= RERENDER_EVERY_MS) {
            this.lastForcedRerender = now;
            splash.appRerender();
            this.forcedRerenders++;
            console.info('RERENDERPROBE n=' + this.forcedRerenders
              + ' slots=' + (splash.webSlots() as Array<string>).join(','));
          }
        }
        this.syncWebSlots();
        // Still drained, but only ever catches traffic produced before
        // setPush registered. Replies now arrive through deliver().
        this.pumpBridge();
        this.pumpPicks();
        this.pumpPainted();
      }, 250);
      return;
    }
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
    if (START_APP !== '') {
      return;
    }
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
    // The native tree underneath, web surfaces on top at the coordinates the
    // DSL declared. A Stack rather than a Column because the overlay is
    // positioned against the page, not laid out in flow with it.
    Stack({ alignContent: Alignment.TopStart }) {
      ContentSlot(this.content)

      // slot = "id|kind|x|y|w|h|url"; kind is 'url' or 'html'.
      ForEach(this.webSlots, (slot: string) => {
        Web({
          // A generated page starts blank and is filled by loadData once the
          // engine is ready; about:blank is the only src that reliably fires
          // onPageEnd without navigating anywhere.
          // A URL slot navigates directly. A generated slot starts on a real
          // https origin and is then replaced by loadData against that same
          // origin -- see loadSlot for why it cannot be data: or origin-less.
          // 'html' starts on a real https origin and is filled by loadData;
          // 'url' and 'app' both navigate straight to their src -- for 'app'
          // that is a splash:// URL the scheme handler answers.
          src: slot.split('|')[1] === 'html' ? WEB_BASE : slot.split('|')[6],
          controller: this.controllerFor(slot.split('|')[0])
        })
          .javaScriptAccess(true)
          .domStorageAccess(true)
          // The JS -> Rust/Splash bridge, attached ONLY to slots showing markup
          // this app generated ('html'). A slot showing a remote page ('url')
          // never sees splash_native at all -- apps/browser.rs loads Wikipedia
          // and Hacker News into slots, and they have no business holding
          // splash.eval or http.get.
          //
          // Rust re-checks on every call regardless. This check sits next to
          // the untrusted content, which makes it the wrong place to depend on.
          .javaScriptProxy(slot.split('|')[1] !== 'url' ? {
            object: this.bridgeFor(slot.split('|')[0]),
            name: 'splash_native',
            methodList: ['invoke'],
            controller: this.controllerFor(slot.split('|')[0]),
          } : undefined)
          // Stops ArkWeb demanding a tap before it will play media.
          .onControllerAttached(() => {
            this.installSchemeHandler(slot.split('|')[0]);
          })
          // Refuse to navigate a trusted slot off its own origin.
          //
          // Trust is recorded per slot, but javaScriptProxy attaches to the
          // component -- so a splash:// page that navigated itself to a remote
          // site kept the bridge, and Rust, checking only the slot, agreed. A
          // document served by example.com called the `log` tool and Rust wrote
          // its text to hilog. This is where that stops.
          //
          // Returning true cancels the load.
          .onLoadIntercept((event) => {
            const id: string = slot.split('|')[0];
            const want: string = splash.slotExpectedOrigin(Number(id)) as string;
            if (want === '') {
              return false;  // a url slot is untrusted whatever it loads
            }
            const url: string = event.data.getRequestUrl();
            if (url.indexOf(want) === 0 || url.indexOf('about:') === 0) {
              return false;
            }
            console.error('NAVGUARD slot ' + id + ' refused navigation to ' + url
              + ' (expected origin ' + want + ')');
            return true;
          })
          .onPageBegin((event) => {
            // Report where the document actually came from, so Rust can refuse
            // to believe a slot whose page is not where it should be.
            const id: string = slot.split('|')[0];
            const u: string = event.url as string;
            const i: number = u.indexOf('/', u.indexOf('://') + 3);
            splash.slotOrigin(Number(id), i < 0 ? u : u.substring(0, i));
          })
          .mediaPlayGestureAccess(false)
          .position({ x: Number(slot.split('|')[2]), y: Number(slot.split('|')[3]) })
          .width(Number(slot.split('|')[4]))
          .height(Number(slot.split('|')[5]))

      }, (slot: string) => slot)
    }
    .width('100%')
    .height('100%')
  }
}
