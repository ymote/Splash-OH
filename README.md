# Splash-OH

Build a HarmonyOS app with a web frontend and Rust for everything else.

A page calls `splash.invoke('device.info')` and gets an answer from Rust. Around
that page, real native ArkUI widgets — built from Rust too, not from ArkTS. The
shape is Tauri's; the widget layer is something Tauri does not have.

```js
const info = await splash.invoke('device.info')
// { productModel: "SUP-AL90", osFullName: "OpenHarmony-6.1.1.120", ... }
```

## Start

```sh
splash-oh new my-app
cd my-app
npm install && npm run build
./build.sh              # builds, installs and launches on a connected phone
```

`./build.sh` needs a Splash-OH checkout to build against — clone one beside your
project or set `SPLASH_OH`. See **[docs/building-an-app.md](docs/building-an-app.md)**.

While developing, skip the rebuild entirely:

```sh
splash-oh dev           # tunnels the phone to your machine over USB
npm run dev
SPLASH_DEV_SERVER=http://127.0.0.1:5173 ./build.sh
```

Frontend edits now reload on the device in about a second. Rust edits still need
a rebuild.

## Documentation

| | |
|---|---|
| [Building an app](docs/building-an-app.md) | the template, the dev loop, `splash.toml`, the build |
| [Plugins](docs/plugins.md) | your own native tools, sync and async |
| [Capabilities](docs/capabilities.md) | what a page may do, and how that is enforced |
| [Releasing](docs/releasing.md) | signing, AGC, and what is not done yet |

## What a page can reach

49 built-in tools, a few of them internal: device, display, battery, sensors,
haptics, location, radio, Wi-Fi, network, filesystem, picker, clipboard, HUKS
keystore, SQLite, Bluetooth, camera, audio, video, crypto, and the Splash VM.
Your own tools go beside them — see [docs/plugins.md](docs/plugins.md).

A page gets only what its surface was granted. Trust is not one bit: each
surface declares which tools, which directories and which hosts it may reach.
See [docs/capabilities.md](docs/capabilities.md).

## The crates

```
crates/splash-oh-native/       the renderer                rlib
crates/splash-oh-core/         registry, Args, Responder   rlib
crates/splash-oh-plugin-demo/  an example plugin           rlib
crates/splash-oh-cli/          host-side tooling           bin: splash-oh
crates/splash-oh/              the bridge and the app      cdylib -> libsplash_oh.so
deveco/                        the ArkTS shell
```

The dependencies run one way, and that is what makes plugins possible.
`splash-oh-native` does not know webviews exist. `splash-oh-core` does not know
the bridge exists — which is why a plugin can depend on it without depending on
the app. `splash-oh` is a `cdylib`, a final artifact nothing links against, so it
is the crate that decides which plugins are in a build.

One `.so` comes out, because ArkTS loads exactly one.

### splash-oh-native

Renders a UI tree to native ArkUI widgets from Rust. ArkTS hands over one
`NodeContent` at startup; after that every widget is created, configured, laid
out and event-wired by native code, with no per-widget and no per-frame ArkTS
call.

Contains the ArkUI NDK binding, the Splash DSL walker, the widget builders, four
ported reference apps (WeChat, Taobao, TikTok, Wonderous) and the Rust-vs-ArkTS
benchmark they exist to run.

#### The component catalog

`assets/catalog.splash` is a Material component catalog written in the DSL and
rendered to native ArkUI — an index plus 28 screens, no makepad, no ArkTS
widgets. All 28 are photographed in `catalog-screens.png`, and every one has been
looked at on a device rather than merely being reachable.

Two were wrong when that check was first run, and both are fixed: Badges drew
unlabelled pills under a caption promising numbers, and Text picker drew empty
rows because nothing set its range. `CATALOG_WALK_MS` in `Index.ets` re-runs the
sweep that found them.

## Where ArkTS still is, and is not

| | |
|---|---|
| free of it | the widget tree, all the capabilities, the XComponent surface carrying camera and video |
| structurally cannot be | the `Web` component — there is no `ARKUI_NODE_WEB` |
| only for want of an NDK | file picker, clipboard, runtime permissions, BLE scan |

`OH_NativeArkWeb_RunJavaScript` resolves on device but the controller's web tag
never binds, so bridge traffic still relays through ArkTS. Measured, not
assumed — see `crates/splash-oh/src/arkweb.rs`.

## Honest status

This runs on real hardware, and everything documented here was verified on a
HarmonyOS 6.1 device rather than inferred. What is not done:

- **Signing for release is not wired.** `sign-hap.sh` has a headless AGC path
  and `splash.toml` has a `[signing]` section; nothing connects them yet. See
  [docs/releasing.md](docs/releasing.md).
- **The shell is a checkout, not a dependency.** A project builds *against* a
  Splash-OH clone, and linking your own plugin is two manual edits in it.
- **No multi-window, updater or tray.** OHOS equivalents are unexplored rather
  than planned.
- **`cargo test` cannot run here.** The crates build only for
  `aarch64-unknown-linux-ohos`, the host cannot exec that, and the device
  refuses under SELinux. The checks that matter run at startup instead and log
  their result — search hilog for `selftest`.
