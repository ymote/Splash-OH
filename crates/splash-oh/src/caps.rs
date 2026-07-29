//! What a surface is allowed to do, rather than whether it is allowed anything.
//!
//! Trust used to be one bit. A slot was either the app's own page, and got all
//! 48 tools plus the keystore plus the filesystem, or it was somebody else's,
//! and got nothing. That is the right answer for a browser tab showing
//! Wikipedia and the wrong one for an app with more than one page in it: a
//! settings screen has no business reading arbitrary files, and a page that
//! embeds a third-party widget should not be a total loss.
//!
//! A capability set is declared where the slot is declared — by the app, in
//! Rust, next to the geometry — so the page cannot ask for more than it was
//! given and nothing it does can change the answer.
//!
//! # Three kinds of rule
//!
//! ```text
//! tools     which tools may be called, by name or by "prefix.*"
//! fs        which directories a path argument may be under
//! http      which hosts http.get may reach
//! ```
//!
//! Tool names gate *whether*; the other two gate *what with*. Both are needed:
//! `fs.read` allowed everywhere is barely narrower than trusted, and this is
//! the distinction the one-bit model could not express.

/// What one surface may do.
#[derive(Clone, Debug)]
pub struct Caps {
    /// Exact tool names, or a `"prefix.*"` wildcard.
    tools: Vec<String>,
    /// Directory prefixes a path argument must be under. Empty denies all
    /// path-taking tools even when their name is allowed.
    fs: Vec<String>,
    /// Hosts `http.get` may reach.
    http: Vec<String>,
}

impl Caps {
    /// Everything.
    ///
    /// What generated cards get, and it is the old one-bit "trusted" by another
    /// name. It stays because the four demo apps and every `.splash` card were
    /// written against it, and quietly narrowing them would break working code
    /// to make a point. New surfaces should state what they need instead.
    pub fn all() -> Self {
        Caps {
            tools: vec!["*".into()],
            fs: vec!["/".into()],
            http: vec!["*".into()],
        }
    }

    /// Nothing. The base to build from.
    pub fn none() -> Self {
        Caps {
            tools: Vec::new(),
            fs: Vec::new(),
            http: Vec::new(),
        }
    }

    pub fn tools(mut self, names: &[&str]) -> Self {
        self.tools.extend(names.iter().map(|s| s.to_string()));
        self
    }

    /// Directories a path argument may be under.
    pub fn fs_scope(mut self, dirs: &[&str]) -> Self {
        self.fs.extend(dirs.iter().map(|s| s.to_string()));
        self
    }

    pub fn http_hosts(mut self, hosts: &[&str]) -> Self {
        self.http.extend(hosts.iter().map(|s| s.to_string()));
        self
    }

    /// May this surface call `tool`?
    pub fn allows_tool(&self, tool: &str) -> bool {
        self.tools.iter().any(|rule| match rule.as_str() {
            "*" => true,
            r => match r.strip_suffix(".*") {
                // "device.*" covers device.info but not devicefoo.info: the
                // separator is part of the prefix, not an afterthought.
                Some(prefix) => tool
                    .strip_prefix(prefix)
                    .is_some_and(|r| r.starts_with('.')),
                None => r == tool,
            },
        })
    }

    /// May this surface touch `path`?
    ///
    /// Compared against a *canonicalised* path, so `files/../../secrets` is
    /// resolved before the prefix test rather than after it — a prefix check on
    /// a raw string is exactly the check `..` defeats.
    pub fn allows_path(&self, path: &str) -> bool {
        let Some(real) = canonical(path) else {
            return false;
        };
        self.fs.iter().any(|dir| {
            if dir == "/" {
                return true;
            }
            let dir = dir.trim_end_matches('/');
            // A prefix must end at a boundary: /data/app must not permit
            // /data/application.
            real == dir || real.starts_with(&format!("{dir}/"))
        })
    }

    pub fn allows_host(&self, host: &str) -> bool {
        self.http
            .iter()
            .any(|h| h == "*" || h == host || host.ends_with(&format!(".{h}")))
    }
}

/// Resolve `.` and `..` textually, and reject anything that escapes the root.
///
/// Not `std::fs::canonicalize`: that requires the path to exist, and a scope
/// check has to be able to refuse a path that does not — otherwise "may I write
/// here" is answerable only after the fact.
fn canonical(path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return None;
    }
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            s => out.push(s),
        }
    }
    Some(format!("/{}", out.join("/")))
}

/// Check the rules on device, and report the result.
///
/// Every one of these is a refusal that has to work the first time it matters,
/// and `cargo test` cannot run on this target. Called from `mount`.
pub fn self_test() -> String {
    let mut bad = Vec::new();
    let mut check = |name: &str, got: bool, want: bool| {
        if got != want {
            bad.push(name.to_string());
        }
    };

    let c = Caps::none()
        .tools(&["device.*", "http.get", "fs.read"])
        .fs_scope(&["/data/storage/el2/base/haps/entry/files"])
        .http_hosts(&["api.example.com"]);

    check("device.info allowed", c.allows_tool("device.info"), true);
    check(
        "device.* is a prefix",
        c.allows_tool("devicefoo.info"),
        false,
    );
    check("exact name allowed", c.allows_tool("fs.read"), true);
    check("undeclared refused", c.allows_tool("secure.sign"), false);
    check(
        "all() allows anything",
        Caps::all().allows_tool("secure.sign"),
        true,
    );
    check(
        "none() allows nothing",
        Caps::none().allows_tool("echo"),
        false,
    );

    let base = "/data/storage/el2/base/haps/entry/files";
    check("in scope", c.allows_path(&format!("{base}/a.txt")), true);
    check("scope root itself", c.allows_path(base), true);
    check(
        "outside scope",
        c.allows_path("/data/storage/el2/base/other"),
        false,
    );
    // The one that matters: a prefix test on the raw string would pass this.
    check(
        "traversal out of scope",
        c.allows_path(&format!("{base}/../../../etc/passwd")),
        false,
    );
    check(
        "sibling with shared prefix",
        c.allows_path(&format!("{base}x/a")),
        false,
    );
    check("relative path refused", c.allows_path("a.txt"), false);

    check("host allowed", c.allows_host("api.example.com"), true);
    check(
        "subdomain allowed",
        c.allows_host("eu.api.example.com"),
        true,
    );
    check("other host refused", c.allows_host("evil.com"), false);
    check(
        "suffix trick refused",
        c.allows_host("notapi.example.com.evil.com"),
        false,
    );

    if bad.is_empty() {
        "caps selftest: ok (16 rules, traversal and prefix tricks refused)".to_string()
    } else {
        format!("caps selftest: FAILED: {}", bad.join(", "))
    }
}
