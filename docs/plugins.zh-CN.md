# 插件

*[English version](plugins.md)*

插件给页面添加可以按名字调用的原生工具。它只依赖 `splash-oh-core`——不依赖桥接
层，不依赖 napi，不依赖 ArkTS。

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

## 参数与返回值

参数永远是 JSON。shim 会把页面传的任何东西字符串化，所以工具可以反序列化成一个
类型，而不是去猜结构：

```rust
args.parse::<T>()   // Result<T, String>，失败时说明原因
args.text()         // 单个字符串参数
args.raw()          // 原始 JSON，给少数确实需要的工具
```

返回值**也是 JSON**，这一点常有人栽跟头：返回一个裸字符串，意味着要带引号返回。

```rust
resp.ok("42")                                    // 数字 42
resp.ok("\"hello\"")                             // 字符串 "hello"
resp.ok(serde_json::to_string(&value).unwrap())  // 通常用这个
resp.err("no such device")                       // promise 被 reject
```

## 稍后应答

需要等待的工具，把 `Responder` 移交到别处，等答案到了再应答。正是这一点让插件能
做网络请求、读数据库，或者任何需要挂起的事。

```rust
r.add("app.slow", "Fetch something", |args: &Args, resp: Responder| {
    let url = args.text();
    std::thread::spawn(move || {
        let body = fetch(&url);          // 要多久就多久
        resp.ok(serde_json::to_string(&body).unwrap_or_default());
    });
});
```

`dispatch` 在工具返回时就返回了，这并不说明它已经应答——这正是重点。

**Responder 必须被应答。** 页面那头握着一个 promise。以前把它丢掉不应答，页面会
永远等下去；现在 `Drop` 会以 `the tool did not answer` 应答——一个你看得见的坏
结果，好过一个你看不见的卡死。超过 45 秒的调用无论如何都会超时。

## 注册

注册是启动时的一次显式调用，不是链接期的把戏：

```rust
// crates/splash-oh/src/lib.rs，在 mount() 里
splash_oh_core::with_registry_mut(|r| {
    splash_oh_plugin_demo::register(r);
    my_app_plugin::register(r);          // 你的
});
```

`linkme` 那类 distributed slice 可以省掉这一行，但它依赖的 section 行为在这个
目标平台上没有验证过。一次没被收集到的注册，表现出来就是"这个工具根本不存在"。
现在这种做法更笨，但不会只成功一半。

**重名会被拒绝，而不是覆盖。** 两个插件抢一个名字是构建期的错误，让后来者获胜
等于让链接顺序去悄悄决定它。先注册的保留名字。

`plugin.list` 返回所有已注册的名字，所以页面可以看到这次构建**实际**包含什么，
而不是文档声称包含什么。

## 把你自己的 crate 接进去

在 Splash-OH 那份源码里改两处，因为 `.so` 在那边构建，而 `cdylib` 是最终产物——
只有产出它的 crate 才能把插件拉进二进制：

1. `crates/splash-oh/Cargo.toml`
   ```toml
   my-app-plugin = { path = "../../my-app/plugin" }
   ```
2. `crates/splash-oh/src/lib.rs`，`mount()` 里已有插件的旁边
   ```rust
   my_app_plugin::register(r);
   ```

外壳是一份源码而不是你项目自己的依赖，这一点是还没自动化的。

## 插件目前做不到的事

需要 ArkTS 的工具——文件选择器、剪贴板、运行时权限弹窗、BLE 扫描——仍然是内置的。
它们会挂起调用、由 ArkTS 那侧来应答，这条路径还没有开放给插件。

其余都可以：插件能开线程、开 socket、调用它链接的任何 NDK 库，想跑多久跑多久。

## 能力

注册一个工具不等于授予它。surface 只能调用它被授予的东西，所以新工具需要加进
对应的 `Caps`——见 [capabilities.zh-CN.md](capabilities.zh-CN.md)。忘了的话会
看到：

```
not permitted: app.greet
```

以及 hilog 里对应的 `bridge: slot 1 may not call app.greet`。
