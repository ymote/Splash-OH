# Capabilities

*[中文版](capabilities.zh-CN.md)*

What a page may do, and why each check exists. Every rule here was verified on
device in both directions — a rule proven only to refuse might refuse
everything.

## The layers

```
1. is this surface the app's own page?      Source::Html | Source::App
2. is its document still on that origin?    observed origin == expected
3. may it call this tool?                   Caps::allows_tool
4. may it use this path / reach this host?  Caps::allows_path / allows_host
5. may it ask the user for this permission? PAGE_REQUESTABLE_PERMISSIONS
```

All five are in Rust. ArkTS also declines to attach the bridge to an untrusted
slot, but that check sits next to the untrusted content and is therefore the
wrong place to depend on.

## 1. Which surfaces get a bridge

`Source::Html` (markup the app generated) and `Source::App` (a page from the
shipped bundle) get one. `Source::Url` — someone else's page — never does. The
browser card loads Wikipedia into a slot, and it holds no `splash_native`.

`Source::App` is deliberately a separate kind from `Source::Url` even though it
navigates to a URL, because the trust answer is the opposite. Folding them
together would make trust a question about string prefixes.

## 2. Origin, not just slot

Trust is recorded per slot; `javaScriptProxy` attaches to the *component*. So a
trusted page that navigated itself elsewhere kept the bridge, and Rust — reading
only the slot's source — agreed.

That was not theoretical. A document served by `https://example.com` called the
`log` tool and Rust wrote its text to hilog:

```
NAVPROBE gate1  "https://example.com proxy=object"
NAVPROBE gate2  "invoked"
page: SPOKE FROM https://example.com
```

Two layers now, each tested with the other disabled:

- **ArkTS refuses the navigation.** `onLoadIntercept` cancels any load off the
  slot's expected origin. The foreign document never arrives.
- **Rust refuses to believe a slot whose document moved.** `onPageBegin` reports
  the real origin; `is_trusted` requires it to match. With the ArkTS guard
  fault-injected off, the proxy *is* injected and `invoke` does *not* throw, and
  the call still dies:

  ```
  webslot: slot 1 declared splash://app but its document is on
           https://example.com -- refusing to treat it as trusted
  bridge: refused log from untrusted slot 1
  ```

The observed-origin record is deliberately **not** cleared when slots are reset:
a rebuild re-declares slots without reloading their documents, so clearing it
would let a slot that had navigated away look untainted on the next rerender.

A dev build's expected origin follows `SPLASH_DEV_SERVER`, or the guard would
refuse the very page the build exists to load.

## 3–4. Capability sets

Declared where the slot is declared — by the app, in Rust, next to the geometry —
so a page cannot ask for more than it was given.

```rust
let caps = Caps::none()
    .tools(&["device.*", "log", "http.get", "fs.list", "app.*"])
    .fs_scope(&["/data/storage/el2/base/haps/entry/files"])
    .http_hosts(&["api.open-meteo.com"]);

declare_app_with("/index.html", caps, x, y, w, h);
```

| rule | gates |
|---|---|
| `tools` | which tools, by exact name or `"prefix.*"` |
| `fs` | which directories a path argument may be under |
| `http` | which hosts `http.get` may reach |

Names gate *whether*; the other two gate *what with*. Both are needed —
`fs.read` allowed everywhere is barely narrower than fully trusted, and that is
the distinction one bit of trust could not express.

`"device.*"` matches `device.info` but not `devicefoo.info`: the separator is
part of the prefix.

Paths are **canonicalised before** the prefix test, so `files/../../etc/passwd`
is resolved and then refused rather than passing a string comparison. Not
`std::fs::canonicalize`, which requires the path to exist — a scope has to be
able to refuse a path that does not.

Hosts match exactly or as a subdomain. `notapi.example.com.evil.com` does not
match `api.example.com`.

`Caps::all()` still exists and is what generated cards get — the old "trusted"
under another name. The demo apps were written against it, and quietly narrowing
them would break working code to make a point. New surfaces state what they
need.

## 5. Permissions

`permission.request` used to forward whatever names a page passed straight to
`requestPermissionsFromUser`, so any trusted page could raise a camera or
microphone dialog at any moment.

Requests are now checked against the five user_grant permissions a page may ask
for, capped at four per request. The other declared permissions (`INTERNET`,
`VIBRATE`, `GET_NETWORK_INFO`, `GET_WIFI_INFO`, `ACCELEROMETER`, `GYROSCOPE`)
are granted at install, so a runtime prompt for them means nothing; asking is a
mistake and is refused as one.

A bad name fails the **whole call** rather than being filtered out. Dropping one
silently would leave the page believing it had asked and the user believing they
had answered.

The app's declared set comes from `splash.toml`; a page can never request
outside it.

## Two bugs worth knowing about

Both were in this code, and both were found only because a test expected a
refusal and did not get one.

**Scope checks that did nothing.** They read `SLOTS`, a `thread_local!` — but
`http.get`, `fs.read` and `fs.list` do their work on a spawned worker, where it
is empty. `caps_for` returned `None` and every scope check passed. The device
showed `http.get` reaching a host the page had not been granted. Capability sets
now live in a process-wide map. The tool-name gate had looked fine only because
it runs before the spawn.

**A check attached to the wrong tool.** There are two `check_https_public` call
sites and the edit landed on the first, so `fs` denials were real while `http`
denials were not.

The lesson both times: a security check that has never been observed refusing
something is a security check you have not tested.

## Verifying

Startup logs the rules verifying themselves:

```
caps selftest: ok (16 rules, traversal and prefix tricks refused)
assets selftest: ok (7 paths + origins, traversal refused, dev_server=None)
registry selftest: ok (added=true duplicate_refused=true other=true len=2 first_wins=true)
```

Refusals name what and why:

```
bridge: slot 1 may not call secure.random
bridge: slot 1 may not touch /data
bridge: slot 1 may not reach media.w3.org
```
