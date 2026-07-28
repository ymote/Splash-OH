# Splash-OH

Splash DSL on OpenHarmony, in two crates.

```
crates/splash-oh-native/    the renderer          rlib
crates/splash-oh/           the bridge            cdylib -> libsplash_oh.so
deveco/                     the ArkTS shell
```

## splash-oh-native

Renders a UI tree to **native ArkUI widgets** from Rust. ArkTS hands over one
`NodeContent` at startup; after that every widget is created, configured, laid
out and event-wired by this crate, with no per-widget and no per-frame ArkTS
calls.

Contains the ArkUI NDK binding, the Splash DSL walker, the widget builders, the
four ported reference apps (WeChat, Taobao, TikTok, Wonderous) and the
Rust-vs-ArkTS benchmark those apps exist to run.

It does not know that webviews exist.

## splash-oh (the bridge)

A Tauri-style `splash.invoke(tool, args)` from a page to Rust, plus the 45
native capabilities behind it: device, display, battery, sensors, haptics,
location, radio, Wi-Fi, network, filesystem, picker, clipboard, keystore,
SQLite, Bluetooth, camera, audio, video, crypto and the Splash VM.

Also owns the web slots, the capability gate, the XComponent surface that camera
and video render into, and the napi surface ArkTS calls.

## Why the split runs this way

The dependency is one-directional, and that is what made it worth doing.
Nothing in `splash-oh-native` mentions the bridge, a web slot, an XComponent or
ArkWeb. Every card in `splash-oh`, by contrast, builds real ArkUI chrome out of
the other crate's widgets — the browser's tab strip, the file card's roots, the
capability dashboard's header are native nodes with a web surface positioned
into the hole they leave.

Two crates, one `.so`, because ArkTS loads exactly one. The artifact is still
`libsplash_oh.so`; only the package names record the split.

One seam needed inverting. `app.rs` used to call the app router directly, which
would have made the renderer depend on the bridge. It now exposes
`app::set_router`, and the bridge crate installs the route at `mount()`.

## Where ArkTS still is, and is not

| | |
|---|---|
| free of it | the widget tree, all 45 capabilities, the XComponent surface carrying camera and video |
| structurally cannot be | the `Web` component — there is no `ARKUI_NODE_WEB` |
| only for want of an NDK | file picker, clipboard, runtime permissions, BLE scan |

`OH_NativeArkWeb_RunJavaScript` resolves on device but the controller's web tag
never binds, so bridge traffic still relays through ArkTS. Measured, not
assumed — see `crates/splash-oh/src/arkweb.rs`.

## Build

```sh
./build.sh          # cargo -> stage .so -> hvigor -> install -> launch
```

Needs `OHOS_BASE_SDK_HOME` or `OHOS_SDK_NATIVE`, and a signed profile — the
device is commercial HarmonyOS, so community signing is rejected.
