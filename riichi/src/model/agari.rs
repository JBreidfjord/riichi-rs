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

    /// The net effect on each player's points due to this win.
    ///
    /// This includes the pot collected by the winner / one of the multi-ron winners, and the bonus
    /// points from Honba.
    pub points_delta: [GamePoints; 4],

    /// Details of the win (the highest-point interpretation).
    pub details: AgariCandidate,
}

impl AgariResult {
    pub fn kind(&self) -> AgariKind {
        if self.winner == self.contributor { AgariKind::Tsumo } else { AgariKind::Ron }
    }
}

#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq)]
#[repr(u8)]
pub enum AgariKind {
    #[default]
    Ron = 0,
    Tsumo = 1,
}

impl AgariKind {
    pub fn to_action_result(self) -> ActionResult {
        ActionResult::Agari(self)
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
