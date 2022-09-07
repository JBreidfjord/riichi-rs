use itertools::Itertools;
use thiserror::Error;

use crate::common::*;
use crate::model::*;
use super::EngineCache;
use super::{RIICHI_POT};
use super::utils::*;

#[derive(Error, Debug)]
pub enum ActionError {
    #[error("Tsumokiri discard tile {0} != drawn tile {1:?}")]
    TsumokiriMismatch(Tile, Option<Tile>),

    #[error("Discarding from the closed hand while under riichi.")]
    DiscardClosedHandUnderRiichi,

    #[error("Discarding {0} is swap-calling (kuikae) due to {1}")]
    NoSwapCalling(Tile, Meld),

    #[error("Tile {0} does not exist in the closed hand.")]
    TileNotExist(Tile),

    #[error("Attempting to declare riichi when already under riichi.")]
    DeclareRiichiAgain,

    #[error("Attempting to declare riichi without enough points.")]
    DeclareRiichiWithoutPoints,

    #[error("Attempting to declare riichi with an open hand.")]
    DeclareRiichiWithOpenMeld,

    #[error("Attempting to declare riichi with a hand not ready after discarding.")]
    DeclareRiichiWhileNotReady,

    #[error("Cannot ankan/kakan on the last draw")]
    CannotKanOnLastDraw,

    #[error("Attempting invalid ankan on {0} under riichi.")]
    InvalidAnkanUnderRiichi(Tile),

    #[error("Cannot ankan on {0}; not enough in hand")]
    NotEnoughForAnkan(Tile),

    #[error("Attempting kakan on {0} without corresponding pon.")]
    NoPonForKakan(Tile),

    #[error("Cannot declare Kyuushuukyuuhai with only {0} kinds of terminals in hand.")]
    NotEnoughKindsForNineKinds(u8),

    #[error("Cannot abort after the first go-around.")]
    NotInitAbortable,

    #[error("Can only declare tsumo-agari (win by self-draw) on the drawn tile.")]
    MustDeclareTsumoAgariOnDraw,
}

pub(crate) fn check_action(
    begin: &RoundBegin,
    state: &State,
    action: Action,
    cache: &mut EngineCache,
) -> Result<(), ActionError> {

    use ActionError::*;

    let actor = state.action_player;
    let actor_i = actor.to_usize();

    // Make a copy of `actor`'s hand; this will be used to determine its
    let mut hand = state.closed_hands[actor.to_usize()];
    if let Some(draw) = state.draw {
        hand[draw] += 1;
    };

    let under_riichi = state.riichi[actor_i].is_active;

    match action {
        Action::Discard(discard) => {
            // D'oh!
            if under_riichi && discard.declares_riichi { return Err(DeclareRiichiAgain); }

            // Discarded tile must be either just drawn, or already in our closed hand.
            if discard.is_tsumokiri {
                if state.draw != Some(discard.tile) {
                    return Err(TsumokiriMismatch(discard.tile, state.draw));
                }
            } else {
                if under_riichi {
                    return Err(DiscardClosedHandUnderRiichi);
                }
            }
            if hand[discard.tile] == 0 { return Err(TileNotExist(discard.tile)); }
            hand[discard.tile] -= 1;
            cache.update_wait_cache(actor, &hand);

            // Declaring riichi requires a closed 3N+1 ready (tenpai) hand after discarding.
            if discard.declares_riichi {
                if begin.points[actor_i] < RIICHI_POT {
                    return Err(DeclareRiichiWithoutPoints);
                }
                // Ankan is considered closed; all other melds are not ok.
                if state.melds[actor_i]
                    .iter()
                    .any(|meld| !matches!(meld, &Meld::Ankan(_)))
                {
                    return Err(DeclareRiichiWithOpenMeld);
                }
                if cache.wait[actor_i].waiting_set.is_empty() {
                    return Err(DeclareRiichiWhileNotReady);
                }
            }

            if let Some(meld) = state.incoming_meld {
                if is_forbidden_swap_call(meld, discard.tile) {
                    return Err(NoSwapCalling(discard.tile, meld));
                }
            }
        }

        Action::Ankan(tile) => {
            let tile = tile.to_normal();

            if is_last_draw(state) { return Err(CannotKanOnLastDraw); }
            if under_riichi && !is_ankan_ok_under_riichi(
                &cache.wait[actor_i].regular, tile) {
                return Err(InvalidAnkanUnderRiichi(tile));
            }
            if let Some(ankan) = Ankan::from_hand(&hand, tile) {
                ankan.consume_from_hand(&mut hand);
                cache.meld[actor_i] = Some(Meld::Ankan(ankan));
                cache.update_wait_cache(actor, &hand);
            } else {
                return Err(NotEnoughForAnkan(tile));
            }
        }
        Action::Kakan(added) => {
            if is_last_draw(state) { return Err(CannotKanOnLastDraw); }
            let &pon = state.melds[actor_i]
                .iter()
                .filter_map(|meld| {
                    if let Meld::Pon(pon) = meld {
                        if pon.called.to_normal() == added.to_normal() {
                            return Some(pon);
                        }
                    }
                    None
                })
                .exactly_one()
                .map_err(|_| NoPonForKakan(added))?;
            if let Some(kakan) = Kakan::from_pon_hand(pon, &hand) {
                kakan.consume_from_hand(&mut hand);
                cache.meld[actor_i] = Some(Meld::Kakan(kakan));
                cache.update_wait_cache(actor, &hand);
            } else {
                return Err(TileNotExist(added));
            }
        }

        Action::TsumoAgari(tile) => {
            if state.draw != Some(tile) { return Err(MustDeclareTsumoAgariOnDraw); }
            // TODO(summivox): agari
        }
        Action::AbortNineKinds => {
            if !is_first_chance(state) { return Err(NotInitAbortable); }
            let n = terminal_kinds(&hand);
            if n < 9 {
                return Err(NotEnoughKindsForNineKinds(n));
            }
        }
    }
    Ok(())
}
