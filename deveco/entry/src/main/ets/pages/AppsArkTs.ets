import { typeNode, FrameNode, UIContext } from '@kit.ArkUI';
import splash from 'libsplash_oh.so';

/**
 * ArkTS twins of the three apps ported in `crates/splash-oh/src/apps/`:
 * Taobao, TikTok and Wonderous.
 *
 * Each builds the identical tree its Rust counterpart builds — same node types,
 * same counts, same attributes, same assets out of `rawfile/<app>/`. The data
 * comes across from Rust so neither side can render different content, and node
 * counts are checked at runtime, because "the other one is faster because it
 * quietly builds less" is the failure mode worth guarding against.
 *
 * The three apps are here rather than one per file because they share every
 * primitive; only the screens differ.
 */

const W: number = 402;
const PAGE_H: number = 780;

// ---------------------------------------------------------------------------
// Shared builders, mirroring apps/ui.rs
// ---------------------------------------------------------------------------

class Ui {
  ctx: UIContext;
  count: number = 0;

  constructor(ctx: UIContext) {
    this.ctx = ctx;
  }

  text(s: string, size: number, color: number, w: number, h: number): FrameNode {
    this.count++;
    const n: typeNode.Text = typeNode.createNode(this.ctx, 'Text');
    n.initialize(s);
    n.attribute.fontSize(size).fontColor(color).width(w).height(h);
    return n;
  }

  textW(s: string, size: number, color: number, w: number, h: number, weight: number): FrameNode {
    this.count++;
    const n: typeNode.Text = typeNode.createNode(this.ctx, 'Text');
    n.initialize(s);
    n.attribute.fontSize(size).fontColor(color).width(w).height(h)
      .fontWeight(weight * 100);
    return n;
  }

  col(w: number, h: number, bg: number): typeNode.Column {
    this.count++;
    const n: typeNode.Column = typeNode.createNode(this.ctx, 'Column');
    n.attribute.width(w).height(h).backgroundColor(bg);
    return n;
  }

  row(w: number, h: number, bg: number): typeNode.Row {
    this.count++;
    const n: typeNode.Row = typeNode.createNode(this.ctx, 'Row');
    n.attribute.width(w).height(h).backgroundColor(bg);
    return n;
  }

  stack(w: number, h: number, bg: number): typeNode.Stack {
    this.count++;
    const n: typeNode.Stack = typeNode.createNode(this.ctx, 'Stack');
    n.attribute.width(w).height(h).backgroundColor(bg);
    return n;
  }

  tapRow(w: number, h: number, bg: number, tap: number, cb: (t: number) => void): typeNode.Row {
    const n = this.row(w, h, bg);
    n.attribute.onClick(() => { cb(tap); });
    return n;
  }

  tapCol(w: number, h: number, bg: number, tap: number, cb: (t: number) => void): typeNode.Column {
    const n = this.col(w, h, bg);
    n.attribute.onClick(() => { cb(tap); });
    return n;
  }

  /** A photo: placeholder fill behind it so a decoding file reads as grey. */
  photo(app: string, file: string, w: number, h: number, radius: number): FrameNode {
    this.count++;
    const n: typeNode.Image = typeNode.createNode(this.ctx, 'Image');
    n.initialize($rawfile(app + '/' + file));
    n.attribute.width(w).height(h).backgroundColor(0xFFD8D8D8)
      .borderRadius(radius).objectFit(ImageFit.Cover);
    return n;
  }

  /** An icon: no fill, or a grey square hides the glyph and hides a failure. */
  icon(app: string, file: string, size: number): FrameNode {
    this.count++;
    const n: typeNode.Image = typeNode.createNode(this.ctx, 'Image');
    n.initialize($rawfile(app + '/' + file));
    n.attribute.width(size).height(size).objectFit(ImageFit.Contain);
    return n;
  }

  /** Scroll pinned to the top; the default centres short content. */
  scroll(h: number): typeNode.Scroll {
    this.count++;
    const n: typeNode.Scroll = typeNode.createNode(this.ctx, 'Scroll');
    n.attribute.width(W).height(h).align(Alignment.Top);
    return n;
  }

  spacer(w: number, h: number): FrameNode {
    return this.col(w, h, 0);
  }

  divider(w: number, color: number): FrameNode {
    return this.col(w, 1, color);
  }
}

// ---------------------------------------------------------------------------
// Taobao
// ---------------------------------------------------------------------------

const TB_BG = 0xFFF2F2F2;
const TB_SURFACE = 0xFFFFFFFF;
const TB_ORANGE = 0xFFFF5000;
const TB_TEXT = 0xFF1A1A1A;
const TB_SUBTLE = 0xFF9A9A9A;
const TB_TERMS: Array<string> = ['连衣裙', '运动鞋', '耳机', '零食', '口红', '手机壳'];

export class Taobao {
  private u: Ui;
  /** "title|subtitle|price|image", from Rust. */
  private products: Array<string> = new Array<string>();
  /** "label|icon", from Rust. */
  private tabs: Array<string> = new Array<string>();
  tab: number = 0;
  detail: number = -1;
  onTap: (t: number) => void = () => {};

  constructor(ctx: UIContext) {
    this.u = new Ui(ctx);
    this.products = splash.appData('taobao.products') as Array<string>;
    this.tabs = splash.appData('taobao.tabs') as Array<string>;
  }

  private header(): FrameNode {
    const u = this.u;
    const h = u.row(W, 50, TB_ORANGE);
    h.appendChild(u.icon('taobao', 'search.svg', 22));
    const field = u.row(W - 110, 32, TB_SURFACE);
    field.attribute.borderRadius(16);
    field.appendChild(u.text('搜索淘宝商品', 13, TB_SUBTLE, W - 130, 20));
    h.appendChild(field);
    h.appendChild(u.icon('taobao', 'meatballs_menu.png', 22));
    return h;
  }

  private tabBar(active: number): FrameNode {
    const u = this.u;
    const bar = u.row(W, 56, TB_SURFACE);
    for (let i = 0; i < this.tabs.length; i++) {
      const p: Array<string> = this.tabs[i].split('|');
      const c: number = i === active ? TB_ORANGE : TB_SUBTLE;
      const t = u.tapCol(W / 5, 56, TB_SURFACE, 200 + i, this.onTap);
      t.appendChild(u.icon('taobao', p[1], 22));
      t.appendChild(u.text(p[0], 10, c, W / 5 - 2, 14));
      bar.appendChild(t);
    }
    return bar;
  }

  private product(i: number): FrameNode {
    const u = this.u;
    const p: Array<string> = this.products[i].split('|');
    const cw = (W - 24) / 2;
    const cell = u.tapCol(cw, 250, TB_SURFACE, 2000 + i, this.onTap);
    cell.attribute.borderRadius(8);
    cell.appendChild(u.photo('taobao', p[3], cw, 150, 8));
    const info = u.col(cw - 12, 92, TB_SURFACE);
    info.appendChild(u.textW(p[0], 13, TB_TEXT, cw - 16, 36, 5));
    info.appendChild(u.text(p[1], 10, TB_SUBTLE, cw - 16, 16));
    info.appendChild(u.textW('¥' + p[2], 16, TB_ORANGE, cw - 16, 24, 7));
    cell.appendChild(info);
    return cell;
  }

  private simpleList(body: FrameNode, rows: number, label: string): void {
    const u = this.u;
    for (let i = 0; i < rows; i++) {
      const r = u.row(W, 62, TB_SURFACE);
      r.appendChild(u.photo('taobao', 'default_avatar.png', 42, 42, 21));
      const c = u.col(W - 90, 50, TB_SURFACE);
      c.appendChild(u.textW(label + ' ' + (i + 1), 14, TB_TEXT, W - 100, 20, 5));
      const p: Array<string> = this.products[i % this.products.length].split('|');
      c.appendChild(u.text(p[1], 11, TB_SUBTLE, W - 100, 18));
      r.appendChild(c);
      body.appendChild(r);
      body.appendChild(u.divider(W, 0xFFEEEEEE));
    }
  }

  private detailScreen(body: FrameNode, idx: number): void {
    const u = this.u;
    const p: Array<string> = this.products[Math.min(idx, this.products.length - 1)].split('|');
    body.appendChild(u.photo('taobao', p[3], W, 300, 0));
    const info = u.col(W, 120, TB_SURFACE);
    info.appendChild(u.textW('¥' + p[2], 24, TB_ORANGE, W - 20, 34, 7));
    info.appendChild(u.textW(p[0], 15, TB_TEXT, W - 20, 44, 5));
    info.appendChild(u.text(p[1], 11, TB_SUBTLE, W - 20, 18));
    body.appendChild(info);
    body.appendChild(u.spacer(W, 8));
    const rows: Array<string> = ['配送 至 北京|shipping_estimate.png', '正品保障|credit_cards.png',
      '官方旗舰店|store.png', '客服中心|help_center.png'];
    for (let i = 0; i < rows.length; i++) {
      const q: Array<string> = rows[i].split('|');
      const r = u.row(W, 46, TB_SURFACE);
      r.appendChild(u.icon('taobao', q[1], 20));
      r.appendChild(u.text(q[0], 13, TB_TEXT, W - 80, 20));
      r.appendChild(u.text('›', 14, TB_SUBTLE, 20, 20));
      body.appendChild(r);
    }
    const buy = u.row(W, 52, TB_SURFACE);
    buy.appendChild(u.icon('taobao', 'chat_bubble.png', 22));
    const cart = u.row(120, 38, 0xFFFFB000);
    cart.attribute.borderRadius(19);
    cart.appendChild(u.text('加入购物车', 13, TB_SURFACE, 112, 20));
    buy.appendChild(cart);
    const now = u.row(120, 38, TB_ORANGE);
    now.attribute.borderRadius(19);
    now.appendChild(u.text('立即购买', 13, TB_SURFACE, 112, 20));
    buy.appendChild(now);
    body.appendChild(buy);
  }

  build(): FrameNode {
    const u = this.u;
    u.count = 0;
    const root = u.col(W, PAGE_H, TB_BG);

    if (this.detail >= 0) {
      const h = u.row(W, 46, TB_SURFACE);
      const b = u.tapRow(56, 46, TB_SURFACE, 210, this.onTap);
      b.appendChild(u.text('‹', 24, TB_TEXT, 44, 30));
      h.appendChild(b);
      h.appendChild(u.textW('商品详情', 16, TB_TEXT, W - 120, 24, 5));
      root.appendChild(h);
      const body = u.col(W, 0, TB_BG);
      this.detailScreen(body, this.detail);
      const sc = u.scroll(PAGE_H - 46);
      sc.appendChild(body);
      root.appendChild(sc);
      this.root = root;
      return root;
    }

    root.appendChild(this.header());
    const body = u.col(W, 0, TB_BG);
    if (this.tab === 0) {
      const car = u.row(W, 140, TB_BG);
      car.appendChild(u.photo('taobao', 'carrousel_1.png', W - 20, 130, 8));
      body.appendChild(car);
      const terms = u.row(W, 40, TB_BG);
      for (let i = 0; i < TB_TERMS.length; i++) {
        const chip = u.row(58, 26, TB_SURFACE);
        chip.attribute.borderRadius(13);
        chip.appendChild(u.text(TB_TERMS[i], 11, TB_TEXT, 54, 16));
        terms.appendChild(chip);
      }
      body.appendChild(terms);
      let i = 0;
      while (i < this.products.length) {
        const r = u.row(W, 258, TB_BG);
        r.appendChild(this.product(i));
        if (i + 1 < this.products.length) {
          r.appendChild(this.product(i + 1));
        }
        body.appendChild(r);
        i += 2;
      }
    } else if (this.tab === 1) {
      this.simpleList(body, 10, '逛逛');
    } else if (this.tab === 2) {
      this.simpleList(body, 12, '消息');
    } else if (this.tab === 3) {
      this.simpleList(body, 8, '购物车');
    } else {
      this.simpleList(body, 9, '我的淘宝');
    }
    const sc = u.scroll(PAGE_H - 50 - 56);
    sc.appendChild(body);
    root.appendChild(sc);
    root.appendChild(this.tabBar(this.tab));
    this.root = root;
    return root;
  }

  private root: FrameNode | null = null;
  node(): FrameNode | null { return this.root; }
  nodes(): number { return this.u.count; }
  clear(): void { this.root = null; }
}

// ---------------------------------------------------------------------------
// TikTok
// ---------------------------------------------------------------------------

const TK_BLACK = 0xFF000000;
const TK_WHITE = 0xFFFFFFFF;
const TK_DIM = 0xFFBFBFBF;

export class TikTok {
  private u: Ui;
  /** "poster|author|caption|likes|comments", from Rust. */
  private reels: Array<string> = new Array<string>();
  tab: number = 1;
  reel: number = 0;
  sheet: boolean = false;
  feed: boolean = false;
  onTap: (t: number) => void = () => {};

  constructor(ctx: UIContext) {
    this.u = new Ui(ctx);
    this.reels = splash.appData('tiktok.reels') as Array<string>;
  }

  private header(active: number): FrameNode {
    const u = this.u;
    const h = u.row(W, 46, TK_BLACK);
    h.appendChild(u.spacer(60, 46));
    const labels: Array<string> = ['Following', 'For You'];
    for (let i = 0; i < labels.length; i++) {
      const c: number = i === active ? TK_WHITE : TK_DIM;
      const t = u.tapCol(110, 46, TK_BLACK, 300 + i, this.onTap);
      t.appendChild(u.textW(labels[i], 15, c, 106, 22, i === active ? 7 : 4));
      if (i === active) {
        t.appendChild(u.col(30, 2, TK_WHITE));
      }
      h.appendChild(t);
    }
    h.appendChild(u.icon('tiktok', 'search_icon.svg', 22));
    return h;
  }

  private action(file: string, count: string, tap: number): FrameNode {
    const u = this.u;
    const a = u.tapCol(60, 62, 0, tap, this.onTap);
    a.appendChild(u.icon('tiktok', file, 30));
    a.appendChild(u.text(count, 11, TK_WHITE, 56, 16));
    return a;
  }

  private reelNode(idx: number): FrameNode {
    const u = this.u;
    const p: Array<string> = this.reels[idx].split('|');
    const mediaH = PAGE_H - 46 - 56;
    const s = u.stack(W, mediaH, TK_BLACK);
    s.appendChild(u.photo('tiktok', p[0], W, mediaH, 0));
    const overlay = u.col(W, mediaH, 0);
    overlay.appendChild(u.spacer(W, mediaH - 130));
    const bottom = u.row(W, 120, 0);
    const cap = u.col(W - 90, 80, 0);
    cap.appendChild(u.textW(p[1], 15, TK_WHITE, W - 100, 22, 7));
    cap.appendChild(u.text(p[2], 13, TK_WHITE, W - 100, 36));
    const music = u.row(W - 100, 20, 0);
    music.appendChild(u.icon('tiktok', 'at_sign.png', 14));
    music.appendChild(u.text('original sound', 11, TK_DIM, 160, 16));
    cap.appendChild(music);
    bottom.appendChild(cap);
    const acts = u.col(70, 340, 0);
    acts.appendChild(u.photo('tiktok', 'default_avatar.png', 46, 46, 23));
    acts.appendChild(this.action('heart.png', p[3], 3001));
    acts.appendChild(this.action('chat_icon.svg', p[4], 3002));
    acts.appendChild(this.action('star_icon.svg', 'Save', 3003));
    acts.appendChild(this.action('share_icon.svg', 'Share', 3004));
    bottom.appendChild(acts);
    overlay.appendChild(bottom);
    s.appendChild(overlay);
    return s;
  }

  private tabBar(): FrameNode {
    const u = this.u;
    const bar = u.row(W, 56, TK_BLACK);
    const labels: Array<string> = ['Home', 'Discover', '', 'Inbox', 'Me'];
    for (let i = 0; i < labels.length; i++) {
      if (labels[i] === '') {
        const plus = u.row(56, 34, TK_WHITE);
        plus.attribute.borderRadius(8);
        plus.appendChild(u.icon('tiktok', 'plus.png', 22));
        bar.appendChild(plus);
        continue;
      }
      const c: number = i === 0 ? TK_WHITE : TK_DIM;
      const t = u.tapCol(W / 5, 56, TK_BLACK, 310 + i, this.onTap);
      t.appendChild(u.text(labels[i], 10, c, W / 5 - 2, 14));
      bar.appendChild(t);
    }
    return bar;
  }

  build(): FrameNode {
    const u = this.u;
    u.count = 0;

    if (this.sheet) {
      const root = u.col(W, PAGE_H, 0xFF161616);
      const h = u.row(W, 46, 0xFF161616);
      const b = u.tapRow(56, 46, 0, 310, this.onTap);
      b.appendChild(u.text('‹', 24, TK_WHITE, 44, 30));
      h.appendChild(b);
      const p0: Array<string> = this.reels[this.reel].split('|');
      h.appendChild(u.textW(p0[4] + ' comments', 15, TK_WHITE, W - 120, 22, 7));
      root.appendChild(h);
      const body = u.col(W, 0, 0xFF161616);
      for (let i = 0; i < 14; i++) {
        const p: Array<string> = this.reels[i % this.reels.length].split('|');
        const r = u.row(W, 62, 0xFF161616);
        r.appendChild(u.photo('tiktok', 'default_avatar.png', 36, 36, 18));
        const c = u.col(W - 130, 52, 0);
        c.appendChild(u.textW(p[1], 12, TK_DIM, W - 140, 18, 5));
        c.appendChild(u.text(p[2], 13, TK_WHITE, W - 140, 20));
        r.appendChild(c);
        const like = u.col(50, 44, 0);
        like.appendChild(u.icon('tiktok', 'heart.png', 16));
        like.appendChild(u.text('12', 10, TK_DIM, 44, 14));
        r.appendChild(like);
        body.appendChild(r);
      }
      const sc = u.scroll(PAGE_H - 46);
      sc.appendChild(body);
      root.appendChild(sc);
      this.root = root;
      return root;
    }

    const root = u.col(W, PAGE_H, TK_BLACK);
    if (this.feed) {
      root.appendChild(this.header(1));
      const body = u.col(W, 0, TK_BLACK);
      for (let i = 0; i < this.reels.length; i++) {
        body.appendChild(this.reelNode(i));
      }
      const sc = u.scroll(PAGE_H - 46 - 56);
      sc.appendChild(body);
      root.appendChild(sc);
      root.appendChild(this.tabBar());
      this.root = root;
      return root;
    }
    root.appendChild(this.header(this.tab));
    root.appendChild(this.reelNode(Math.min(this.reel, this.reels.length - 1)));
    root.appendChild(this.tabBar());
    this.root = root;
    return root;
  }

  private root: FrameNode | null = null;
  node(): FrameNode | null { return this.root; }
  nodes(): number { return this.u.count; }
  clear(): void { this.root = null; }
}

// ---------------------------------------------------------------------------
// Wonderous
// ---------------------------------------------------------------------------

const WO_BG = 0xFF1B1B1B;
const WO_SURFACE = 0xFF262626;
const WO_CREAM = 0xFFEDE9E1;
const WO_BRONZE = 0xFFC0A062;
const WO_SUBTLE = 0xFF9A948A;

export class Wonderous {
  private u: Ui;
  /** "title|image|place", from Rust. */
  private wonders: Array<string> = new Array<string>();
  private tabs: Array<string> = new Array<string>();
  private sections: Array<string> = new Array<string>();
  private artifacts: Array<string> = new Array<string>();
  private timeline: Array<string> = new Array<string>();
  tab: number = 0;
  wonder: number = 0;
  onTap: (t: number) => void = () => {};

  constructor(ctx: UIContext) {
    this.u = new Ui(ctx);
    this.wonders = splash.appData('wonderous.wonders') as Array<string>;
    this.tabs = splash.appData('wonderous.tabs') as Array<string>;
    this.sections = splash.appData('wonderous.sections') as Array<string>;
    this.artifacts = splash.appData('wonderous.artifacts') as Array<string>;
    this.timeline = splash.appData('wonderous.timeline') as Array<string>;
  }

  private hero(w: number): FrameNode {
    const u = this.u;
    const p: Array<string> = this.wonders[w].split('|');
    const s = u.stack(W, 320, WO_BG);
    s.appendChild(u.photo('wonderous', p[1], W, 320, 0));
    const overlay = u.col(W, 320, 0);
    overlay.appendChild(u.spacer(W, 200));
    overlay.appendChild(u.textW(p[0], 26, WO_CREAM, W - 40, 36, 7));
    overlay.appendChild(u.text(p[2], 12, WO_BRONZE, W - 40, 20));
    s.appendChild(overlay);
    return s;
  }

  private tabBar(active: number): FrameNode {
    const u = this.u;
    const bar = u.row(W, 56, WO_SURFACE);
    for (let i = 0; i < this.tabs.length; i++) {
      const p: Array<string> = this.tabs[i].split('|');
      const c: number = i === active ? WO_BRONZE : WO_SUBTLE;
      const t = u.tapCol(W / 4, 56, WO_SURFACE, 400 + i, this.onTap);
      t.appendChild(u.icon('wonderous', p[1], 20));
      t.appendChild(u.text(p[0], 10, c, W / 4 - 2, 14));
      bar.appendChild(t);
    }
    return bar;
  }

  build(): FrameNode {
    const u = this.u;
    u.count = 0;
    const root = u.col(W, PAGE_H, WO_BG);
    const h = u.row(W, 46, WO_BG);
    const b = u.tapRow(56, 46, WO_BG, 410, this.onTap);
    b.appendChild(u.text('‹', 22, WO_CREAM, 44, 28));
    h.appendChild(b);
    h.appendChild(u.textW(this.wonders[this.wonder].split('|')[0], 15, WO_CREAM, W - 120, 22, 5));
    root.appendChild(h);

    const body = u.col(W, 0, WO_BG);
    if (this.tab === 0) {
      body.appendChild(this.hero(this.wonder));
      for (let i = 0; i < this.sections.length; i++) {
        const p: Array<string> = this.sections[i].split('|');
        const sep = u.row(W, 30, WO_BG);
        sep.appendChild(u.divider(120, WO_BRONZE));
        body.appendChild(sep);
        body.appendChild(u.textW(p[0], 15, WO_BRONZE, W - 40, 24, 7));
        const lines = Math.max(Math.ceil(p[1].length / 46), 1);
        body.appendChild(u.text(p[1], 13, WO_CREAM, W - 40, lines * 20 + 8));
      }
      body.appendChild(u.spacer(W, 20));
    } else if (this.tab === 1) {
      for (let i = 0; i < this.wonders.length; i++) {
        const p: Array<string> = this.wonders[i].split('|');
        body.appendChild(u.photo('wonderous', p[1], W, 220, 6));
        body.appendChild(u.text(p[2], 11, WO_SUBTLE, W - 40, 18));
        body.appendChild(u.spacer(W, 10));
      }
    } else if (this.tab === 2) {
      const cw = (W - 24) / 2;
      let i = 0;
      while (i < this.artifacts.length) {
        const r = u.row(W, 150, WO_BG);
        for (let k = 0; k < 2; k++) {
          if (i + k >= this.artifacts.length) {
            break;
          }
          const cell = u.tapCol(cw, 142, WO_SURFACE, 4000 + i + k, this.onTap);
          cell.attribute.borderRadius(8);
          cell.appendChild(u.photo('wonderous', this.artifacts[i + k], cw, 142, 8));
          r.appendChild(cell);
        }
        body.appendChild(r);
        i += 2;
      }
    } else {
      for (let i = 0; i < this.timeline.length; i++) {
        const p: Array<string> = this.timeline[i].split('|');
        const r = u.row(W, 60, WO_BG);
        r.appendChild(u.textW(p[0], 12, WO_BRONZE, 76, 20, 7));
        r.appendChild(u.divider(1, WO_SUBTLE));
        r.appendChild(u.text(p[1], 13, WO_CREAM, W - 110, 40));
        body.appendChild(r);
      }
    }
    const sc = u.scroll(PAGE_H - 46 - 56);
    sc.appendChild(body);
    root.appendChild(sc);
    root.appendChild(this.tabBar(this.tab));
    this.root = root;
    return root;
  }

  private root: FrameNode | null = null;
  node(): FrameNode | null { return this.root; }
  nodes(): number { return this.u.count; }
  clear(): void { this.root = null; }
}
