//! A frontend bundle, hosted the way Tauri hosts one.
//!
//! Every other web surface in this app is a string of markup Rust built and
//! handed to `loadData`. This one is not: it is a set of separate files served
//! over the `splash://` scheme by `assets.rs`, navigated to by URL, exactly as
//! a `dist/` from any bundler would be.
//!
//! The distinction is the whole point. A single self-contained document proves
//! `loadData` works. A document that has to go back for a stylesheet, an image,
//! a runtime-imported chunk and a `fetch` is what proves an app you did not
//! write can be hosted here — and that is the thing the bridge was missing.
//!
//! The native chrome above it is not decoration. It is there to show the two
//! halves in one tree: real ArkUI widgets built by Rust, with a web surface
//! positioned into the space they leave.

use crate::caps::Caps;
use crate::webslot::declare_app_with;
use splash_oh_native::arkui::Node;
use splash_oh_native::ui::*;

const CHROME: u32 = 0xFF14161A;
const BAR_H: f32 = 38.0;

pub fn build() -> Option<Node> {
    let mut root = col(W, PAGE_H, CHROME)?;

    let mut bar = row(W, BAR_H, CHROME)?;
    bar = bar.child(text(
        "FRONTEND BUNDLE · splash://app",
        11.0,
        0xFF6E6E88,
        W - 20.0,
        16.0,
    )?);
    root = root.child(bar);

    let body_h = PAGE_H - BAR_H;
    // A transparent placeholder of exactly the right size, so the native
    // layout reserves the space; ArkTS puts the real `Web` at these
    // coordinates. Same mechanism as every other slot -- only the source
    // differs, and with it the trust answer.
    // What this page may do, stated here rather than inherited. It reads
    // device facts, calls its own plugin, asks about permissions and fetches
    // one weather host -- and cannot touch the keystore, the camera, Bluetooth
    // or the filesystem, because it was never given them.
    let caps = Caps::none()
        .tools(&[
            "slot.ready",
            "echo",
            "log",
            "device.*",
            "demo.*",
            "plugin.list",
            "permission.request",
            "http.get",
            "fs.list",
        ])
        // One directory, so the path rule has something to allow as well as
        // something to refuse. A scope that only ever denies is not a scope.
        .fs_scope(&["/data/storage/el2/base/haps/entry/files"])
        .http_hosts(&["api.open-meteo.com"]);
    declare_app_with("/index.html", caps, 0.0, BAR_H, W, body_h);
    root = root.child(col(W, body_h, 0x00000000)?);
    Some(root)
}
