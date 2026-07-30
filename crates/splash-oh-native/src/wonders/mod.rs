//! Wonderous, rebuilt from the Flutter app with native ArkUI components.
//!
//! Not the benchmark port in `wonderous.rs` — that one approximates a makepad
//! translation for node-count comparison. This is the real app: the same eight
//! wonders, the same artwork, the same layout rules, read out of
//! `gskinnerTeam/flutter-wonderous-app` rather than eyeballed.
//!
//! No Flutter, no makepad, no ArkTS widgets. Every node here is created through
//! the ArkUI NDK from Rust.

pub mod data;
pub mod details;
pub mod editorial_data;
pub mod home;
pub mod illustration;
pub mod tabbar;
