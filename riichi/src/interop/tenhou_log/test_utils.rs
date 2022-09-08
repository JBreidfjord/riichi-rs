use crate::prelude::*;
use super::*;

pub fn run_a_round(num_reds: [u8; 3], recovered: &RecoveredRound, end_info: &TenhouEndInfo) {
    println!("\n\n{:?}", recovered.begin.round_id);
    let mut engine = Engine::new();

    let mut begin = recovered.begin.clone();
    let missing_tiles = wall::get_missing_tiles_in_partial_wall(
        &recovered.known_wall, num_reds);
    // no shuffle --- just put them back for now
    begin.wall = wall::fill_missing_tiles_in_partial_wall(
        &recovered.known_wall, missing_tiles.into_iter());
    // wall::print(&begin.wall);

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
        ActionResult::Abort(_abort_reason) => {
            // println!("engine says: {:?}", _abort_reason);
        }
        ActionResult::Agari(_agari_kind) => {
            // println!("engine says: {:?}", _agari_kind);
            let end = engine.end().clone().unwrap();
            println!("{:?}", end.agari_result);
            // Exclude cases where Pao / Liability apply.
            if end_info.agari.iter().all(|x| x.liable_player == x.winner) {
                assert_eq!(end.points_delta, end_info.overall_delta);
            }
        }
        _ => {}
    }
    assert_eq!(action_result, recovered.final_result);
}
