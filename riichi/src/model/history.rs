use std::fmt::{Display, Formatter};

use itertools::Itertools;

use crate::common::*;

use super::{
    action::*,
    action_result::*,
    boundary::*,
    reaction::*,
    state::*,
};

/// Bundle of a turn's action and the resolved (highest-priority) reactor + reaction (if any).
///
/// Note that Multi-Ron cannot be represented by this; additional info is needed.
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of the fields.
///
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Bundle of a turn's action + reaction (if any) from the players, and resolved result + the next
/// state from the game engine.
///
/// Note that Multi-Ron cannot be represented by this; additional info is needed.
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of the fields.
///
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameStep {
    pub actor: Player,
    pub action: Action,
    pub reactor_reaction: Option<(Player, Reaction)>,
    pub action_result: ActionResult,
    pub next_state_core: Option<StateCore>,
}

/// A prefix of the full history of a round; i.e. the begin condition + each turn's [`GameStep`].
///
/// Multi-Ron can be encoded in the `ron` field.
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of the fields.
///
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoundHistory {
    /// Begin condition of the round.
    pub begin: RoundBegin,

    /// Each turn's action, reaction, resolved result, and the next state core.
    pub steps: Vec<GameStep>,

    /// Which players have declared Ron at the last game step.
    /// This can encode the multi-Ron condition.
    pub ron: [bool; 4],
}

/// The action-defined history (prefix) of a round, i.e. the begin condition and each turn's
/// [`ActionReaction`]. Compared to [`RoundHistory`], this is missing the states after each turn.
///
/// Multi-Ron can be encoded in the `ron` field.
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of the fields.
///
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoundHistoryLite {
    /// Begin condition of the round.
    pub begin: RoundBegin,

    /// Each turn's action and reaction. State is implicit from the game logic.
    pub action_reactions: Vec<ActionReaction>,

    /// Whether a player has declared Ron at the last `action_reactions`.
    /// This can encode the multi-Ron condition.
    pub ron: [bool; 4],
}

impl Display for RoundHistoryLite {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}, pot={}, points={:?}, multi_ron={:?}",
                 self.begin.round_id,
                 self.begin.pot,
                 self.begin.points,
                 self.ron,
        )?;
        for action_reaction in self.action_reactions.iter() {
            writeln!(f, "{}", action_reaction)?;
        }
        Ok(())
    }
}

// Reconstruction of the full state from core
impl State {
    /// Current full [`State`] + [`Action`] + next [`StateCore`] => next full [`State`].
    /// This _mutates_ the non-Core state variables, then absorbs the next Core.
    ///
    /// The following are the full list of mutations:
    ///
    /// - Self draw from the beginning of this turn is absorbed into the actor's closed hand.
    /// - Discard from the actor (if any) is moved to the actor's discards section;
    ///   if it got called, then the `called_by` is populated.
    /// - Any meld for the next turn is moved from the meld-maker's closed hand to the meld section.
    ///
    /// Reason this is in `model` instead of `engine`: It defines the relationship between non-Core
    /// and Core fields of the state based on the invariants of them. There is no interpretation of
    /// the game rule in this definition.
    ///
    pub fn evolve(&mut self, action: Action, next: StateCore) {
        let actor = self.core.actor;
        let actor_i = actor.to_usize();
        let next_actor = next.actor;
        let next_actor_i = next_actor.to_usize();

        // Merge the self draw of this turn into the closed hand (it had been kept separate).
        if let Some(draw) = self.core.draw {
            self.closed_hands[actor_i][draw] += 1;

            log::debug!("P{}:Draw({}) => hand={}", actor_i, draw, self.closed_hands[actor_i]);
        }

        // Move the discard of this turn from the closed hand to the discard section.
        match action {
            Action::Discard(mut discard) => {
                self.closed_hands[actor_i][discard.tile] -= 1;
                discard.called_by =
                    if next.incoming_meld.is_some() { next_actor } else { actor };
                self.discards[actor_i].push(discard);
                self.discard_sets[actor_i].set(discard.tile);

                log::debug!("P{}:{} => hand={}", actor_i, discard, self.closed_hands[actor_i]);
            }
            Action::Kakan(_) | Action::Ankan(_) => {}  // No-op, but valid.
            _ => panic!("inconsistent")
        }

        // Move the meld of this turn from the closed hand to the meld section.
        // Same handling for both Kakan/Ankan (action) and Chii/Pon/Daiminkan (reaction).
        if let Some(meld) = next.incoming_meld {
            meld.consume_from_hand(&mut self.closed_hands[next_actor_i]);
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

            log::debug!("P{}:Meld({}) => hand={}",
                next_actor_i, meld, self.closed_hands[next_actor_i]);
        }

        // finally replace the core wholesale
        self.core = next;
    }

    /// Same as [`Self::evolve()`] but with a [`GameStep`].
    pub fn apply_step(&mut self, game_step: &GameStep) {
        if let Some(next) = game_step.next_state_core {
            self.evolve(game_step.action, next);
        }
    }

    /// Same as [`Self::evolve()`] but with many [`GameStep`]s.
    pub fn apply_steps<'a>(&mut self, game_step: impl IntoIterator<Item=&'a GameStep>) {
        for step in game_step {
            self.apply_step(step);
        }
    }
}
