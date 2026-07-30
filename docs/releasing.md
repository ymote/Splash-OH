# Releasing

*[中文版](releasing.zh-CN.md)*

## Status

You can build an app and install it on your own device today. Handing it to
anyone else needs signing material this repository does not have, and the wiring
for it is **not finished**. This page says exactly where the line is.

| | |
|---|---|
| build and install on your own phone | works, this is what every check here runs on |
| your own name, id, version, icon | works — `splash-oh apply` |
| release signing | **not wired**: the pieces exist, nothing connects them |
| AppGallery distribution | needs an AGC account and a release profile |

## Why signing is not optional

A commercial HarmonyOS device rejects the community signing chain at install:

```
code:9568257 fail to verify pkcs7 file
```

Nothing in this repository changes that. It is Huawei policy, not a build
setting. An AppGallery Connect account is a prerequisite for putting an app on
any commercial phone but your own development device.

## What exists

**`sign-hap.sh`** (in the SDK tooling, not this repo) already signs headlessly
with AGC material:

```sh
OHOS_SIGN_P12       keystore .p12
OHOS_SIGN_P12_PWD   keystore password
OHOS_SIGN_ALIAS     key alias
OHOS_SIGN_KEY_PWD   key password
OHOS_SIGN_CERT      certificate .cer
OHOS_SIGN_PROFILE   provisioning profile .p7b
```

It exists because hvigor's own `SignHap` task is unusable headlessly: it
requires the password fields in `build-profile.json5` to be DevEco-encrypted
blobs, and only the IDE can produce those.

**`splash.toml`** has a `[signing]` section naming the three files and the
alias. The password is read from `SPLASH_SIGN_PWD` in the environment, never
from the file.

**`splash-oh apply`** validates the profile against your bundle id before
anything is written:

```
splash-oh: the provisioning profile is issued for "com.example.myapplication",
           but splash.toml says "com.futurewei.weatherdeck".
```

A profile is issued for exactly one bundle id. Without this check a mismatch
surfaces at install as a numeric code naming neither id.

**What is missing** is the step that turns `[signing]` into those environment
variables and calls the signer. It is small. It is not written, because signing
that has never been run is not signing that works, and testing it needs release
material that only the account holder has.

## Getting there

1. **An AGC account**, with the app registered under your real bundle id. Longest
   lead time and nothing else depends on it — start here.

2. **A release profile and certificate** for that bundle id. What is on this
   machine today is `"type":"debug"`, bound to `com.example.myapplication`.

3. **Set `[app] bundle-id` to match**, and point `[signing] profile` at the new
   `.p7b`. Run `splash-oh apply` — it will tell you at once if they disagree.

4. **Wire `[signing]` to `sign-hap.sh`.** The remaining code, best written and
   tested against real material in one sitting.

5. **A release build.** Everything here has been debug-built. A release build
   with LTO, `panic = "abort"` and real signing has not been run, and is the
   kind of thing that surprises people.

## Before you ship

**An icon.** `[app] icon` is copied by `apply` and warns when it is missing,
which by default it is.

**Check what your app declares.** `[permissions] declare` is the app's whole
set; a page can never request outside it. Trimming it is the cheapest security
work available.

**Check what each surface may do.** See [capabilities.md](capabilities.md). A
surface built with `Caps::all()` has everything, and that is the default for
generated cards.

**Signing credentials must not be committed.** `deveco/build-profile.json5`
carries DevEco-encrypted passwords, and this repository is public. The `.p12`
itself is not committed, but the encrypted passwords are, and they are in git
history. Rotating the debug key and rewriting that history is unfinished
business.
