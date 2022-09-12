use log::log_enabled;
use crate::{
    analysis::IrregularWait,
    common::*,
    engine::distribute_points,
    model::*
};
use crate::rules::Ruleset;
use super::{
    utils::*,
    EngineCache,
    RIICHI_POT
};

/// Process normal end-of-turn flow (no abort, no win).
/// Each change to the state is processed in chronological order, gradually morphing the current
/// state to the next. This avoids copying the entire state.
pub fn next_normal(
    begin: &RoundBegin,
    state: &State,
    action: Action,
    action_result: ActionResult,
    cache: &EngineCache,
) -> StateCore {
    let mut next = state.core;
    let actor = state.core.actor;
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
    if begin.ruleset.dora_allow_kan {
        if let Some(Meld::Kakan(_)) | Some(Meld::Daiminkan(_)) = state.core.incoming_meld {
            next.num_dora_indicators += 1;
        }
    }

    next.seq += 1;

    // Commit the action.
    // Note that the round has not ended. This means if there's an reaction, it must be a call
    // (Chii/Pon/Daiminkan) on this turn's Discard. Therefore we can merge reaction handling
    // into discard handling.
    match action {
        Action::Discard(discard) => {
            let caller =
                if let ActionResult::CalledBy(caller) = action_result { caller } else { actor };

            // Handle both existing and new riichi.
            if state.core.riichi[actor_i].is_active {
                // Ippatsu naturally expires after the first discard since declaring riichi.
                next.riichi[actor_i].is_ippatsu = false;
            } else if discard.declares_riichi {
                // Round has not ended => the new riichi is successful.
                next.riichi[actor_i] = RiichiFlags {
                    is_active: true,
                    is_ippatsu: caller == actor,  // no ippatsu if immediately called
                    is_double: is_first_chance(state),
                }
            }

            if caller == actor {
                // No one called. Next turn is the next player (surprise!).
                next.actor = actor.succ();
                next.incoming_meld = None;
                next.draw = Some(begin.wall[state.core.num_drawn_head as usize]);
                next.num_drawn_head += 1;
            } else {
                // Someone called and will take the next turn instead.
                let meld = cache.meld[caller.to_usize()].unwrap();

                next.actor = caller;
                next.incoming_meld = Some(meld);
                if meld.is_kan() {
                    next.draw = Some(wall::kan_draw(&begin.wall, state.core.num_drawn_tail as usize));
                    next.num_drawn_tail += 1;
                } else {
                    next.draw = None
                }
            }

            // Check Furiten status for the discarding player.
            // furiten-by-discard == some tile in the waiting set exists in the discard set
            if !state.core.furiten[actor_i].miss_permanent {
                let discard_set = state.discard_sets[actor_i];
                let waiting_set = cache.wait[actor_i].waiting_set;
                debug_assert_eq!(discard_set, TileMask34::from_iter(
                    state.discards[actor_i].iter().map(|discard| discard.tile)));

                next.furiten[actor_i].by_discard = discard_set.0 & waiting_set.0 > 0;

                if log_enabled!(log::Level::Trace) && (discard_set.0 & waiting_set.0 > 0) {
                    log::trace!("P{} is in discard furiten.", actor_i);
                    log::trace!("P{} discard: {}", actor_i, discard_set);
                    log::trace!("P{} waiting: {}", actor_i, waiting_set);
                    log::trace!("P{} waiting details: {}", actor_i, cache.wait[actor_i]);
                }
            }
            // Temporary miss expires after discarding.
            next.furiten[actor_i].miss_temporary = false;

        }

        Action::Ankan(_) | Action::Kakan(_) => {
            // The current player has made an Ankan/Kakan and is entitled to a bonus turn.
            // The round has not ended => no reaction is possible on this.
            let ankan_or_kakan = cache.meld[actor_i].unwrap();

            next.actor = actor;
            next.incoming_meld = Some(ankan_or_kakan);
            next.draw = Some(wall::kan_draw(&begin.wall, state.core.num_drawn_tail as usize));
            next.num_drawn_tail += 1;

            // Only for Ankan: reveal the next dora indicator immediately.
            // For Kakan, it will only be revealed at the end of the next turn, in the same way
            // as Daiminkan (see above).
            if begin.ruleset.dora_allow_kan {
                if let Action::Ankan(_) = action {
                    next.num_dora_indicators += 1;
                }
            }
        }

        Action::TsumoAgari(_) | Action::AbortNineKinds => panic!()
    }

    // Any kind of meld will forcefully break any active riichi ippatsu.
    if next.incoming_meld.is_some() {
        for player in ALL_PLAYERS {
            next.riichi[player.to_usize()].is_ippatsu = false;
        }
    }

    // Check Furiten status for other players.
    // For another player who misses the chance to win (discard in waiting set):
    // - Immediately enters temporary miss state
    // - Immediately enters permanent miss state if under riichi
    let action_tile = action.tile().unwrap();
    for other_player in other_players_after(actor) {
        let other_player_i = other_player.to_usize();
        let furiten = &mut next.furiten[other_player_i];

        if furiten.miss_permanent { continue; }
        if let Action::Ankan(tile) = action {
            // Handle kokushi-ankan interaction.
            if begin.ruleset.kokushi_chankan_allow_ankan &&
                cache.wait[other_player_i].irregular == Some(
                    IrregularWait::ThirteenOrphans(tile)) {
                furiten.miss_temporary = true;
                furiten.miss_permanent = state.core.riichi[other_player_i].is_active;
            }
        } else {
            if cache.wait[other_player_i].waiting_set.has(action_tile) {
                furiten.miss_temporary = true;
                furiten.miss_permanent = state.core.riichi[other_player_i].is_active;
            }
        }
    }

    next
}

pub fn next_agari(
    begin: &RoundBegin,
    state: &State,
    action: Action,
    reactions: &[Option<Reaction>; 4],
    agari_kind: AgariKind,
    cache: &EngineCache,
) -> RoundEnd {
    let mut agari_result: [Option<AgariResult>; 4] = [None, None, None, None];
    let mut delta = [0; 4];
    let mut extra_dora_indicator = 0;

    // Workaround for a corner case:
    // 1. Kakan/Daiminkan
    // 2. Draw from the tail of the wall
    // 3. Discard
    // 4. Ron
    //
    // #2, #3, #4 are in the same turn. However, due to the Ron, we haven't triggered the delayed
    // reveaing logic in [`next_normal`], and instead got here. But the discard did happen!
    // To make up, we must reveal one more dora indicator, but only for Ron.
    if let Some(Meld::Kakan(_)) | Some(Meld::Daiminkan(_)) = state.core.incoming_meld {
        extra_dora_indicator = 1;
    }

    match agari_kind {
        AgariKind::Tsumo => {
            let winner = state.core.actor;
            let winning_tile = state.core.draw.unwrap();
            let agari_result_one = finalize_agari(
                begin, state, cache,
                winner, winner, winning_tile,
                true, 0,
            );
            delta = agari_result_one.points_delta;
            agari_result[winner.to_usize()] = Some(agari_result_one);
        }

        AgariKind::Ron => {
            // TODO(summivox): ruleset (atama-hane)
            let contributor = state.core.actor;
            let winning_tile = action.tile().unwrap();
            let mut take_pot = true;
            for winner in other_players_after(contributor) {
                if let Some(Reaction::RonAgari) = reactions[winner.to_usize()] {
                    let agari_result_one = finalize_agari(
                        begin, state, cache,
                        winner, contributor, winning_tile,
                        take_pot, extra_dora_indicator,
                    );
                    for i in 0..4 { delta[i] += agari_result_one.points_delta[i]; }
                    agari_result[winner.to_usize()] = Some(agari_result_one);
                    take_pot = false;
                }
            }
        }
    }

    // apply pot contributions in this round
    let pot_delta = calc_pot_delta(&state.core.riichi);
    for i in 0..4 { delta[i] += pot_delta[i]; }

    // apply delta to points
    let mut points = begin.points;
    for i in 0..4 { points[i] += delta[i]; }
    let renchan = agari_result[begin.round_id.button().to_usize()].is_some();

    // determine the next round
    let next_round_id = if renchan {
        begin.round_id.next_honba(true)
    } else {
        begin.round_id.next_kyoku()
    };
    // filter the game-end condition
    let next_round_id_or_end = next_round_id_or_game_end(
        &begin.ruleset,
        &points,
        begin.round_id.button(),
        renchan,
        next_round_id,
    );

    RoundEnd {
        round_result: ActionResult::Agari(agari_kind),
        pot: 0,
        points,
        points_delta: delta,
        renchan,
        next_round_id: next_round_id_or_end,
        agari_result,
    }
}

fn finalize_agari(
    begin: &RoundBegin,
    state: &State,
    cache: &EngineCache,
    winner: Player,
    contributor: Player,
    winning_tile: Tile,
    take_pot: bool,
    extra_dora_indicator: u8,
) -> AgariResult {
    let winner_i = winner.to_usize();
    let all_tiles = get_all_tiles(
        &state.closed_hands[winner_i],
        winning_tile,
        &state.melds[winner_i],
    );
    let dora_hits = count_doras(
        &begin.ruleset,
        &all_tiles,
        state.core.num_dora_indicators + extra_dora_indicator,
        &begin.wall,
        state.core.riichi[winner_i].is_active,
    );
    let candidates = &cache.win[winner.to_usize()];
    let mut best_candidate = candidates.iter().max_by_key(|candidate| {
        (Scoring { dora_hits, ..candidate.scoring }).basic_points()
    }).unwrap().clone();
    best_candidate.scoring.dora_hits = dora_hits;
    let mut delta = distribute_points(
        &begin.ruleset,
        begin.round_id,
        take_pot,
        winner,
        contributor,
        best_candidate.scoring.basic_points(),
    );
    if take_pot {
        delta[winner_i] += begin.pot + RIICHI_POT * num_active_riichi(state) as GamePoints;
    }
    AgariResult {
        winner,
        contributor,
        liable_player: winner,  // TODO(summivox): Pao
        points_delta: delta,
        details: best_candidate,
    }
}

pub fn next_abort(
    begin: &RoundBegin,
    state: &State,
    abort_reason: AbortReason,
    cache: &EngineCache,
) -> RoundEnd {
    let mut end = RoundEnd {
        round_result: ActionResult::Abort(abort_reason),
        pot: begin.pot + (num_active_riichi(state) as GamePoints * RIICHI_POT),
        points: begin.points,
        ..RoundEnd::default()
    };

    let round_id = begin.round_id;
    let button = round_id.button();
    // ugly syntax gets around array::map moving the Vec value
    let waiting = [0, 1, 2, 3].map(|i| cache.wait[i].waiting_set.any() as u8);
    let waiting_renchan = waiting[button.to_usize()] > 0;
    let next_round_id;
    match abort_reason {
        AbortReason::WallExhausted => {
            end.points_delta = calc_wall_exhausted_delta(waiting);
            end.renchan = waiting_renchan;
            next_round_id = round_id.next_honba(waiting_renchan);
        }
        AbortReason::NagashiMangan => {
            end.points_delta = calc_nagashi_mangan_delta(state, button);
            end.renchan = waiting_renchan;
            next_round_id = round_id.next_honba(waiting_renchan);
        }

        AbortReason::NineKinds | AbortReason::FourKan | AbortReason::FourWind |
        AbortReason::FourRiichi | AbortReason::DoubleRon | AbortReason::TripleRon => {
            // force renchan with honba + 1
            end.renchan = true;
            next_round_id = round_id.next_honba(true);
        }
    }

    for i in 0..4 { end.points[i] += end.points_delta[i]; }

    // filter the game-end condition
    end.next_round_id = next_round_id_or_game_end(
        &begin.ruleset,
        &end.points,
        begin.round_id.button(),
        end.renchan,
        next_round_id,
    );

    end
}

fn next_round_id_or_game_end(
    ruleset: &Ruleset,
    points: &[GamePoints; 4],
    button: Player,
    renchan: bool,
    next_round_id: RoundId,
) -> Option<RoundId> {
    if next_round_id.kyoku > ruleset.kyoku_max_hard {
        // hard stop
        None
    } else if next_round_id.kyoku > ruleset.kyoku_max_soft {
        // soft stop: sudden death
        if points.iter().all(|p| *p < ruleset.points_min_qualify) {
            Some(next_round_id)
        } else {
            None
        }
    } else if next_round_id.kyoku == ruleset.kyoku_max_soft && renchan &&
        other_players_after(button).iter()
            .all(|p| points[p.to_usize()] < points[button.to_usize()]) {
        None
    } else {
        Some(next_round_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_end_condition_examples() {
        let ruleset = &Ruleset {
            kyoku_max_soft: 7,
            kyoku_max_hard: 15,
            points_min_qualify: 30000,
            ..Ruleset::default()
        };

        assert_eq!(next_round_id_or_game_end(
            ruleset,
            &[26000, 24000, 26000, 24000],
            P0,
            true,
            RoundId { kyoku: 15, honba: 3 },
        ), Some(RoundId { kyoku: 15, honba: 3 }));
        assert_eq!(next_round_id_or_game_end(
            ruleset,
            &[26000, 24000, 26000, 24000],
            P0,
            true,
            RoundId { kyoku: 16, honba: 3 },
        ), None);

        assert_eq!(next_round_id_or_game_end(
            ruleset,
            &[30000, 20000, 26000, 24000],
            P0,
            true,
            RoundId { kyoku: 15, honba: 3 },
        ), None);

        assert_eq!(next_round_id_or_game_end(
            ruleset,
            &[26000, 24000, 26000, 24000],
            P3,
            true,
            RoundId { kyoku: 7, honba: 1 },
        ), Some(RoundId { kyoku: 7, honba: 1 }));

        assert_eq!(next_round_id_or_game_end(
            ruleset,
            &[26000, 24000, 20000, 30000],
            P3,
            false,
            RoundId { kyoku: 7, honba: 0 },
        ), Some(RoundId { kyoku: 7, honba: 0 }));

        assert_eq!(next_round_id_or_game_end(
            ruleset,
            &[26000, 24000, 20000, 30000],
            P3,
            true,
            RoundId { kyoku: 7, honba: 5 },
        ), None);
    }
}
