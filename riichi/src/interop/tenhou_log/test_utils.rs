use log::log_enabled;
use rand::prelude::*;

use crate::prelude::*;
use crate::engine::utils::calc_pot_delta;
use super::*;

/// Fully simulate/replay a [`RecoveredRound`] through our [`Engine`], validating the states along
/// the way. Useful for cross-checking the implementation of our [`Engine`].
pub fn run_a_round(num_reds: [u8; 3], recovered: &RecoveredRound, end_info: &TenhouEndInfo) {
    println!("\n\n{:?}", recovered.begin.round_id);
    let mut engine = Engine::new();

    let mut begin = recovered.begin.clone();
    let mut missing_tiles = wall::get_missing_tiles_in_partial_wall(
        &recovered.known_wall, num_reds);
    missing_tiles[..].shuffle(&mut thread_rng());
    begin.wall = wall::fill_missing_tiles_in_partial_wall(
        &recovered.known_wall, missing_tiles.into_iter());
    if log_enabled!(log::Level::Debug) {
        wall::print(&begin.wall);
    }

    engine.begin_round(begin);
    let mut action_result = ActionResult::Pass;
    for (seq, action_reaction) in recovered.action_reactions.iter().enumerate() {
        // println!("{}", engine.state().core);
        // println!("{}", action_reaction);
        assert_eq!(engine.state().core.seq, seq as u8);
        assert_eq!(engine.state().core.action_player, action_reaction.actor);
        engine.register_action(action_reaction.action).unwrap();
        if let Some((reactor, reaction)) = action_reaction.reactor_reaction {
            engine.register_reaction(reactor, reaction).unwrap();
        }
        if seq == recovered.action_reactions.len() - 1 {
            // handle multi-ron
            let mut multi_ron = recovered.multi_ron;
            if recovered.final_result == ActionResult::Abort(AbortReason::TripleRon) {
                for p in other_players_after(action_reaction.actor) {
                    multi_ron[p.to_usize()] = true;
                }
            }
            for i in 0..4 {
                if multi_ron[i] {
                    engine.register_reaction(
                        Player::new(i as u8),
                        Reaction::RonAgari,
                    ).unwrap();
                }
            }
        }
        action_result = engine.step();
    }
    match action_result {
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
    assert_eq!(action_result, recovered.final_result);
}
