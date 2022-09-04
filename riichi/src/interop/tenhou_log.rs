mod end_info;
mod entry;
mod meld;
mod round;
mod strings;
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
    // reveal initial deal
    let deal = [&round.deal0, &round.deal1, &round.deal2, &round.deal3];
    let button = dbg!(recovered.begin.round_id.button());
    for i in 0..4 {
        let p = button.wrapping_add(Player::new(i as u8)).to_usize();
        if deal[p].len() != 13 { return None; }
        for (j, draw) in deal[p].iter().enumerate() {
            if let &TenhouIncoming::Draw(tile) = draw {
                recovered.known_wall[wall::DEAL_INDEX[i][j]] = Some(tile);
            } else { panic!() }
        }
    }
    // reveal dora indicators
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
            _ => {
                // TODO(summivox): tenhou6 end condition
                println!("ron/wall-exhaust/4wind/4riichi/4kan");
                break
            }
        }
        match out_iter[actor_i].next() {
            Some(&TenhouOutgoing::DaiminkanDummy) => {
                kan = true;
                continue;
            }
            Some(&TenhouOutgoing::Discard(mut discard)) => {
                let mut next_actor = player_succ(actor);
                let mut recovered_reactor = actor;
                let mut recovered_reaction: Option<Reaction> = None;
                if discard.is_tsumokiri { discard.tile = draw?; }
                for reactor in other_players_after(actor) {
                    let reactor_i = reactor.to_usize();
                    if let Some(TenhouIncoming::ChiiPonDaiminkan(meld)) = in_iter[reactor_i].peek() {
                        if meld.called() != Some(discard.tile) { continue; }
                        if let Some(dir) = meld.dir() {
                            if reactor.wrapping_add(dir) != actor { continue; }
                        } else { continue; }
                        next_actor = reactor;
                        recovered_reactor = reactor;
                        recovered_reaction = Reaction::from_meld(*meld);
                    }
                }
                recovered.action_reactions.push(ActionReaction {
                    actor,
                    action: Action::Discard(discard),
                    reactor: recovered_reactor,
                    reaction: recovered_reaction,
                });
                actor = next_actor;
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
            _ => {
                // TODO(summivox): tenhou6 end condition
                println!("tsumo/99");
                break
            }
        }
    }
    assert!(in_iter.iter_mut().all(|it| it.peek() == None));
    Some(recovered)
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use super::*;

    #[test]
    fn print_some_example() {
        let round_json = serde_json::json!([
      [0, 0, 0],
      [25000, 25000, 25000, 25000],
      [33],
      [],

      [13, 13, 17, 21, 24, 25, 31, 32, 33, 33, 36, 42, 44],
      [42, 11, 47, 17, 47, 16, 31, 52, 23, 14, 44, 23],
      [44, 21, 60, 42, 42, 47, 11, 31, 33, 36, 60, 13],

      [12, 19, 19, 21, 25, 26, 28, 41, 42, 43, 43, 46, 47],
      [12, 19, 38, 22, 11, 22, 12, 17, 43, 31],
      [28, 21, 60, 60, 42, 60, 47, 26, 25, 60],

      [13, 15, 51, 17, 26, 32, 35, 53, 38, 41, 41, 43, 45],
      [26, 16, 13, 34, 22, 15, 35, 41, 47, 29, "13p1313"],
      [38, 32, 43, 45, 60, 35, 60, 26, 60, 60, 26],

      [12, 16, 21, 23, 23, 24, 27, 27, 34, 36, 36, 37, 45],
      [11, 18, 45, 46, 32, 16, 15, 28, 44, "3636p36", 18, 36],
      [45, 11, 60, 60, 60, 37, 21, 60, 60, 24, 12, "3636k3636"],

      ["和了", [0, 0, 8000, -8000],
        [2, 3, 2, "満貫8000点", "槍槓(1飜)", "場風 東(1飜)", "ドラ(1飜)", "赤ドラ(2飜)"]
      ]
    ]);
        let raw_round: TenhouRoundRaw = serde_json::from_value(round_json).unwrap();
        let recovered = recover_round(&raw_round).unwrap();
        println!("{:?}", (recovered.begin.round_id, recovered.begin.pot, recovered.begin.points));
        for x in &recovered.known_wall.iter().chunks(8) {
            for y in x {
                if let Some(tile) = y {
                    print!("{} ", tile);
                } else {
                    print!("?? ");
                }
            }
            println!();
        }
        for ar in recovered.action_reactions {
            println!("{:?}", ar);
        }
    }
}
