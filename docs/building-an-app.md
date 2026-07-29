# Building an app

## What you get

```sh
splash-oh new my-app
```

```
my-app/
  splash.toml        name, bundle id, version, icon, permissions
  index.html         must load /__splash.js — see below
  src/main.js        your frontend
  src/style.css
  public/__splash.js the bridge shim, generated
  plugin/            your own native code, in Rust
  build.sh           builds against a Splash-OH checkout
  README.md
```

Any bundler works. The template uses Vite because it needs to use something;
nothing here knows what produced the `dist/`.

## The one contract

A frontend must load the shim before it can call native code:

```html
<script src="/__splash.js"></script>
```

Without it there is no `window.splash`. In a release build the app serves that
URL itself; a dev server does not know about it, which is why `splash-oh new`
writes a copy into `public/` and `splash-oh shim` refreshes it.

It is a served file rather than something injected into your HTML on the way
past. Rewriting someone else's markup is the kind of convenience that becomes
impossible to debug the first time it goes wrong.

## Building

```sh
npm run build
./build.sh
```

`build.sh` finds a Splash-OH checkout — `SPLASH_OH`, or a sibling directory —
and hands off to its build with `SPLASH_FRONTEND_DIR` pointing at your `dist/`.
That build compiles the Rust, stages the `.so`, runs hvigor, installs and
launches.

Everything under `dist/` is embedded: nested folders, content-hashed filenames,
any file type. A `build.rs` walks the directory and generates the asset table,
so `index-a3f2c9.js` changing on every build costs nothing.

Served over a custom scheme, so relative URLs resolve the way a web page
expects:

```
splash://app/index.html
splash://app/assets/index-CJUgn_EW.js
```

## The dev loop

```sh
splash-oh dev                                       # once, per session
npm run dev                                         # terminal 1
SPLASH_DEV_SERVER=http://127.0.0.1:5173 ./build.sh  # terminal 2
```

The app now loads your frontend from the bundler instead of from the embedded
bundle, so edits reload on the device with no rebuild. Rust changes still need
one.

`splash-oh dev` opens a USB tunnel with `hdc rport`, mapping the phone's
`127.0.0.1:5173` to your machine's. It is over USB rather than Wi-Fi on purpose.
Pointing the phone at the host's LAN address is the obvious approach and it
failed on this hardware — both ends shared a `192.168.125.x` subnet over the USB
link and requests still came back `ERR_ADDRESS_UNREACHABLE`. The tunnel
sidesteps the question: there is no network to configure and no firewall to
open.

Copying files onto the phone is not an alternative. `hdc file send` resolves
outside the app's mount namespace, so they never arrive anywhere the app can
read them. Hence a server.

The tunnel does not survive a replug or a reboot. Rerun `splash-oh dev` if the
page stops loading.

`SPLASH_DEV_SERVER` is read at build time, not runtime. A debug convenience that
could be switched on in a shipped app is a way for someone else's server to
become your frontend.

The embedded bundle is still compiled into a dev build, so pointing at a server
that is not running gives a page that fails to load rather than an app with no
frontend at all.

## splash.toml

```toml
[app]
name         = "Weather Deck"
bundle-id    = "com.example.weatherdeck"
version      = "0.3.1"
version-code = 1000301
icon         = "public/icon.png"

[frontend]
dist       = "dist"
dev-server = "http://localhost:5173"

[signing]
# profile = "~/.ohos/config/release.p7b"

[permissions]
declare = [
  "ohos.permission.INTERNET",
  "ohos.permission.GET_NETWORK_INFO",
]
```

```sh
splash-oh apply
```

writes all of it into the shell: bundle id, version name and code, both labels,
the icon, and the declared permission list.

**Both labels**, because there are two and they appear in different places.
`app_name` is what Settings and the app list show; `EntryAbility_label` is the
caption under the launcher icon. Setting only one leaves the icon reading
`label`, which looks like a bug in your app rather than an unset field.

If `[signing]` names a provisioning profile, `apply` checks that the profile's
bundle id matches yours **before writing anything**:

```
splash-oh: the provisioning profile is issued for "com.example.myapplication",
           but splash.toml says "com.futurewei.weatherdeck".
           Change app.bundle-id to match, or get a profile for this id in
           AppGallery Connect.
```

A profile is issued for exactly one bundle id, and a mismatch otherwise fails at
install with a numeric code naming neither.

Passwords never go in this file. Signing reads `SPLASH_SIGN_PWD` from the
environment.

## Adding native code

See [plugins.md](plugins.md). The short version: write a tool in
`plugin/src/lib.rs`, call it from JS by name.

```rust
r.add("app.greet", "Say hello", |args: &Args, resp: Responder| {
    let g: Greet = match args.parse() { Ok(g) => g, Err(e) => return resp.err(e) };
    resp.ok(serde_json::to_string(&format!("hello, {}", g.name)).unwrap_or_default())
});
```

```js
await splash.invoke('app.greet', { name: 'world' })
```

Linking it in is currently two edits in the Splash-OH checkout, because the
`.so` is built there and only the crate producing it can pull a plugin into the
binary. The template's example says `not linked yet — see README` until you make
them. That is the part of the story still to be automated.

## Troubleshooting

**Blank page.** Check hilog for `SPLASHASSET`: every served file logs there with
its status and size. A 404 line names the path that was asked for.

**Nothing served at all.** Look for `SPLASHSCHEME registered splash://` and
`handler installed on slot 1`. Missing registration means the scheme could not
be registered before the web engine started.

**`not permitted: <tool>`.** The surface was not granted that tool. See
[capabilities.md](capabilities.md).

**A call never resolves.** Every tool answers or times out at 45 s; a plugin
that drops its `Responder` rejects with `the tool did not answer`.

**Startup checks.** Search hilog for `selftest` — path handling, the registry's
rules and the capability rules all verify themselves at startup and log the
result.
