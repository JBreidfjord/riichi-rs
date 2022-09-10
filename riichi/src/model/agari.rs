use crate::{
    analysis::Wait,
    common::*,
};
use super::{ActionResult, YakuValues};

/// Describes a finalized winning hand.
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]  // due to `RegularWait`
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

/// RonAgari (ロン) or Tsumo (ツモ和ガリ).
#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq)]
#[repr(u8)]
pub enum AgariKind {
    #[default]
    /// Win-by-steal / RonAgari (ロン)
    Ron = 0,
    /// Win-by-self-draw / TsumoAgari (ツモ和ガリ)
    Tsumo = 1,
}

impl AgariKind {
    pub fn to_action_result(self) -> ActionResult {
        ActionResult::Agari(self)
    }
}

/// Describes one possible way a hand can win.
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]  // due to `RegularWait`
pub struct AgariCandidate {
    /// Waiting pattern for this hand --- regular or irregular, based on which this hand is being
    /// considered for a win.
    pub wait: Wait,

    /// Scoring components for this hand (Yakuman, Yaku, Doras, Fu).
    pub scoring: Scoring,

    /// What [Yaku's](super::Yaku) are awarded to this hand, and the Han-value of each Yaku.
    /// There must be at least one Yaku for a hand to be eligible for winning.
    pub yaku_values: YakuValues,
}

/// Scoring components for a hand.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Scoring {
    /// The total "plies" of Yakuman (役満); e.g. 1 => single Yakuman, 2 => double Yakuman, ...
    ///
    /// If 0, there is no Yakuman-valued Yaku's for this hand, but other Yakus might still push it
    /// to Kazoe-Yakuman status.
    pub yakuman_total_value: u8,

    /// The total Han-value (飜) of all Yakus. 0 if there is at least one Yakuman-valued Yaku.
    pub yaku_total_value: u8,

    /// How many Doras (of each kind) are counted in this hand.
    /// When evaluating for [`AgariCandidate`], this can be left 0.
    pub dora_hits: DoraHits,

    /// The "mini-points" of a hand (符).
    ///
    /// <https://riichi.wiki/Fu>
    pub fu: u8,
}

impl Scoring {
    /// Total Han-value (飜) of this hand, from both Yaku's and Dora hits.
    pub fn han(&self) -> u8 {
        self.yaku_total_value + self.dora_hits.sum()
    }
}

/// Number of Dora's (ドラ) counted in a hand.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct DoraHits {
    /// Number of Dora's as indicated by the indicator(s) revealed on the top of the wall (ドラ).
    /// This may include the effect of any additional indicators revealed through Kan (カンドラ).
    pub dora: u8,

    /// Number of Dora's as indicated by the indicator(s) revealed only after winning by Riichi
    /// (裏ドラ).
    /// Likewise, this may be affected by Kan (カン裏ドラ).
    pub ura_dora: u8,

    /// The mere presence of red 5's in the hand also counts as Dora hits (赤ドラ).
    pub aka_dora: u8,
}

impl DoraHits {
    /// Total Han-value (飜) of all Dora's.
    pub fn sum(self) -> u8 {
        self.dora + self.ura_dora + self.aka_dora
    }
}
