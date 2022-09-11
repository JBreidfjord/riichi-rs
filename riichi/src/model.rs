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
//! - From the post-reaction state, we can automatically determine either the next pre-action state,
//!   or the end of the round.
//!
//! This design has the desirable property of only one state per turn, making the "round history"
//! a simple repeated structure of {[`State`], [`Action`], [`Reaction`]}.
//!

mod action;
mod action_result;
mod agari;
mod boundary;
mod reaction;
mod state;
mod yaku;

use std::fmt::{Display, Formatter};

use itertools::Itertools;

use crate::common::*;

pub use self::{
    action::*,
    action_result::*,
    agari::*,
    boundary::*,
    reaction::*,
    state::*,
    yaku::*,
};

/// A discarded tile.
/// This has (mostly) the same representation in both [`Action`] and [`State`].
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Discard {
    /// The discarded tile.
    pub tile: Tile,

    /// If called by another player, that player; otherwise the player who discarded this tile.
    /// Since this is unknown at the time the action is made, it is ignored in [`Action::Discard`].
    pub called_by: Player,

    /// Whether this tile was discarded as a part of declaring Riichi (立直, リーチ).
    pub declares_riichi: bool,

    /// Whether this tile was discarded immediately after being drawn (ツモ切り).
    pub is_tsumokiri: bool,
}

impl Display for Discard {
    // NOTE: we won't be showing `called_by` here; most of the time it's redundant
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.declares_riichi {
            write!(f, "RIICHI!({}{})",
                   self.tile,
                   if self.is_tsumokiri { "*" } else { " " })
        } else {
            write!(f, "discard({}{})",
                   self.tile,
                   if self.is_tsumokiri { "*" } else { " " })
        }
    }
}

/// Bundle of a turn's action and any reaction.
#[derive(Copy, Clone, Debug)]
pub struct ActionReaction {
    pub actor: Player,
    pub action: Action,
    pub reactor_reaction: Option<(Player, Reaction)>,
}

impl Display for ActionReaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}:{}", self.actor, self.action)?;
        if let Some((reactor, reaction)) = self.reactor_reaction {
            write!(f, " => P{}:{}", reactor, reaction)?;
        }
        Ok(())
    }
}

/// Bundle of a turn's action, any reaction, and results.
#[derive(Clone, Debug)]
pub struct GameStep {
    pub actor: Player,
    pub action: Action,
    pub reactor_reaction: Option<(Player, Reaction)>,
    pub action_result: ActionResult,
    pub next: Option<StateCore>,
}

impl State {
    // Reason this is in `model` instead of `engine`: This defines the relationship between
    // `State` and `StateCore` fields and is a rather straightforward transformation based on the
    // assumptions of each state variable.
    pub fn apply_step(&mut self, game_step: &GameStep) {
        if let Some(next) = game_step.next {
            let actor = game_step.actor;
            let actor_i = actor.to_usize();
            let next_actor = next.actor;
            let next_actor_i = next_actor.to_usize();
            assert_eq!(self.core.actor, actor);

            // Merge the self draw of this turn into the closed hand (it had been kept separate).
            if let Some(draw) = self.core.draw {
                self.closed_hands[actor_i][draw] += 1;

                log::debug!("P{}:Draw({}) => hand={}",
                    actor_i, draw, self.closed_hands[actor_i]);
            }

            // Move the discard of this turn from the closed hand to the discard section.
            match game_step.action {
                Action::Discard(mut discard) => {
                    discard.called_by = actor;
                    self.closed_hands[actor_i][discard.tile] -= 1;

                    log::debug!("P{}:{} => hand={}", actor_i, discard, self.closed_hands[actor_i]);

                    if let Some((reactor, reaction)) = game_step.reactor_reaction {
                        assert_eq!(reactor, next_actor);
                        assert_ne!(reaction, Reaction::RonAgari);
                        discard.called_by = reactor;
                    }
                    self.discards[actor_i].push(discard);
                    self.discard_sets[actor_i].set(discard.tile);
                }
                Action::Kakan(_) | Action::Ankan(_) => {}  // No-op, but valid.
                _ => panic!("inconsistent")
            }

            // Move the meld of this turn from the closed hand to the meld section.
            // Same handling for both Kakan/Ankan (action) and Chii/Pon/Daiminkan (reaction).
            if let Some(meld) = next.incoming_meld {
                meld.consume_from_hand(&mut self.closed_hands[next_actor_i]);

                log::debug!("P{}:{} => hand={}", next_actor_i, meld, self.closed_hands[next_actor_i]);

                if let Meld::Kakan(kakan) = meld {
                    // Kakan will replace the existing Pon.
                    let (pon_i, _) = self.melds[next_actor_i].iter()
                        .find_position(|&&meld| meld == Meld::Pon(kakan.pon))
                        .unwrap();
                    self.melds[next_actor_i][pon_i] = meld;
                } else {
                    // Chii/Pon/Daiminkan/Ankan all introduce a new meld.
                    self.melds[next_actor_i].push(meld);
                }
            }

            // finally replace core wholesale
            self.core = next;
        }
    }

    pub fn apply_steps<'a>(&mut self, game_step: impl IntoIterator<Item=&'a GameStep>) {
        for step in game_step {
            self.apply_step(step);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn round_id_computes_correct_self_wind() {
        let round_id = RoundId { kyoku: 6, honba: 0 };
        assert_eq!(round_id.self_wind_for_player(P2), Wind::new(0));
        assert_eq!(round_id.self_wind_for_player(P3), Wind::new(1));
        assert_eq!(round_id.self_wind_for_player(P0), Wind::new(2));
        assert_eq!(round_id.self_wind_for_player(P1), Wind::new(3));
    }
}
