/*
 * Wonderous, built in ArkTS through the imperative ArkUI node API.
 *
 * The other arm of the comparison. `crates/splash-oh-native/src/wonders/` builds
 * the same screens from Rust through the NDK; this builds them from ArkTS
 * through `typeNode`. Both end up at the same C++ patterns inside libace, so
 * everything downstream — measure, layout, paint — is identical native code and
 * cancels out. What differs is the language the tree is constructed in, which
 * is exactly what the comparison is meant to isolate.
 *
 * For that to hold, the two must build the *same* tree: same node types, same
 * counts, same geometry. The data comes from `WonderousData.ets`, generated
 * from the tables the Rust arm compiles in, and the assets are the same
 * rawfiles. Anywhere the numbers here differ from `wonders/*.rs`, the
 * comparison is measuring the difference rather than the language.
 */

import { typeNode, FrameNode, UIContext } from '@kit.ArkUI';
import { WONDERS, EDITORIAL, PIECES, ARTIFACTS, TIMELINES, SUGGESTIONS, CORPUS,
  COLLECTIBLES, WonderT, PieceT, ArtifactT, EventT, FoundT } from './WonderousData';

const APP: string = 'wonders';
const SHEET: number = 0xFFF8ECE5;
const BODY: number = 0xFF514F4D;
const CAPTION_C: number = 0xFF7D7873;
const GREY_STRONG: number = 0xFF272625;
const ACCENT1: number = 0xFFE4935D;
const DISPLAY: string = 'YesevaOne';
const SERIF_UI: string = 'TenorSans';
const BODY_FONT: string = 'Raleway';
const BODY_ITALIC: string = 'RalewayItalic';

/** Node primitives, one per Rust `ui.rs` helper, so both arms count alike. */
class W {
  ctx: UIContext;
  count: number = 0;

  constructor(ctx: UIContext) {
    this.ctx = ctx;
  }

  text(s: string, size: number, color: number, w: number, h: number, family?: string,
       align?: number): FrameNode {
    this.count++;
    const n: typeNode.Text = typeNode.createNode(this.ctx, 'Text');
    n.initialize(s);
    n.attribute.fontSize(size).fontColor(color).width(w).height(h);
    if (family !== undefined) {
      n.attribute.fontFamily(family);
    }
    if (align !== undefined) {
      n.attribute.textAlign(align);
    }
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

  scroll(w: number, h: number): typeNode.Scroll {
    this.count++;
    const n: typeNode.Scroll = typeNode.createNode(this.ctx, 'Scroll');
    n.attribute.width(w).height(h).align(Alignment.Top);
    return n;
  }

  img(path: string, w: number, h: number, radius: number, fit: number): FrameNode {
    this.count++;
    const n: typeNode.Image = typeNode.createNode(this.ctx, 'Image');
    n.initialize($rawfile(path));
    n.attribute.width(w).height(h).borderRadius(radius).objectFit(fit);
    return n;
  }

  /** Absolute placement, as the Rust arm's `position` attribute does. */
  at(n: FrameNode, x: number, y: number): FrameNode {
    n.commonAttribute.position({ x: x, y: y });
    return n;
  }
}

/** Which screen is up. Mirrors the Rust arm's `Screen`. */
export const S_INTRO: number = 0;
export const S_HOME: number = 1;
export const S_DETAILS: number = 2;
export const S_MENU: number = 3;
export const S_COLLECTION: number = 4;
export const S_TIMELINE: number = 5;
export const S_SEARCH: number = 6;
export const S_ARTIFACT: number = 7;
export const S_PHOTO: number = 8;
export const S_FOUND: number = 9;

const INTRO: string[][] = [
  ['Journey to the past', 'Navigate the intersection of time, art, and culture.'],
  ['Explore places', 'Uncover remarkable human-made structures from around the world.'],
  ['Discover artifacts', 'Learn about cultures throughout time by examining things they left behind.'],
];

const TABS: string[] = ['icon-info.png', 'icon-wallpaper.png', 'icon-collection.png', 'icon-timeline.png'];

/** Tap ids, the same meanings the Rust arm gives them. */
export const T_INTRO_NEXT: number = 7300;
export const T_MENU: number = 7103;
export const T_PREV: number = 7101;
export const T_NEXT: number = 7102;
export const T_DETAILS: number = 7104;
export const T_TAB: number = 7200;
export const T_HOME: number = 7210;
export const T_MENU_CLOSE: number = 7310;
export const T_MENU_BASE: number = 7320;
export const T_COLLECTION: number = 7341;
export const T_TIMELINE: number = 7342;
export const T_CLOSE: number = 7340;
export const T_BROWSE: number = 7372;
export const T_ART_PREV: number = 7370;
export const T_ART_NEXT: number = 7371;
export const T_ART_OPEN: number = 7373;
export const T_PHOTO_OPEN: number = 7375;
export const T_COLLECT: number = 7460;
export const T_FOUND_CLOSE: number = 7490;
export const T_VIEWER_CLOSE: number = 7450;

export class WonderousArkTs {
  private u: W;
  private w: number = 0;
  private h: number = 0;
  /** Nodes in the last tree built, and how long building it took. */
  nodes: number = 0;
  /** Where the app is. The Rust arm keeps the same four. */
  screen: number = S_INTRO;
  wonder: number = 0;
  tab: number = 0;
  page: number = 0;
  photoSel: number = 13;
  artSel: number = 0;
  found: boolean[] = new Array<boolean>(24).fill(false);
  showing: number = 0;
  /** Set when a tap changed state, so the host knows to rebuild. */
  onTap: (t: number) => void = (t: number) => {};

  constructor(ctx: UIContext) {
    this.u = new W(ctx);
  }

  /** Build one screen and return its root. */
  build(screen: number, wonder: number, tab: number, page: number,
        w: number, h: number): FrameNode | null {
    this.u.count = 0;
    this.w = w;
    this.h = h;
    let root: FrameNode | null = null;
    if (screen === S_INTRO) {
      root = this.intro(page);
    } else if (screen === S_HOME) {
      root = this.home(wonder);
    } else if (screen === S_DETAILS) {
      root = this.details(wonder, tab);
    } else if (screen === S_MENU) {
      root = this.menu(wonder);
    } else if (screen === S_COLLECTION) {
      root = this.collection();
    } else if (screen === S_TIMELINE) {
      root = this.timeline(wonder);
    } else if (screen === S_SEARCH) {
      root = this.search(wonder);
    } else if (screen === S_ARTIFACT) {
      root = this.artifact(wonder);
    } else if (screen === S_PHOTO) {
      root = this.photoViewer(wonder);
    } else {
      root = this.foundScreen();
    }
    this.nodes = this.u.count;
    return root;
  }

  /**
   * Every tap target on a screen, in one overlay.
   *
   * The same construction the Rust arm settled on and for the same reasons: a
   * positioned node is hit-tested where it was laid out rather than where it
   * draws, one full-frame column per target means only the last one fires, and
   * a single vertical flow drops any two targets that share a y. So: one
   * full-frame column, targets grouped into bands by y, laid out left to right
   * within a band with spacers.
   */
  private hits(fw: number, fh: number, targets: number[][]): FrameNode {
    const sorted: number[][] = targets.slice().sort((a: number[], b: number[]) =>
      a[1] === b[1] ? a[0] - b[0] : a[1] - b[1]);
    const column = this.u.col(fw, fh, 0x00000000);
    column.commonAttribute.hitTestBehavior(HitTestMode.None);
    let cursor: number = 0;
    let i: number = 0;
    while (i < sorted.length) {
      let bottom: number = sorted[i][1] + sorted[i][3];
      let j: number = i + 1;
      while (j < sorted.length && sorted[j][1] < bottom) {
        bottom = Math.max(bottom, sorted[j][1] + sorted[j][3]);
        j++;
      }
      const top: number = sorted[i][1];
      if (top > cursor) {
        const sp = this.u.col(fw, top - cursor, 0x00000000);
        sp.attribute.hitTestBehavior(HitTestMode.None);
        column.appendChild(sp);
      }
      const line = this.u.row(fw, bottom - top, 0x00000000);
      line.attribute.hitTestBehavior(HitTestMode.None);
      let x: number = 0;
      for (let k = i; k < j; k++) {
        const t: number[] = sorted[k];
        if (t[0] + t[2] > fw + 0.5 || t[0] < x) {
          continue;
        }
        if (t[0] > x) {
          const sp2 = this.u.col(t[0] - x, t[3], 0x00000000);
          sp2.attribute.hitTestBehavior(HitTestMode.None);
          line.appendChild(sp2);
        }
        const cell = this.u.col(t[2], t[3], 0x00000000);
        const id: number = t[4];
        cell.attribute.onClick(() => { this.tap(id); });
        line.appendChild(cell);
        x = t[0] + t[2];
      }
      column.appendChild(line);
      cursor = bottom;
      i = j;
    }
    return column;
  }

  /** The three collectibles a wonder hides, by index into COLLECTIBLES. */
  private forWonder(wi: number): number[] {
    const out: number[] = [];
    for (let i = 0; i < COLLECTIBLES.length; i++) {
      if (COLLECTIBLES[i].wonder === wi) {
        out.push(i);
      }
    }
    return out;
  }

  /** Which cell of the 5x5 wall hides one — a different corner per wonder. */
  private galleryCell(wi: number): number {
    if (wi === 4 || wi === 2) {
      return 0;
    }
    if (wi === 3 || wi === 0) {
      return 4;
    }
    if (wi === 7 || wi === 5) {
      return 24;
    }
    return 20;
  }

  /** The little icon a lost collectible is drawn as. */
  private badge(i: number, size: number): FrameNode {
    const b = this.u.stack(size, size, 0x33F8ECE5);
    b.attribute.borderRadius(size / 2);
    b.appendChild(this.u.img(APP + '/_common/collectibles/' + COLLECTIBLES[i].icon + '.png',
      size * 0.55, size * 0.55, 0, ImageFit.Contain));
    return b;
  }

  /** Route a tap. Mirrors the Rust arm's `handle`. */
  tap(t: number): void {
    const n: number = WONDERS.length;
    if (t >= T_MENU_BASE && t < T_MENU_BASE + n) {
      this.wonder = t - T_MENU_BASE;
      this.screen = S_HOME;
    } else if (t >= T_TAB && t < T_TAB + 4) {
      this.tab = t - T_TAB;
      this.screen = S_DETAILS;
    } else if (t >= T_COLLECT && t < T_COLLECT + 24) {
      this.found[t - T_COLLECT] = true;
      this.showing = t - T_COLLECT;
      this.screen = S_FOUND;
    } else if (t === T_INTRO_NEXT) {
      if (this.page < 2) {
        this.page++;
      } else {
        this.screen = S_HOME;
      }
    } else if (t === T_MENU) {
      this.screen = S_MENU;
    } else if (t === T_NEXT) {
      this.wonder = (this.wonder + 1) % n;
    } else if (t === T_PREV) {
      this.wonder = (this.wonder + n - 1) % n;
    } else if (t === T_DETAILS) {
      this.tab = 0;
      this.photoSel = 13;
      this.artSel = 0;
      this.screen = S_DETAILS;
    } else if (t === T_HOME || t === T_MENU_CLOSE || t === T_CLOSE) {
      this.screen = S_HOME;
    } else if (t === T_COLLECTION) {
      this.screen = S_COLLECTION;
    } else if (t === T_TIMELINE) {
      this.screen = S_TIMELINE;
    } else if (t === T_BROWSE) {
      this.screen = S_SEARCH;
    } else if (t === T_ART_NEXT || t === T_ART_PREV) {
      const list: ArtifactT[] = ARTIFACTS[this.wonder];
      const m: number = list.length;
      this.artSel = (this.artSel + (t === T_ART_NEXT ? 1 : m - 1)) % m;
    } else if (t === T_ART_OPEN) {
      this.screen = S_ARTIFACT;
    } else if (t === T_PHOTO_OPEN) {
      this.screen = S_PHOTO;
    } else if (t === T_VIEWER_CLOSE || t === T_FOUND_CLOSE) {
      this.screen = S_DETAILS;
    }
    this.onTap(t);
  }

  // --- the illustration, by the app's own rule -----------------------------
  // height = max(minH, frameH * heightFactor); width = height * aspect;
  // then anchor, then translate by a fraction of the piece's own size.
  private piece(wo: WonderT, p: PieceT, fw: number, fh: number): FrameNode {
    const ph: number = Math.max(p.minH, fh * p.heightFactor);
    const pw: number = ph * p.aspect;
    let ax: number = 0;
    let ay: number = 0;
    if (p.anchor === 0) {
      ax = (fw - pw) / 2;
      ay = (fh - ph) / 2;
    } else if (p.anchor === 2) {
      ax = (fw - pw) / 2;
    } else if (p.anchor === 3) {
      ax = fw - pw;
    } else if (p.anchor === 4) {
      ay = fh - ph;
    } else if (p.anchor === 5) {
      ax = (fw - pw) / 2;
      ay = fh - ph;
    } else if (p.anchor === 6) {
      ax = fw - pw;
      ay = fh - ph;
    }
    const n = this.u.img(APP + '/' + wo.dir + '/' + p.file, pw, ph, 0, ImageFit.Fill);
    return this.u.at(n, ax + p.fracX * pw + p.offX, ay + p.fracY * ph + p.offY);
  }

  private isForeground(f: string): boolean {
    return f.startsWith('foreground') || f === 'top-left.png' || f === 'top-right.png';
  }

  private illustration(wi: number, fw: number, fh: number): FrameNode {
    const wo: WonderT = WONDERS[wi];
    const root = this.u.stack(fw, fh, wo.fg);
    root.commonAttribute.clip(true);
    root.appendChild(this.u.at(
      this.u.img(APP + '/' + wo.dir + '/texture.png', fw, fh, 0, ImageFit.Cover), 0, 0));
    const ps: PieceT[] = PIECES[wi];
    // Background pieces, then the wonder, then the foreground.
    let mgAt: number = 0;
    for (let i = 0; i < ps.length; i++) {
      if (!this.isForeground(ps[i].file)) {
        mgAt = i;
      }
    }
    for (let i = 0; i < mgAt; i++) {
      if (!this.isForeground(ps[i].file)) {
        root.appendChild(this.piece(wo, ps[i], fw, fh));
      }
    }
    root.appendChild(this.piece(wo, ps[mgAt], fw, fh));
    for (let i = 0; i < ps.length; i++) {
      if (this.isForeground(ps[i].file)) {
        root.appendChild(this.piece(wo, ps[i], fw, fh));
      }
    }
    return root;
  }

  // --- screens -------------------------------------------------------------

  private intro(page: number): FrameNode {
    const w = this.w;
    const h = this.h;
    const p: string[] = INTRO[page % INTRO.length];
    const root = this.u.stack(w, h, 0xFF1E1B18);
    root.appendChild(this.u.at(
      this.u.img(APP + '/_common/app-logo.png', w * 0.62, h * 0.4, 0, ImageFit.Contain),
      w * 0.19, h * 0.08));
    root.appendChild(this.u.at(
      this.u.text('WONDEROUS', 12, ACCENT1, w, 22, SERIF_UI, TextAlign.Center), 0, h * 0.56));
    root.appendChild(this.u.at(
      this.u.text(p[0], 28, SHEET, w, 40, DISPLAY, TextAlign.Center), 0, h * 0.60));
    root.appendChild(this.u.at(
      this.u.text(p[1], 13, 0x99F8ECE5, w - 80, 60, BODY_FONT, TextAlign.Center), 40, h * 0.68));
    const btn = this.u.col(w * 0.5, 52, ACCENT1);
    btn.attribute.borderRadius(26);
    btn.appendChild(this.u.text(page === INTRO.length - 1 ? 'ENTER' : 'NEXT', 13, GREY_STRONG,
      w * 0.5, 52, SERIF_UI, TextAlign.Center));
    root.appendChild(this.u.at(btn, w * 0.25, h * 0.89));
    root.appendChild(this.hits(w, h, [[w * 0.25, h * 0.89, w * 0.5, 52, T_INTRO_NEXT]]));
    return root;
  }

  private home(wi: number): FrameNode {
    const w = this.w;
    const h = this.h;
    const wo: WonderT = WONDERS[wi];
    const root = this.u.stack(w, h, wo.bg);
    // All eight, as the Rust arm mounts them: the page change is a cross-fade
    // over trees that are already there, not a rebuild. Building only the
    // current one would make this arm's tree a fifth the size and the
    // comparison meaningless.
    for (let i = 0; i < WONDERS.length; i++) {
      const bg = this.u.col(w, h, WONDERS[i].bg);
      bg.attribute.opacity(i === wi ? 1 : 0);
      root.appendChild(this.u.at(bg, 0, 0));
    }
    for (let i = 0; i < WONDERS.length; i++) {
      const art = this.illustration(i, w, h);
      art.commonAttribute.opacity(i === wi ? 1 : 0);
      root.appendChild(art);
    }

    // Title block: the name, the article beside line two, then the dots.
    const size: number = Math.min(Math.max(w * 0.115, 30), 52);
    const lineH: number = size * 1.28;
    const oneLine: boolean = wo.line2.length === 0;
    const blockH: number = (oneLine ? lineH : lineH * 2) + 44;
    const y: number = Math.max(h - blockH - h * 0.115, h * 0.5);
    const cap = this.u.col(w, blockH, 0x00000000);
    cap.appendChild(this.u.text(wo.line1, size, SHEET, w, lineH, DISPLAY, TextAlign.Center));
    if (!oneLine) {
      const r = this.u.row(w, lineH, 0x00000000);
      if (wo.article.length > 0) {
        r.appendChild(this.u.text(wo.article, size * 0.42, SHEET, size * 1.6, lineH,
          BODY_ITALIC, TextAlign.End));
      }
      r.appendChild(this.u.text(wo.line2, size, SHEET, w * 0.7, lineH, DISPLAY, TextAlign.Start));
      cap.appendChild(r);
    }
    const dots = this.u.row(w, 36, 0x00000000);
    for (let i = 0; i < WONDERS.length; i++) {
      const d = this.u.col(i === wi ? 16 : 8, 8, SHEET);
      d.attribute.borderRadius(4).margin({ left: 5, right: 5 });
      dots.appendChild(d);
    }
    cap.appendChild(dots);
    root.appendChild(this.u.at(cap, 0, y));

    const menu = this.u.stack(62, 62, 0x59272625);
    menu.attribute.borderRadius(31);
    menu.appendChild(this.u.img(APP + '/_common/icons/icon-menu.png', 24, 24, 0, ImageFit.Contain));
    root.appendChild(this.u.at(menu, 12, h * 0.045));
    root.appendChild(this.u.at(
      this.u.img(APP + '/_common/icons/icon-next-large.png', 26, 26, 0, ImageFit.Contain),
      (w - 26) / 2, h * 0.94));
    root.appendChild(this.hits(w, h, [
      [12, h * 0.045, 62, 62, T_MENU],
      [0, h * 0.14, w * 0.35, h * 0.6, T_PREV],
      [w * 0.65, h * 0.14, w * 0.35, h * 0.6, T_NEXT],
      [0, h * 0.80, w, h * 0.17, T_DETAILS],
    ]));
    return root;
  }

  private tabBar(wi: number, tab: number): FrameNode {
    const w = this.w;
    const wo: WonderT = WONDERS[wi];
    const barH: number = 72;
    const bar = this.u.row(w, barH, wo.bg);
    const d: number = 52;
    const portrait = this.u.stack(d + 20, barH, 0x00000000);
    portrait.appendChild(this.u.at(
      this.u.img(APP + '/' + wo.dir + '/button.png', d, d, d / 2, ImageFit.Cover),
      14, (barH - d) / 2));
    bar.appendChild(portrait);
    const cell: number = (w - d - 20) / TABS.length;
    for (let i = 0; i < TABS.length; i++) {
      const c = this.u.col(cell, barH, 0x00000000);
      c.appendChild(this.u.at(
        this.u.img(APP + '/_common/icons/' + TABS[i], 26, 26, 0, ImageFit.Contain),
        (cell - 26) / 2, barH * 0.28));
      if (i === tab) {
        const u = this.u.col(22, 2, ACCENT1);
        u.attribute.borderRadius(1);
        c.appendChild(this.u.at(u, (cell - 22) / 2, barH * 0.72));
      }
      bar.appendChild(c);
    }
    return bar;
  }

  private details(wi: number, tab: number): FrameNode {
    const w = this.w;
    const h = this.h;
    const wo: WonderT = WONDERS[wi];
    const barH: number = 72;
    const root = this.u.stack(w, h, wo.bg);
    let body: FrameNode;
    if (tab === 1) {
      body = this.photoWall(wi, w, h - barH);
    } else if (tab === 2) {
      body = this.artifacts(wi, w, h - barH);
    } else if (tab === 3) {
      body = this.events(wi, w, h - barH);
    } else {
      body = this.editorial(wi, w, h - barH);
    }
    root.appendChild(this.u.at(body, 0, 0));
    root.appendChild(this.u.at(this.tabBar(wi, tab), 0, h - barH));
    const d: number = 52;
    const cell: number = (w - d - 20) / 4;
    const targets: number[][] = [[0, h - barH, d + 20, barH, T_HOME]];
    for (let i = 0; i < 4; i++) {
      targets.push([d + 20 + cell * i, h - barH, cell, barH, T_TAB + i]);
    }
    if (tab === 1) {
      const bh: number = h - barH;
      const iw: number = w * 0.66;
      const ih: number = bh * 0.5;
      targets.push([(w - iw) / 2, (bh - ih) / 2, iw, ih, T_PHOTO_OPEN]);
    }
    if (tab === 2) {
      const bh: number = h - barH;
      const bottomH: number = bh / 2.75;
      const itemH: number = Math.min(Math.max(bh - 200 - bottomH, 250), 400);
      const iw: number = itemH * 0.666;
      const tall: number = iw * 1.5;
      const ay: number = (bh - tall) / 2 - tall * 0.25;
      const cx: number = (w - iw) / 2;
      targets.push([0, ay, cx, tall, T_ART_PREV]);
      targets.push([cx, ay, iw, tall, T_ART_OPEN]);
      targets.push([cx + iw, ay, w - cx - iw, tall, T_ART_NEXT]);
      targets.push([w * 0.14, h - barH - 66, w * 0.72, 52, T_BROWSE]);
    }
    root.appendChild(this.hits(w, h, targets));
    return root;
  }

  /** How tall a paragraph will be once wrapped — the Rust arm's rule. */
  /** A year as `StringUtils.formatYr` writes it. */
  private yearLabel(y: number): string {
    const v: number = y === 0 ? 1 : y;
    return v < 0 ? ('' + (-v) + ' BCE') : ('' + v + ' CE');
  }

  private textH(s: string, size: number, w: number): number {
    const perLine: number = Math.max(w * 2.163 * 0.94 / size, 1);
    const lines: number = Math.max(Math.ceil(s.length / perLine), 1);
    return lines * size * 1.5 + size * 0.35;
  }

  private editorial(wi: number, w: number, h: number): FrameNode {
    const wo: WonderT = WONDERS[wi];
    const e = EDITORIAL[wi];
    const pad: number = 24;
    const cw: number = w - pad * 2;
    const bandH: number = 250;

    const s = this.u.scroll(w, h);
    const sheet = this.u.col(w, 0, SHEET);

    // The band: background and mid-ground only, as `_TopIllustration` builds.
    const band = this.u.stack(w, bandH, wo.fg);
    band.commonAttribute.clip(true);
    band.appendChild(this.u.at(
      this.u.img(APP + '/' + wo.dir + '/texture.png', w * 3.5, bandH * 3.5, 0, ImageFit.Cover),
      -w * 1.25, -bandH * 1.25));
    const ps: PieceT[] = PIECES[wi];
    for (let i = 0; i < ps.length && i < 2; i++) {
      band.appendChild(this.piece(wo, ps[i], w, bandH));
    }
    sheet.appendChild(band);

    // The masthead: a rule, the sub-title, a rule, the name, the region.
    const cap = this.u.col(w, 150, 0x00000000);
    const rule = this.u.row(w, 30, 0x00000000);
    rule.appendChild(this.u.col((w - 200) / 2, 1, wo.fg));
    rule.appendChild(this.u.text(e.subTitle.toUpperCase(), 11, GREY_STRONG, 152, 20,
      SERIF_UI, TextAlign.Center));
    rule.appendChild(this.u.col((w - 200) / 2, 1, wo.fg));
    cap.appendChild(rule);
    cap.appendChild(this.u.text(wo.title, Math.min(Math.max(w * 0.10, 26), 40), GREY_STRONG,
      w - 48, 60, DISPLAY, TextAlign.Center));
    cap.appendChild(this.u.text(e.region, 12, CAPTION_C, w - 48, 22, BODY_FONT, TextAlign.Center));
    sheet.appendChild(cap);

    const body = this.u.col(cw, 0, 0x00000000);
    let bodyH: number = 0;
    const paras: string[] = [e.history1, e.history2, e.construction1, e.construction2,
      e.location1, e.location2];
    const callouts: string[] = [e.callout1, e.callout2];
    for (let i = 0; i < paras.length; i++) {
      const th: number = this.textH(paras[i], 15, cw);
      body.appendChild(this.u.text(paras[i], 15, BODY, cw, th, BODY_FONT));
      bodyH += th;
      if (i === 0 || i === 2) {
        const k: number = i === 0 ? 0 : 1;
        if (callouts[k].length > 0) {
          const ch: number = this.textH(callouts[k], 19, cw - 22) + 56;
          const q = this.u.row(cw, ch, 0x00000000);
          q.appendChild(this.u.col(2, ch - 20, ACCENT1));
          q.appendChild(this.u.text(callouts[k], 19, GREY_STRONG, cw - 22, ch - 20, BODY_ITALIC));
          body.appendChild(q);
          bodyH += ch;
        }
      }
      // A section arc between the thirds, as the app sets them.
      if (i === 1 || i === 3) {
        const secH: number = w * 0.62 * 0.16 + 70 + 62;
        const sec = this.u.col(cw, secH, 0x00000000);
        sec.appendChild(this.u.img(
          APP + '/_common/' + (i === 1 ? 'arc-construction.png' : 'arc-location.png'),
          cw * 0.62, cw * 0.62 * 0.16, 0, ImageFit.Contain));
        sec.appendChild(this.u.img(
          APP + '/_common/' + (i === 1 ? 'construction.png' : 'geography.png'),
          48, 48, 0, ImageFit.Contain));
        body.appendChild(sec);
        bodyH += secH;
      }
      // The film still, after the first construction paragraph.
      if (i === 2 && e.videoCaption.length > 0) {
        const vh: number = cw * 0.56;
        const frame = this.u.stack(cw, vh, 0xFF000000);
        frame.appendChild(this.u.img(APP + '/' + wo.dir + '/photo-2.jpg', cw, vh, 0, ImageFit.Cover));
        const play = this.u.stack(54, 54, 0x66000000);
        play.attribute.borderRadius(27);
        play.appendChild(this.u.text('▶', 20, SHEET, 54, 54, undefined, TextAlign.Center));
        frame.appendChild(this.u.at(play, (cw - 54) / 2, (vh - 54) / 2));
        body.appendChild(frame);
        body.appendChild(this.u.text(e.videoCaption, 13, CAPTION_C, cw, 60, BODY_ITALIC));
        bodyH += vh + 70;
      }
    }
    // Two of the wonder's three collectibles, tucked into the article.
    const hidden: number[] = this.forWonder(wi);
    for (let k = 1; k < hidden.length; k++) {
      if (!this.found[hidden[k]]) {
        const id: number = hidden[k];
        const b = this.badge(id, 48);
        b.commonAttribute.onClick(() => { this.tap(T_COLLECT + id); });
        body.appendChild(b);
        bodyH += 64;
      }
    }
    body.attribute.height(bodyH);
    const wrap = this.u.col(w, bodyH + 68, 0x00000000);
    wrap.attribute.padding({ top: 28, left: pad, bottom: 40, right: pad });
    wrap.appendChild(body);
    sheet.appendChild(wrap);
    sheet.attribute.height(bandH + 150 + bodyH + 68);
    s.appendChild(sheet);
    return s;
  }

  /** The 5x5 wall, cells two thirds wide and half tall, all 25 mounted. */
  private photoWall(wi: number, w: number, h: number): FrameNode {
    const wo: WonderT = WONDERS[wi];
    const sel: number = 13;
    const grid: number = 5;
    const iw: number = w * 0.66;
    const ih: number = h * 0.5;
    const pad: number = 24;
    const sx: number = iw + pad;
    const sy: number = ih + pad;
    const ox: number = (w - iw) / 2;
    const oy: number = (h - ih) / 2;
    const root = this.u.stack(w, h, wo.bg);
    root.commonAttribute.clip(true);
    for (let i = 0; i < grid * grid; i++) {
      const x: number = ox + (i % grid) * sx - (sel % grid) * sx;
      const y: number = oy + Math.floor(i / grid) * sy - Math.floor(sel / grid) * sy;
      const name: string = (i % 24) < 10 ? '0' + (i % 24) : '' + (i % 24);
      root.appendChild(this.u.at(
        this.u.img(APP + '/' + wo.dir + '/gallery/' + name + '.jpg', iw, ih, 0, ImageFit.Cover),
        x, y));
    }
    // The cutout scrim: four rectangles around the centred cell.
    const scrim: number = 0xB3000000;
    root.appendChild(this.u.at(this.u.col(w, oy, scrim), 0, 0));
    root.appendChild(this.u.at(this.u.col(w, Math.max(h - oy - ih, 0), scrim), 0, oy + ih));
    root.appendChild(this.u.at(this.u.col(Math.max(ox, 0), ih, scrim), 0, oy));
    root.appendChild(this.u.at(
      this.u.col(Math.max(w - ox - iw, 0), ih, scrim), ox + iw, oy));
    // The gallery collectible, on the cell the app hides it in, over the scrim.
    const hid: number[] = this.forWonder(wi);
    if (sel === this.galleryCell(wi) && hid.length > 0 && !this.found[hid[0]]) {
      const id: number = hid[0];
      const b = this.badge(id, 56);
      b.commonAttribute.onClick(() => { this.tap(T_COLLECT + id); });
      root.appendChild(this.u.at(b, ox + iw - 76, oy + ih - 76));
    }
    return root;
  }

  private artifacts(wi: number, w: number, h: number): FrameNode {
    const wo: WonderT = WONDERS[wi];
    const root = this.u.stack(w, h, wo.bg);
    root.commonAttribute.clip(true);
    const bottomH: number = h / 2.75;
    const itemH: number = Math.min(Math.max(h - 200 - bottomH, 250), 400);
    const iw: number = itemH * 0.666;
    const tall: number = iw * 1.5;
    const ay: number = (h - tall) / 2 - tall * 0.25;
    const ax: number = (w - iw) / 2;

    root.appendChild(this.u.at(this.u.col(w, h, 0x99000000), 0, 0));
    const dome = this.u.col(2000, 2000, 0xCCF8ECE5);
    dome.attribute.borderRadius({ topLeft: 1000, topRight: 1000 });
    root.appendChild(this.u.at(dome, (w - 2000) / 2, h * 0.5));

    const list: ArtifactT[] = ARTIFACTS[wi];
    const n: number = list.length;
    const sel: number = 0;
    // Every piece mounted, placed by its distance from the selection, as
    // `_CollapsingCarouselItem` does and as the Rust arm does.
    for (let i = 0; i < n; i++) {
      let k: number = (i - sel) % n;
      if (k * 2 > n) {
        k = k - n;
      }
      const drop: number = Math.abs(k) === 0 ? 0
        : (Math.abs(k) === 1 ? iw * 0.5 : (Math.abs(k) === 2 ? iw * 0.825 : iw));
      const fh: number = k === 0 ? tall : iw;
      const x: number = ax + k * iw;
      const y: number = ay + (tall - fh) / 2 + drop;
      const op: number = Math.abs(k) <= 2 ? 1 : 0;
      const frame = this.u.col(iw, fh, 0x00000000);
      frame.attribute.borderRadius(iw / 2).borderWidth(1).borderColor(SHEET).opacity(op);
      root.appendChild(this.u.at(frame, x, y));
      const inset: number = k === 0 ? 8 : iw * 0.1;
      const pic = this.u.img(APP + '/artifacts/' + list[i].id + '.jpg',
        iw - inset * 2, fh - inset * 2, (iw - inset * 2) / 2, ImageFit.Cover);
      pic.commonAttribute.opacity(op);
      root.appendChild(this.u.at(pic, x + inset, y + inset));
      // The blurred backdrop, one per piece, cross-faded.
      const bg = this.u.img(APP + '/artifacts/' + list[i].id + '.jpg',
        w * 1.25, h * 1.25, 0, ImageFit.Cover);
      bg.commonAttribute.opacity(i === sel ? 1 : 0);
      root.appendChild(this.u.at(bg, -w * 0.125, -h * 0.05));
    }
    // One dot per piece, the current one twice as wide.
    const dd: number = 6;
    const dtot: number = (n + 1) * dd + (n - 1) * 10;
    for (let i = 0; i < n; i++) {
      const dot = this.u.col(i === sel ? dd * 2 : dd, dd, ACCENT1);
      dot.attribute.borderRadius(dd / 2);
      root.appendChild(this.u.at(dot, (w - dtot) / 2 + i * (dd + 10), h - 88));
    }

    root.appendChild(this.u.at(
      this.u.text('ARTIFACTS', 13, ACCENT1, w, 30, SERIF_UI, TextAlign.Center), 0, 22));
    root.appendChild(this.u.at(
      this.u.text(ARTIFACTS[wi][0].title, 30, 0xFF1E1B18, w - 48, 36, DISPLAY, TextAlign.Center),
      0, h - bottomH + 24));
    root.appendChild(this.u.at(
      this.u.text(ARTIFACTS[wi][0].date, 14, 0xB31E1B18, w, 24, BODY_FONT, TextAlign.Center),
      0, h - bottomH + 64));
    const btn = this.u.col(w * 0.72, 52, GREY_STRONG);
    btn.attribute.borderRadius(4);
    btn.appendChild(this.u.text('BROWSE ALL ARTIFACTS', 13, SHEET, w * 0.72, 52,
      SERIF_UI, TextAlign.Center));
    root.appendChild(this.u.at(btn, w * 0.14, h - 66));
    return root;
  }

  private events(wi: number, w: number, h: number): FrameNode {
    const wo: WonderT = WONDERS[wi];
    const e = EDITORIAL[wi];
    const pad: number = 24;
    const cw: number = w - pad * 2;
    const s = this.u.scroll(w, h);
    const inner = this.u.col(w, 0, wo.bg);
    inner.attribute.padding({ top: 36, left: pad, bottom: 40, right: pad });
    let total: number = 96;
    const head = this.u.col(cw, 96, 0x00000000);
    head.appendChild(this.u.text(wo.title, 26, SHEET, cw, 40, DISPLAY));
    head.appendChild(this.u.text(e.region, 13, ACCENT1, cw, 24, SERIF_UI));
    inner.appendChild(head);
    // Six dated entries, as each wonder carries.
    const evs: EventT[] = TIMELINES[wi].events;
    for (let i = 0; i < evs.length; i++) {
      const th: number = this.textH(evs[i].text, 14, cw - 18);
      const block: number = th + 46;
      const r = this.u.row(cw, block, 0x00000000);
      r.appendChild(this.u.col(2, block - 12, 0x44E4935D));
      const c = this.u.col(cw - 18, block, 0x00000000);
      c.attribute.padding({ left: 16 });
      c.appendChild(this.u.text(this.yearLabel(evs[i].year), 12, ACCENT1, cw - 18, 22, SERIF_UI));
      c.appendChild(this.u.text(evs[i].text, 14, 0xCCF8ECE5, cw - 18, th, BODY_FONT));
      r.appendChild(c);
      inner.appendChild(r);
      total += block;
    }
    inner.attribute.height(total + 76);
    s.appendChild(inner);
    return s;
  }

  private menu(wi: number): FrameNode {
    const w = this.w;
    const h = this.h;
    const wo: WonderT = WONDERS[wi];
    const root = this.u.stack(w, h, wo.bg);
    root.appendChild(this.u.at(
      this.u.text('WONDEROUS', 13, ACCENT1, w, 26, SERIF_UI, TextAlign.Center), 0, h * 0.055));
    const top: number = h * 0.125;
    const rowH: number = Math.min(Math.max((h * 0.86 - top) / 10.5, 44), 74);
    for (let i = 0; i < WONDERS.length; i++) {
      const y: number = top + rowH * i;
      const r = this.u.row(w, rowH, 0x00000000);
      r.appendChild(this.u.img(APP + '/' + WONDERS[i].dir + '/button.png', 34, 34, 17, ImageFit.Cover));
      const t = this.u.col(w - 110, rowH, 0x00000000);
      t.appendChild(this.u.text(WONDERS[i].title, 21, i === wi ? SHEET : 0xB3F8ECE5,
        w - 110, 32, DISPLAY));
      t.appendChild(this.u.text(EDITORIAL[i].region, 12, 0x8CF8ECE5, w - 110, 20, BODY_FONT));
      r.appendChild(t);
      root.appendChild(this.u.at(r, 24, y));
    }
    const extras: string[][] = [['Collection', 'icon-collection.png'], ['Timeline', 'icon-timeline.png']];
    const ey: number = top + rowH * WONDERS.length + rowH * 0.15;
    for (let i = 0; i < extras.length; i++) {
      const y: number = ey + rowH * i;
      root.appendChild(this.u.at(
        this.u.img(APP + '/_common/icons/' + extras[i][1], 22, 22, 0, ImageFit.Contain),
        38, y + rowH * 0.25));
      root.appendChild(this.u.at(
        this.u.text(extras[i][0], 16, 0xCCF8ECE5, w - 90, 26, SERIF_UI), 86, y + rowH * 0.22));
    }
    const close = this.u.stack(46, 46, 0xB3272625);
    close.attribute.borderRadius(23);
    close.appendChild(this.u.img(APP + '/_common/icons/icon-close.png', 20, 20, 0, ImageFit.Contain));
    root.appendChild(this.u.at(close, w - 66, h * 0.055));
    const mt: number[][] = [[w - 76, 6, 66, 60, T_MENU_CLOSE]];
    for (let i = 0; i < WONDERS.length; i++) {
      mt.push([0, top + rowH * i, w, rowH, T_MENU_BASE + i]);
    }
    mt.push([0, ey, w, rowH, T_COLLECTION]);
    mt.push([0, ey + rowH, w, rowH, T_TIMELINE]);
    root.appendChild(this.hits(w, h, mt));
    return root;
  }

  private collection(): FrameNode {
    const w = this.w;
    const h = this.h;
    const root = this.u.stack(w, h, GREY_STRONG);
    root.appendChild(this.u.at(
      this.u.text('COLLECTION', 13, ACCENT1, w, 26, SERIF_UI, TextAlign.Center), 0, h * 0.06));
    root.appendChild(this.u.at(
      this.u.text('0 of ' + COLLECTIBLES.length + ' discovered', 14, 0x99F8ECE5, w, 24,
        BODY_FONT, TextAlign.Center),
      0, h * 0.10));
    const cols: number = 4;
    const pad: number = 20;
    const cell: number = (w - pad * 2 - 8 * (cols - 1)) / cols;
    const top: number = h * 0.16;
    for (let i = 0; i < COLLECTIBLES.length; i++) {
      const x: number = pad + (cell + 8) * (i % cols);
      const y: number = top + (cell + 26) * Math.floor(i / cols);
      const c = this.u.stack(cell, cell, 0x0DF8ECE5);
      c.attribute.borderRadius(6);
      c.appendChild(this.u.img(APP + '/_common/collectibles/' + COLLECTIBLES[i].icon + '.png',
        cell * 0.34, cell * 0.34, 0, ImageFit.Contain));
      root.appendChild(this.u.at(c, x, y));
    }
    const close = this.u.stack(46, 46, 0x33F8ECE5);
    close.attribute.borderRadius(23);
    close.appendChild(this.u.img(APP + '/_common/icons/icon-close.png', 20, 20, 0, ImageFit.Contain));
    root.appendChild(this.u.at(close, w - 66, h * 0.035));
    root.appendChild(this.hits(w, h, [[w - 76, h * 0.025, 66, 66, T_CLOSE]]));
    return root;
  }

  private timeline(wi: number): FrameNode {
    const w = this.w;
    const h = this.h;
    const root = this.u.stack(w, h, GREY_STRONG);
    root.appendChild(this.u.at(
      this.u.text('TIMELINE', 13, ACCENT1, w, 26, SERIF_UI, TextAlign.Center), 0, h * 0.055));
    root.appendChild(this.u.at(
      this.u.text('2600 BCE — 1931 CE', 13, 0x99F8ECE5, w, 24, BODY_FONT, TextAlign.Center),
      0, h * 0.095));
    const top: number = h * 0.16;
    const rowH: number = 68;
    const trackX: number = 96;
    const trackW: number = w - trackX - 24;
    for (let i = 0; i < WONDERS.length; i++) {
      const y: number = top + rowH * i;
      root.appendChild(this.u.at(
        this.u.img(APP + '/' + WONDERS[i].dir + '/button.png', 34, 34, 17, ImageFit.Cover),
        20, y + 6));
      root.appendChild(this.u.at(
        this.u.text(WONDERS[i].title, 11, i === wi ? SHEET : 0x99F8ECE5, trackX - 26, 18, BODY_FONT),
        20, y + 44));
      root.appendChild(this.u.at(this.u.col(trackW, 2, 0x22F8ECE5), trackX, y + 22));
      let lo: number = TIMELINES[0].startYr;
      let hi: number = TIMELINES[0].endYr;
      for (let k = 0; k < TIMELINES.length; k++) {
        lo = Math.min(lo, TIMELINES[k].startYr);
        hi = Math.max(hi, TIMELINES[k].endYr);
      }
      const span: number = Math.max(hi - lo, 1);
      const x0: number = trackX + (TIMELINES[i].startYr - lo) / span * trackW;
      const x1: number = trackX + (TIMELINES[i].endYr - lo) / span * trackW;
      const bar = this.u.col(Math.max(x1 - x0, 3), 10, i === wi ? ACCENT1 : WONDERS[i].fg);
      bar.attribute.borderRadius(5);
      root.appendChild(this.u.at(bar, x0, y + 18));
      root.appendChild(this.u.at(
        this.u.text(this.yearLabel(TIMELINES[i].startYr), 10, 0x8CF8ECE5, 90, 16, BODY_FONT),
        Math.min(x0, w - 96), y + 34));
    }
    const close = this.u.stack(46, 46, 0x33F8ECE5);
    close.attribute.borderRadius(23);
    close.appendChild(this.u.img(APP + '/_common/icons/icon-close.png', 20, 20, 0, ImageFit.Contain));
    root.appendChild(this.u.at(close, w - 66, h * 0.04));
    root.appendChild(this.hits(w, h, [[w - 76, h * 0.03, 66, 66, T_CLOSE]]));
    return root;
  }

  private search(wi: number): FrameNode {
    const w = this.w;
    const h = this.h;
    const wo: WonderT = WONDERS[wi];
    const root = this.u.stack(w, h, GREY_STRONG);
    root.appendChild(this.u.at(
      this.u.text('SEARCH ARTIFACTS', 13, ACCENT1, w, 26, SERIF_UI, TextAlign.Center), 0, h * 0.05));
    root.appendChild(this.u.at(
      this.u.text(wo.title, 22, SHEET, w, 34, DISPLAY, TextAlign.Center), 0, h * 0.09));
    const field = this.u.col(w - 48, 46, 0x1AF8ECE5);
    field.attribute.borderRadius(23);
    field.appendChild(this.u.text('Search the collection', 15, 0x66F8ECE5, w - 48, 46, BODY_FONT));
    root.appendChild(this.u.at(field, 24, h * 0.15));
    const cw: number = (w - 48 - 24) / 4;
    const chipTop: number = h * 0.24;
    for (let i = 0; i < 8; i++) {
      const x: number = 24 + (cw + 8) * (i % 4);
      const y: number = chipTop + 46 * Math.floor(i / 4);
      const c = this.u.col(cw, 36, 0x1AF8ECE5);
      c.attribute.borderRadius(18);
      const words: string[] = SUGGESTIONS[wi];
      c.appendChild(this.u.text(i < words.length ? words[i] : '', 12, 0xCCF8ECE5, cw, 36,
        BODY_FONT, TextAlign.Center));
      root.appendChild(this.u.at(c, x, y));
    }
    root.appendChild(this.u.at(
      this.u.text('' + CORPUS[wi].length + ' artifact(s)', 12, 0x99F8ECE5, w, 22,
        BODY_FONT, TextAlign.Center),
      0, chipTop + 104));
    const gw: number = (w - 48 - 16) / 3;
    const gtop: number = chipTop + 134;
    for (let i = 0; i < 9; i++) {
      const x: number = 24 + (gw + 8) * (i % 3);
      const y: number = gtop + (gw + 26) * Math.floor(i / 3);
      const hit: FoundT = CORPUS[wi][i];
      // The thumbnails are network images in the Rust arm too, but a bare URL
      // string is not what typeNode.Image.initialize accepts here; the shipped
      // artifact photographs stand in, and the node count is what the
      // comparison turns on either way.
      root.appendChild(this.u.at(
        this.u.img(APP + '/artifacts/' + ARTIFACTS[wi][i % ARTIFACTS[wi].length].id + '.jpg',
          gw, gw, 6, ImageFit.Cover), x, y));
      root.appendChild(this.u.at(
        this.u.text(hit.title, 10, 0x99F8ECE5, gw, 20, BODY_FONT, TextAlign.Center),
        x, y + gw + 3));
    }
    return root;
  }

  /** One gallery photograph, full screen. */
  private photoViewer(wi: number): FrameNode {
    const w = this.w;
    const h = this.h;
    const wo: WonderT = WONDERS[wi];
    const root = this.u.stack(w, h, 0xFF000000);
    const i: number = this.photoSel % 24;
    const nm: string = i < 10 ? '0' + i : '' + i;
    const pic = this.u.img(APP + '/' + wo.dir + '/gallery/' + nm + '.jpg', w, h * 0.72, 0,
      ImageFit.Contain);
    pic.commonAttribute.backgroundColor(0xFF000000);
    root.appendChild(this.u.at(pic, 0, h * 0.14));
    root.appendChild(this.u.at(
      this.u.text('' + (i + 1) + ' / 24', 12, 0x99F8ECE5, w, 20, BODY_FONT, TextAlign.Center),
      0, h - 120));
    const close = this.u.stack(46, 46, 0x59000000);
    close.attribute.borderRadius(23);
    close.appendChild(this.u.img(APP + '/_common/icons/icon-close.png', 20, 20, 0, ImageFit.Contain));
    root.appendChild(this.u.at(close, 24, h * 0.03));
    root.appendChild(this.hits(w, h, [[14, h * 0.02, 66, 66, T_VIEWER_CLOSE]]));
    return root;
  }

  /** The screen that shows off a collectible the moment it is found. */
  private foundScreen(): FrameNode {
    const w = this.w;
    const h = this.h;
    const c = COLLECTIBLES[this.showing % COLLECTIBLES.length];
    let n: number = 0;
    for (let i = 0; i < this.found.length; i++) {
      if (this.found[i]) {
        n++;
      }
    }
    const root = this.u.stack(w, h, 0xF21E1B18);
    root.appendChild(this.u.at(
      this.u.img(APP + '/artifacts/' + c.artifactId + '.jpg', w * 0.62, h * 0.42, 8,
        ImageFit.Contain), w * 0.19, h * 0.16));
    root.appendChild(this.u.at(
      this.u.text('COLLECTIBLE FOUND', 12, ACCENT1, w, 22, SERIF_UI, TextAlign.Center),
      0, h * 0.62));
    root.appendChild(this.u.at(
      this.u.text(c.title, 26, SHEET, w - 56, 36, DISPLAY, TextAlign.Center), 0, h * 0.66));
    root.appendChild(this.u.at(
      this.u.text('' + n + ' of ' + COLLECTIBLES.length + ' found', 13, 0x99F8ECE5, w, 22,
        BODY_FONT, TextAlign.Center), 0, h * 0.72));
    const btn = this.u.col(w * 0.5, 48, GREY_STRONG);
    btn.attribute.borderRadius(4);
    btn.appendChild(this.u.text('CONTINUE', 12, SHEET, w * 0.5, 48, SERIF_UI, TextAlign.Center));
    root.appendChild(this.u.at(btn, w * 0.25, h - 96));
    root.appendChild(this.hits(w, h, [[w * 0.25, h - 96, w * 0.5, 48, T_FOUND_CLOSE]]));
    return root;
  }

  private artifact(wi: number): FrameNode {
    const w = this.w;
    const h = this.h;
    const wo: WonderT = WONDERS[wi];
    const e = EDITORIAL[wi];
    const root = this.u.stack(w, h, SHEET);
    const ih: number = h * 0.46;
    root.appendChild(this.u.at(
      this.u.img(APP + '/artifacts/' + ARTIFACTS[wi][0].id + '.jpg', w, ih, 0, ImageFit.Cover),
      0, 0));
    let y: number = ih + 26;
    root.appendChild(this.u.at(
      this.u.text(ARTIFACTS[wi][0].title, 28, GREY_STRONG, w - 56, 40, DISPLAY), 28, y));
    y += 54;
    root.appendChild(this.u.at(this.u.col(w - 56, 1, 0x33272625), 28, y));
    y += 18;
    const art: ArtifactT = ARTIFACTS[wi][0];
    const rows: string[][] = [['Date', art.date], ['Culture', art.culture]];
    for (let i = 0; i < rows.length; i++) {
      root.appendChild(this.u.at(
        this.u.text(rows[i][0], 11, 0xFF9D9995, w - 56, 18, SERIF_UI), 28, y));
      y += 18;
      root.appendChild(this.u.at(
        this.u.text(rows[i][1], 14, BODY, w - 56, 21, BODY_FONT), 28, y));
      y += 33;
    }
    const close = this.u.stack(46, 46, 0x33272625);
    close.attribute.borderRadius(23);
    close.appendChild(this.u.img(APP + '/_common/icons/icon-close.png', 20, 20, 0, ImageFit.Contain));
    root.appendChild(this.u.at(close, w - 66, h * 0.045));
    root.appendChild(this.hits(w, h, [[w - 76, h * 0.035, 66, 66, T_VIEWER_CLOSE]]));
    return root;
  }
}
