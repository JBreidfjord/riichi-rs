//! State-Action-Reaction representation of the game.
//!
//! This module mainly provides data model definitions and some straightforward helpers.
//! Game logic belongs to [`crate::engine`].
//!
//! ## The original state machine diagram
//!
//! ```asciiart
//!      ┌───────┐                            ┌─────┐
//!      │ Deal  │                            │ END │
//!      │(Start)│                            └─────┘
//!      └─┬─────┘                               ▲
//!        │                            #3       │Yes
//!        │                            ┌────────┴─────────┐
//!        │    ┌───────────────────────┤ Forced abortion? │◄────────────────────┐
//!        │    │                       └──────────────────┘                     │
//!        ▼    ▼             #1                                   #2            │
//!      ┌────────┐ Draw=Y    ┌────────────┐           ┌─────────────┐ Nothing   │
//!      │DrawHead├──────────►│            │           │             ├───────────┘
//!      └────────┘ Meld=N    │            │  Discard  │             │
//!      #3                   │            ├──────────►│             │
//!                           │            │  Riichi   │             │
//!                           │  In-turn   │           │ Resolved    │
//!                           │  player's  │           │ declaration │
//!      ┌────────┐ Draw=Y    │  decision  │           │ from        │ Daiminkan
//!   ┌─►│DrawTail├──────────►│            │           │ out-of-turn ├───────────┐
//!   │  └────────┘ Meld=Y    │  (Action)  │           │ players     │           │
//!   │  #4                   │            │           │             │           │
//!   │                       │            │           │ (Reaction)  │           │
//!   │                       │            │  Kakan    │             │           │
//!   │  ┌────────┐ Draw=N    │            ├──────────►│             │ Chii      │
//!   │  │Chii/Pon├──────────►│            │  Ankan    │             ├─────────┐ │
//!   │  └────────┘ Meld=Y    └──────┬─────┘           └──────┬──────┘ Pon     │ │
//!   │  #4   ▲             NineKinds│Tsumo                   │Ron             │ │
//!   │       │                      ▼                        ▼                │ │
//!   │       │                   ┌─────┐                  ┌─────┐             │ │
//!   │       │                   │ END │                  │ END │             │ │
//!   │       │                   └─────┘                  └─────┘             │ │
//!   │       │                                                                │ │
//!   │       └────────────────────────────────────────────────────────────────┘ │
//!   │                                                                          │
//!   └──────────────────────────────────────────────────────────────────────────┘
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
//! 3. In case the resolved reaction type is no-op, additionally we need to check for involuntary
//!    end conditions of this round.
//!
//! 4. All done, then the next player gains draw and/or meld depending on what has happened so far,
//!    marking the beginning of the next turn.
//!
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
pub use action::*;
pub use action_result::*;
pub use agari::*;
pub use boundary::*;
pub use reaction::*;
pub use state::*;
pub use yaku::*;

use crate::common::*;

trait PartiallyObservable {
    fn observe_by(&self, player: Player) -> Self;
}

/// A discarded tile.
/// This has the same representation in [`Action`] and [`State`].
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Discard {
    /// The discarded tile.
    pub tile: Tile,
    /// If called by another player, that player; otherwise the player who discarded this tile.
    /// This is explicitly ignored when wrapped in [`Action::Discard`].
    pub called_by: Player,
    /// Whether this tile was discarded as a part of declaring riichi.
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

/// Bundle of a turn's action, any reaction, and results
#[derive(Clone, Debug)]
pub struct GameStep {
    pub actor: Player,
    pub action: Action,
    pub reactor_reaction: Option<(Player, Reaction)>,
    pub action_result: ActionResult,
    pub next: Option<StateCore>,
}

impl RoundBegin {
    /// Returns the initial state of a round, with all 4 players' initial hand dealt (13 x 4),
    /// and the button player's first self draw added.
    pub fn to_initial_state(&self) -> State {
        let wall = &self.wall;
        let button = self.round_id.button();
        State {
            core: StateCore {
                seq: 0,
                action_player: button,
                num_drawn_head: 53,  // 13 x 4 + 1
                num_drawn_tail: 0,
                num_dora_indicators: 1,
                draw: Some(self.wall[52]),
                incoming_meld: None,
                furiten: Default::default(),
                riichi: Default::default(),
            },
            closed_hands: wall::deal(wall, button),
            melds: Default::default(),
            discards: Default::default(),
            discard_sets: Default::default(),
        }
    }
}

impl State {
    // Reason this is in `model` instead of `engine`: This defines the relationship between
    // `State` and `StateCore` fields and is a rather straightforward transformation based on the
    // assumptions of each state variable.
    pub fn apply_step(&mut self, game_step: &GameStep) {
        if let Some(next) = game_step.next {
            let actor = game_step.actor;
            let actor_i = actor.to_usize();
            let next_actor = next.action_player;
            let next_actor_i = next_actor.to_usize();
            assert_eq!(self.core.action_player, actor);

            // Merge the self draw of this turn into the closed hand (it had been kept separate).
            if let Some(draw) = self.core.draw {
                self.closed_hands[actor_i][draw] += 1;

                log::debug!("P{}:Draw({}) => hand={}",
                    next_actor_i, draw, self.closed_hands[next_actor_i]);
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
