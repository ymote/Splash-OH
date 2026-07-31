//! Your app's own native code.
//!
//! Anything here is callable from the frontend by name. This crate depends only
//! on splash-oh-core — it cannot see the bridge, and does not need to.

use splash_oh_core::{Args, Registry};

#[derive(serde::Deserialize)]
struct Greet {
    name: String,
}

/// Called once at startup. Add a tool per capability your app needs.
pub fn register(r: &mut Registry) {
    r.add("app.greet", "Say hello, to prove the plugin is wired", |args: &Args| {
        let g: Greet = args.parse()?;
        // Return JSON. A bare string still has to be quoted.
        Ok(serde_json::to_string(&format!("hello, {}", g.name)).unwrap_or_default())
    });
}
