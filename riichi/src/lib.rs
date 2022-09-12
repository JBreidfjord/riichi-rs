//! # Riichi Mahjong Game Engine
//!
//! This crate is a complete implementation of the standard Japanese Riichi Mahjong game engine.
//!
//! The main game engine is encapsulated as [`engine::Engine`], which drives the
//! [state-action-reaction representation](model) of each round of the game, which in turn consists
//! of [building blocks](common) like [Tile]s, [HandGroup]s, [Meld]s, etc.
//!
//! [Tile]: common::Tile
//! [HandGroup]: common::HandGroup
//! [Meld]: common::Meld
//!
//!
//! ## Optional features
//!
//! ### `serde`
//!
//! Default: enabled.
//!
//! Defines a JSON-centric serialization format for most of the [`common`] and [`model`] types.
//!
//! This simplifies interop with external programs (bots, client-server, analysis, data processing),
//! persistence of game states, etc..
//!
//! See each individual type's docs for the detailed format.
//!
//! ### `tenhou-log`
//!
//! Default: enabled.
//!
//! Defines an intermediate data model that serde from/to Tenhou's JSON-formatted logs, and
//! reconstruction of each round's preconditions, action-reactions, and end conditions into our own
//! [data model](model).
//!
//! See [`interop::tenhou_log`] mod-level docs for details.
//!
//!
//! ## References
//!
//! - <https://riichi.wiki/Japanese_mahjong>
//! - <https://en.wikipedia.org/wiki/Mahjong>
//! - <https://ja.wikipedia.org/wiki/%E9%BA%BB%E9%9B%80>

use once_cell::sync::Lazy;
use semver::Version;

pub mod analysis;
pub mod common;
pub mod engine;
pub mod model;
pub mod interop;
pub mod rules;

pub mod prelude {
    //! Convenient re-exports of commonly imported items.
    pub use super::{
        common::*,
        model::*,
        engine::Engine,
        rules::Ruleset,
    };
}

/// Version of this crate (as a string).
pub const VERSION_STR: &str = env!("CARGO_PKG_VERSION");

/// Version of this crate (parsed).
pub static VERSION: Lazy<Version> = Lazy::new(|| VERSION_STR.parse().unwrap());
