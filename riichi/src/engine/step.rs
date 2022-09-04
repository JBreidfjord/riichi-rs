use itertools::Itertools;

use crate::common::*;
use crate::model::*;
use super::EngineCache;
use super::utils::*;
use super::{RIICHI_POT};

/// Process normal end-of-turn flow (no abort, no win).
/// Each change to the state is processed in chronological order, gradually morphing the current
/// state to the next. This avoids copying the entire state.
pub(crate) fn next_normal(
    begin: &RoundBegin,
    state: &mut State,
    action: Action,
    reactions: &[Option<Reaction>; 4],
    action_result: ActionResult,
    cache: &EngineCache,
) {
    let actor = state.action_player;
    let actor_i = actor.to_usize();

    // Special case: Deferred revealing of new dora indicators due to Kakan/Daiminkan.
    // Why this is special:
    //
    // - **Timing**: The player who did a Kakan/Daiminkan has finished their turn after drawing
    //   from the tail of the wall. Revealing now makes sure that the player did not know what
    //   the new dora indicator is right after making the call.
    //
    // - **Information**: This solely relies on the result of the previous turn. Actions and
    //   reactions during this turn has no effect on this.
    //
    // TODO(summivox): rules (kan-dora)
    match state.incoming_meld {
        Some(Meld::Kakan(_)) | Some(Meld::Daiminkan(_)) => {
            state.num_dora_indicators += 1;
        }
        _ => {}
    }

    // After handling the only special case, we can start mutating the current state from
    // begin of this turn to begin of the next turn.

    state.seq += 1;

    // Commit the draw to the player's closed hand.
    if let Some(draw) = state.draw {
        state.closed_hands[actor_i][draw] += 1;
    };

    // Commit the action.
    // Note that the round has not ended. This means if there's an reaction, it must be a call
    // (Chii/Pon/Daiminkan) on this turn's Discard. Therefore we can merge reaction handling
    // into [`Action::Discard`].
    match action {
        Action::Discard(discard) => {
            // If there is indeed a call (according to `action_result`), find who called.
            let caller =
                other_players_after(actor).into_iter().filter(|reactor|
                    match reactions[reactor.to_usize()] {
                        Some(Reaction::Chii(_, _)) => action_result == ActionResult::Chii,
                        Some(Reaction::Pon(_, _)) => action_result == ActionResult::Pon,
                        Some(Reaction::Daiminkan) => action_result == ActionResult::Daiminkan,
                        _ => false
                    }
                ).exactly_one().unwrap_or(actor);

            // Commit the discard first.
            state.closed_hands[actor_i][discard.tile] -= 1;
            state.discards[actor_i].push(Discard { called_by: caller, ..discard });

            // Handle both existing and new riichi.
            if state.riichi[actor_i].is_active {
                // Ippatsu naturally expires after the first discard since declaring riichi.
                state.riichi[actor_i].is_ippatsu = false;
            } else if discard.declares_riichi {
                // Round has not ended => the new riichi is successful.
                state.riichi[actor_i] = RiichiFlags {
                    is_active: true,
                    is_ippatsu: caller == actor,  // no ippatsu if immediately called
                    is_double: is_init_abortable(state),
                }
            }

            if caller == actor {
                // No one called. Next turn is the next player (surprise!).
                state.action_player = player_succ(actor);
                state.incoming_meld = None;
                // state.melds => no-op
                state.draw = Some(begin.wall[state.num_drawn_head as usize]);
                state.num_drawn_head += 1;
            } else {
                // Someone called and will take the next turn instead.
                let meld = cache.meld[caller.to_usize()].unwrap();
                meld.consume_from_hand(&mut state.closed_hands[caller.to_usize()]);

                state.action_player = caller;
                state.incoming_meld = Some(meld);
                state.melds[caller.to_usize()].push(meld);
                state.draw = None;
                // state.num_drawn_* => no-op
            }

            // Check Furiten status for the discarding player.
            // furiten-by-discard == some tile in the waiting set exists in the discard set
            if !state.furiten[actor_i].by_discard && !state.furiten[actor_i].miss_permanent {
                let discard_set = TileMask34::from_iter(
                    state.discards[actor_i].iter().map(|discard| discard.tile));
                let waiting_set = cache.wait[actor_i].waiting_set;

                state.furiten[actor_i].by_discard = discard_set.0 & waiting_set.0 > 0
            }
            // Temporary miss expires after discarding.
            state.furiten[actor_i].miss_temporary = false;

        }

        Action::Ankan(_) | Action::Kakan(_) => {
            // The current player has made an Ankan/Kakan and is entitled to a bonus turn.
            // The round has not ended => no reaction is possible on this.
            let ankan_or_kakan = cache.meld[actor_i].unwrap();
            ankan_or_kakan.consume_from_hand(&mut state.closed_hands[actor_i]);

            state.action_player = actor;
            state.incoming_meld = Some(ankan_or_kakan);
            state.melds[actor_i].push(ankan_or_kakan);
            state.draw = Some(wall::kan_draw(&begin.wall, state.num_drawn_tail as usize));
            state.num_drawn_tail += 1;

            // Only for Ankan: reveal the next dora indicator immediately.
            // For Kakan, it will only be revealed at the end of the next turn, in the same way
            // as Daiminkan (see above).
            // TODO(summivox): rules (kan-dora)
            if let Action::Ankan(_) = action {
                state.num_dora_indicators += 1;
            }
        }

        Action::TsumoAgari(_) | Action::AbortNineKinds => panic!()
    }

    // Any kind of meld will forcefully break riichi ippatsu.
    if state.incoming_meld.is_some() {
        for player in all_players() {
            state.riichi[player.to_usize()].is_ippatsu = false;
        }
    }

    // Check Furiten status for other players.
    // For another player who misses the chance to win (discard in waiting set):
    // - Immediately enters temporary miss state
    // - Immediately enters permanent miss state if under riichi
    // TODO(summivox): ankan should only affects kokushi-tenpai here, although kakan is treated
    //     the same as Ron.
    // TODO(summivox): rules (kokushi-ankan)
    for other_player in other_players_after(actor) {
        let other_player_i = other_player.to_usize();
        let furiten = &mut state.furiten[other_player_i];

        if furiten.miss_permanent { continue; }
        if cache.wait[other_player_i].waiting_set.has(action.tile().unwrap()) {
            furiten.miss_temporary = true;
            furiten.miss_permanent = state.riichi[other_player_i].is_active;
        }
    }
}

pub(crate) fn next_agari(action_result: ActionResult) -> RoundEnd {
    // TODO(summivox): agari
    RoundEnd {
        round_result: action_result,
        pot: 0,
        points: [0; 4],
        points_delta: [0; 4],
        renchan: false,
        next_round_id: None,
        agari_result: Default::default(),
    }
}

pub(crate) fn next_abort(
    begin: &RoundBegin,
    state: &State,
    action_result: ActionResult,
    cache: &EngineCache,
) -> RoundEnd {
    let mut end = RoundEnd {
        round_result: action_result,
        pot: begin.pot + (num_active_riichi(state) as GamePoints * RIICHI_POT),
        points: begin.points,
        ..RoundEnd::default()
    };

    let round_id = begin.round_id;
    // ugly syntax gets around array::map moving the Vec value
    let waiting = [0, 1, 2, 3].map(|i| cache.wait[i].waiting_set.any() as u8);
    match action_result {
        ActionResult::AbortWallExhausted | ActionResult::AbortNagashiMangan => {
            // The latter is only a special case of the former, with points delta being the
            // only real distinction. Therefore, we merge the handling.
            (end.points_delta, end.renchan) =
                resolve_wall_exhausted(state, waiting, round_id.button());
            end.next_round_id = Some(round_id.next_honba(end.renchan));
        }

        ActionResult::AbortFourKan | ActionResult::AbortFourWind |
        ActionResult::AbortFourRiichi | ActionResult::AbortTripleRon => {
            // force renchan with honba + 1
            end.renchan = true;
            end.next_round_id = Some(round_id.next_honba(true));
        }

        _ => panic!()
    }

    for i in 0..4 { end.points[i] += end.points_delta[i]; }

    end
}
