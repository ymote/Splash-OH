import { typeNode, FrameNode, UIContext } from '@kit.ArkUI';

/**
 * The same WeChat-shaped app as `crates/splash-oh/src/wechat.rs`, built in
 * ArkTS through `typeNode`.
 *
 * `typeNode.createNode` produces the same underlying native node the NDK
 * produces, so this is the apples-to-apples control: identical tree, identical
 * node count, identical attributes. Only the language building it differs.
 *
 * Node counts are asserted against the Rust side at runtime. "The ArkTS version
 * is faster because it quietly builds less" is the obvious way for a comparison
 * like this to be wrong, so it is checked rather than assumed.
 *
 * Row shapes come from project-robius/makepad_wechat:
 *   chat preview 6 nodes, message 4, contact 3, discover 4, moment 5.
 */

const BG: number = 0xFFEDEDED;
const SURFACE: number = 0xFFFFFFFF;
const NAV: number = 0xFFF7F7F7;
const TEXT_C: number = 0xFF191919;
const SUBTLE: number = 0xFF9A9A9A;
const GREEN: number = 0xFF07C160;
const AVATAR: number = 0xFFD8D8D8;
const W: number = 402;

const NAMES: Array<string> = [
  'Rik Arends', 'Eddy Bruel', 'Ken Wu', 'Julian Montes de Oca', 'Edward Tan',
  'Alex Zhang', 'Jorge Bejar', 'Sandra Li', 'Wei Chen', 'Tom Xu'
];
const PREVIEWS: Array<string> = [
  "Hi there! I'm using WeChat",
  "Let's meet at 3pm tomorrow",
  'Sent a sticker',
  'Did you see the new build?',
  '[Photo]'
];
const DISCOVER: Array<string> = [
  'Moments', 'Channels', 'Scan', 'Shake', 'Search', 'Top Stories',
  'Mini Programs', 'Nearby'
];

export class WeChatArkTs {
  private ctx: UIContext;
  private count: number = 0;
  /** Held so the screen's memory cost shows up in RSS, same as the Rust side. */
  private root: FrameNode | null = null;

  constructor(ctx: UIContext) {
    this.ctx = ctx;
  }

  private text(s: string, size: number, color: number, w: number, h: number): FrameNode {
    this.count++;
    const n: typeNode.Text = typeNode.createNode(this.ctx, 'Text');
    n.initialize(s);
    n.attribute.fontSize(size).fontColor(color).width(w).height(h);
    return n;
  }

  private col(w: number, h: number, bg: number): typeNode.Column {
    this.count++;
    const n: typeNode.Column = typeNode.createNode(this.ctx, 'Column');
    n.attribute.width(w).height(h).backgroundColor(bg);
    return n;
  }

  private row(w: number, h: number, bg: number): typeNode.Row {
    this.count++;
    const n: typeNode.Row = typeNode.createNode(this.ctx, 'Row');
    n.attribute.width(w).height(h).backgroundColor(bg);
    return n;
  }

  /** Real Image node, no source — same as the Rust side, so no decode cost. */
  private avatar(size: number): FrameNode {
    this.count++;
    const n: typeNode.Image = typeNode.createNode(this.ctx, 'Image');
    n.attribute.width(size).height(size).backgroundColor(AVATAR).borderRadius(6);
    return n;
  }

  /** Nav bar: 3 nodes. */
  private nav(title: string): FrameNode {
    const n = this.row(W, 44, NAV);
    n.appendChild(this.text(title, 17, TEXT_C, 300, 24));
    n.appendChild(this.text('+', 20, TEXT_C, 40, 24));
    return n;
  }

  /** Tab bar: 10 nodes. */
  private tabs(active: number): FrameNode {
    const bar = this.row(W, 52, NAV);
    const labels: Array<string> = ['Chats', 'Contacts', 'Discover', 'Me'];
    for (let i = 0; i < labels.length; i++) {
      const c: number = i === active ? GREEN : SUBTLE;
      bar.appendChild(this.col(20, 20, c));
      bar.appendChild(this.text(labels[i], 10, c, 70, 14));
    }
    return bar;
  }

  /** Search bar: 2 nodes. */
  private search(): FrameNode {
    const s = this.row(W - 16, 32, SURFACE);
    s.appendChild(this.text('Search', 13, SUBTLE, 200, 18));
    return s;
  }

  /** ChatPreview: 6 nodes. */
  private chatRow(i: number): FrameNode {
    const r = this.row(W, 64, SURFACE);
    r.appendChild(this.avatar(40));
    const mid = this.col(W - 130, 56, SURFACE);
    mid.appendChild(this.text(NAMES[i % NAMES.length], 15, TEXT_C, W - 140, 20));
    mid.appendChild(this.text(PREVIEWS[i % PREVIEWS.length], 12, SUBTLE, W - 140, 18));
    r.appendChild(mid);
    r.appendChild(this.text('yesterday', 10, SUBTLE, 60, 16));
    return r;
  }

  /** Chat bubble: 4 nodes. */
  private messageRow(i: number): FrameNode {
    const r = this.row(W, 52, BG);
    r.appendChild(this.avatar(34));
    const b = this.col(W - 120, 40, i % 2 === 0 ? SURFACE : GREEN);
    b.appendChild(this.text(PREVIEWS[i % PREVIEWS.length], 14,
      i % 2 === 0 ? TEXT_C : SURFACE, W - 130, 20));
    r.appendChild(b);
    return r;
  }

  /** Contact: 3 nodes. */
  private contactRow(i: number): FrameNode {
    const r = this.row(W, 48, SURFACE);
    r.appendChild(this.avatar(32));
    r.appendChild(this.text(NAMES[i % NAMES.length], 15, TEXT_C, W - 80, 20));
    return r;
  }

  /** Discover entry: 4 nodes. */
  private discoverRow(i: number): FrameNode {
    const r = this.row(W, 50, SURFACE);
    r.appendChild(this.avatar(24));
    r.appendChild(this.text(DISCOVER[i % DISCOVER.length], 15, TEXT_C, W - 100, 20));
    r.appendChild(this.text('›', 15, SUBTLE, 20, 20));
    return r;
  }

  /** Moment: 5 nodes. */
  private momentRow(i: number): FrameNode {
    const m = this.col(W, 150, SURFACE);
    m.appendChild(this.avatar(36));
    m.appendChild(this.text(NAMES[i % NAMES.length], 14, GREEN, W - 20, 20));
    m.appendChild(this.text(PREVIEWS[i % PREVIEWS.length], 13, TEXT_C, W - 20, 20));
    m.appendChild(this.avatar(64));
    return m;
  }

  /** Build one screen. Returns the node count; time is measured by the caller. */
  build(screen: string, rows: number): number {
    this.count = 0;
    const root = this.col(W, 780, BG);

    let title: string = 'WeChat';
    if (screen === 'chat') {
      title = 'Chat';
    } else if (screen === 'contacts') {
      title = 'Contacts';
    } else if (screen === 'discover') {
      title = 'Discover';
    } else if (screen === 'moments') {
      title = 'Moments';
    } else if (screen === 'me') {
      title = 'Me';
    }
    root.appendChild(this.nav(title));

    if (screen === 'chats') {
      root.appendChild(this.search());
      for (let i = 0; i < rows; i++) {
        root.appendChild(this.chatRow(i));
      }
    } else if (screen === 'chat') {
      for (let i = 0; i < rows; i++) {
        root.appendChild(this.messageRow(i));
      }
      const c = this.row(W, 48, NAV);
      c.appendChild(this.text('Message', 14, SUBTLE, W - 100, 20));
      c.appendChild(this.text('Send', 14, GREEN, 50, 20));
      root.appendChild(c);
    } else if (screen === 'contacts') {
      root.appendChild(this.search());
      for (let i = 0; i < rows; i++) {
        root.appendChild(this.contactRow(i));
      }
    } else if (screen === 'discover') {
      for (let i = 0; i < rows; i++) {
        root.appendChild(this.discoverRow(i));
      }
    } else if (screen === 'moments') {
      for (let i = 0; i < rows; i++) {
        root.appendChild(this.momentRow(i));
      }
    } else if (screen === 'me') {
      const h = this.row(W, 110, SURFACE);
      h.appendChild(this.avatar(64));
      const hc = this.col(W - 120, 70, SURFACE);
      hc.appendChild(this.text('Rik Arends', 20, TEXT_C, 200, 26));
      hc.appendChild(this.text('WeChat ID: rik', 12, SUBTLE, 200, 18));
      hc.appendChild(this.text('＋ Status', 12, SUBTLE, 200, 18));
      h.appendChild(hc);
      h.appendChild(this.avatar(20));
      h.appendChild(this.text('›', 15, SUBTLE, 20, 20));
      root.appendChild(h);
      for (let i = 0; i < rows; i++) {
        root.appendChild(this.discoverRow(i));
      }
    }

    root.appendChild(this.tabs(0));
    this.root = root;
    return this.count;
  }

  /** Drop the held screen. */
  clear(): void {
    this.root = null;
  }

  /**
   * Screens accumulated for the memory arm. A super-app keeps many pages alive
   * at once, so what matters is the cost of a stack of them, not of one.
   */
  private kept: Array<FrameNode> = new Array<FrameNode>();

  /** Build a screen and keep it. Returns how many are now held. */
  keep(screen: string, rows: number): number {
    this.build(screen, rows);
    if (this.root !== null) {
      this.kept.push(this.root);
    }
    this.root = null;
    return this.kept.length;
  }

  /** Drop every kept screen. */
  dropKept(): number {
    const n = this.kept.length;
    this.kept.length = 0;
    return n;
  }
}

/**
 * A super-app's normal background diet, run on the ArkTS thread.
 *
 * This is the whole point of the exercise. The microbenchmark measured both
 * paths on an idle JS thread, which is not the condition a super-app operates
 * in — its ArkTS thread is continuously parsing messages, resolving promises
 * and running mini-program logic. The napi round trip went from 31 µs idle to
 * 1051 µs under exactly that kind of load in the octos-one port, and 700 µs of
 * that was queue wait.
 *
 * So: JSON parse/stringify (message payloads), array churn (list models), and
 * promise resolution (async plumbing). Deliberately allocation-heavy, because
 * that is what makes a JS thread slow in a way that is hard to schedule around.
 */
export class LoadGenerator {
  private timer: number = -1;
  private sink: Array<string> = new Array<string>();

  /** One unit of work: roughly what handling a batch of messages costs. */
  private static tick(): void {
    const msgs: Array<string> = new Array<string>();
    for (let i = 0; i < 120; i++) {
      const o: Record<string, string> = {
        'id': 'msg-' + i,
        'from': 'user-' + (i % 17),
        'body': "Hi there! I'm using WeChat, message number " + i,
        'ts': '1785134173153'
      };
      msgs.push(JSON.stringify(o));
    }
    let total: number = 0;
    for (let i = 0; i < msgs.length; i++) {
      const parsed: Record<string, string> = JSON.parse(msgs[i]) as Record<string, string>;
      total += parsed['body'].length;
    }
    if (total < 0) {
      console.log('unreachable');
    }
  }

  /**
   * Start hammering the JS thread. `periodMs` between bursts — small enough
   * that the thread is rarely idle, large enough that the app is not declared
   * unresponsive and killed.
   */
  start(periodMs: number): void {
    if (this.timer !== -1) {
      return;
    }
    this.timer = setInterval(() => {
      LoadGenerator.tick();
      Promise.resolve(0).then((v: number) => {
        this.sink.push('' + v);
        if (this.sink.length > 64) {
          this.sink.length = 0;
        }
      });
    }, periodMs);
  }

  stop(): void {
    if (this.timer !== -1) {
      clearInterval(this.timer);
      this.timer = -1;
    }
    this.sink.length = 0;
  }
}
