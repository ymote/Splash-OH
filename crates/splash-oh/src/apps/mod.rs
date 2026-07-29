//! Four makepad reference apps, ported to native ArkUI and driven from Rust.
//!
//! | app | shape | why it is here |
//! |---|---|---|
//! | [`wechat`] | text-heavy lists | many small nodes |
//! | [`taobao`] | two-column product grid | many nodes *and* many images |
//! | [`tiktok`] | full-screen media plus overlays | few nodes, one big image |
//! | [`wonderous`] | editorial and photo grid | moderate nodes, large images |
//! | [`browser`] | native chrome around a web surface | exercises `webslot` |
//!
//! Four different widget mixes rather than four versions of the same list,
//! because a single app cannot tell you whether the Rust-vs-ArkTS gap is a
//! per-node constant or something that varies with what the nodes are. Each has
//! an ArkTS twin building the identical tree, and node counts are checked at
//! runtime.
//!
//! Every app's assets are the reference app's own, shipped under
//! `rawfile/<app>/`.

pub mod browser;
pub mod files;
pub mod frontend;
pub mod native;

pub mod weather_web;

use splash_oh_native::arkui::Node;
use std::cell::RefCell;
use std::time::Instant;

/// Which app is on screen.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum App {
    WeChat,
    Taobao,
    TikTok,
    Wonderous,
    /// Not part of the benchmark set — it contains an ArkTS `Web` surface, so
    /// it has no ArkTS twin to be compared against and no meaningful node count.
    Browser,
    /// The Material catalog, rendered entirely by the Splash VM.
    ///
    /// The one app here with no Rust building its tree: `catalog.splash` is
    /// 482 lines of DSL evaluated on device, and every widget it produces is a
    /// real ArkUI node. No makepad anywhere in the path.
    Catalog,
    /// The shipped frontend bundle, served over `splash://` by `assets.rs`.
    ///
    /// The one app here that is a web page this crate did not generate: it
    /// arrives as separate files through the scheme handler, the way a real
    /// frontend build would.
    Frontend,
    /// Every bridge tool on one screen with live values. The fixture the
    /// bridge is verified against, so a new capability is one row rather than
    /// a new panel bolted onto whichever card was nearest.
    Native,
    /// A file browser in a web surface. Like Browser it has no ArkTS twin,
    /// and it exists to find out how far into the phone a web surface can see.
    Files,
    /// The web-rendered weather card. Its native twin is
    /// `assets/weather.splash`, which draws the same Open-Meteo data with real
    /// widgets — the pair is there to compare the two renderers on one source.
    WeatherWeb,
}

impl App {
    pub fn id(&self) -> &'static str {
        match self {
            App::WeChat => "wechat",
            App::Taobao => "taobao",
            App::TikTok => "tiktok",
            App::Wonderous => "wonderous",
            App::Browser => "browser",
            App::WeatherWeb => "weatherweb",
            App::Files => "files",
            App::Native => "native",
            App::Frontend => "frontend",
            App::Catalog => "catalog",
        }
    }
    pub fn from_id(s: &str) -> App {
        match s {
            "taobao" => App::Taobao,
            "tiktok" => App::TikTok,
            "wonderous" => App::Wonderous,
            "browser" => App::Browser,
            "weatherweb" => App::WeatherWeb,
            "files" => App::Files,
            "native" => App::Native,
            "frontend" => App::Frontend,
            "catalog" => App::Catalog,
            _ => App::WeChat,
        }
    }
    /// Tabs and pushed routes each app is toured over, as "tab|route".
    pub fn tour(&self) -> &'static [&'static str] {
        match self {
            App::WeChat => &[
                "0|root",
                "1|root",
                "2|root",
                "3|root",
                "0|chat",
                "0|moments",
            ],
            App::Taobao => &["0|root", "1|root", "2|root", "3|root", "4|root", "0|detail"],
            App::TikTok => &[
                "1|reel0", "1|reel1", "1|reel2", "0|root", "1|sheet", "1|feed",
            ],
            App::Wonderous => &["0|root", "1|root", "2|root", "3|root", "0|w1", "1|w2"],
            App::Browser => &["0|root", "1|root", "2|root", "3|root", "0|root", "1|root"],
            App::WeatherWeb => &["0|root", "1|root", "2|root", "3|root", "0|root", "1|root"],
            App::Files => &["0|root", "1|root", "2|root", "3|root", "4|root", "0|root"],
            App::Native => &["0|root", "0|root", "0|root", "0|root", "0|root", "0|root"],
            App::Frontend => &["0|root"],
            App::Catalog => &[
                "0|root",
                "0|buttons",
                "0|chips",
                "0|lists",
                "0|color",
                "0|root",
            ],
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
    /// TikTok's whole-feed route, which is not a tab or a pushed view and so
    /// needs its own flag. Without it `set_route` silently fell through to a
    /// single reel while the ArkTS twin built all five — which showed up as a
    /// bogus +50% memory result for TikTok and nothing else.
    pub feed: bool,
}

thread_local! {
    static NAV: RefCell<Nav> = const {
        RefCell::new(Nav { app: App::WeChat, tab: 0, sub: 0, pushed: false, feed: false })
    };
}

pub fn set_app(app: App) {
    NAV.with(|n| {
        let mut n = n.borrow_mut();
        n.app = app;
        n.tab = 0;
        n.sub = 0;
        n.pushed = false;
        n.feed = false;
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
                splash_oh_native::taobao::BACK => {
                    let was = nav.pushed;
                    nav.pushed = false;
                    was
                }
                t if (splash_oh_native::taobao::TAB_BASE
                    ..splash_oh_native::taobao::TAB_BASE + 5)
                    .contains(&t) =>
                {
                    nav.tab = (t - splash_oh_native::taobao::TAB_BASE) as usize;
                    nav.pushed = false;
                    true
                }
                t if t >= splash_oh_native::taobao::ITEM_BASE => {
                    nav.sub = (t - splash_oh_native::taobao::ITEM_BASE) as usize;
                    nav.pushed = true;
                    true
                }
                _ => false,
            },
            App::TikTok => match target {
                splash_oh_native::tiktok::BACK => {
                    let was = nav.pushed;
                    nav.pushed = false;
                    was
                }
                t if (splash_oh_native::tiktok::TAB_BASE
                    ..splash_oh_native::tiktok::TAB_BASE + 2)
                    .contains(&t) =>
                {
                    nav.tab = (t - splash_oh_native::tiktok::TAB_BASE) as usize;
                    true
                }
                // The comment action opens the sheet; the others are no-ops
                // visually, exactly as in the reference app.
                t if t == splash_oh_native::tiktok::REEL_BASE + 2 => {
                    nav.pushed = true;
                    true
                }
                _ => false,
            },
            App::Wonderous => match target {
                splash_oh_native::wonderous::BACK => {
                    let was = nav.sub != 0;
                    nav.sub = 0;
                    was
                }
                t if (splash_oh_native::wonderous::TAB_BASE
                    ..splash_oh_native::wonderous::TAB_BASE + 4)
                    .contains(&t) =>
                {
                    nav.tab = (t - splash_oh_native::wonderous::TAB_BASE) as usize;
                    true
                }
                t if t >= splash_oh_native::wonderous::ART_BASE => {
                    nav.sub = (t - splash_oh_native::wonderous::ART_BASE) as usize
                        % splash_oh_native::wonderous::WONDERS.len();
                    true
                }
                _ => false,
            },
            App::Browser => match target {
                // Derived from TABS rather than hardcoded: a fifth tab was
                // added and this range was not, so the new tab drew but did
                // not respond to a tap.
                t if (browser::TAB_BASE..browser::TAB_BASE + browser::TABS.len() as i32)
                    .contains(&t) =>
                {
                    nav.tab = (t - browser::TAB_BASE) as usize;
                    true
                }
                // Reload re-declares the same slot, which is enough for ArkTS
                // to rebind and re-load the page.
                browser::RELOAD => true,
                _ => false,
            },
            App::WeatherWeb => match target {
                t if (weather_web::CITY_BASE..weather_web::CITY_BASE + 4).contains(&t) => {
                    nav.tab = (t - weather_web::CITY_BASE) as usize;
                    true
                }
                _ => false,
            },
            App::Files => match target {
                t if (files::TAB_BASE..files::TAB_BASE + files::ROOTS.len() as i32)
                    .contains(&t) =>
                {
                    nav.tab = (t - files::TAB_BASE) as usize;
                    true
                }
                _ => false,
            },
            // No navigation: one page, every tool live on it.
            App::Native => false,
            App::Frontend => false,
            // The DSL owns the ids: NAV_BASE + row index opens a screen, and
            // NAV_BACK returns to the index. `sub` carries the screen, with 0
            // meaning the index itself.
            App::Catalog => match target {
                splash_oh_native::dsl::CATALOG_NAV_BACK => {
                    let was = nav.sub != 0;
                    nav.sub = 0;
                    was
                }
                t if t >= splash_oh_native::dsl::CATALOG_NAV_BASE => {
                    let idx = (t - splash_oh_native::dsl::CATALOG_NAV_BASE) as usize;
                    if idx < splash_oh_native::dsl::CATALOG_SCREENS.len() {
                        nav.sub = idx + 1;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            App::WeChat => splash_oh_native::wechat::handle(target),
        }
    })
}

/// Point the catalog at a screen index (0 = the index page).
pub fn set_catalog_screen(idx: usize) {
    NAV.with(|n| n.borrow_mut().sub = idx);
}

/// Build the current app's current screen. Returns (root, nodes, µs).
pub fn build() -> (Option<Node>, usize, f64) {
    // Whichever thread renders is a thread that must not block on the network.
    splash_oh_native::net::mark_ui_thread();
    let (app, tab, sub, pushed, feed) = NAV.with(|n| {
        let n = n.borrow();
        (n.app, n.tab, n.sub, n.pushed, n.feed)
    });
    if app == App::WeChat {
        // WeChat predates this module and keeps its own builder and counter.
        return splash_oh_native::wechat::build();
    }
    splash_oh_native::ui::reset_count();
    crate::webslot::reset();
    let t0 = Instant::now();
    let node = match app {
        App::Taobao => splash_oh_native::taobao::build(tab, if pushed { Some(sub) } else { None }),
        App::TikTok => {
            if feed {
                splash_oh_native::tiktok::build_feed()
            } else {
                splash_oh_native::tiktok::build(tab, sub, pushed)
            }
        }
        App::Wonderous => splash_oh_native::wonderous::build(
            tab,
            sub % splash_oh_native::wonderous::WONDERS.len(),
        ),
        App::Browser => browser::build(tab),
        App::WeatherWeb => weather_web::build(tab),
        App::Files => files::build(tab),
        App::Native => native::build(),
        App::Frontend => frontend::build(),
        App::Catalog => {
            let screen = if sub == 0 {
                ""
            } else {
                splash_oh_native::dsl::CATALOG_SCREENS[sub - 1]
            };
            splash_oh_native::dsl::build_screen(screen, None)
        }
        App::WeChat => unreachable!(),
    };
    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    (node, splash_oh_native::ui::count(), us)
}

/// Build one named route without mounting it, for timing.
/// `route` is the second half of a tour entry.
pub fn build_route(app: App, tab: usize, route: &str) -> (usize, f64) {
    if app == App::WeChat {
        let r = match route {
            "chat" => splash_oh_native::wechat::Route::Chat(1),
            "moments" => splash_oh_native::wechat::Route::Moments,
            _ => splash_oh_native::wechat::Route::Root,
        };
        return splash_oh_native::wechat::build_timed(tab, r);
    }
    splash_oh_native::ui::reset_count();
    let t0 = Instant::now();
    let node = match app {
        App::Taobao => {
            splash_oh_native::taobao::build(tab, if route == "detail" { Some(0) } else { None })
        }
        App::TikTok => match route {
            "sheet" => splash_oh_native::tiktok::build(tab, 0, true),
            "feed" => splash_oh_native::tiktok::build_feed(),
            r => {
                let idx = r
                    .strip_prefix("reel")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                splash_oh_native::tiktok::build(tab, idx, false)
            }
        },
        App::Wonderous => {
            let w = route
                .strip_prefix('w')
                .and_then(|s| s.parse().ok())
                .unwrap_or(0usize);
            splash_oh_native::wonderous::build(tab, w % splash_oh_native::wonderous::WONDERS.len())
        }
        App::Browser => browser::build(tab),
        App::WeatherWeb => weather_web::build(tab),
        App::Files => files::build(tab),
        App::Native => native::build(),
        App::Frontend => frontend::build(),
        App::Catalog => {
            // The tour names screens directly ("0|chips"), so "root" is the
            // index and anything else is the screen id verbatim.
            let screen = if route == "root" { "" } else { route };
            splash_oh_native::dsl::build_screen(screen, None)
        }
        App::WeChat => unreachable!(),
    };
    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    let n = splash_oh_native::ui::count();
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
        (b.app, b.tab, b.sub, b.pushed, b.feed)
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
        b.feed = saved.4;
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
        b.feed = route == "feed";
        b.sub = match route {
            r if r.starts_with("reel") => r[4..].parse().unwrap_or(0),
            r if r.starts_with('w') && r.len() > 1 => r[1..].parse().unwrap_or(0),
            _ => 0,
        };
    });
    if app == App::WeChat {
        let r = match route {
            "chat" => splash_oh_native::wechat::Route::Chat(1),
            "moments" => splash_oh_native::wechat::Route::Moments,
            _ => splash_oh_native::wechat::Route::Root,
        };
        splash_oh_native::wechat::set_route(tab, r);
    }
}
