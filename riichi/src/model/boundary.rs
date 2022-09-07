//! Boundary conditions of a round (begin and end).

use crate::common::*;
use crate::common::wall::{make_dummy_wall};
use crate::rules::Rules;
use super::ActionResult;
use super::AgariCandidate;
use super::PartiallyObservable;

/// Kyoku-Honba (局-本場) pair that uniquely identifies a round in a game.
///
/// Ref:
/// - <https://riichi.wiki/Kyoku>
/// - <https://riichi.wiki/Honba>
/// - <https://ja.wikipedia.org/wiki/%E9%80%A3%E8%8D%98>
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
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
        Player::new(self.kyoku + wind.to_u8())
    }

    /// Index of the self wind (自風).
    pub fn self_wind_for_player(self, player: Player) -> Wind {
        Wind::from(player.wrapping_sub(self.button()))
    }

    /// TODO(summivox): doc
    pub const fn next_kyoku(self) -> Self {
        Self {
            kyoku: self.kyoku + 1,
            honba: 0,
        }
    }

    /// TODO(summivox): doc
    pub const fn next_honba(self, renchan: bool) -> Self {
        Self {
            kyoku: if renchan { self.kyoku } else { self.kyoku + 1 },
            honba: self.honba + 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RoundBegin {
    pub rules: Rules,

    /// Kyoku-honba that identifies this round.
    pub round_id: RoundId,

    /// The tile wall right after shuffling and cutting (full 136 tiles).  Drawing and revealing
    /// (of dora indicators) are "virtual", always referring to this original wall.
    pub wall: Wall,

    /// Points left on the table (供託), up for grabs by the next winner.
    /// Commonly 1000-pt sticks from riichi.
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
            wall: make_dummy_wall(),
            pot: 0,
            points: [0; 4],
        }
    }
}

impl PartiallyObservable for RoundBegin {
    fn observe_by(&self, _player: Player) -> Self {
        let mut observed = self.clone();
        observed.wall = make_dummy_wall();
        observed
    }
}

#[derive(Clone, Debug, Default)]
pub struct RoundEnd {
    /// The result of the round; equal to the last `ActionResult` before round ended.
    /// Guaranteed to be "terminal" (see [`ActionResult::is_terminal`]).
    pub round_result: ActionResult,

    /// Same definition as [`RoundBeginState::pot`] but at round end.
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
    pub agari_result: [Option<AgariCandidate>; 4],
}
