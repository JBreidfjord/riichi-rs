mod hand_common;
mod regular_wait_common;
mod yaku_detectors;

use std::cmp::min;
use itertools::Itertools;
use riichi_decomp_table::WaitingKind;
use crate::{
    analysis::{IrregularWait, RegularWait, Wait},
    common::*,
    engine::{
        utils::*,
        WaitingInfo,
    },
    model::*,
    Rules,
};
use self::{
    hand_common::*,
    regular_wait_common::*,
    yaku_detectors::*,
};

#[derive(Debug)]
pub struct AgariInput<'a> {
    pub round_id: RoundId,

    // from the winner
    pub winner: Player,
    pub closed_hand: &'a TileSet37,
    pub riichi_flags: RiichiFlags,
    pub melds: &'a [Meld],
    pub waiting_info: &'a WaitingInfo,

    // from the contributor
    pub contributor: Player,
    pub winning_tile: Tile,
    pub incoming_is_kan: bool,
    pub action_is_kan: bool,

    // from the table
    pub is_first_chance: bool,
    pub is_last_draw: bool,
}

pub fn make_agari_input<'a>(
    round_id: RoundId,
    state: &'a State,
    waiting_info: &'a WaitingInfo,
    action: Action,
    winner: Player,
    contributor: Player,
) -> AgariInput<'a> {
    let winner_i = winner.to_usize();
    AgariInput {
        round_id,

        winner,
        closed_hand: &state.closed_hands[winner_i],
        riichi_flags: state.riichi[winner_i],
        melds: &state.melds[winner_i],
        waiting_info,

        contributor,
        winning_tile: action.tile().unwrap(),  // assumed not NineKinds
        // TODO(summivox): rust (is_some_with)
        incoming_is_kan: state.incoming_meld.filter(|m| m.is_kan()).is_some(),
        action_is_kan: action.is_kan(),

        is_first_chance: is_first_chance(state),
        is_last_draw: is_last_draw(state),
    }
}

pub fn agari_candidates(
    rules: &Rules,
    input: &AgariInput,
) -> Vec<AgariCandidate> {
    let hand_common = calc_hand_common(rules, input);

    let regular_waits = input.waiting_info.regular.iter()
        .filter(|wait|
            wait.waiting_tile == input.winning_tile)
        .map(|wait|
            (wait, calc_regular_wait_common(rules, input, &hand_common, wait)));

    let irregular_wait = input.waiting_info.irregular.filter(|irregular|
        match irregular {
            IrregularWait::SevenPairs(t) | IrregularWait::ThirteenOrphans(t) =>
                *t == input.winning_tile,
            IrregularWait::ThirteenOrphansAll => true,
        });

    let mut candidates = regular_waits
        .filter_map(|(regular_wait, wait_common)|
            calc_regular_agari_candidate(rules, input, &hand_common, regular_wait, &wait_common))
        .collect_vec();
    candidates.extend(irregular_wait
        .and_then(|irregular|
            calc_irregular_agari_candidate(rules, input, &hand_common, irregular)));
    candidates
}

fn calc_regular_agari_candidate(
    rules: &Rules,
    input: &AgariInput,
    hand_common: &HandCommon,
    regular_wait: &RegularWait,
    wait_common: &RegularWaitCommon,
) -> Option<AgariCandidate> {
    let mut yaku_builder = YakuBuilder::new();
    detect_yakus_for_regular(rules, &mut yaku_builder,
                             input, hand_common, regular_wait, wait_common);
    let yaku_values = yaku_builder.build();
    if yaku_values.is_empty() { return None; }
    let scoring = calc_scoring(rules,
                               &yaku_values,
                               Wait::Regular(*regular_wait),
                               DoraHits::default(),  // ignored for now
                               hand_common.agari_kind,
                               hand_common.is_closed,
                               wait_common.extra_fu);
    Some(AgariCandidate {
        wait: Wait::Regular(*regular_wait),
        yaku_values,
        scoring,
    })
}

fn calc_irregular_agari_candidate(
    rules: &Rules,
    input: &AgariInput,
    hand_common: &HandCommon,
    irregular: IrregularWait,
) -> Option<AgariCandidate> {
    let mut yaku_builder = YakuBuilder::new();
    detect_yakus_for_irregular(rules, &mut yaku_builder,
                               input, hand_common, irregular);
    let yaku_values = yaku_builder.build();
    if yaku_values.is_empty() { return None; }
    let scoring = calc_scoring(rules,
                               &yaku_values,
                               Wait::Irregular( input.waiting_info.irregular.unwrap()),
                               DoraHits::default(),  // ignored for now
                               hand_common.agari_kind,
                               hand_common.is_closed,
                               0);
    Some(AgariCandidate {
        wait: Wait::Irregular(irregular),
        yaku_values,
        scoring,
    })
}

fn calc_scoring(
    rules: &Rules,
    yaku_values: &YakuValues,
    wait: Wait,
    dora_hits: DoraHits,
    agari_kind: AgariKind,
    is_closed: bool,
    extra_fu: u8,
) -> Scoring {
    let value_sum = yaku_values.values().sum::<i8>();
    if value_sum < 0 {
        Scoring {
            yakuman_total_value: (-value_sum) as u8,
            yaku_total_value: 0,
            dora_hits,
            fu: 0,
        }
    } else if value_sum > 0 {
        Scoring {
            yakuman_total_value: 0,
            yaku_total_value: value_sum as u8,
            dora_hits,
            fu: match wait {
                Wait::Irregular(IrregularWait::SevenPairs(_)) => 25,
                _ => calc_regular_fu(rules, agari_kind, is_closed, extra_fu),
            },
        }
    } else { panic!() }
}

impl Scoring {
    pub fn basic_points(&self) -> GamePoints {
        if self.yakuman_total_value > 0 {
            return 8000 * self.yakuman_total_value as GamePoints
        }
        match self.yaku_total_value {
            0 => 0,
            // TODO(summivox): rust (DivCeil)
            1..=5 => min(2000,
                         fu_han_formula_rounded(self.fu, self.han())),  // mangan or less
            6..=7 => 3000,  // haneman (1.5x mangan)
            8..=10 => 4000,  // baiman (2x mangan)
            11..=12 => 6000,  // sanbaiman (3x mangan)
            _ => 8000,  // kazoe-yakuman (4x mangan)
        }
    }

    pub fn basic_points_aotenjou(&self) -> GamePoints {
        fu_han_formula_rounded(self.fu, self.yakuman_total_value * 13 + self.han())
    }
}

fn fu_han_formula_raw(fu: u8, han: u8) -> GamePoints {
    fu as GamePoints * (1 << (2 + han as GamePoints))
}
fn fu_han_formula_rounded(fu: u8, han: u8) -> GamePoints {
    (fu_han_formula_raw(fu, han) + 99) / 100 * 100
}

/// See: <https://riichi.wiki/Fu>
fn calc_regular_fu(
    _rules: &Rules,
    agari_kind: AgariKind,
    is_closed: bool,
    extra_fu: u8,
) -> u8 {
    use AgariKind::*;
    static TABLE: [[[u8; 2]; 2]; 2] = [
        // [open ron, closed ron], [open tsumo, closed tsumo]
        [  [30,       30        ], [30,         20          ]],  // pinfu-style
        [  [20,       30        ], [22,         22          ]],  // not pinfu
    ];
    let fu_before_rounding = extra_fu + TABLE
        [match extra_fu { 0 => 0, _ => 1 }]
        [match agari_kind { Ron => 0, Tsumo => 1 }]
        [is_closed as usize];
    // TODO(summivox): rust (DivCeil)
    (fu_before_rounding + 9) / 10 * 10
}

#[cfg(test)]
mod tests;
