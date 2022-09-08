//! State-Action representation of the game.
//!
//! This module mainly provides data model definitions and some straightforward helpers.
//! Game logic belongs to [`crate::engine`].
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
        let first_draw = self.wall[52];
        let mut closed_hands = wall::deal(wall, button);
        closed_hands[button.to_usize()][first_draw] += 1;
        State {
            core: StateCore {
                seq: 0,
                action_player: button,
                num_drawn_head: 53,
                num_drawn_tail: 0,
                num_dora_indicators: 1,
                draw: Some(first_draw),
                incoming_meld: None,
                furiten: Default::default(),
                riichi: Default::default(),
            },
            closed_hands,
            discards: [vec![], vec![], vec![], vec![]],
            melds: [vec![], vec![], vec![], vec![]],
        }
    }
}

impl State {
    pub fn apply_step(&mut self, game_step: &GameStep) {
        if let Some(next) = game_step.next {
            let actor = game_step.actor;
            let actor_i = actor.to_usize();
            let next_actor = next.action_player;
            let next_actor_i = next_actor.to_usize();

            // action: affects hand, discards
            match game_step.action {
                Action::Discard(mut discard) => {
                    discard.called_by = actor;
                    self.closed_hands[actor_i][discard.tile] -= 1;

                    // TODO DEBUG
                    // println!("P{}:{} => {}", actor_i, discard, self.closed_hands[actor_i]);

                    if let Some((reactor, reaction)) = game_step.reactor_reaction {
                        assert_eq!(reactor, next_actor);
                        assert_ne!(reaction, Reaction::RonAgari);
                        discard.called_by = reactor;
                    }
                    self.discards[actor_i].push(discard);
                }
                Action::Kakan(_) | Action::Ankan(_) => {}  // No-op, but valid.
                _ => panic!("inconsistent")
            }

            // draw: affects hand
            if let Some(draw) = next.draw {
                self.closed_hands[next_actor_i][draw] += 1;

                // TODO DEBUG
                // println!("P{}:Draw({}) => {}", next_actor_i, draw, self.closed_hands[next_actor_i]);
            }

            // meld: affects hand, melds
            if let Some(meld) = next.incoming_meld {
                // This covers _all_ ways of meld --- both action and reaction.
                meld.consume_from_hand(&mut self.closed_hands[next_actor_i]);

                // TODO DEBUG
                // println!("P{}:{} => {}", next_actor_i, meld, self.closed_hands[next_actor_i]);

                if let Meld::Kakan(kakan) = meld {
                    // Special case: Kakan will replace the existing Pon
                    let (pon_i, _) = self.melds[next_actor_i].iter()
                        .find_position(|&&meld| meld == Meld::Pon(kakan.pon))
                        .unwrap();
                    self.melds[next_actor_i][pon_i] = meld;
                } else {
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
    use assert2::check;
    use itertools::Itertools;

    #[test]
    fn reaction_ordering_is_correct() {
        let reactions = [
            Reaction::Chii(Tile::MIN, Tile::MIN.succ().unwrap()),
            Reaction::Chii(Tile::MIN.succ().unwrap(), Tile::MIN.succ2().unwrap()),
            Reaction::Pon(Tile::MIN, Tile::MIN),
            Reaction::Pon(Tile::MIN.succ().unwrap(), Tile::MIN.succ().unwrap()),
            Reaction::Daiminkan,
            Reaction::RonAgari,
        ];
        for (low, high) in reactions.iter().tuple_windows() {
            check!(low < high);
        }
    }

    #[test]
    fn round_id_computes_correct_self_wind() {
        let round_id = RoundId { kyoku: 6, honba: 0 };
        check!(round_id.self_wind_for_player(P2) == Wind::new(0));
        check!(round_id.self_wind_for_player(P3) == Wind::new(1));
        check!(round_id.self_wind_for_player(P0) == Wind::new(2));
        check!(round_id.self_wind_for_player(P1) == Wind::new(3));
    }
}
