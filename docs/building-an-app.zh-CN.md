# 做一个应用

*[English version](building-an-app.md)*

## 你会拿到什么

```sh
splash-oh new my-app
```

```
my-app/
  splash.toml        名称、bundle id、版本、图标、权限
  index.html         必须加载 /__splash.js，见下
  src/main.js        你的前端
  src/style.css
  public/__splash.js 桥接 shim，自动生成
  plugin/            你自己的原生代码，Rust
  build.sh           依托一份 Splash-OH 源码来构建
  README.md
```

用什么打包工具都行。模板用 Vite 只是因为总得用一个；这里没有任何东西关心
`dist/` 是谁生成的。

## 唯一的约定

前端要调原生代码，必须先加载 shim：

```html
<script src="/__splash.js"></script>
```

没有它就没有 `window.splash`。发布构建里这个 URL 由应用自己响应；开发服务器不
知道有这么个东西，所以 `splash-oh new` 会在 `public/` 里写一份，`splash-oh shim`
负责更新它。

它是一个被"提供"出来的文件，而不是在你的 HTML 经过时被注入进去的东西。改写别人
的标记是那种一旦出问题就极难排查的便利。

## 构建

```sh
npm run build
./build.sh
```

`build.sh` 会去找一份 Splash-OH 源码——`SPLASH_OH`，或者同级目录——然后带上指向
你 `dist/` 的 `SPLASH_FRONTEND_DIR` 交给它去构建。那边负责编译 Rust、放置
`.so`、跑 hvigor、安装并启动。

`dist/` 下的所有东西都会被打进二进制：嵌套目录、带内容哈希的文件名、任意类型。
`build.rs` 会遍历目录并生成资源表，所以 `index-a3f2c9.js` 每次构建都在变也没有
任何成本。

通过自定义 scheme 提供，因此相对路径的解析和网页里的预期一致：

```
splash://app/index.html
splash://app/assets/index-CJUgn_EW.js
```

## 开发回路

```sh
splash-oh dev                                       # 每次开工执行一次
npm run dev                                         # 终端 1
SPLASH_DEV_SERVER=http://127.0.0.1:5173 ./build.sh  # 终端 2
```

此后应用从打包工具那里加载前端，而不是从内嵌的资源，所以改动会直接在设备上刷新，
不用重新构建。改 Rust 仍然要。

`splash-oh dev` 用 `hdc rport` 打通一条 USB 隧道，把手机的 `127.0.0.1:5173` 映射
到你机器的同一端口。**走 USB 而不是 Wi-Fi 是刻意的。** 让手机去连宿主机的局域网
地址是最直觉的做法，在这台设备上它失败了：两端通过 USB 链路处在同一个
`192.168.125.x` 网段，请求依然返回 `ERR_ADDRESS_UNREACHABLE`。隧道绕开了这个
问题——没有网络要配，没有防火墙要开。

把文件拷到手机上不是替代方案。`hdc file send` 的路径在应用的 mount namespace
之外解析，文件永远到不了应用能读到的地方。所以才需要一个服务器。

隧道在重新插拔或手机重启后就没了。页面加载不出来时重跑一次 `splash-oh dev`。

`SPLASH_DEV_SERVER` 在构建时读取，不是运行时。一个能在已发布应用里被打开的调试
便利，等于给别人的服务器留了一条成为你前端的路。

开发构建里仍然内嵌了一份资源包，所以指向一个没在跑的服务器时，是页面加载失败，
而不是一个完全没有前端的应用。

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

会把这些全部写进外壳：bundle id、版本名与版本号、两个 label、图标，以及声明的
权限列表。

**是两个 label，不是一个**，而且出现在不同地方。`app_name` 是设置和应用列表里
显示的；`EntryAbility_label` 是桌面图标下面那行字。只设一个的话，图标下面会写着
`label`——看起来像你的应用有 bug，而不像一个没填的字段。

如果 `[signing]` 指定了描述文件，`apply` 会**在写入任何东西之前**检查它的
bundle id 和你的是否一致：

```
splash-oh: the provisioning profile is issued for "com.example.myapplication",
           but splash.toml says "com.futurewei.weatherdeck".
```

一份描述文件只对应一个 bundle id，不一致的话否则要到安装时才失败，而且报的是一个
两个 id 都不提的数字码。

密码永远不写进这个文件。签名从环境变量 `SPLASH_SIGN_PWD` 读。

## 加原生代码

见 [plugins.zh-CN.md](plugins.zh-CN.md)。一句话版本：在 `plugin/src/lib.rs` 里
写一个工具，从 JS 里按名字调用。

```rust
r.add("app.greet", "Say hello", |args: &Args, resp: Responder| {
    let g: Greet = match args.parse() { Ok(g) => g, Err(e) => return resp.err(e) };
    resp.ok(serde_json::to_string(&format!("hello, {}", g.name)).unwrap_or_default())
});
```

```js
await splash.invoke('app.greet', { name: 'world' })
```

把它链接进去目前需要在 Splash-OH 那份源码里改两处，因为 `.so` 是在那边构建的，
只有产出它的那个 crate 才能把插件拉进二进制。在你动手之前，模板里的例子会显示
`not linked yet — see README`。这部分是还没自动化的。

## 排查

**白屏。** 在 hilog 里搜 `SPLASHASSET`：每个提供出去的文件都会带状态码和大小
打一行。404 那行会写出被请求的路径。

**什么都没提供。** 找 `SPLASHSCHEME registered splash://` 和
`handler installed on slot 1`。没有注册那行，说明 scheme 没能在 web 引擎启动前
注册上。

**`not permitted: <tool>`。** 这个 surface 没有被授予那个工具。见
[capabilities.zh-CN.md](capabilities.zh-CN.md)。

**调用一直不返回。** 每个工具要么应答、要么 45 秒超时；插件如果把 `Responder`
丢掉了，会以 `the tool did not answer` 拒绝。

**启动自检。** 在 hilog 里搜 `selftest`——路径处理、注册表规则、能力规则都会在
启动时自检并打印结果。注意 hilog 大约一分钟就会被 Chromium 的日志灌满并把这几行
挤掉，启动半分钟后再去搜通常什么都没有。要看就立刻看。
