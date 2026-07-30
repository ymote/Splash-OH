# 能力

*[English version](capabilities.md)*

页面能做什么，以及每一道检查为什么存在。这里的每条规则都在真机上双向验证过——
一条只被证明"会拒绝"的规则，可能是在拒绝一切。

## 分层

```
1. 这个 surface 是应用自己的页面吗？    Source::Html | Source::App
2. 它的文档还在那个 origin 上吗？        observed origin == expected
3. 它可以调这个工具吗？                  Caps::allows_tool
4. 它可以用这个路径 / 访问这个主机吗？    Caps::allows_path / allows_host
5. 它可以向用户申请这个权限吗？          PAGE_REQUESTABLE_PERMISSIONS
```

五道全在 Rust 里。ArkTS 也会拒绝把桥接对象挂到不受信任的 slot 上，但那道检查
就挨着不受信任的内容，因此不是一个可以依赖的位置。

## 1. 哪些 surface 拿得到桥

`Source::Html`（应用自己生成的标记）和 `Source::App`（来自随包资源的页面）拿得
到。`Source::Url`——别人的页面——永远拿不到。浏览器卡片把维基百科加载进一个 slot，
它身上没有 `splash_native`。

`Source::App` 特意和 `Source::Url` 分成两种，尽管它也是导航到一个 URL，因为信任
的答案是相反的。把它们合并会让"是否可信"变成一个关于字符串前缀的问题。

## 2. 看 origin，不只看 slot

信任是按 slot 记录的；而 `javaScriptProxy` 是挂在**组件**上的。于是一个可信页面
把自己导航到别处之后，桥还在，而 Rust——只看 slot 的 source——也认了。

这不是假设。一个由 `https://example.com` 提供的文档调用了 `log` 工具，Rust 把它
的文本写进了 hilog：

```
NAVPROBE gate1  "https://example.com proxy=object"
NAVPROBE gate2  "invoked"
page: SPOKE FROM https://example.com
```

现在是两层，每一层都在关掉另一层的情况下测过：

- **ArkTS 拒绝这次导航。** `onLoadIntercept` 会取消任何偏离该 slot 预期 origin
  的加载。外来文档根本到不了。
- **Rust 拒绝相信一个文档已经跑掉的 slot。** `onPageBegin` 报告真实 origin，
  `is_trusted` 要求它匹配。把 ArkTS 那层故障注入关掉之后，代理**确实**被注入了、
  `invoke` **也没有**抛异常，调用依然死在这里：

  ```
  webslot: slot 1 declared splash://app but its document is on
           https://example.com -- refusing to treat it as trusted
  bridge: refused log from untrusted slot 1
  ```

已观测 origin 的记录**特意不**随 slot 重置而清空：一次重建会重新声明 slot 但不会
重新加载它们的文档，清空的话，一个已经跑掉的 slot 在下一次重绘时就又显得干净了。

开发构建的预期 origin 跟随 `SPLASH_DEV_SERVER`，否则这道守卫会拒绝掉这个构建
存在的意义所在的那个页面。

## 3–4. 能力集

在声明 slot 的地方声明——由应用、在 Rust 里、紧挨着几何位置——所以页面无法索取
超出它被给予的东西。

```rust
let caps = Caps::none()
    .tools(&["device.*", "log", "http.get", "fs.list", "app.*"])
    .fs_scope(&["/data/storage/el2/base/haps/entry/files"])
    .http_hosts(&["api.open-meteo.com"]);

declare_app_with("/index.html", caps, x, y, w, h);
```

| 规则 | 管什么 |
|---|---|
| `tools` | 哪些工具，按完整名字或 `"prefix.*"` |
| `fs` | 路径参数必须位于哪些目录之下 |
| `http` | `http.get` 可以访问哪些主机 |

名字管的是**能不能**；另外两个管的是**拿什么去做**。两者缺一不可——`fs.read`
如果哪里都能读，就比"完全可信"窄不了多少，而这正是一个布尔值表达不出来的区别。

`"device.*"` 匹配 `device.info` 但不匹配 `devicefoo.info`：分隔符是前缀的一部分。

路径在前缀比较**之前**先规范化，所以 `files/../../etc/passwd` 是先被解析、再被
拒绝，而不是通过一次字符串比较。没有用 `std::fs::canonicalize`，那个要求路径必须
存在——而一个作用域必须能够拒绝一个不存在的路径。

主机要精确匹配或匹配子域。`notapi.example.com.evil.com` 不匹配 `api.example.com`。

`Caps::all()` 仍然存在，是生成式卡片拿到的——就是过去那个"可信"换了个名字。示例
应用都是照着它写的，为了立个规矩去悄悄收紧它们，等于把能用的代码弄坏。新的
surface 应该声明自己需要什么。

## 5. 权限

`permission.request` 过去会把页面传来的任何名字直接转给
`requestPermissionsFromUser`，于是任何可信页面都能在任意时刻弹出相机或麦克风的
授权框。

现在会对照页面可申请的那五个 user_grant 权限做检查，且单次最多四个。其余已声明
的权限（`INTERNET`、`VIBRATE`、`GET_NETWORK_INFO`、`GET_WIFI_INFO`、
`ACCELEROMETER`、`GYROSCOPE`）是安装时授予的，运行时弹窗对它们没有意义；申请
它们是个错误，也就当成错误拒绝掉。

一个不合法的名字会让**整次调用**失败，而不是把它从列表里过滤掉。悄悄丢掉一个，
会让页面以为自己申请过了，也让用户以为自己回答过了。

应用声明的集合来自 `splash.toml`；页面永远无法申请这个集合之外的东西。

## 两个值得知道的 bug

两个都在这份代码里，两个都是因为"测试期望一次拒绝、却没等到"才被发现的。

**什么都没做的作用域检查。** 它们读的是 `SLOTS`，一个 `thread_local!`——但
`http.get`、`fs.read` 和 `fs.list` 都在派生出来的工作线程上干活，那里它是空的。
`caps_for` 返回 `None`，于是每一次作用域检查都放行。真机上看到 `http.get` 访问了
一个页面并未被授予的主机。能力集现在存在一个进程级的表里。工具名那道闸看起来
没问题，只是因为它跑在派生线程之前。

**一道挂在错工具上的检查。** `check_https_public` 有两处调用点，改动落在了第一
处，于是 `fs` 的拒绝是真的，而 `http` 的拒绝是假的。

两次的教训是同一个：一道从没被观测到真的拒绝过什么的安全检查，是一道你没测过的
安全检查。

## 怎么验证

启动时会打印这些规则的自检结果：

```
caps selftest: ok (16 rules, traversal and prefix tricks refused)
assets selftest: ok (7 paths + origins, traversal refused, dev_server=None)
registry selftest: ok (added=true duplicate_refused=true other=true len=2 first_wins=true)
```

hilog 大约一分钟就会被 Chromium 的日志灌满并把这几行挤掉，所以要在启动后立刻看。

拒绝时会写清楚拒了什么、为什么：

```
bridge: slot 1 may not call secure.random
bridge: slot 1 may not touch /data
bridge: slot 1 may not reach media.w3.org
```
