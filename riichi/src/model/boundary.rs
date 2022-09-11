//! Boundary conditions of a round (begin and end).

use crate::{
    common::*,
    rules::Rules,
};
use super::{
    ActionResult,
    AgariResult,
};

/// Kyoku-Honba (局-本場) pair that uniquely identifies a round in a game.
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of all fields: `{"kyoku": 7, "honba": 2}`.
///
/// ## Ref
///
/// - <https://riichi.wiki/Kyoku>
/// - <https://riichi.wiki/Honba>
/// - <https://ja.wikipedia.org/wiki/%E9%80%A3%E8%8D%98>
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoundId {
    /// Index of the wind-round (局), enumerated in combination with the prevailing wind:
    ///
    /// - 0 => east 1 (東1局) -- min
    /// - 3 => east 4 (東4局)
    /// - 4 => south 1 (南1局)
    /// - 7 => south 4 (南4局)
    /// - 8 => west 1 (西1局)
    /// - 15 => north 4 (北4局) -- max
    ///
    /// NOTE: The theoretical max value is not enforced here.
    pub kyoku: u8,

    /// The "sub round" number (本場数), commonly represented as the number of 100-pt sticks placed
    /// on the table.
    ///
    /// NOTE: There are no real limits in the rules, so theoretically this can grow towards +inf.
    /// Saturation arithmetic should be used to ensure sanity.
    pub honba: u8,
}

impl RoundId {
    /// Index of the prevailing wind (場風).
    ///
    /// This is shared by all players (unlike "self wind").
    pub const fn prevailing_wind(self) -> Wind {
        Wind::new(self.kyoku / 4)
    }

    /// Index of the dealer/button/east-wind player (荘家).
    ///
    /// NOTE: "button" refers to the similar concept in Texas Hold'em, a.k.a. dealer
    pub const fn button(self) -> Player { Player::new(self.kyoku % 4) }

    /// Index of the player with given self wind.
    /// - east-wind player == button
    /// - south-wind player == button + 1
    /// - west-wind player == button + 2
    /// - north-wind player == button + 3
    pub fn player_with_self_wind(self, wind: Wind) -> Player {
        self.button().add(wind)
    }

    /// Index of the self wind (自風).
    pub fn self_wind_for_player(self, player: Player) -> Wind {
        Wind::from(player.sub(self.button()))
    }

    /// Returns the "real" actual round. This happens when the current round ends in a win, and the
    /// button player is not among the winner(s).
    pub const fn next_kyoku(self) -> Self {
        Self {
            kyoku: self.kyoku + 1,
            honba: 0,
        }
    }

    /// Returns the next sub-round. This happens when the button player wins (`renchan == true`;
    /// 連荘) or the current round ends in an abortion.
    ///
    /// Additionally, for [`WallExhausted`] or [`NagashiMangan`], if the button player has a waiting
    /// hand at the end, then the `kyoku` number will remain the same. This condition is also
    /// indicated by `renchan == true` (連荘).
    ///
    /// [`WallExhausted`]: super::AbortReason::WallExhausted
    /// [`NagashiMangan`]: super::AbortReason::NagashiMangan
    ///
    pub const fn next_honba(self, renchan: bool) -> Self {
        Self {
            kyoku: if renchan { self.kyoku } else { self.kyoku + 1 },
            honba: self.honba + 1,
        }
    }
}

/// Meta-states at the beginning of the round.
///
/// ## Optional `serde` suppport
///
/// Straightforward struct mapping of all fields.
///
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoundBegin {
    pub rules: Rules,

    /// Kyoku-Honba that identifies this round.
    pub round_id: RoundId,

    /// The tile wall right after shuffling and cutting (full 136 tiles).  Drawing and revealing
    /// (of dora indicators) are "virtual", always referring to this original wall.
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub wall: Wall,

    /// Points left on the table (供託), up for grabs by the next winner.
    /// Commonly 1000-pt sticks from Riichi.
    ///
    /// Ref:
    /// - <https://ja.wikipedia.org/wiki/%E9%BA%BB%E9%9B%80%E3%81%AE%E7%82%B9#%E4%BE%9B%E8%A8%97>
    pub pot: GamePoints,

    /// Points for each player.
    pub points: [GamePoints; 4],
}

impl Default for RoundBegin {
    fn default() -> Self {
        Self {
            rules: Default::default(),
            round_id: Default::default(),
            wall: wall::make_dummy_wall(),
            pot: 0,
            points: [0; 4],
        }
    }
}

/// Details of how a round concluded, including the points differences and the breakdown of each
/// winning hand.
///
/// ## Optional `serde` support
///
/// Serialization only.
/// Straightforward struct mapping of all fields.
///
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RoundEnd {
    /// The result of the round; equal to the last `ActionResult` before round ended.
    /// Guaranteed to be "terminal" (agari or abort).
    pub round_result: ActionResult,

    /// Same definition as [`RoundBegin::pot`] but at round end.
    pub pot: GamePoints,
    /// Points for each player at round end.
    pub points: [GamePoints; 4],
    /// Point increments for each player (end - begin)
    pub points_delta: [GamePoints; 4],

    /// Whether the next round is "this round + 1 honba".
    pub renchan: bool,
    /// Id of the next round; `None` if the game ends.
    pub next_round_id: Option<RoundId>,

    /// If a player has won this round (non-exclusive due to multi-ron), how they did so.
    pub agari_result: [Option<AgariResult>; 4],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_id_computes_correct_self_wind() {
        let round_id = RoundId { kyoku: 6, honba: 0 };
        assert_eq!(round_id.self_wind_for_player(P2), Wind::new(0));
        assert_eq!(round_id.self_wind_for_player(P3), Wind::new(1));
        assert_eq!(round_id.self_wind_for_player(P0), Wind::new(2));
        assert_eq!(round_id.self_wind_for_player(P1), Wind::new(3));
    }
}
