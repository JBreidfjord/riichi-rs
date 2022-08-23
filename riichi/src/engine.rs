//! Core game logic, i.e. state transitions.

mod utils;
use utils::*;

use itertools::Itertools;
use thiserror::Error;

use crate::analysis;
use crate::analysis::{Decomposer, FullHandWaitingPattern};
use crate::common::*;
use crate::model::*;

#[derive(Error, Debug)]
pub enum ActionError {
    #[error("Tsumokiri discard tile {0} != drawn tile {1:?}")]
    TsumokiriMismatch(Tile, Option<Tile>),

    #[error("Discarding from the closed hand while under riichi.")]
    DiscardClosedHandUnderRiichi,

    #[error("Tile {0} does not exist in the closed hand.")]
    TileNotExist(Tile),

    #[error("Attempting to declare riichi with an open hand.")]
    DeclareRiichiWithOpenMeld,

    #[error("Attempting to declare riichi with a hand not ready after discarding.")]
    DeclareRiichiWhileNotReady,

    #[error("Attempting invalid ankan on {0} under riichi.")]
    InvalidAnkanUnderRiichi(Tile),

    #[error("Cannot ankan on {0} with only {} in hand.")]
    NotEnoughForAnkan(Tile, u8),

    #[error("Attempting kakan on {0} without corresponding pon.")]
    NoPonForKakan(Tile),

    #[error("Cannot kyuushuukyuuhai with only {0} kinds of terminals in hand.")]
    NotEnoughForKyuushuukyuuhai(u8),

    #[error("Cannot abort after the first go-around.")]
    NotInitAbortable,
}

#[derive(Error, Debug)]
pub enum ReactionError {
    #[error("No action to react to.")]
    NoAction,

    #[error("Tile {0} does not exist in the closed hand.")]
    TileNotExist(Tile),

    #[error("You can only call a discarded tile (is actually {0:?})")]
    NotDiscard(Action),

    #[error("Can only Chii on the previous player's discard.")]
    CanOnlyChiiPrevPlayer,

    #[error("Cannot Chii {2} with own {0}{1}.")]
    InvalidChii(Tile, Tile, Tile),

    #[error("Cannot Pon {2} with own {0}{1}.")]
    InvalidPon(Tile, Tile, Tile),

    #[error("Cannot Daiminkan.")]
    InvalidDaiminkan,

    #[error("No ron when you are furiten: {0:?}")]
    Furiten(FuritenFlags),
}

pub struct Engine {
    s: PreActionState,
    action: Option<Action>,
    reactions: [Option<Reaction>; 4],

    decomposer: Decomposer<'static>,

    /// The closed hand after the player takes action --- including draw and discard.
    hand_after_action: TileSet37,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            s: Default::default(),
            action: None,
            reactions: [None; 4],

            decomposer: Decomposer::new(),

            hand_after_action: TileSet37::default(),
        }
    }

    pub fn pre_action(&mut self, pre_action: PreActionState) -> &mut Self {
        self.s = pre_action;
        self
    }

    pub fn action(&mut self, action: Action) -> Result<&mut Self, ActionError> {
        self.action = None;
        self.reactions = [None; 4];
        self.check_action(action)?;
        self.action = Some(action);
        Ok(self)
    }

    fn check_action(&mut self, action: Action) -> Result<(), ActionError> {
        use ActionError::*;

        let s = &self.s;
        let p = s.action_player;
        let pp = p.to_usize();

        let mut hand = s.closed_hands[p.to_usize()];
        if let Some(draw) = s.draw {
            hand[draw] += 1;
        };

        match action {
            Action::Discard {
                tile,
                riichi,
                tsumokiri,
            } => {
                // Discarded tile must be either just drawn, or already in our closed hand.
                if tsumokiri {
                    if s.draw != Some(tile) {
                        return Err(TsumokiriMismatch(tile, s.draw));
                    }
                } else {
                    if s.riichi[pp].is_active {
                        return Err(DiscardClosedHandUnderRiichi);
                    }
                }
                if hand[tile] == 0 { return Err(TileNotExist(tile)); }
                hand[tile] -= 1;

                // Declaring riichi requires a closed 3N+1 ready (tenpai) hand after discarding.
                if riichi {
                    // Ankan is considered closed; all other melds are not ok.
                    if s.melds[pp]
                        .iter()
                        .any(|meld| !matches!(meld, &Meld::Ankan(_)))
                    {
                        return Err(DeclareRiichiWithOpenMeld);
                    }
                    if self.decomposer.keys(TileSet34::from(hand).packed())
                        .iter().next().is_none() {
                        return Err(DeclareRiichiWhileNotReady);
                    }
                }
            }
            Action::Ankan(tile) => {
                if hand[tile] != 4 { return Err(NotEnoughForAnkan(tile, hand[tile])); }
                hand[tile] = 0;

                if s.riichi[pp].is_active {
                    // TODO(summivox): check ankan-riichi conflict using 3N+1 tenpai
                    if false {
                        return Err(InvalidAnkanUnderRiichi(tile));
                    }
                }
            }
            Action::Kakan(added) => {
                if hand[added] == 0 { return Err(TileNotExist(added)); }
                hand[added] -= 1;

                let (_i, pon) = s.melds[pp]
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &meld)| {
                        if let Meld::Pon(pon) = meld {
                            if pon.called.to_normal() == added {
                                return Some((i, pon));
                            }
                        }
                        None
                    })
                    .exactly_one()
                    .map_err(|_| NoPonForKakan(added))?;

                // TODO(summivox): cache result?
                let _kakan = Kakan::from_pon_added(pon, added).unwrap();
            }
            Action::TsumoAgari(_tile) => {
                // TODO(summivox): agari
            }
            Action::Kyuushuukyuuhai => {
                if !s.is_init_abortable() { return Err(NotInitAbortable); }
                let n = hand.terminal_kinds();
                if n < 9 {
                    return Err(NotEnoughForKyuushuukyuuhai(n));
                }
            }
        }
        self.hand_after_action = hand;
        Ok(())
    }

    pub fn reaction(
        &mut self,
        reactor: Player,
        reaction: Reaction,
    ) -> Result<&mut Self, ReactionError> {
        self.reactions[reactor.to_usize()] = None;
        self.check_reaction(reactor, reaction)?;
        self.reactions[reactor.to_usize()] = Some(reaction);
        Ok(self)
    }

    fn check_reaction(&mut self, reactor: Player, reaction: Reaction) -> Result<(), ReactionError> {
        use ReactionError::*;

        let s = &self.s;
        let action = self.action.ok_or(NoAction)?;
        let p = s.action_player;
        let pp = p.to_usize();
        let p_hand = &self.hand_after_action;
        let q = reactor;
        let qq = q.to_usize();
        // reactor's hand (copy to make presence test easier)
        let mut q_hand = s.closed_hands[qq];

        match reaction {
            Reaction::Chii(own0, own1) => {
                if q_hand[own0] == 0 { return Err(TileNotExist(own0)); }
                q_hand[own0] -= 1;
                if q_hand[own1] == 0 { return Err(TileNotExist(own1)); }
                q_hand[own1] -= 1;
                if p.wrapping_add(Player::new(1)) != q {
                    return Err(CanOnlyChiiPrevPlayer);
                }
                if let Action::Discard { tile: called, .. } = action {
                    if let Some(chii) = Chii::from_tiles(own0, own1, called) {
                        // TODO(summivox): maybe cache
                    } else {
                        return Err(InvalidChii(own0, own1, called));
                    }
                } else {
                    return Err(NotDiscard(action));
                }
            }
            Reaction::Pon(own0, own1) => {
                if q_hand[own0] == 0 {
                    return Err(TileNotExist(own0));
                }
                q_hand[own0] -= 1;
                if q_hand[own1] == 0 {
                    return Err(TileNotExist(own1));
                }
                q_hand[own1] -= 1;
                if let Action::Discard { tile: called, .. } = action {
                    if let Some(pon) = Pon::from_tiles_dir(own0, own1, called, p.wrapping_sub(q)) {
                        // TODO(summivox): maybe cache
                    } else {
                        return Err(InvalidPon(own0, own1, called));
                    }
                } else {
                    return Err(NotDiscard(action));
                }
            }
            Reaction::Daiminkan => {
                if let Action::Discard { tile: called, .. } = action {
                    let t = called.to_normal();
                    // recover count of both normal and red from reactor's hand
                    let n_normal = q_hand[t] as usize;
                    let n_red = if t.num() == 5 { q_hand[t.to_red()] as usize } else { 0 };
                    let n = n_normal + n_red;
                    if n != 3 {
                        return Err(TileNotExist(called.to_normal()));
                    }
                    // color the correct # of reds
                    let mut own = [t, t, t];
                    for i in 0..n_red {
                        own[i] = t.to_red();
                    }

                    if let Some(pon) =
                        Daiminkan::from_tiles_dir(own[0], own[1], own[2], called, p.wrapping_sub(q))
                    {
                        // TODO(summivox): maybe cache
                    } else { return Err(InvalidDaiminkan); }
                } else { return Err(NotDiscard(action)); }
            }
            Reaction::RonAgari => {
                // TODO(summivox): all kinds of fun stuff here:
                // furiten, agari, chankan, kokushi-ankan, ...
                if s.furiten[pp].any() { return Err(Furiten(s.furiten[pp])); }

            }
        }
        Ok(())
    }

    pub fn resolve_reaction(
        &self,
        pre_action: &PreActionState,
        action: Action,
        reactions: &[Option<Reaction>; 4],
    ) -> ActionResult {
        unimplemented!()
    }

    pub fn step(
        &self,
        pre_action: &PreActionState,
        action: Action,
        reactions: &[Option<Reaction>; 4],
    ) -> (ActionResult, NextOrEnd) {
        let action_result = self.resolve_reaction(pre_action, action, reactions);
        if action_result.is_abort() {
            // TODO
            (
                action_result,
                NextOrEnd::End(RoundEndState {
                    round_result: action_result,
                    pot: 0,
                    points: [0; 4],
                    points_delta: [0; 4],
                    next_round_id: None,
                    renchan: false,
                    agari_summary: None,
                }),
            )
        } else if action_result.is_agari() {
            // TODO
            (
                action_result,
                NextOrEnd::End(RoundEndState {
                    round_result: action_result,
                    pot: 0,
                    points: [0; 4],
                    points_delta: [0; 4],
                    next_round_id: None,
                    renchan: false,
                    agari_summary: None,
                }),
            )
        } else {
            let mut next_state = pre_action.clone();
            // TODO
            (action_result, NextOrEnd::Next(next_state))
        }
    }
}
