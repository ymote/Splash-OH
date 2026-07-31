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
    /// A page from the bundle this app ships, served over the `splash://`
    /// scheme by `assets.rs`. The string is the path within the bundle.
    ///
    /// Distinct from `Url` even though it navigates to one, because the trust
    /// answer is opposite: a `Url` is somebody else's page and gets no bridge,
    /// while this is the app's own frontend and is exactly what the bridge is
    /// for. Folding it into `Url` would have made "is this trusted" a question
    /// about string prefixes.
    App(String),
}

/// A web surface the DSL asked for, in vp, relative to the page.
#[derive(Clone)]
pub struct WebSlot {
    pub id: u32,
    pub source: Source,
    /// What this surface may do. Declared here, next to the geometry, because
    /// the page must not be able to influence it.
    pub caps: crate::caps::Caps,
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
    /// Reset per build, so the Nth slot of a screen keeps the same id every
    /// time that screen is built. See `reset`.
    static NEXT_ID: RefCell<u32> = const { RefCell::new(1) };
}

/// Start a build's slot list over.
///
/// **`NEXT_ID` resets too, and that is the point.** It used to increase
/// monotonically for the life of the process, which meant every rebuild minted
/// fresh ids for the same slots. ArkTS keys its `ForEach` on the encoded slot
/// string, and that string begins with the id — so a new id was a new key, and
/// a new key destroys the `Web` component and builds another one.
///
/// The consequences were all the ones you would predict and none of them were
/// being attributed here:
///
///   - a page reloaded from scratch on every `appRerender`, so nothing in it
///     could hold state across a native data update;
///   - `controllerFor` cached a `WebviewController` per id, so the map grew by
///     one dead controller per rebuild for the life of the process;
///   - a surface that went white on reload looked like a load bug rather than
///     a component that had just been thrown away and replaced.
///
/// Per-build ids are stable because a screen declares its slots in the same
/// order every time it is built. They are *positional*, not identities: slot 1
/// of one app and slot 1 of another are different pages. Nothing downstream
/// assumes otherwise — ArkTS keys `loaded` on id *and content*, so a different
/// page at the same id still reloads.
pub fn reset() {
    SLOTS.with(|s| s.borrow_mut().clear());
    NEXT_ID.with(|n| *n.borrow_mut() = 1);
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

/// Declare a surface showing a page from the shipped bundle, with everything.
///
/// Prefer [`declare_app_with`]. This exists because the demo apps were written
/// before capabilities did.
pub fn declare_app(path: &str, x: f32, y: f32, w: f32, h: f32) -> u32 {
    declare_source(Source::App(path.to_string()), x, y, w, h)
}

/// Declare a bundle surface and state what it may do.
pub fn declare_app_with(
    path: &str,
    caps: crate::caps::Caps,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> u32 {
    declare_with_caps(Source::App(path.to_string()), caps, x, y, w, h)
}

fn declare_source(source: Source, x: f32, y: f32, w: f32, h: f32) -> u32 {
    declare_with_caps(source, crate::caps::Caps::all(), x, y, w, h)
}

fn declare_with_caps(
    source: Source,
    caps: crate::caps::Caps,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> u32 {
    let id = NEXT_ID.with(|n| {
        let mut n = n.borrow_mut();
        let v = *n;
        *n = v.wrapping_add(1).max(1);
        v
    });
    remember_caps(id, &caps);
    SLOTS.with(|s| {
        s.borrow_mut().push(WebSlot {
            id,
            source,
            caps,
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
        matches!(self, Source::Html(_) | Source::App(_))
    }
}

/// The origin a slot's declared source should produce.
///
/// `None` for a `Url` slot, which is untrusted whatever it loads.
pub fn expected_origin(id: u32) -> Option<String> {
    SLOTS.with(|s| {
        s.borrow()
            .iter()
            .find(|x| x.id == id)
            .and_then(|x| match &x.source {
                // Follows the dev server when there is one: otherwise the
                // navigation guard would refuse the very page this build is meant
                // to load, and the bridge would disown it on arrival.
                Source::App(_) => Some(crate::assets::app_origin()),
                // Generated markup is installed with WEB_BASE as its baseUrl.
                Source::Html(_) => Some("https://localhost".to_string()),
                Source::Url(_) => None,
            })
    })
}

thread_local! {
    /// The origin each slot has actually been observed on, reported by ArkTS
    /// when a document starts loading.
    ///
    /// Deliberately NOT cleared by `reset`: a rebuild re-declares the slots but
    /// does not reload their documents, so dropping this would let a slot that
    /// had navigated away look untainted again on the very next rerender.
    static OBSERVED: RefCell<Vec<(u32, String)>> = const { RefCell::new(Vec::new()) };
}

/// Record where a slot's document actually came from.
pub fn set_observed_origin(id: u32, origin: &str) {
    OBSERVED.with(|o| {
        let mut o = o.borrow_mut();
        match o.iter_mut().find(|(i, _)| *i == id) {
            Some(e) => e.1 = origin.to_string(),
            None => o.push((id, origin.to_string())),
        }
    });
}

fn observed_origin(id: u32) -> Option<String> {
    OBSERVED.with(|o| {
        o.borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, s)| s.clone())
    })
}

/// Is `id` a slot this app generated? Unknown ids are untrusted.
pub fn is_trusted(id: u32) -> bool {
    let by_source = SLOTS.with(|s| {
        s.borrow()
            .iter()
            .find(|x| x.id == id)
            .map(|x| x.source.trusted())
            .unwrap_or(false)
    });
    if !by_source {
        return false;
    }
    // The source says trusted. That is not enough on its own, because trust was
    // recorded per *slot* while `javaScriptProxy` attaches to the *component*:
    // a trusted page that navigated itself elsewhere kept the bridge, and both
    // gates agreed. Measured, not theorised -- a document served by
    // example.com called the `log` tool and Rust wrote its text to hilog.
    //
    // So the document's actual origin has to match the one the slot's source
    // implies. An unobserved slot is allowed through: the first call from a
    // page can arrive before onPageBegin has reported, and ArkTS also refuses
    // the navigation that would create the mismatch in the first place.
    match (expected_origin(id), observed_origin(id)) {
        (Some(want), Some(got)) if want != got => {
            crate::log(&format!(
                "webslot: slot {id} declared {want} but its document is on {got} \
                 -- refusing to treat it as trusted"
            ));
            false
        }
        _ => true,
    }
}

/// Capability sets, reachable from any thread.
///
/// A process-wide map rather than a read of `SLOTS`, and the difference is not
/// stylistic: `SLOTS` is a `thread_local!`, and the tools that most need a
/// scope check -- `http.get`, `fs.read`, `fs.list` -- do their work on a
/// spawned worker. There `SLOTS` is empty, so `caps_for` returned `None` and
/// every scope check quietly passed. The host scope was measured letting a
/// request through to a host it did not grant.
///
/// Not cleared on `reset`, for the same reason `OBSERVED` is not: a rebuild
/// re-declares slots without stopping work already in flight, and a worker that
/// found no entry would be a worker with no scope.
static CAPS: std::sync::Mutex<Vec<(u32, crate::caps::Caps)>> = std::sync::Mutex::new(Vec::new());

fn remember_caps(id: u32, caps: &crate::caps::Caps) {
    if let Ok(mut m) = CAPS.lock() {
        match m.iter_mut().find(|(i, _)| *i == id) {
            Some(e) => e.1 = caps.clone(),
            None => m.push((id, caps.clone())),
        }
    }
}

/// The capability set for `id`. Nothing for an unknown slot.
pub fn caps_for(id: u32) -> Option<crate::caps::Caps> {
    CAPS.lock()
        .ok()
        .and_then(|m| m.iter().find(|(i, _)| *i == id).map(|(_, c)| c.clone()))
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
                // A real URL, because this one does navigate: the scheme
                // handler answers it. Nothing is pushed through loadData.
                // A dev build points this at the bundler instead, so the
                // page comes from the live-reloading server.
                Source::App(p) => match crate::assets::dev_server() {
                    Some(dev) => format!("{dev}{p}"),
                    None => format!("{}://app{}", crate::assets::SCHEME, p),
                },
            };
            let kind = match &s.source {
                Source::Url(_) => "url",
                Source::Html(_) => "html",
                Source::App(_) => "app",
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
            // Neither of these is pushed as markup: a URL navigates, and an app
            // slot navigates to a splash:// URL the scheme handler answers.
            Source::Url(_) | Source::App(_) => None,
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

use splash_oh_arkui::arkui::Node;
use splash_oh_arkui::ui::col;

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
    let with_shim = format!("{}{}", crate::bridge::shim(), html);
    self::declare_html(with_shim, x, y, w, h);
    col(w, h, 0x00000000)
}
