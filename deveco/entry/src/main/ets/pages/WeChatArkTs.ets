import { typeNode, FrameNode, UIContext } from '@kit.ArkUI';
import splash from 'libsplash_oh.so';

/**
 * The WeChat demo, built in ArkTS through `typeNode`.
 *
 * The same app as `crates/splash-oh/src/wechat/mod.rs`: same twelve chats, same
 * CJK message bodies, same four tabs, same pushed routes. The data comes across
 * from Rust so neither implementation can render different content, and both
 * mount into the same `NodeContent`, so they can be swapped at runtime and
 * compared in one process on one device.
 *
 * `typeNode.createNode` produces the same underlying native node the NDK
 * produces. That is what makes this a control rather than a different app that
 * happens to look similar.
 */

const BG: number = 0xFFEDEDED;
const SURFACE: number = 0xFFFFFFFF;
const NAV_BG: number = 0xFFF7F7F7;
const TEXT_C: number = 0xFF191919;
const SUBTLE: number = 0xFF9A9A9A;
const DIVIDER: number = 0xFFE5E5E5;
const GREEN: number = 0xFF07C160;
const BUBBLE_OUT: number = 0xFF95EC69;
const AVATAR_BG: number = 0xFFC8C8C8;
const LINK_BLUE: number = 0xFF576B95;

const W: number = 402;
const PAGE_H: number = 780;

// Event ids, mirroring wechat/mod.rs.
export const TAB_BASE: number = 100;
export const BACK: number = 110;
export const LINK_MOMENTS: number = 120;
export const LINK_MY_PROFILE: number = 121;
export const LINK_ADD_CONTACT: number = 122;
export const TOGGLE_IMPL: number = 130;
export const CHAT_BASE: number = 1000;


/** Route on top of the navigation stack, mirroring the Rust enum. */
export enum RouteKind { Root, Chat, Moments, AddContact, MyProfile }

export class WeChatArkTs {
  private ctx: UIContext;
  private count: number = 0;
  private root: FrameNode | null = null;
  /** Chats as "namepreviewtimestamp", straight from Rust. */
  private chats: Array<string> = new Array<string>();
  private tabs_: Array<string> = new Array<string>();
  private discover: Array<string> = new Array<string>();
  private profile: Array<string> = new Array<string>();
  private contactActions: Array<string> = new Array<string>();
  private contacts: Array<string> = new Array<string>();
  private moments: Array<string> = new Array<string>();

  tab: number = 0;
  route: RouteKind = RouteKind.Root;
  chatId: number = 1;

  constructor(ctx: UIContext) {
    this.ctx = ctx;
    this.chats = splash.wechatChats() as Array<string>;
    this.tabs_ = splash.wechatAssets('tabs') as Array<string>;
    this.discover = splash.wechatAssets('discover') as Array<string>;
    this.profile = splash.wechatAssets('profile') as Array<string>;
    this.contactActions = splash.wechatAssets('contacts') as Array<string>;
    this.contacts = splash.wechatContacts() as Array<string>;
    this.moments = splash.wechatMoments() as Array<string>;
  }

  // --- leaves --------------------------------------------------------------

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

  /** A tappable row. ArkTS gets a real closure rather than an integer id. */
  private tapRow(w: number, h: number, bg: number, tap: number): typeNode.Row {
    const n = this.row(w, h, bg);
    n.attribute.onClick(() => { this.onTap(tap); });
    return n;
  }

  /**
   * An image from the reference app's own asset set, in `rawfile/wechat/`.
   * The placeholder fill stays behind photos so a still-decoding file shows as
   * a grey box rather than a hole.
   */
  private image(file: string, w: number, h: number, radius: number): FrameNode {
    this.count++;
    const n: typeNode.Image = typeNode.createNode(this.ctx, 'Image');
    n.initialize($rawfile('wechat/' + file));
    n.attribute.width(w).height(h).backgroundColor(AVATAR_BG)
      .borderRadius(radius).objectFit(ImageFit.Cover);
    return n;
  }

  private avatar(file: string, size: number): FrameNode {
    return this.image(file, size, size, size * 0.12);
  }

  /** A menu icon. No placeholder fill: grey behind a monochrome glyph hides it. */
  private icon(file: string, size: number): FrameNode {
    this.count++;
    const n: typeNode.Image = typeNode.createNode(this.ctx, 'Image');
    n.initialize($rawfile('wechat/' + file));
    n.attribute.width(size).height(size).objectFit(ImageFit.Contain);
    return n;
  }

  private divider(): FrameNode {
    return this.col(W, 1, DIVIDER);
  }

  // --- navigation ----------------------------------------------------------

  /** Called by the switcher in Index.ets when the toggle is hit. */
  onToggle: (() => void) | null = null;

  private onTap(tap: number): void {
    if (tap === 0) {
      return;
    }
    if (tap === TOGGLE_IMPL) {
      if (this.onToggle !== null) {
        this.onToggle();
      }
      return;
    }
    if (tap === BACK) {
      if (this.route === RouteKind.Root) {
        return;
      }
      this.route = RouteKind.Root;
    } else if (tap === LINK_MOMENTS) {
      this.route = RouteKind.Moments;
    } else if (tap === LINK_MY_PROFILE) {
      this.route = RouteKind.MyProfile;
    } else if (tap === LINK_ADD_CONTACT) {
      this.route = RouteKind.AddContact;
    } else if (tap >= TAB_BASE && tap < TAB_BASE + 4) {
      this.tab = tap - TAB_BASE;
      this.route = RouteKind.Root;
    } else if (tap >= CHAT_BASE) {
      this.chatId = tap - CHAT_BASE;
      this.route = RouteKind.Chat;
    } else {
      return;
    }
    if (this.remount !== null) {
      this.remount();
    }
  }

  /** Set by Index.ets so a tap can re-render and re-mount. */
  remount: (() => void) | null = null;

  // --- chrome --------------------------------------------------------------

  private header(title: string, withBack: boolean): FrameNode {
    const h = this.row(W, 46, NAV_BG);
    if (withBack) {
      const b = this.tapRow(56, 46, NAV_BG, BACK);
      b.appendChild(this.text('‹', 24, TEXT_C, 44, 30));
      h.appendChild(b);
    } else {
      h.appendChild(this.col(56, 46, NAV_BG));
    }
    h.appendChild(this.text(title, 17, TEXT_C, W - 170, 26));
    const t = this.tapRow(90, 46, NAV_BG, TOGGLE_IMPL);
    t.appendChild(this.text('ArkTS ⇄', 12, GREEN, 84, 20));
    h.appendChild(t);
    return h;
  }

  private tabBar(active: number): FrameNode {
    const bar = this.row(W, 56, NAV_BG);
    for (let i = 0; i < this.tabs_.length; i++) {
      const p: Array<string> = this.tabs_[i].split('|');
      const c: number = i === active ? GREEN : SUBTLE;
      const t = this.tapRow(W / 4, 56, NAV_BG, TAB_BASE + i);
      const inner = this.col(W / 4 - 4, 50, NAV_BG);
      inner.appendChild(this.icon(p[1], 24));
      inner.appendChild(this.text(p[0], 11, c, W / 4 - 6, 16));
      t.appendChild(inner);
      bar.appendChild(t);
    }
    return bar;
  }

  private searchBar(): FrameNode {
    const s = this.row(W, 44, BG);
    const inner = this.row(W - 24, 32, SURFACE);
    inner.attribute.borderRadius(6);
    inner.appendChild(this.text('🔍  Search', 13, SUBTLE, 200, 20));
    s.appendChild(inner);
    return s;
  }

  // --- rows ----------------------------------------------------------------

  private chatRow(i: number): FrameNode {
    const parts: Array<string> = this.chats[i].split('|');
    const r = this.tapRow(W, 66, SURFACE, CHAT_BASE + i + 1);
    r.appendChild(this.avatar(parts[3], 44));
    const mid = this.col(W - 150, 60, SURFACE);
    mid.appendChild(this.text(parts[0], 16, TEXT_C, W - 160, 22));
    mid.appendChild(this.text(parts[1], 13, SUBTLE, W - 160, 20));
    r.appendChild(mid);
    r.appendChild(this.text(parts[2], 11, SUBTLE, 70, 18));
    return r;
  }

  private messageRow(spec: string): FrameNode {
    const parts: Array<string> = spec.split('|');
    const outgoing: boolean = parts[0] === 'o';
    const r = this.row(W, 58, BG);
    if (outgoing) {
      r.appendChild(this.col(W - 250, 50, BG));
    } else {
      r.appendChild(this.avatar(parts[2], 38));
    }
    const bubble = this.col(200, 44, outgoing ? BUBBLE_OUT : SURFACE);
    bubble.attribute.borderRadius(6);
    bubble.appendChild(this.text(parts[1], 15, TEXT_C, 188, 24));
    r.appendChild(bubble);
    if (outgoing) {
      r.appendChild(this.avatar(parts[2], 38));
    }
    return r;
  }

  private menuRow(label: string, file: string, tap: number): FrameNode {
    const r = this.tapRow(W, 52, SURFACE, tap);
    r.appendChild(this.icon(file, 26));
    r.appendChild(this.text(label, 15, TEXT_C, W - 110, 22));
    r.appendChild(this.text('›', 16, SUBTLE, 24, 22));
    return r;
  }

  private contactRow(name: string, file: string): FrameNode {
    const r = this.tapRow(W, 50, SURFACE, 0);
    r.appendChild(this.avatar(file, 34));
    r.appendChild(this.text(name, 15, TEXT_C, W - 90, 22));
    return r;
  }

  private sectionLabel(s: string): FrameNode {
    const r = this.row(W, 26, BG);
    r.appendChild(this.text(s, 12, SUBTLE, W - 20, 18));
    return r;
  }

  private momentRow(spec: string): FrameNode {
    const parts: Array<string> = spec.split('|');
    const photo: boolean = parts[3] !== '';
    const m = this.row(W, photo ? 170 : 82, SURFACE);
    m.appendChild(this.avatar(parts[2], 40));
    const c = this.col(W - 80, photo ? 160 : 72, SURFACE);
    c.appendChild(this.text(parts[0], 14, LINK_BLUE, W - 90, 20));
    c.appendChild(this.text(parts[1], 14, TEXT_C, W - 90, 22));
    if (photo) {
      c.appendChild(this.image(parts[3], 110, 100, 4));
    }
    m.appendChild(c);
    return m;
  }

  // --- build ---------------------------------------------------------------

  /** Build the whole app for the current state. Returns the node count. */
  build(): number {
    this.count = 0;
    const root = this.col(W, PAGE_H, BG);

    let title: string = 'WeChat';
    let withBack: boolean = false;
    if (this.route === RouteKind.Root) {
      const titles: Array<string> = ['WeChat', 'Contacts', 'Discover', 'Me'];
      title = titles[this.tab];
    } else if (this.route === RouteKind.Chat) {
      title = this.chats[this.chatId - 1].split('|')[0];
      withBack = true;
    } else if (this.route === RouteKind.Moments) {
      title = 'Moments';
      withBack = true;
    } else if (this.route === RouteKind.AddContact) {
      title = 'Add Contact';
      withBack = true;
    } else {
      title = 'My Profile';
      withBack = true;
    }
    root.appendChild(this.header(title, withBack));

    this.count++;
    const scroll: typeNode.Scroll = typeNode.createNode(this.ctx, 'Scroll');
    // Align top: a Scroll centres content shorter than itself by default.
    scroll.attribute.width(W).height(PAGE_H - 46 - 56).align(Alignment.Top);
    const body = this.col(W, 0, BG);

    if (this.route === RouteKind.Root) {
      if (this.tab === 0) {
        body.appendChild(this.searchBar());
        for (let i = 0; i < this.chats.length; i++) {
          body.appendChild(this.chatRow(i));
          body.appendChild(this.divider());
        }
      } else if (this.tab === 1) {
        body.appendChild(this.searchBar());
        for (let i = 0; i < this.contactActions.length; i++) {
          const p: Array<string> = this.contactActions[i].split('|');
          body.appendChild(this.menuRow(p[0], p[1], LINK_ADD_CONTACT));
        }
        for (let i = 0; i < this.contacts.length; i++) {
          const p: Array<string> = this.contacts[i].split('|');
          if (p[0] === '#') {
            body.appendChild(this.sectionLabel(p[1]));
          } else {
            body.appendChild(this.contactRow(p[0], p[1]));
          }
        }
      } else if (this.tab === 2) {
        for (let i = 0; i < this.discover.length; i++) {
          const p: Array<string> = this.discover[i].split('|');
          body.appendChild(this.menuRow(p[0], p[1], p[0] === 'Moments' ? LINK_MOMENTS : 0));
          if (i === 0 || i === 2 || i === 4) {
            body.appendChild(this.col(W, 10, BG));
          }
        }
      } else {
        const head = this.tapRow(W, 116, SURFACE, LINK_MY_PROFILE);
        head.appendChild(this.avatar('default_avatar.png', 64));
        const hc = this.col(W - 160, 80, SURFACE);
        hc.appendChild(this.text('Rik Arends', 20, TEXT_C, W - 170, 28));
        hc.appendChild(this.text('WeChat ID: rikarends', 12, SUBTLE, W - 170, 20));
        hc.appendChild(this.text('＋ Status', 12, SUBTLE, W - 170, 20));
        head.appendChild(hc);
        head.appendChild(this.icon('qr_icon.png', 20));
        head.appendChild(this.text('›', 16, SUBTLE, 24, 22));
        body.appendChild(head);
        body.appendChild(this.col(W, 10, BG));
        for (let i = 0; i < this.profile.length; i++) {
          const p: Array<string> = this.profile[i].split('|');
          body.appendChild(this.menuRow(p[0], p[1], p[0] === 'My Posts' ? LINK_MOMENTS : 0));
          if (i === 2) {
            body.appendChild(this.col(W, 10, BG));
          }
        }
      }
    } else if (this.route === RouteKind.Chat) {
      const msgs: Array<string> = splash.wechatMessages(this.chatId) as Array<string>;
      for (let i = 0; i < msgs.length; i++) {
        body.appendChild(this.messageRow(msgs[i]));
      }
      const c = this.row(W, 52, NAV_BG);
      c.appendChild(this.icon('smiley_face_bw.png', 28));
      const input = this.col(W - 130, 34, SURFACE);
      input.attribute.borderRadius(5);
      c.appendChild(input);
      c.appendChild(this.text('😊', 18, TEXT_C, 30, 26));
      c.appendChild(this.text('＋', 18, TEXT_C, 30, 26));
      body.appendChild(c);
    } else if (this.route === RouteKind.Moments) {
      body.appendChild(this.image('hero.jpg', W, 150, 0));
      for (let i = 0; i < this.moments.length; i++) {
        body.appendChild(this.momentRow(this.moments[i]));
        body.appendChild(this.divider());
      }
    } else if (this.route === RouteKind.AddContact) {
      body.appendChild(this.searchBar());
      body.appendChild(this.sectionLabel('My WeChat ID: rikarends'));
      const entries: Array<string> = ['Scan|scan_qr.png', 'Mobile Contacts|mobile_contacts.png',
        'Official Accounts|official_accounts.png', 'WeCom Contacts|wecom_contacts.png'];
      for (let i = 0; i < entries.length; i++) {
        const p: Array<string> = entries[i].split('|');
        body.appendChild(this.menuRow(p[0], p[1], 0));
      }
    } else {
      const entries: Array<string> = ['Profile Photo|default_avatar.png', 'Name|tags.png',
        'WeChat ID|qr_icon.png', 'My QR Code|qr_green.png', 'More|settings.png'];
      for (let i = 0; i < entries.length; i++) {
        const p: Array<string> = entries[i].split('|');
        body.appendChild(this.menuRow(p[0], p[1], 0));
      }
    }

    scroll.appendChild(body);
    root.appendChild(scroll);

    if (this.route === RouteKind.Root) {
      root.appendChild(this.tabBar(this.tab));
    } else {
      root.appendChild(this.col(W, 56, NAV_BG));
    }

    this.root = root;
    return this.count;
  }

  /** The built tree, for mounting. */
  node(): FrameNode | null {
    return this.root;
  }

  clear(): void {
    this.root = null;
  }
}

/**
 * A super-app's normal background diet, run on the ArkTS thread.
 *
 * The microbenchmark measured both paths on an idle JS thread, which is not the
 * condition a super-app operates in — its ArkTS thread is continuously parsing
 * messages, resolving promises and running mini-program logic. JSON round trips,
 * array churn and promise resolution, because allocation is what makes a JS
 * thread slow in a way that is hard to schedule around.
 */
export class LoadGenerator {
  private timer: number = -1;
  private sink: Array<string> = new Array<string>();

  private static tick(): void {
    const msgs: Array<string> = new Array<string>();
    for (let i = 0; i < 120; i++) {
      const o: Record<string, string> = {
        'id': 'msg-' + i,
        'from': 'user-' + (i % 17),
        'body': '体議速人幅触無持編聞組込 ' + i,
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
