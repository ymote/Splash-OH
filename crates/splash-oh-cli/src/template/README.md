# My Splash App

A native OpenHarmony app: a web frontend, Rust for everything else.

## Build it

The app shell — the ArkTS entry point, the DevEco project, the Rust bridge —
lives in a Splash-OH checkout rather than in here. This project is the frontend
and your own native code. Clone one beside this directory, or set `SPLASH_OH`:

    git clone https://github.com/ymote/Splash-OH.git ../Splash-OH

    npm install
    npm run build
    ./build.sh

## Develop it

    splash-oh dev        # opens a USB tunnel to the phone
    npm run dev
    SPLASH_DEV_SERVER=http://127.0.0.1:5173 ./build.sh

Frontend edits reload on the device. Rust edits still need a rebuild.

The tunnel is over USB rather than Wi-Fi, so there is no network to configure.

## Add a native capability

Write a tool in `plugin/src/lib.rs` and call it by name from the frontend:

    window.splash.invoke('app.greet', { name: 'world' })

Two edits in the Splash-OH checkout link it in, because the `.so` is built
there and only the crate that produces it can pull a plugin into the binary:

  * add `my-splash-app-plugin = { path = "../my-app/plugin" }` to
    `crates/splash-oh/Cargo.toml`
  * add `my_splash_app_plugin::register(r);` beside the existing plugin in
    `mount()`, in `crates/splash-oh/src/lib.rs`

That is the part of the story still to be automated: today the shell is a
checkout you build against rather than a dependency your project owns.

## Configure it

`splash.toml` holds the app's name, bundle id, version, icon, frontend
directory and declared permissions.
