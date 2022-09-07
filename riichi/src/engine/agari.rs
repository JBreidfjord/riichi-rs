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
    pub begin: &'a RoundBegin,

    pub winner: Player,
    pub contributor: Player,
    pub action: Action,

    pub num_dora_indicators: u8,
    pub num_draws: u8,
    pub is_init_abortable: bool,
    pub closed_hand: &'a TileSet37,
    pub waiting_info: &'a WaitingInfo,
    pub riichi_flags: RiichiFlags,
    pub melds: &'a [Meld],
    pub incoming_meld: Option<Meld>,
}

pub fn make_input<'a>(
    begin: &'a RoundBegin,
    winner: Player,
    contributor: Player,
    action: Action,
    state: &'a State,
    waiting_info: &'a WaitingInfo,
) -> AgariInput<'a> {
    let winner_i = winner.to_usize();
    AgariInput {
        begin,
        winner,
        contributor,
        action,
        num_dora_indicators: state.num_dora_indicators,
        num_draws: num_draws(state),
        is_init_abortable: is_init_abortable(state),
        closed_hand: &state.closed_hands[winner_i],
        waiting_info,
        riichi_flags: state.riichi[winner_i],
        melds: &state.melds[winner_i],
        incoming_meld: state.incoming_meld,
    }
}

pub fn calc_best_agari_candidate(
    rules: &Rules,
    input: &AgariInput,
) -> Option<(AgariCandidate, GamePoints)> {
    let hand_common = calc_hand_common(rules, input);
    let mut best_basic_points: GamePoints = 0;
    let mut best_candidate: Option<AgariCandidate> = None;

    let regular_waits = input.waiting_info.regular.iter()
        .filter(|wait| wait.waiting_tile == hand_common.winning_tile)
        .map(|wait| (wait, calc_regular_wait_common(input, &hand_common, wait)));

    let irregular_wait = input.waiting_info.irregular.filter(|irregular|
        match irregular {
            IrregularWait::SevenPairs(t) | IrregularWait::ThirteenOrphans(t) =>
                *t == hand_common.winning_tile,
            IrregularWait::ThirteenOrphansAll => true,
        });

    for (regular_wait, wait_common) in regular_waits {
        if let Some((candidate, basic_points)) =
        calc_regular_agari_candidate(rules, input, &hand_common, regular_wait, &wait_common) {
            if basic_points > best_basic_points {
                best_basic_points = basic_points;
                best_candidate = Some(candidate);
            }
        }
    }

    if let Some(irregular) = irregular_wait {
        if let Some((candidate, basic_points)) =
        calc_irregular_agari_candidate(rules, input, &hand_common, irregular) {
            if basic_points > best_basic_points {
                best_basic_points = basic_points;
                best_candidate = Some(candidate);
            }
        }
    }

    best_candidate.map(|candidate| (candidate, best_basic_points))
}

fn calc_regular_agari_candidate(
    rules: &Rules,
    input: &AgariInput,
    hand_common: &HandCommon,
    regular_wait: &RegularWait,
    wait_common: &RegularWaitCommon,
) -> Option<(AgariCandidate, GamePoints)> {
    let mut yaku_builder = YakuBuilder::new();
    detect_yakus_for_regular(rules, &mut yaku_builder,
                             input, &hand_common, regular_wait, &wait_common);
    let yaku_values = yaku_builder.build();
    if yaku_values.is_empty() { return None; }
    let scoring = calc_scoring(rules,
                               &yaku_values,
                               Wait::Regular(*regular_wait),
                               hand_common.dora_hits,
                               hand_common.agari_kind,
                               hand_common.is_closed,
                               wait_common.extra_fu);
    Some((
        AgariCandidate {
            wait: Wait::Regular(*regular_wait),
            yaku_values,
            scoring,
        },
        scoring.basic_points(),
    ))
}

fn calc_irregular_agari_candidate(
    rules: &Rules,
    input: &AgariInput,
    hand_common: &HandCommon,
    irregular: IrregularWait,
) -> Option<(AgariCandidate, GamePoints)> {
    let mut yaku_builder = YakuBuilder::new();
    detect_yakus_for_irregular(rules, &mut yaku_builder,
                               input, &hand_common, irregular);
    let yaku_values = yaku_builder.build();
    if yaku_values.is_empty() { return None; }
    let scoring = calc_scoring(rules,
                               &yaku_values,
                               Wait::Irregular( input.waiting_info.irregular.unwrap()),
                               hand_common.dora_hits,
                               hand_common.agari_kind,
                               hand_common.is_closed,
                               0);
    Some((
        AgariCandidate {
            wait: Wait::Irregular(irregular),
            yaku_values,
            scoring,
        },
        scoring.basic_points(),
    ))
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
            yakuman_sum: (-value_sum) as u8,
            yaku_sum: 0,
            dora_hits,
            fu: 0,
        }
    } else if value_sum > 0 {
        Scoring {
            yakuman_sum: 0,
            yaku_sum: value_sum as u8,
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
        if self.yakuman_sum > 0 {
            return 8000 * self.yakuman_sum as GamePoints
        }
        match self.yaku_sum {
            0 => 0,
            1..=5 => min(2000, fu_han_formula(self.fu, self.han())),  // mangan or less
            6..=7 => 3000,  // haneman (1.5x mangan)
            8..=10 => 4000,  // baiman (2x mangan)
            11..=12 => 6000,  // sanbaiman (3x mangan)
            _ => 8000,  // kazoe-yakuman (4x mangan)
        }
    }

    pub fn basic_points_aotenjou(&self) -> GamePoints {
        fu_han_formula(self.fu, self.yakuman_sum * 13 + self.han())
    }
}

fn fu_han_formula(fu: u8, han: u8) -> GamePoints {
    fu as GamePoints * (1 << (2 + han as GamePoints))
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
    (fu_before_rounding + 9) / 10 * 10
}
