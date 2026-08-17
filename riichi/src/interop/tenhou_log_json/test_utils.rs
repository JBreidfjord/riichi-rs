use itertools::Itertools;
use rand::prelude::*;

use crate::prelude::*;
use crate::engine::utils::calc_pot_delta;
use super::*;

/// Fully simulate/replay a [`RecoveredRound`] through our [`Engine`], validating the states along
/// the way. Useful for cross-checking the implementation of our [`Engine`].
///
/// # This is a real oracle, not a smoke test
///
/// Worth stating plainly, because the caller in `tests/tenhou_log.rs` writes a `.txt` dump and it
/// is easy to mistake the dump for the point. Every replayed round asserts:
///
/// - the engine's `seq` and `actor` match the log's, at every step;
/// - every logged action and reaction is **accepted** by `check_action` / `check_reaction`
///   (the `unwrap`s below) -- i.e. the engine agrees with Tenhou on legality;
/// - the final `ActionResult` matches what the log concluded;
/// - on a win, the whole points delta matches `end_info.overall_delta` (Pao excepted, which the
///   engine does not model).
///
/// # Using this for sanma
///
/// The variant comes off `recovered.history.begin.ruleset`, so the driver itself is ready. What
/// is not ready is the *input*: `RecoveredRound` is produced by `recover_round` from tenhou/6
/// **JSON**, and that format has no established encoding for a Kita -- see `to_tenhou_meld`,
/// which refuses to guess one. A sanma oracle therefore needs a `RecoveredRound` built from the
/// `mjlog` XML path (where a nuki is its own `<N>` element) rather than through tenhou/6 JSON.
pub fn run_a_round(
    num_reds: [u8; 3],
    recovered: &RecoveredRound,
    end_info: &TenhouEndInfo
) -> RoundHistory {
    let lite = &recovered.history;
    let variant = lite.begin.ruleset.variant;
    println!("\n{:?} ({:?})", lite.begin.round_id, variant);

    let mut engine = Engine::new();

    let mut begin = lite.begin.clone();
    let mut missing_tiles = wall::get_missing_tiles_in_partial_wall_in(
        variant, &recovered.known_wall, num_reds).iter_tiles().collect_vec();
    missing_tiles[..].shuffle(&mut thread_rng());
    begin.wall = wall::fill_missing_tiles_in_partial_wall_in(
        variant, &recovered.known_wall, missing_tiles.into_iter());
    log::debug!("{}", begin.wall.display());

    let mut full = RoundHistory {
        begin: begin.clone(),
        steps: vec![],
        ron: lite.ron,
    };

    engine.begin_round(begin);
    let mut last_step = None;
    for (seq, action_reaction) in lite.action_reactions.iter().enumerate() {
        // println!("{}", engine.state().core);
        // println!("{}", action_reaction);
        assert_eq!(engine.state().core.seq, seq as u8);
        assert_eq!(engine.state().core.actor, action_reaction.actor);
        engine.register_action(action_reaction.action).unwrap();
        if let Some((reactor, reaction)) = action_reaction.reactor_reaction {
            engine.register_reaction(reactor, reaction).unwrap();
        }
        if seq == lite.action_reactions.len() - 1 {
            // handle multi-ron
            let ron = &mut full.ron;
            if recovered.final_result == ActionResult::Abort(AbortReason::TripleRon) {
                for p in variant.other_active_players_after(action_reaction.actor) {
                    ron[p.to_usize()] = true;
                }
            }
            for i in 0..4 {
                if ron[i] {
                    engine.register_reaction(
                        Player::new(i as u8),
                        Reaction::RonAgari,
                    ).unwrap();
                }
            }
        }
        let step = engine.step();
        full.steps.push(step.clone());
        last_step = Some(step);
    }
    if let Some(step) = last_step {
        match step.action_result {
            ActionResult::Abort(abort_reason) => {
                log::info!("engine says: {:?}", abort_reason);
            }
            ActionResult::Agari(agari_kind) => {
                log::info!("engine says: {:?}", agari_kind);

                let end = engine.end().clone().unwrap();
                println!("{:?}", end.agari_result);

                // Deduct newly added pot from players under riichi.
                // They are not included anyway.
                let mut delta = end.points_delta;
                let pot_delta = calc_pot_delta(&engine.state().core.riichi);
                for i in 0..4 { delta[i] -= pot_delta[i]; }

                // Exclude cases where Pao / Liability apply.
                if end_info.agari.iter().all(|x| x.liable_player == x.winner) {
                    assert_eq!(delta, end_info.overall_delta);
                }
            }
            _ => {}
        }
        assert_eq!(step.action_result, recovered.final_result);
    }
    full
}
