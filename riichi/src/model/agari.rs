use std::cmp::min;
use crate::analysis::RegularWait;
use crate::common::*;
use super::{ActionResult, Yaku};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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

    pub details: AgariDetails,
}

impl AgariResult {
    pub fn kind(&self) -> AgariKind {
        if self.winner == self.contributor {
            AgariKind::Tsumo
        } else {
            AgariKind::Ron
        }
    }

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
    Tsumo,
}

impl AgariKind {
    pub fn to_action_result(self) -> ActionResult {
        match self {
            AgariKind::Ron => ActionResult::RonAgari,
            AgariKind::Tsumo => ActionResult::TsumoAgari,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgariDetails {
    pub regular_wait: Option<RegularWait>,
    pub scoring: Scoring,
    pub yaku_value: Vec<(Yaku, i8)>,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Scoring {
    pub yakuman_sum: u8,
    pub yaku_sum: u8,
    pub num_reds: u8,
    pub num_dora_hits: u8,
    pub num_ura_dora_hits: u8,
    pub fu: u8,
}

impl Scoring {
    pub fn han(&self) -> u8 {
        self.yaku_sum + self.num_reds + self.num_dora_hits + self.num_ura_dora_hits
    }

    pub fn basic_points(&self) -> GamePoints {
        if self.yakuman_sum > 0 {
            return 8000 * self.yakuman_sum as GamePoints
        }
        match self.yaku_sum {
            0 => 0,
            1..=5 => min(
                self.fu as GamePoints * ((1 as GamePoints) << (2 + self.han() as GamePoints)),
                2000),  // mangan and below
            6..=7 => 3000,  // haneman (1.5x mangan)
            8..=10 => 4000,  // baiman (2x mangan)
            11..=12 => 6000,  // sanbaiman (3x mangan)
            _ => 8000,  // kazoe-yakuman (4x mangan)
        }
    }

    pub fn basic_points_aotenjou(&self) -> GamePoints {
        let han = self.yakuman_sum * 13 + self.han();
        self.fu as GamePoints * ((1 as GamePoints) << (2 + han as GamePoints))
    }
}
