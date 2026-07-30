# 发布

*[English version](releasing.md)*

## 现状

今天你可以构建一个应用并装到自己的设备上。要把它交给别人，需要这个仓库没有的
签名材料，而且相关的接线**还没做完**。这一页把界线说清楚。

| | |
|---|---|
| 构建并装到自己的手机上 | 可以，本文所有检查都是这么跑的 |
| 用你自己的名称、id、版本、图标 | 可以——`splash-oh apply` |
| 发布签名 | **没接通**：零件都在，但没连起来 |
| 上架应用市场 | 需要 AGC 账号和一份发布描述文件 |

## 为什么签名不是可选项

商用鸿蒙设备在安装时会拒绝社区签名链：

```
code:9568257 fail to verify pkcs7 file
```

这个仓库里没有任何东西能改变它。这是华为的政策，不是一个构建选项。除了你自己的
开发设备之外，要把应用装到任何一台商用手机上，AGC 账号都是前提。

## 已经有的东西

**`sign-hap.sh`**（在 SDK 工具里，不在本仓库）已经可以用 AGC 材料无 IDE 签名：

```sh
OHOS_SIGN_P12       keystore .p12
OHOS_SIGN_P12_PWD   keystore 密码
OHOS_SIGN_ALIAS     key alias
OHOS_SIGN_KEY_PWD   key 密码
OHOS_SIGN_CERT      证书 .cer
OHOS_SIGN_PROFILE   描述文件 .p7b
```

它之所以存在，是因为 hvigor 自带的 `SignHap` 任务在无 IDE 环境下不可用：它要求
`build-profile.json5` 里的密码字段是 DevEco 加密过的串，而那种串只有 IDE 能生成。

**`splash.toml`** 里有一个 `[signing]` 段，指明那三个文件和 alias。密码从环境
变量 `SPLASH_SIGN_PWD` 读，绝不写进文件。

**`splash-oh apply`** 会在写入任何东西之前，用你的 bundle id 校验描述文件：

```
splash-oh: the provisioning profile is issued for "com.example.myapplication",
           but splash.toml says "com.futurewei.weatherdeck".
```

一份描述文件只对应一个 bundle id。没有这道检查，不一致会在安装时表现为一个两个
id 都不提的数字码。

**缺的那一步**，是把 `[signing]` 变成上面那些环境变量并调用签名器。它不大。它
没写，是因为**从没跑过的签名不算能用的签名**，而要测它，需要只有账号持有者手里
才有的发布材料。

## 怎么走到那一步

1. **一个 AGC 账号**，用你真实的 bundle id 注册应用。这一步周期最长、别的都不
   依赖它——从这里开始。

2. **对应那个 bundle id 的发布描述文件和证书。** 这台机器上现在的那份是
   `"type":"debug"`，绑定在 `com.example.myapplication` 上。

3. **把 `[app] bundle-id` 改成一致**，并让 `[signing] profile` 指向新的 `.p7b`。
   跑 `splash-oh apply`——不一致的话它会立刻告诉你。

4. **把 `[signing]` 接到 `sign-hap.sh`。** 剩下的代码，最好是拿到真实材料后一次
   写完并测完。

5. **一次发布构建。** 这里的一切都是 debug 构建。开了 LTO、`panic = "abort"`
   并带真实签名的发布构建还没跑过，而这种事往往会给人惊喜。

## 发布之前

**图标。** `[app] icon` 由 `apply` 复制，缺失时会警告——默认就是缺失的。

**看一眼应用声明了什么。** `[permissions] declare` 是应用的全集；页面永远无法
申请集合之外的东西。精简它是性价比最高的安全工作。

**看一眼每个 surface 能做什么。** 见 [capabilities.zh-CN.md](capabilities.zh-CN.md)。
用 `Caps::all()` 建的 surface 什么都能做，而那是生成式卡片的默认值。

**签名凭据不能提交进仓库。** `deveco/build-profile.json5` 里带着 DevEco 加密过的
密码，而这个仓库是公开的。`.p12` 本身没有提交，但那些加密密码提交了，而且已经在
git 历史里。轮换那把 debug key 并重写这段历史，是尚未了结的事。
