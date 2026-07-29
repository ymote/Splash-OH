//! Web surfaces the DSL can place, as a first-class node type.
//!
//! # Why this is not an ArkUI node
//!
//! There is no `ARKUI_NODE_WEB`. All 48 node types in `native_node.h` were
//! checked; the NDK exposes no web component at all, the same gap as video. So
//! Rust cannot create a web view the way it creates a `Text` or a `Column`, and
//! a webview in a Splash tree has to be an **ArkTS `Web` component positioned
//! on top of the native tree**.
//!
//! # How the hole is cut
//!
//! The DSL emits a `{t: "web", url: ..., w, h}` node. Rust builds a
//! transparent placeholder of exactly that size so the native layout reserves
//! the space, and records the geometry here. ArkTS reads the record and puts a
//! real `Web` at those coordinates in a `Stack` above the `ContentSlot`.
//!
//! This works because of a property this codebase already has: native ArkUI
//! nodes do not auto-size, so the DSL states every width and height explicitly.
//! That means Rust knows the geometry at build time and does not have to wait
//! for a layout pass to find out where the hole ended up.
//!
//! # What this replaces
//!
//! A build-time `YOUTUBE_MODE` flag that swapped the whole page layout for a
//! hardcoded `Web({src: 'https://www.youtube.com/embed/...'})`. That could only
//! ever be one webview, at one fixed position, with a URL ArkTS owned. A DSL
//! that cannot say *what* to load is not driving anything.

use std::cell::RefCell;

/// What a surface should show.
#[derive(Clone)]
pub enum Source {
    /// Navigate to a URL.
    Url(String),
    /// Render markup the app generated. Delivered by `loadData` rather than a
    /// `data:` URL, which has a length limit a real page overruns.
    Html(String),
}

/// A web surface the DSL asked for, in vp, relative to the page.
#[derive(Clone)]
pub struct WebSlot {
    pub id: u32,
    pub source: Source,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

thread_local! {
    /// Slots declared by the tree currently being built. Cleared at the start
    /// of every build, because a stale slot leaves a webview floating over a
    /// screen that no longer has one.
    static SLOTS: RefCell<Vec<WebSlot>> = const { RefCell::new(Vec::new()) };
    static NEXT_ID: RefCell<u32> = const { RefCell::new(1) };
}

pub fn reset() {
    SLOTS.with(|s| s.borrow_mut().clear());
}

/// Record a web surface. Returns its id, which ArkTS uses to address the
/// controller for `loadUrl` / `runJavaScript` / back-forward.
pub fn declare(url: &str, x: f32, y: f32, w: f32, h: f32) -> u32 {
    declare_source(Source::Url(url.to_string()), x, y, w, h)
}

/// Declare a surface showing generated markup.
pub fn declare_html(html: String, x: f32, y: f32, w: f32, h: f32) -> u32 {
    declare_source(Source::Html(html), x, y, w, h)
}

fn declare_source(source: Source, x: f32, y: f32, w: f32, h: f32) -> u32 {
    let id = NEXT_ID.with(|n| {
        let mut n = n.borrow_mut();
        let v = *n;
        *n = v.wrapping_add(1).max(1);
        v
    });
    SLOTS.with(|s| {
        s.borrow_mut().push(WebSlot {
            id,
            source,
            x,
            y,
            w,
            h,
        })
    });
    id
}

/// Whether a slot's content is trusted with the native bridge.
///
/// Markup this app generated is trusted; a remote page is not. The rule is the
/// source, not a flag someone can set: `apps/browser.rs` loads Wikipedia and
/// Hacker News into slots, and without this every one of those pages would get
/// the same `splash_native` object the weather card has, with `splash.eval` and
/// `http.get` on it.
impl Source {
    pub fn trusted(&self) -> bool {
        matches!(self, Source::Html(_))
    }
}

/// Is `id` a slot this app generated? Unknown ids are untrusted.
pub fn is_trusted(id: u32) -> bool {
    SLOTS.with(|s| {
        s.borrow()
            .iter()
            .find(|x| x.id == id)
            .map(|x| x.source.trusted())
            .unwrap_or(false)
    })
}

pub fn slots() -> Vec<WebSlot> {
    SLOTS.with(|s| s.borrow().clone())
}

/// Base64. Unused since generated pages stopped travelling as `data:` URLs,
/// kept because the next thing that needs to inline an asset will want it.
#[allow(dead_code)]
fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Serialised for the napi boundary as `id|kind|x|y|w|h|src`.
///
/// Generated markup travels as a base64 `data:` URL in the `src` field rather
/// than through `loadData`. `loadData` reported success and then rendered
/// nothing: ArkWeb gives content loaded that way an opaque origin, and without
/// a `baseUrl` the page never paints — no exception, no console error, just a
/// white surface. A `data:` URL is also declarative, so the page arrives with
/// the component instead of needing a second call once the controller is ready,
/// which removes the readiness dance entirely.
///
/// The cost is that the markup rides in a polled list. At ~3 KB a page that is
/// fine; a page big enough to care about would want `loadData` with a real
/// baseUrl instead.
pub fn encoded() -> Vec<String> {
    slots()
        .iter()
        .map(|s| {
            let src = match &s.source {
                Source::Url(u) => u.clone(),
                // Generated pages do NOT travel as a data: URL. ArkWeb is
                // Chromium-based and Chromium blocks top-level navigation to
                // data:, so the surface stayed white with no error. They are
                // fetched by id and installed with loadData + a baseUrl.
                Source::Html(_) => String::new(),
            };
            let kind = match &s.source {
                Source::Url(_) => "url",
                Source::Html(_) => "html",
            };
            // `kind` doubles as the trust marker: ArkTS attaches the bridge
            // only to `html` slots. Rust re-checks anyway -- see bridge::invoke.
            format!("{}|{}|{}|{}|{}|{}|{}", s.id, kind, s.x, s.y, s.w, s.h, src)
        })
        .collect()
}

/// The markup for a slot, or empty if it is a URL slot or no longer exists.
pub fn html_for(id: u32) -> String {
    slots()
        .iter()
        .find(|s| s.id == id)
        .and_then(|s| match &s.source {
            Source::Html(h) => Some(h.clone()),
            Source::Url(_) => None,
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// The two builders that make a hole in a native tree.
//
// These lived in the renderer's ui.rs until the crates were split, and that is
// where the seam turned out to be: every other builder there is pure geometry
// and colour, while these two reach into the web slot registry and the bridge
// shim. A renderer that has to know what a bridge is has not really been
// separated from one.
// ---------------------------------------------------------------------------

use splash_oh_native::arkui::Node;
use splash_oh_native::ui::col;

/// A web surface at an absolute page position.
///
/// Emits a transparent placeholder so the native layout reserves the space,
/// and records the geometry for ArkTS to put a real `Web` component there.
/// Absolute coordinates rather than flow-relative ones because the ArkTS
/// overlay is positioned against the page, not against this node's parent --
/// and because the DSL sizes everything explicitly anyway, so they are known.
pub fn web(url: &str, x: f32, y: f32, w: f32, h: f32) -> Option<Node> {
    self::declare(url, x, y, w, h);
    // Fully transparent: the Web component draws here, not this node.
    col(w, h, 0x00000000)
}

/// A web surface showing markup the app generated, rather than a URL.
///
/// The bridge shim is prepended, so every generated page can call
/// `splash.invoke(tool, args)` without having to carry the plumbing itself.
pub fn web_html(html: String, x: f32, y: f32, w: f32, h: f32) -> Option<Node> {
    let with_shim = format!("{}{}", crate::bridge::SHIM, html);
    self::declare_html(with_shim, x, y, w, h);
    col(w, h, 0x00000000)
}
