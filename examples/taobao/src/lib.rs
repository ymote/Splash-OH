//! Taobao, ported from
//! [project-robius/makepad_taobao](https://github.com/project-robius/makepad_taobao).
//!
//! Five tabs (首页 / 逛逛 / 消息 / 购物车 / 我的淘宝), a search header, a
//! carousel, a row of search terms, and a two-column product grid — which is
//! the interesting part for this comparison, because it is the most
//! image-dense screen of the four apps ported here.
//!
//! `CatalogItem` in the reference app is a `ClickableView` wrapping a
//! `RoundedView` with an `Image`, an info `View`, and two `Label`s, so six
//! nodes per product. That shape is preserved.
//!
//! Product titles, subtitles and prices are the reference app's own, from
//! `home/catalog_data.rs`.

use splash_oh_native::arkui::Node;
use splash_oh_native::ui::*;

const APP: &str = "taobao";

// Palette from the reference app's styles.rs.
const BG: u32 = 0xFFF2F2F2;
const SURFACE: u32 = 0xFFFFFFFF;
const ORANGE: u32 = 0xFFFF5000;
const TEXT: u32 = 0xFF1A1A1A;
const SUBTLE: u32 = 0xFF9A9A9A;
const NAV_BG: u32 = 0xFFFFFFFF;

pub const TAB_BASE: i32 = 200;
pub const ITEM_BASE: i32 = 2000;
pub const BACK: i32 = 210;

pub const TABS: &[(&str, &str)] = &[
    ("首页", "home.svg"),
    ("逛逛", "search.svg"),
    ("消息", "chat_bubble.png"),
    ("购物车", "store.png"),
    ("我的淘宝", "my_taobao.svg"),
];

/// The reference app's fifteen products: (title, subtitle, price, image).
pub const PRODUCTS: &[(&str, &str, &str, &str)] = &[
    ("男士人字拖 2023", "500+人付费", "58", "flip_flops.png"),
    ("巧克力大地色调", "10000+人付费", "8.9", "cosmetics.png"),
    (
        "冰丝防沙发垫夏季垫t",
        "50+人付费",
        "20.9",
        "living_furniture.png",
    ),
    ("胡萝卜奶锅婴儿不粘锅", "100+人付费", "89", "meal.png"),
    (
        "夏季新款连衣裙",
        "2000+人付费",
        "129",
        "seul_collection.png",
    ),
    ("无线蓝牙耳机", "8000+人付费", "199", "router.png"),
    ("户外折叠椅便携", "300+人付费", "75", "protein.png"),
    ("儿童益智积木", "1500+人付费", "45", "ring.png"),
    ("不锈钢保温杯", "6000+人付费", "39", "carrousel_1.png"),
    ("电动牙刷软毛", "900+人付费", "119", "carrousel_2.png"),
    ("纯棉四件套床品", "4000+人付费", "168", "carrousel_3.png"),
    ("运动跑步鞋男", "700+人付费", "228", "flip_flops.png"),
    ("厨房收纳置物架", "250+人付费", "56", "living_furniture.png"),
    ("护眼台灯 LED", "1100+人付费", "88", "cosmetics.png"),
    ("双肩包大容量", "3200+人付费", "99", "meal.png"),
];

const SEARCH_TERMS: &[&str] = &["连衣裙", "运动鞋", "耳机", "零食", "口红", "手机壳"];

/// Header: search field plus the two icon actions the reference app has.
fn header() -> Option<Node> {
    let mut h = row(W, 50.0, ORANGE)?;
    h = h.child(icon(APP, "search.svg", 22.0)?);
    let mut field = row(W - 110.0, 32.0, SURFACE)?.radius(16.0);
    field = field.child(text("搜索淘宝商品", 13.0, SUBTLE, W - 130.0, 20.0)?);
    h = h.child(field);
    h = h.child(icon(APP, "meatballs_menu.png", 22.0)?);
    Some(h)
}

/// The five-tab bar.
fn tab_bar(active: usize) -> Option<Node> {
    let mut bar = row(W, 56.0, NAV_BG)?;
    for (i, (label, file)) in TABS.iter().enumerate() {
        let c = if i == active { ORANGE } else { SUBTLE };
        let mut t = tap_col(W / 5.0, 56.0, NAV_BG, TAB_BASE + i as i32)?;
        t = t.child(icon(APP, file, 22.0)?);
        t = t.child(text(label, 10.0, c, W / 5.0 - 2.0, 14.0)?);
        bar = bar.child(t);
    }
    Some(bar)
}

/// The promo carousel at the top of the home screen.
fn carousel() -> Option<Node> {
    let mut c = row(W, 140.0, BG)?;
    c = c.child(photo(APP, "buy_it_banner.png", W - 20.0, 130.0, 8.0)?);
    Some(c)
}

/// The horizontal row of suggested search terms.
fn search_terms() -> Option<Node> {
    let mut r = row(W, 40.0, BG)?;
    for t in SEARCH_TERMS {
        let mut chip = row(58.0, 26.0, SURFACE)?.radius(13.0);
        chip = chip.child(text(t, 11.0, TEXT, 54.0, 16.0)?);
        r = r.child(chip);
    }
    Some(r)
}

/// One product cell: container, image, info column, title, subtitle, price.
fn product(i: usize) -> Option<Node> {
    let (title, subtitle, price, img) = PRODUCTS[i];
    let cw = (W - 24.0) / 2.0;
    let mut cell = tap_col(cw, 250.0, SURFACE, ITEM_BASE + i as i32)?.radius(8.0);
    cell = cell.child(photo(APP, img, cw, 150.0, 8.0)?);
    let mut info = col(cw - 12.0, 92.0, SURFACE)?;
    info = info.child(text_w(title, 13.0, TEXT, cw - 16.0, 36.0, 5)?);
    info = info.child(text(subtitle, 10.0, SUBTLE, cw - 16.0, 16.0)?);
    info = info.child(text_w(
        &format!("¥{price}"),
        16.0,
        ORANGE,
        cw - 16.0,
        24.0,
        7,
    )?);
    cell = cell.child(info);
    Some(cell)
}

/// The two-column grid, laid out as rows of two.
fn product_grid(body: Node) -> Option<Node> {
    let mut b = body;
    let mut i = 0usize;
    while i < PRODUCTS.len() {
        let mut r = row(W, 258.0, BG)?;
        r = r.child(product(i)?);
        if i + 1 < PRODUCTS.len() {
            r = r.child(product(i + 1)?);
        }
        b = b.child(r);
        i += 2;
    }
    Some(b)
}

/// A product detail page, the reference app's `catalog_item_screen`.
fn detail(body: Node, idx: usize) -> Option<Node> {
    let (title, subtitle, price, img) = PRODUCTS[idx.min(PRODUCTS.len() - 1)];
    let mut b = body;
    b = b.child(photo(APP, img, W, 300.0, 0.0)?);
    let mut info = col(W, 120.0, SURFACE)?;
    info = info.child(text_w(
        &format!("¥{price}"),
        24.0,
        ORANGE,
        W - 20.0,
        34.0,
        7,
    )?);
    info = info.child(text_w(title, 15.0, TEXT, W - 20.0, 44.0, 5)?);
    info = info.child(text(subtitle, 11.0, SUBTLE, W - 20.0, 18.0)?);
    b = b.child(info);
    b = b.child(spacer(W, 8.0)?);
    for (label, file) in [
        ("配送 至 北京", "shipping_estimate.png"),
        ("正品保障", "credit_cards.png"),
        ("官方旗舰店", "store.png"),
        ("客服中心", "help_center.png"),
    ] {
        let mut r = row(W, 46.0, SURFACE)?;
        r = r.child(icon(APP, file, 20.0)?);
        r = r.child(text(label, 13.0, TEXT, W - 80.0, 20.0)?);
        r = r.child(text("›", 14.0, SUBTLE, 20.0, 20.0)?);
        b = b.child(r);
    }
    // Buy bar.
    let mut buy = row(W, 52.0, SURFACE)?;
    buy = buy.child(icon(APP, "chat_bubble.png", 22.0)?);
    let mut cart = row(120.0, 38.0, 0xFFFFB000)?.radius(19.0);
    cart = cart.child(text("加入购物车", 13.0, SURFACE, 112.0, 20.0)?);
    buy = buy.child(cart);
    let mut now = row(120.0, 38.0, ORANGE)?.radius(19.0);
    now = now.child(text("立即购买", 13.0, SURFACE, 112.0, 20.0)?);
    buy = buy.child(now);
    b = b.child(buy);
    Some(b)
}

/// A simple list screen, used for the tabs that are not the storefront.
fn simple_list(body: Node, rows: usize, label: &str) -> Option<Node> {
    let mut b = body;
    for i in 0..rows {
        let mut r = row(W, 62.0, SURFACE)?;
        r = r.child(photo(APP, "default_avatar.png", 42.0, 42.0, 21.0)?);
        let mut c = col(W - 90.0, 50.0, SURFACE)?;
        c = c.child(text_w(
            &format!("{label} {}", i + 1),
            14.0,
            TEXT,
            W - 100.0,
            20.0,
            5,
        )?);
        c = c.child(text(
            PRODUCTS[i % PRODUCTS.len()].1,
            11.0,
            SUBTLE,
            W - 100.0,
            18.0,
        )?);
        r = r.child(c);
        b = b.child(r);
        b = b.child(divider(W, 0xFFEEEEEE)?);
    }
    Some(b)
}

/// Build the app for a tab, or a product detail if `detail_idx` is set.
pub fn build(tab: usize, detail_idx: Option<usize>) -> Option<Node> {
    let mut root = col(W, PAGE_H, BG)?;

    if let Some(idx) = detail_idx {
        let mut h = row(W, 46.0, SURFACE)?;
        h = h.child(tap_row(56.0, 46.0, SURFACE, BACK)?.child(text("‹", 24.0, TEXT, 44.0, 30.0)?));
        h = h.child(text_w("商品详情", 16.0, TEXT, W - 120.0, 24.0, 5)?);
        root = root.child(h);
        let body = col(W, 0.0, BG)?;
        root = root.child(scroll(PAGE_H - 46.0)?.child(detail(body, idx)?));
        return Some(root);
    }

    root = root.child(header()?);
    let body = col(W, 0.0, BG)?;
    let body = match tab {
        0 => {
            let mut b = body;
            b = b.child(carousel()?);
            b = b.child(search_terms()?);
            product_grid(b)?
        }
        1 => simple_list(body, 10, "逛逛")?,
        2 => simple_list(body, 12, "消息")?,
        3 => simple_list(body, 8, "购物车")?,
        _ => simple_list(body, 9, "我的淘宝")?,
    };
    root = root.child(scroll(PAGE_H - 50.0 - 56.0)?.child(body));
    root = root.child(tab_bar(tab)?);
    Some(root)
}
