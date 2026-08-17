use thiserror::Error;

use riichi_elements::prelude::*;

use crate::{
    engine::agari::{agari_candidates, AgariInput},
    model::*
};
use super::{
    EngineCache,
    RIICHI_POT,
    utils::*
};

#[derive(Error, Debug)]
pub enum ActionError {
    #[error("Tsumogiri discard tile {0} != drawn tile {1:?}")]
    TsumogiriMismatch(Tile, Option<Tile>),

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

    #[error("Can only discard after Chii/Pon.")]
    DiscardOnlyAfterChiiPon,

    #[error("Cannot ankan/kakan on the last draw")]
    CannotKanOnLastDraw,

    #[error("Kita (North extraction) is not allowed in {0:?}.")]
    KitaNotAllowed(Variant),

    #[error("Cannot Kita on {0}; only North can be extracted.")]
    KitaNotNorth(Tile),

    #[error("Cannot Kita on the last draw")]
    CannotKitaOnLastDraw,

    #[error("Cannot Kita; no North in the closed hand.")]
    NoNorthForKita,

    #[error("Cannot Kita under riichi unless the drawn tile is the North (drew {0:?}).")]
    InvalidKitaUnderRiichi(Option<Tile>),

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

    #[error("Can only declare Tsumo-Agari (win by self-draw) on the drawn tile.")]
    MustDeclareTsumoAgariOnDraw,

    #[error("Cannot declare Tsumo-Agari (not waiting or no yaku).")]
    CannotTsumoAgari,
}

pub fn check_action(
    begin: &RoundBegin,
    state: &State,
    action: Action,
    cache: &mut EngineCache,
) -> Result<(), ActionError> {

    use ActionError::*;

    let actor = state.core.actor;
    let actor_i = actor.to_usize();

    // Make a copy of `actor`'s hand; this will be updated along the way to reflect what happens
    // at each step.
    let mut hand = state.closed_hands[actor.to_usize()].clone();
    let under_riichi = state.core.riichi[actor_i].is_some();

    if let Some(draw) = state.core.draw {
        hand[draw] += 1;  // 3N+2
    } else if let Some(meld @ Meld::Chii(_)) | Some(meld @ Meld::Pon(_)) = state.core.incoming_meld {
        // The only valid action right after Chii/Pon is to discard under swap-call restrictions.
        if let Action::Discard(discard) = action {
            if is_forbidden_swap_call(&begin.ruleset, meld, discard.tile) {
                return Err(NoSwapCalling(discard.tile, meld));
            }
        } else {
            return Err(DiscardOnlyAfterChiiPon);
        }
    }

    match action {
        Action::Discard(discard) => {
            // No need to re-declare riichi.
            if under_riichi && discard.declares_riichi { return Err(DeclareRiichiAgain); }

            // Discarded tile must be either just drawn, or already in our closed hand.
            if discard.is_tsumogiri {
                if state.core.draw != Some(discard.tile) {
                    return Err(TsumogiriMismatch(discard.tile, state.core.draw));
                }
            } else {
                if under_riichi {
                    return Err(DiscardClosedHandUnderRiichi);
                }
            }

            // Update hand: remove the discard.
            if hand[discard.tile] == 0 { return Err(TileNotExist(discard.tile)); }
            hand[discard.tile] -= 1;  // 3N+2 - 1 = 3N+1
            cache.update_wait_cache(actor, &hand);

            // Declaring riichi requires a closed 3N+1 waiting hand _after discarding_.
            if discard.declares_riichi {
                // Need enough points to put into the pot
                if begin.points[actor_i] < RIICHI_POT {
                    return Err(DeclareRiichiWithoutPoints);
                }
                // Hand must be closed
                if !state.melds[actor_i].iter().all(|meld| meld.is_closed()) {
                    return Err(DeclareRiichiWithOpenMeld);
                }
                // Hand must be waiting
                if cache.wait[actor_i].waiting_tiles.is_empty() {
                    return Err(DeclareRiichiWhileNotReady);
                }
            }
        }

        Action::Ankan(tile) => {
            let tile = tile.to_normal();
            if is_last_draw(state) { return Err(CannotKanOnLastDraw); }
            if under_riichi && !is_ankan_ok_under_riichi(
                &begin.ruleset,
                &mut cache.decomposer,
                &hand,
                &cache.wait[actor_i],
                state.core.draw.unwrap_or(tile),
                tile,
            ) {
                return Err(InvalidAnkanUnderRiichi(tile));
            }

            if let Some(ankan) = Ankan::from_hand(&hand, tile) {
                ankan.consume_from_hand(&mut hand); // 3N+2 - 4 = 3M+1
                cache.meld[actor_i] = Some(Meld::Ankan(ankan));
                cache.update_wait_cache(actor, &hand);
            } else {
                return Err(NotEnoughForAnkan(tile));
            }
        }
        Action::Kakan(added) => {
            if is_last_draw(state) { return Err(CannotKanOnLastDraw); }
            let pon = state.melds[actor_i]
                .iter()
                .find_map(|meld| {
                    if let Meld::Pon(pon) = meld {
                        if pon.called.to_normal() == added.to_normal() {
                            return Some(pon);
                        }
                    }
                    None
                })
                .ok_or(NoPonForKakan(added))?;
            if let Some(kakan) = Kakan::from_pon_hand(*pon, &hand) {
                kakan.consume_from_hand(&mut hand);  // 3N+2 - 1 = 3N+1
                cache.meld[actor_i] = Some(Meld::Kakan(kakan));
                cache.update_wait_cache(actor, &hand);
            } else {
                return Err(TileNotExist(added));
            }
        }

        Action::Kita(tile) => {
            let variant = begin.ruleset.variant;
            if !variant.allows_kita() { return Err(KitaNotAllowed(variant)); }
            if tile.to_normal() != NORTH { return Err(KitaNotNorth(tile)); }

            // Barred on the Haitei draw, exactly like a Kan. Not stated anywhere in Tenhou's rule
            // text, but decisive in the logs: of 23,104 extractions none happens after the 55th
            // (last) draw, and at draw 55 the behaviour flips --- 34 players drew a North and
            // discarded it, 0 extracted.
            if num_draws(state) >= variant.max_num_draws() { return Err(CannotKitaOnLastDraw); }

            // 「ポンの直後は抜けない」 --- no extraction right after one's own Pon. This needs no
            // code: the Chii/Pon prologue above already rejects every non-Discard action there.

            // Under riichi the hand is committed, so only a *just-drawn* North may be extracted;
            // pulling a North out of the standing hand would change the wait (a North in a
            // 13-tile tenpai hand is always load-bearing).
            //
            // UNSETTLED: no source states this and issue #115 did not test it --- all it
            // establishes is that extraction after riichi is legal, routine and optional. This is
            // the conservative reading, mirroring `riichi_ankan_strict_mode`'s Okuri-Kan
            // prohibition. Since Norths are fungible, it costs a riichi player nothing.
            if under_riichi && state.core.draw.map(|t| t.to_normal()) != Some(NORTH) {
                return Err(InvalidKitaUnderRiichi(state.core.draw));
            }

            // Kita takes **any** North in the concealed hand, not only the drawn one: of 23,104
            // extractions in the houou 3p sample, ~12,600 were of a North already held.
            // (`hand` already has the draw merged in above.)
            if let Some(kita) = Kita::from_hand(&hand) {
                kita.consume_from_hand(&mut hand);  // 3N+2 - 1 = 3N+1
                cache.meld[actor_i] = Some(Meld::Kita(kita));
                cache.update_wait_cache(actor, &hand);
            } else {
                return Err(NoNorthForKita);
            }
        }

        Action::TsumoAgari(tile) => {
            if state.core.draw != Some(tile) { return Err(MustDeclareTsumoAgariOnDraw); }

            // Special case when the current player started the turn by Daiminkan:
            //
            // - Before Daiminkan: 3N+1 hand
            // - After Daiminkan: 3N+1 - 3 = 3M+1 hand, with 1 draw from the tail of the wall.
            //
            // Note that although the closed hand retains 3N+1 form, it has nevertheless changed.
            // In case the hand remains waiting, the player _is_ eligible for TsumoAgari, with the
            // extra bonus of [`Yaku::Rinshankaihou`]. Therefore we need to re-calc the wait.
            if let Some(Meld::Daiminkan(_)) = state.core.incoming_meld {
                hand[state.core.draw.unwrap()] -= 1;  // undo the merging of draw above; now 3N+1.
                cache.update_wait_cache(actor, &hand);
            }

            let agari_input = AgariInput::new(
                begin.round_id,
                state,
                &cache.wait[actor_i],
                action,
                actor,
                actor,
            );
            let candidates = agari_candidates(&begin.ruleset, &agari_input);
            if candidates.is_empty() { return Err(CannotTsumoAgari); }
            cache.win[actor_i] = candidates;
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
