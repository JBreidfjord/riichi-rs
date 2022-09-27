#![doc = include_str!("../README.lib.md")]

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
