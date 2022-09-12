//! State-Action-Reaction representation of a round of game.
//!
//! This module defines the data models, their relationships, and helpers of a structured
//! representation of a round of game.
//!
//! ## The original state machine diagram
//!
//! ```asciiart
//!    ┌──────┐
//!    │ Deal │
//!    └─┬────┘
//!      │
//!      │    ┌────────────────────────────────────────────────────────────────┐
//!      │    │                                                                │
//!      ▼    ▼             #1                                   #2            │
//!    ┌────────┐ Draw=Y    ┌────────────┐           ┌─────────────┐ Nothing   │
//!    │DrawHead├──────────►│            │           │             ├───────────┤
//!    └────────┘ Meld=N    │            │  Discard  │             │           │
//!    #4                   │            ├──────────►│             │  #3       ▼
//!                         │            │  Riichi   │             │  ┌─────────────────┐
//!                         │  In-turn   │           │ Resolved    │  │ Forced abortion │
//!                         │  player's  │           │ declaration │  └─────────────────┘
//!    ┌────────┐ Draw=Y    │  decision  │           │ from        │           ▲
//! ┌─►│DrawTail├──────────►│            │           │ out-of-turn │           │
//! │  └────────┘ Meld=Y    │  (Action)  │           │ players     │ Daiminkan │
//! │  #4                   │            │           │             ├───────────┤
//! │                       │            │           │ (Reaction)  │           │
//! │                       │            │  Kakan    │             │           │
//! │  ┌────────┐ Draw=N    │            ├──────────►│             │ Chii      │
//! │  │Chii/Pon├──────────►│            │  Ankan    │             ├─────────┐ │
//! │  └────┬───┘ Meld=Y    └──┬───────┬─┘           └──────┬──────┘ Pon     │ │
//! │  #4   │                  │       │                    │                │ │
//! │       │         NineKinds│       │Tsumo               │Ron             │ │
//! │       │                  ▼       ▼                    ▼                │ │
//! │       │         ┌──────────┐   ┌─────┐             ┌─────┐             │ │
//! │       │         │ Abortion │   │ Win │             │ Win │             │ │
//! │       │         └──────────┘   └─────┘             └─────┘             │ │
//! │       │                                                                │ │
//! │       └────────────────────────────────────────────────────────────────┘ │
//! │                                                                          │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! There are multiple states within one logical turn of a round of game.
//!
//! 1. The player in turn is ready to make an action, after incoming draw and/or meld.
//!    This action might be terminal (abortion by nine kinds, or win by self draw).
//!
//! 2. Each other player may independently declare an reaction: Chii, Pon, Daiminkan, or Ron.
//!    The resolved reaction type determines the next state.
//!
//! 3. After reaction resolution, we need to check for any involuntary round-ending conditions.
//!
//! 4. All done, then the next player gains draw and/or meld depending on what has happened so far,
//!    marking the beginning of the next turn.
//!
//! Not all actions are valid at all times; the validity often depends on state variables not
//! illustrated in the state machine diagram.
//!
//!
//! ## The cyclical state
//!
//! <!-- TODO: explain why we decided to model this way -->
//!
//! We would only encode the state of a round of game at the point before the player in turn takes
//! their action. This is referred to as the pre-action state, or simply [`State`].
//!
//! Other states can be derived from this definition:
//!
//! - The post-action state is simply the concatenation of the pre-action state and the action.
//! - The state after any resolved reaction is likewise the concatenation of the pre-action state,
//!   the action, and the resolved reaction.
//! - From the post-reaction state, the engine determines either the next pre-action state,
//!   or the end of the round.
//!
//! This design has the desirable property of only one state per turn, making the "round history"
//! a simple repeated structure of {[`State`], [`Action`], [`Reaction`]}.
//!

mod action;
mod action_result;
mod agari;
mod boundary;
mod discard;
mod history;
mod reaction;
mod state;
mod yaku;
mod yaku_utils;

pub use self::{
    action::*,
    action_result::*,
    agari::*,
    boundary::*,
    discard::*,
    history::*,
    reaction::*,
    state::*,
    yaku::*,
    yaku_utils::*,
};
