# My Splash App

A native OpenHarmony app: a web frontend, Rust for everything else.

## Build it

    npm install
    npm run build
    SPLASH_FRONTEND_DIR=$PWD/dist ./build.sh

## Develop it

    splash-oh dev        # opens a USB tunnel to the phone
    npm run dev
    SPLASH_DEV_SERVER=http://127.0.0.1:5173 ./build.sh

Frontend edits reload on the device. Rust edits still need a rebuild.

The tunnel is over USB rather than Wi-Fi, so there is no network to configure.

## Add a native capability

Add a tool in `plugin/src/lib.rs`, then call it by name from the frontend:

    window.splash.invoke('app.greet', { name: 'world' })

## Configure it

`splash.toml` holds the app's name, bundle id, version, icon, frontend
directory and declared permissions.
