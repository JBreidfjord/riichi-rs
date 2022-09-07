use crate::analysis::Wait;
use crate::common::*;
use super::{ActionResult, YakuValues};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgariResult {
    /// Who won?
    pub winner: Player,

    /// Who supplied the winning tile?
    /// - Ron: another player.
    /// - Tsumo: `winner`.
    pub contributor: Player,

    /// The player falling victim to special Pao (パオ) / Sekinin-barai (責任払い) / Liability rules.
    /// If none applies, this should be set to `winner`.
    pub liable_player: Player,

    pub points_delta_before_pot: [GamePoints; 4],

    /// Actual pot collected by the winner.
    /// Note that in a multi-ron scenario, only one player will get the pot.
    /// During (ron) declaration, it's unknown who will collect the pot. In this case, we assume 0
    /// pot collected, and only backfill this field during (multi-ron) resolution.
    /// TODO(summivox): rules (atama-hane)
    pub pot_gained: GamePoints,

    pub hand: TileSet37,
    pub melds: Vec<Meld>,
    pub winning_tile: Tile,

    pub best_candidate: AgariCandidate,
}

impl AgariResult {
    pub fn points_delta_after_pot(&self) -> [GamePoints; 4] {
        let mut delta = self.points_delta_before_pot;
        delta[self.winner.to_usize()] += self.pot_gained;
        delta
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum AgariKind {
    #[default]
    Ron = 0,
    Tsumo = 1,
}

impl AgariKind {
    pub fn to_action_result(self) -> ActionResult {
        match self {
            AgariKind::Ron => ActionResult::RonAgari,
            AgariKind::Tsumo => ActionResult::TsumoAgari,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgariCandidate {
    pub wait: Wait,
    pub scoring: Scoring,
    pub yaku_values: YakuValues,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Scoring {
    pub yakuman_total_value: u8,
    pub yaku_total_value: u8,
    pub dora_hits: DoraHits,
    pub fu: u8,
}

impl Scoring {
    pub fn han(&self) -> u8 {
        self.yaku_total_value + self.dora_hits.sum()
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct DoraHits {
    pub dora: u8,
    pub ura_dora: u8,
    pub aka_dora: u8,
}

impl DoraHits {
    pub fn sum(self) -> u8 {
        self.dora + self.ura_dora + self.aka_dora
    }
}
