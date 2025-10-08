//! State-Action-Reaction representation of a round of game.
//!
//! This module defines the data models for representing a complete round of game:
//!
//! - Starting conditions for the round: [`RoundBegin`]
//! - Game state at the start of each turn: [`State`] and [`StateCore`]
//! - Player's actions: [`Action`], [`Reaction`]
//! - Result of the turn: [`ActionResult`]
//! - End conditions for the round: [`RoundEnd`] (including [`AgariResult`])
//!
//! These building blocks can be aggregated to represent the full [`history`] of a round, with or
//! without intermediate [`State`]s.
//!
//! See [crate-level docs](crate) for a detailed description of the modeling.
//!

mod action;
mod action_result;
mod agari;
mod boundary;
mod discard;
pub mod history;
pub mod reaction;
mod state;

pub use self::{
    action::*,
    action_result::*,
    agari::*,
    boundary::*,
    discard::*,
    history::*,
    reaction::*,
    state::*,
};
