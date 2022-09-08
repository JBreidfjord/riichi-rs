use std::fmt::{Display, Formatter};

use itertools::Itertools;

use crate::{
    common::*,
    model::*,
};
use super::{
    entry::*,
    TenhouRoundRaw,
};

#[derive(Clone, Debug)]
pub struct RecoveredRound {
    pub begin: RoundBegin,
    pub known_wall: PartialWall,
    pub action_reactions: Vec<ActionReaction>,
    pub final_result: ActionResult,
    pub multi_ron: [bool; 4],
}

pub fn recover_round(round: &TenhouRoundRaw) -> Option<RecoveredRound> {
    let mut recovered = RecoveredRound {
        begin: RoundBegin {
            rules: Default::default(),
            round_id: round.round_id_and_pot.round_id(),
            wall: wall::make_dummy_wall(),  // TODO(summivox): offer reconstruction for this
            pot: round.round_id_and_pot.pot_count as GamePoints * 1000,
            points: round.points,
        },
        known_wall: [None; 136],
        action_reactions: vec![],
        final_result: round.end_info.result,
        multi_ron: [false; 4],
    };
    // reveal the initial deal
    let deal = [&round.deal0, &round.deal1, &round.deal2, &round.deal3];
    let button = recovered.begin.round_id.button();
    for i in 0..4 {
        let player_i = button.wrapping_add(Player::new(i as u8)).to_usize();
        if deal[player_i].len() != 13 { return None; }
        for (j, draw) in deal[player_i].iter().enumerate() {
            if let &TenhouIncoming::Draw(tile) = draw {
                recovered.known_wall[wall::DEAL_INDEX[i][j]] = Some(tile);
            } else { panic!() }
        }
    }
    // reveal the dora indicators
    for (i, di) in round.dora_indicators.iter().enumerate() {
        if let TenhouIncoming::Draw(tile) = di {
            recovered.known_wall[wall::DORA_INDICATOR_INDEX[i]] = Some(*tile);
        }
    }
    for (i, udi) in round.ura_dora_indicators.iter().enumerate() {
        if let TenhouIncoming::Draw(tile) = udi {
            recovered.known_wall[wall::URA_DORA_INDICATOR_INDEX[i]] = Some(*tile);
        }
    }
    // Simulate the game.
    // This really is a parallel construction of our Engine, adapting Tenhou's conventions to ours.
    let incoming = [&round.incoming0, &round.incoming1, &round.incoming2, &round.incoming3];
    let outgoing = [&round.outgoing0, &round.outgoing1, &round.outgoing2, &round.outgoing3];
    let mut in_iter = incoming.map(|x| x.iter().peekable());
    let mut out_iter = outgoing.map(|x| x.iter());
    let mut actor = recovered.begin.round_id.button();
    let mut num_drawn_front = 52;
    let mut num_drawn_back = 0;
    let mut kan = false;
    loop {
        let actor_i = actor.to_usize();
        let mut draw: Option<Tile> = None;
        let mut daiminkan = false;
        match in_iter[actor_i].next() {
            Some(&TenhouIncoming::Draw(tile)) => {
                draw = Some(tile);
                if kan {
                    recovered.known_wall[wall::KAN_DRAW_INDEX[num_drawn_back]] = Some(tile);
                    num_drawn_back += 1;
                } else {
                    recovered.known_wall[num_drawn_front] = Some(tile);
                    num_drawn_front += 1;
                }
                kan = false;
            }
            Some(&TenhouIncoming::ChiiPonDaiminkan(meld)) => {
                match meld {
                    Meld::Chii(_) | Meld::Pon(_) => {}
                    Meld::Daiminkan(_) => { daiminkan = true }
                    _ => return None,
                }
            }
            _ => {
                // The round has ended due to RonAgari or an end-of-turn abort condition.
                if round.end_info.result == ActionResult::Agari(AgariKind::Ron) {
                    // backfill (multi-)ron reaction(s)
                    if let Some(last) = recovered.action_reactions.last_mut() {
                        for agari in round.end_info.agari.iter() {
                            if last.reactor_reaction.is_none() {
                                last.reactor_reaction = Some((agari.winner, Reaction::RonAgari));
                            }
                            recovered.multi_ron[agari.winner.to_usize()] = true;
                        }
                    } else { return None; }
                }
                break
            }
        }
        match out_iter[actor_i].next() {
            Some(&TenhouOutgoing::DaiminkanDummy) | None if daiminkan => {
                // Reason for `None`: Tenhou will elide a DaiminkanDummy if it's the last one in the
                // series. We will tolerate this and reuse the same logic.
                kan = true;
                continue;
            }
            Some(&TenhouOutgoing::Discard(mut discard)) => {
                let mut next_actor = player_succ(actor);
                let mut reactor_reaction = None;
                if discard.is_tsumokiri { discard.tile = draw?; }
                for reactor in other_players_after(actor) {
                    let reactor_i = reactor.to_usize();
                    if let Some(TenhouIncoming::ChiiPonDaiminkan(meld)) = in_iter[reactor_i].peek() {
                        if meld.called() != Some(discard.tile) { continue; }
                        if let Some(dir) = meld.dir() {
                            if reactor.wrapping_add(dir) != actor { continue; }
                        } else { continue; }
                        next_actor = reactor;
                        discard.called_by = reactor;
                        reactor_reaction = Reaction::from_meld(*meld)
                            .map(|reaction| (reactor, reaction));
                    }
                }
                recovered.action_reactions.push(ActionReaction {
                    actor,
                    action: Action::Discard(discard),
                    reactor_reaction,
                });
                actor = next_actor;
            }
            Some(&TenhouOutgoing::KakanAnkan(meld)) => {
                recovered.action_reactions.push(ActionReaction {
                    actor,
                    action: Action::from_meld(meld)?,
                    reactor_reaction: None,
                });
                kan = true;
            }
            _ => {
                // The round has ended due to TsumoAgari or NineKinds, both requiring a final
                // explicit action (and no possible reaction).
                recovered.action_reactions.push(ActionReaction {
                    actor,
                    action: match recovered.final_result {
                        ActionResult::Agari(AgariKind::Tsumo) => Action::TsumoAgari(draw?),
                        ActionResult::Abort(AbortReason::NineKinds) => Action::AbortNineKinds,
                        _ => return None
                    },
                    reactor_reaction: None,
                });
                break
            }
        }
        /*
        // TODO DEBUG
        println!("[{}] = {}",
                 recovered.action_reactions.len() - 1,
                 recovered.action_reactions.last().unwrap());
         */
    }
    // At this point we should have walked through all entries; there shouldn't be any more.
    if !in_iter.iter_mut().all(|it| it.peek() == None) {
        println!("ERROR: likely corrupted file (too many incomings).");
        return None;
    }
    if !out_iter.iter_mut().all(|it| it.next() == None) {
        println!("ERROR: likely corrupted file (too many outgoings).");
        return None;
    }
    Some(recovered)
}

// Pretty printer for debugging only
impl Display for RecoveredRound {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "kyoku={}, honba={}, pot={}, points={:?}, result={:?}, multi_ron={:?}",
                 self.begin.round_id.kyoku,
                 self.begin.round_id.honba,
                 self.begin.pot,
                 self.begin.points,
                 self.final_result,
                 self.multi_ron,
        )?;
        // TODO(summivox): dedupe with `wall::print_partial`
        for x in &self.known_wall.iter().chunks(8) {
            for y in x {
                if let Some(tile) = y {
                    write!(f, "{} ", tile)?;
                } else {
                    write!(f, "?? ")?;
                }
            }
            writeln!(f)?;
        }
        for action_reaction in self.action_reactions.iter() {
            writeln!(f, "{}", action_reaction)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_some_example() {
        let round_json = serde_json::json!([
          [2, 0, 0],
          [22000, 22000, 27000, 29000],
          [15, 16, 46, 12, 28],
          [],

          [14, 14, 51, 17, 21, 27, 29, 31, 32, 32, 53, 44, 45],
          [33, 52, 44, 35, 42, 42, 25, 15, 44, 16, 37, "c343233", "c161451", 19, 37, 34, 32, 43, 18],
          [21, 44, 45, 44, 29, 42, 42, 31, 60, 27, 60, 32, 14, 60, 60, 60, 60, 60, 60],

          [12, 12, 23, 24, 24, 25, 36, 38, 41, 43, 44, 45, 47],
          [22, 36, 43, 26, 31, 16, 13, 12, 22, 41, 13, 35, "c373536", 22, 42, 38, 27],
          [43, 45, 60, 47, 44, 41, 31, 38, 16, 13, 41, 22, 36, 13, 60, 60, 22],

          [13, 14, 18, 18, 23, 25, 31, 36, 37, 43, 45, 45, 46],
          [47, 33, "p454545", 27, 26, 38, 33, 11, 26, 37, 32, 18, 13, 39, 26, 19, 24, 28],
          [43, 31, 33, 23, 36, 60, 37, 33, 47, 60, 60, 26, 11, 60, 60, 13, 60, 60],

          [11, 11, 19, 21, 23, 28, 29, 29, 29, 31, 39, 41, 46],
          [21, "2121p21", 24, 36, 17, "292929m29", 42, 47, 39, 27, 34, 15, 34, 21, 16, 11, "m11111111", 47, "p393939", 39, 22, 17, 33, 19],
          [28, 23, 60, 60, 41, 0, 60, 60, 17, 60, 60, 60, 60, "2121k2121", 60, 46, 0, 60, 31, "k39393939", 60, 60, 60],

          ["和了", [-16000, -16000, -32000, 64000],
            [3, 3, 3, "役満16000-32000点", "四槓子(役満)", "清老頭(役満)"]
          ]
        ]);
        let raw_round: TenhouRoundRaw = serde_json::from_value(round_json).unwrap();
        let recovered = recover_round(&raw_round).unwrap();
        println!("{}", recovered);
    }
}
