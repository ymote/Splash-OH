# Plugins

A plugin adds native tools a page can call by name. It depends on
`splash-oh-core` and nothing else — not on the bridge, not on napi, not on
ArkTS.

```rust
use splash_oh_core::{Args, Registry, Responder};

#[derive(serde::Deserialize)]
struct Greet { name: String }

pub fn register(r: &mut Registry) {
    r.add("app.greet", "Say hello", |args: &Args, resp: Responder| {
        match args.parse::<Greet>() {
            Ok(g) => resp.ok(serde_json::to_string(&format!("hello, {}", g.name)).unwrap_or_default()),
            Err(e) => resp.err(e),
        }
    });
}
```

```js
await splash.invoke('app.greet', { name: 'world' })   // "hello, world"
```

## Arguments and results

Arguments are always JSON. The shim stringifies whatever the page passes, so a
tool can deserialize into a type rather than guess at a shape:

```rust
args.parse::<T>()   // Result<T, String>, naming the failure
args.text()         // a single string argument
args.raw()          // the JSON, for the few tools that want it
```

A result is **JSON too**, which catches people out: returning a bare string
means returning it quoted.

```rust
resp.ok("42")                                    // the number 42
resp.ok("\"hello\"")                             // the string "hello"
resp.ok(serde_json::to_string(&value).unwrap())  // usually this
resp.err("no such device")                       // the promise rejects
```

## Answering later

A tool that has to wait moves its `Responder` somewhere else and answers when
the answer arrives. This is what lets a plugin do a network call, a database
read, or anything that parks.

```rust
r.add("app.slow", "Fetch something", |args: &Args, resp: Responder| {
    let url = args.text();
    std::thread::spawn(move || {
        let body = fetch(&url);          // takes as long as it takes
        resp.ok(serde_json::to_string(&body).unwrap_or_default());
    });
});
```

`dispatch` returns as soon as the tool returns, which says nothing about whether
it has answered — that is the point.

**A Responder must be answered.** The page is holding a promise. Dropping one
without answering used to be a page that waits forever; now `Drop` answers with
`the tool did not answer`, which is a bad outcome you can see rather than a hang
you cannot. A tool that takes more than 45 s is timed out regardless.

## Registering

Registration is an explicit call at startup, not a link-time trick:

```rust
// crates/splash-oh/src/lib.rs, in mount()
splash_oh_core::with_registry_mut(|r| {
    splash_oh_plugin_demo::register(r);
    my_app_plugin::register(r);          // yours
});
```

`linkme`-style distributed slices would remove the line, and they depend on
section behaviour that is not proven on this target. A registration that
silently failed to be collected would present as a tool that simply is not
there. This is duller and cannot half-work.

**Duplicate names are refused, not overwritten.** Two plugins claiming one name
is a build mistake, and letting the later one win would resolve it silently and
by link order. The first registration keeps the name.

`plugin.list` returns everything registered, so a page can see what a build
actually contains rather than what the documentation claims.

## Wiring your own crate in

Two edits in the Splash-OH checkout, because the `.so` is built there and a
`cdylib` is a final artifact — only the crate producing it can pull a plugin
into the binary:

1. `crates/splash-oh/Cargo.toml`
   ```toml
   my-app-plugin = { path = "../../my-app/plugin" }
   ```
2. `crates/splash-oh/src/lib.rs`, beside the existing plugin in `mount()`
   ```rust
   my_app_plugin::register(r);
   ```

That the shell is a checkout rather than a dependency your project owns is the
part of this still to be automated.

## What a plugin cannot do yet

Tools that need ArkTS — the file picker, the clipboard, runtime permission
prompts, BLE scanning — remain built in. Those park a call and are answered from
the ArkTS side, and that path is not exposed to plugins.

Everything else is available: a plugin can spawn threads, open sockets, call
into any NDK library it links, and take as long as it likes.

## Capabilities

Registering a tool does not grant it. A surface calls only what it was granted,
so a new tool needs adding to the relevant `Caps` — see
[capabilities.md](capabilities.md). Forgetting shows up as:

```
not permitted: app.greet
```

with a matching `bridge: slot 1 may not call app.greet` in hilog.
