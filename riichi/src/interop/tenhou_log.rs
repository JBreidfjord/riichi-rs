mod entry;
mod meld;
mod round;
mod tile;

use once_cell::sync::OnceCell;
use regex::Regex;
use serde::{
    ser::{Serialize, Serializer},
    de::{Deserialize, Deserializer, Error, Visitor},
};

use crate::{
    common::*,
    model::*,
    utils::*,
};
pub use self::{
    entry::*,
    meld::*,
    round::*,
    tile::*,
};

#[derive(Clone, Debug)]
pub struct RecoveredRound {
    pub begin: RoundBegin,
    pub action_reactions: Vec<ActionReaction>,
    pub known_wall: [Option<Tile>; 136],
}

#[derive(Copy, Clone, Debug)]
pub struct ActionReaction {
    pub actor: Player,
    pub action: Action,
    pub reactor: Player,
    pub reaction: Option<Reaction>,
}

pub fn recover_round(round: &TenhouRoundRaw) -> Option<RecoveredRound> {
    let mut recovered = RecoveredRound {
        begin: RoundBegin {
            rules: Default::default(),
            round_id: round.round_id_and_pot.round_id(),
            wall: wall::make_dummy_wall(),
            pot: round.round_id_and_pot.pot,
            points: round.points,
        },
        action_reactions: vec![],
        known_wall: [None; 136],
    };
    // deal
    let deal = [&round.deal0, &round.deal1, &round.deal2, &round.deal3];
    for p in 0..4 {
        if deal[p].len() != 13 { return None; }
        for (i, draw) in deal[p].iter().enumerate() {
            if let &TenhouIncoming::Draw(tile) = draw {
                recovered.known_wall[wall::DEAL_INDEX[p][i]] = Some(tile);
            } else { panic!() }
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
                    Meld::Daiminkan(_) => {}
                    _ => return None,
                }
            }
            _ => break
        }
        match out_iter[actor_i].next() {
            Some(&TenhouOutgoing::DaiminkanDummy) => {
                kan = true;
                continue;
            }
            Some(&TenhouOutgoing::Discard(mut discard)) => {
                let mut next_actor = player_succ(actor);
                let mut action_reaction = ActionReaction {
                    actor,
                    action: Action::Discard(discard),
                    reactor: actor,
                    reaction: None,
                };
                if discard.is_tsumokiri { discard.tile = draw?; }
                for reactor in other_players_after(actor) {
                    let reactor_i = reactor.to_usize();
                    if let Some(TenhouIncoming::ChiiPonDaiminkan(meld)) = in_iter[reactor_i].peek() {
                        if meld.called() != Some(discard.tile) { continue; }
                        if let Some(dir) = meld.dir() {
                            if reactor.wrapping_add(dir) != actor { continue; }
                        } else { continue; }
                        next_actor = reactor;
                        action_reaction.reactor = reactor;
                        action_reaction.reaction = Reaction::from_meld(*meld);
                    }
                }
                actor = next_actor;
                recovered.action_reactions.push(action_reaction);
            }
            Some(&TenhouOutgoing::KakanAnkan(meld)) => {
                recovered.action_reactions.push(ActionReaction {
                    actor,
                    action: Action::from_meld(meld)?,
                    reactor: actor,
                    reaction: None,  // TODO(summivox): chankan
                });
                kan = true;
            }
            _ => break
        }
    }

    Some(recovered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_some_example() {
        let round_json = serde_json::json!([
          [3, 2, 2],
          [45200, 19500, 7300, 26000],
          [39, 18],
          [],

          [11, 11, 14, 17, 26, 27, 31, 34, 35, 38, 44, 45, 46],
          [21, 23, 43, 26, 41, 12, 21, 39, 46],
          [60, 31, 44, 43, 23, 17, 14, 46, 60],

          [11, 12, 14, 17, 25, 25, 31, 32, 32, 35, 36, 37, 41],
          [28, 24, 39, 13, "c232425", 13, 38, 36, 37],
          [11, 41, 28, 39, 31, 60, 17, 25],

          [19, 19, 19, 21, 23, 23, 24, 52, 27, 27, 32, 43, 46],
          [42, 26, 34, 22, 17, 19, 14, 13, 28],
          [43, 42, 46, "r23", 60, "191919a19", 60, 60, 60],

          [15, 15, 16, 21, 24, 29, 31, 33, 34, 38, 42, 43, 47],
          [28, 18, 44, 32, 43, 18, 46, 22, 33],
          [21, 42, 60, 43, 60, 31, 60, 16, 28],

          ["和了", [-500, 4700, -500, -700], [1, 1, 1, "30符1飜300-500点", "断幺九(1飜)"]]
        ]);
        let raw_round: TenhouRoundRaw = serde_json::from_value(round_json).unwrap();
        let recovered = recover_round(&raw_round);
        println!("{:?}", recovered);
    }
}
