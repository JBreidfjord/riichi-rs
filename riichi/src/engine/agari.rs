mod hand_common;
mod regular_wait_common;
mod yaku_detector;

use itertools::Itertools;
use riichi_decomp_table::WaitingKind;
use crate::{
    common::*,
    engine::{
        utils::*,
        WaitingInfo,
    },
    model::*,
    Rules
};
use crate::analysis::{IrregularWait, RegularWait};
use self::{
    hand_common::*,
    regular_wait_common::*,
    yaku_detector::*,
};

pub fn calc_agari(input: &AgariInput) -> Option<AgariResult> {
    let hand_common = calc_hand_common(input);
    let regular_waits = input.waiting_info.regular.iter()
        .filter(|wait| wait.waiting_tile == input.winning_tile)
        .map(|wait| (wait, calc_regular_wait_common(input, &hand_common, wait)));
    unimplemented!()
}

#[derive(Debug)]
pub struct AgariInput<'a> {
    pub begin: &'a RoundBegin,

    pub winner: Player,
    pub contributor: Player,
    pub winning_tile: Tile,

    pub num_dora_indicators: u8,
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
    winning_tile: Tile,
    state: &'a State,
    waiting_info: &'a WaitingInfo,
) -> AgariInput<'a> {
    let winner_i = winner.to_usize();
    AgariInput {
        begin,
        winner,
        contributor,
        winning_tile,
        num_dora_indicators: state.num_dora_indicators,
        closed_hand: &state.closed_hands[winner_i],
        waiting_info,
        riichi_flags: state.riichi[winner_i],
        melds: &state.melds[winner_i],
        incoming_meld: state.incoming_meld,
    }
}
