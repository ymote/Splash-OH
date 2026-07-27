//! Four makepad reference apps, ported to native ArkUI and driven from Rust.
//!
//! | app | shape | why it is here |
//! |---|---|---|
//! | [`wechat`] | text-heavy lists | many small nodes |
//! | [`taobao`] | two-column product grid | many nodes *and* many images |
//! | [`tiktok`] | full-screen media plus overlays | few nodes, one big image |
//! | [`wonderous`] | editorial and photo grid | moderate nodes, large images |
//!
//! Four different widget mixes rather than four versions of the same list,
//! because a single app cannot tell you whether the Rust-vs-ArkTS gap is a
//! per-node constant or something that varies with what the nodes are. Each has
//! an ArkTS twin building the identical tree, and node counts are checked at
//! runtime.
//!
//! Every app's assets are the reference app's own, shipped under
//! `rawfile/<app>/`.

pub mod taobao;
pub mod tiktok;
pub mod ui;
pub mod wonderous;

use crate::arkui::Node;
use std::cell::RefCell;
use std::time::Instant;

/// Which app is on screen.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum App {
    WeChat,
    Taobao,
    TikTok,
    Wonderous,
}

impl App {
    pub fn id(&self) -> &'static str {
        match self {
            App::WeChat => "wechat",
            App::Taobao => "taobao",
            App::TikTok => "tiktok",
            App::Wonderous => "wonderous",
        }
    }
    pub fn from_id(s: &str) -> App {
        match s {
            "taobao" => App::Taobao,
            "tiktok" => App::TikTok,
            "wonderous" => App::Wonderous,
            _ => App::WeChat,
        }
    }
    /// Tabs and pushed routes each app is toured over, as "tab|route".
    pub fn tour(&self) -> &'static [&'static str] {
        match self {
            App::WeChat => &["0|root", "1|root", "2|root", "3|root", "0|chat", "0|moments"],
            App::Taobao => &["0|root", "1|root", "2|root", "3|root", "4|root", "0|detail"],
            App::TikTok => &["1|reel0", "1|reel1", "1|reel2", "0|root", "1|sheet", "1|feed"],
            App::Wonderous => &["0|root", "1|root", "2|root", "3|root", "0|w1", "1|w2"],
        }
    }
}

/// Navigation state, shared across the apps.
pub struct Nav {
    pub app: App,
    pub tab: usize,
    /// App-specific sub-route index; meaning is up to the app.
    pub sub: usize,
    /// A pushed detail/sheet is open.
    pub pushed: bool,
}

thread_local! {
    static NAV: RefCell<Nav> = const {
        RefCell::new(Nav { app: App::WeChat, tab: 0, sub: 0, pushed: false })
    };
}

pub fn set_app(app: App) {
    NAV.with(|n| {
        let mut n = n.borrow_mut();
        n.app = app;
        n.tab = 0;
        n.sub = 0;
        n.pushed = false;
    });
}

pub fn current_app() -> App {
    NAV.with(|n| n.borrow().app)
}

/// Route a click. Returns true if the state changed and a rebuild is needed.
pub fn handle(target: i32) -> bool {
    NAV.with(|n| {
        let mut nav = n.borrow_mut();
        // Each app owns a range of ids, so one handler can serve all of them.
        match nav.app {
            App::Taobao => match target {
                taobao::BACK => {
                    let was = nav.pushed;
                    nav.pushed = false;
                    was
                }
                t if (taobao::TAB_BASE..taobao::TAB_BASE + 5).contains(&t) => {
                    nav.tab = (t - taobao::TAB_BASE) as usize;
                    nav.pushed = false;
                    true
                }
                t if t >= taobao::ITEM_BASE => {
                    nav.sub = (t - taobao::ITEM_BASE) as usize;
                    nav.pushed = true;
                    true
                }
                _ => false,
            },
            App::TikTok => match target {
                tiktok::BACK => {
                    let was = nav.pushed;
                    nav.pushed = false;
                    was
                }
                t if (tiktok::TAB_BASE..tiktok::TAB_BASE + 2).contains(&t) => {
                    nav.tab = (t - tiktok::TAB_BASE) as usize;
                    true
                }
                // The comment action opens the sheet; the others are no-ops
                // visually, exactly as in the reference app.
                t if t == tiktok::REEL_BASE + 2 => {
                    nav.pushed = true;
                    true
                }
                _ => false,
            },
            App::Wonderous => match target {
                wonderous::BACK => {
                    let was = nav.sub != 0;
                    nav.sub = 0;
                    was
                }
                t if (wonderous::TAB_BASE..wonderous::TAB_BASE + 4).contains(&t) => {
                    nav.tab = (t - wonderous::TAB_BASE) as usize;
                    true
                }
                t if t >= wonderous::ART_BASE => {
                    nav.sub = (t - wonderous::ART_BASE) as usize % wonderous::WONDERS.len();
                    true
                }
                _ => false,
            },
            App::WeChat => crate::wechat::handle(target),
        }
    })
}

/// Build the current app's current screen. Returns (root, nodes, µs).
pub fn build() -> (Option<Node>, usize, f64) {
    let (app, tab, sub, pushed) = NAV.with(|n| {
        let n = n.borrow();
        (n.app, n.tab, n.sub, n.pushed)
    });
    if app == App::WeChat {
        // WeChat predates this module and keeps its own builder and counter.
        return crate::wechat::build();
    }
    ui::reset_count();
    let t0 = Instant::now();
    let node = match app {
        App::Taobao => taobao::build(tab, if pushed { Some(sub) } else { None }),
        App::TikTok => tiktok::build(tab, sub, pushed),
        App::Wonderous => wonderous::build(tab, sub % wonderous::WONDERS.len()),
        App::WeChat => unreachable!(),
    };
    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    (node, ui::count(), us)
}

/// Build one named route without mounting it, for timing.
/// `route` is the second half of a tour entry.
pub fn build_route(app: App, tab: usize, route: &str) -> (usize, f64) {
    if app == App::WeChat {
        let r = match route {
            "chat" => crate::wechat::Route::Chat(1),
            "moments" => crate::wechat::Route::Moments,
            _ => crate::wechat::Route::Root,
        };
        return crate::wechat::build_timed(tab, r);
    }
    ui::reset_count();
    let t0 = Instant::now();
    let node = match app {
        App::Taobao => taobao::build(tab, if route == "detail" { Some(0) } else { None }),
        App::TikTok => match route {
            "sheet" => tiktok::build(tab, 0, true),
            "feed" => tiktok::build_feed(),
            r => {
                let idx = r.strip_prefix("reel").and_then(|s| s.parse().ok()).unwrap_or(0);
                tiktok::build(tab, idx, false)
            }
        },
        App::Wonderous => {
            let w = route.strip_prefix('w').and_then(|s| s.parse().ok()).unwrap_or(0usize);
            wonderous::build(tab, w % wonderous::WONDERS.len())
        }
        App::WeChat => unreachable!(),
    };
    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    let n = ui::count();
    drop(node);
    (n, us)
}

/// Screens kept alive for the memory arm.
thread_local! {
    static KEPT: RefCell<Vec<Node>> = const { RefCell::new(Vec::new()) };
}

/// Build every screen of `app` once and keep them all. Returns the total kept.
pub fn keep_all(app: App) -> usize {
    let saved = NAV.with(|n| {
        let b = n.borrow();
        (b.app, b.tab, b.sub, b.pushed)
    });
    for entry in app.tour() {
        let (tab, route) = split(entry);
        set_route(app, tab, route);
        let (node, _, _) = build();
        if let Some(node) = node {
            KEPT.with(|k| k.borrow_mut().push(node));
        }
    }
    NAV.with(|n| {
        let mut b = n.borrow_mut();
        b.app = saved.0;
        b.tab = saved.1;
        b.sub = saved.2;
        b.pushed = saved.3;
    });
    KEPT.with(|k| k.borrow().len())
}

pub fn drop_kept() -> usize {
    KEPT.with(|k| {
        let v = std::mem::take(&mut *k.borrow_mut());
        let n = v.len();
        drop(v);
        n
    })
}

fn split(entry: &str) -> (usize, &str) {
    let mut it = entry.splitn(2, '|');
    let tab = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (tab, it.next().unwrap_or("root"))
}

fn set_route(app: App, tab: usize, route: &str) {
    NAV.with(|n| {
        let mut b = n.borrow_mut();
        b.app = app;
        b.tab = tab;
        b.pushed = matches!(route, "detail" | "sheet");
        b.sub = match route {
            r if r.starts_with("reel") => r[4..].parse().unwrap_or(0),
            r if r.starts_with('w') && r.len() > 1 => r[1..].parse().unwrap_or(0),
            _ => 0,
        };
    });
    if app == App::WeChat {
        let r = match route {
            "chat" => crate::wechat::Route::Chat(1),
            "moments" => crate::wechat::Route::Moments,
            _ => crate::wechat::Route::Root,
        };
        crate::wechat::set_route(tab, r);
    }
}
