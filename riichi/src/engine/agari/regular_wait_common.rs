use std::cmp::min;

use riichi_decomp_table::WaitingKind;

use crate::{
    common::*,
    model::*,
    rules::Ruleset,
};
use super::{AgariKind, AgariInput, HandCommon, RegularWait};

#[derive(Clone, Debug)]
pub struct RegularWaitCommon {
    pub wait_group: Option<HandGroup>,
    pub extra_fu: u8,
}

pub fn calc_regular_wait_common(
    _ruleset: &Ruleset,
    input: &AgariInput,
    hand_common: &HandCommon,
    wait: &RegularWait,
) -> RegularWaitCommon {
    let wait_group = calc_waiting_group(wait);
    let extra_fu = calc_extra_fu(
        input.round_id,
        input.winner,
        input.melds,
        hand_common,
        wait,
        wait_group,
    );
    RegularWaitCommon {
        wait_group,
        extra_fu,
    }
}

fn calc_waiting_group(wait: &RegularWait) -> Option<HandGroup> {
    use HandGroup::*;
    use WaitingKind::*;
    let t_pat = wait.pattern_tile;
    let t_wait = wait.waiting_tile;
    match wait.waiting_kind {
        Tanki => None,
        Shanpon => Some(Koutsu(t_pat)),
        Kanchan => Some(Shuntsu(t_pat)),
        RyanmenHigh | RyanmenLow | RyanmenBoth => Some(Shuntsu(min(t_wait, t_pat))),
    }
}

fn calc_extra_fu(
    round_id: RoundId,
    winner: Player,
    melds: &[Meld],
    hand_common: &HandCommon,
    wait: &RegularWait,
    wait_group: Option<HandGroup>,
) -> u8 {
    // known open groups (melds)
    let meld_fu: u8 =
        melds.iter().map(meld_extra_fu).sum();

    // known closed groups (melds)
    let group_fu: u8 =
        wait.groups().map(group_extra_fu).sum::<u8>() * 2;  // x2 because these are always closed

    // the waiting group (non-pair) is considered either open (if ron) or closed (if tsumo)
    let wait_group_fu =
        if let Some(g) = wait_group {
            group_extra_fu(g) * kind_fu_multiplier(hand_common.agari_kind)
        } else { 0 };

    // known waiting kind
    let wait_fu =
        if wait.is_true_ryanmen() || wait.waiting_kind == WaitingKind::Shanpon { 0 } else { 2 };

    // pair of yakuhai (dragons + {self,prevalent} winds)
    let yakuhai_pair_fu = if let Some(pair) = wait.pair_or_tanki() {
        if pair.is_dragon() {
            2
        } else if let Some(wind) = pair.wind() {
            // TODO(summivox): rules (double wind pair fu)
            2 * ((wind == round_id.prevailing_wind()) as u8 +
                 (wind == round_id.self_wind_for_player(winner)) as u8)
        } else { 0 }
    } else { 0 };

    meld_fu + group_fu + wait_group_fu + wait_fu + yakuhai_pair_fu
}

fn group_extra_fu(group: HandGroup) -> u8 {
    match group {
        HandGroup::Koutsu(k) => 2 * terminal_fu_multiplier(k),
        HandGroup::Shuntsu(_) => 0,
    }
}

fn meld_extra_fu(meld: &Meld) -> u8 {
    match meld {
        Meld::Chii(_) => 0,
        Meld::Pon(pon) => 2 * terminal_fu_multiplier(pon.called),
        Meld::Kakan(kakan) => 8 * terminal_fu_multiplier(kakan.added),
        Meld::Daiminkan(daiminkan) => 8 * terminal_fu_multiplier(daiminkan.called),
        Meld::Ankan(ankan) => 16 * terminal_fu_multiplier(ankan.own[0]),
    }
}

fn kind_fu_multiplier(agari_kind: AgariKind) -> u8 {
    match agari_kind {
        AgariKind::Ron => 1,
        AgariKind::Tsumo => 2,
    }
}

fn terminal_fu_multiplier(tile: Tile) -> u8 {
    if tile.is_terminal() { 2 } else { 1 }
}
