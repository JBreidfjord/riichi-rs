//! Boundary conditions of a round (begin and end).

use riichi_elements::prelude::*;

use crate::{
    rules::Ruleset,
};
use super::{
    ActionResult,
    AgariResult,
};

/// "[Ba]-[Kyoku]-[Honba]" (場-局-本場) triplet that uniquely identifies a round, represented as a
/// pair of [Ba]-[Kyoku] (combined) and [Honba].
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of all fields: `{"kyoku": 7, "honba": 2}` <=> 南4局 2本場
///
/// ## Ref
///
/// - <https://riichi.wiki/Kyoku>
/// - <https://riichi.wiki/Honba>
/// - <https://ja.wikipedia.org/wiki/%E9%80%A3%E8%8D%98>
///
/// [Ba]: https://riichi.wiki/Ba
/// [Kyoku]: https://riichi.wiki/Kyoku
/// [Honba]: https://riichi.wiki/Honba
///
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoundId {
    /// Index of the round (局) together with the prevailing wind (場).
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
    /// NOTE: There are no real limits in the ruleset, so theoretically this can grow towards +inf.
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
    ///
    /// **[`Variant::Yonma`] only**; see [`Self::player_with_self_wind_in`].
    pub fn player_with_self_wind(self, wind: Wind) -> Player {
        self.button().add(wind)
    }

    /// Index of the self wind (自風).
    ///
    /// **[`Variant::Yonma`] only**; see [`Self::self_wind_for_player_in`].
    pub fn self_wind_for_player(self, player: Player) -> Wind {
        Wind::from(player.sub(self.button()))
    }

    /// Index of the player with the given self wind, in the given [`Variant`].
    ///
    /// Sanma has only East/South/West; asking for North yields the East player, which is
    /// meaningless — callers should not ask. (North is a guest wind in sanma:
    /// 「北を面子で使用した場合はオタ風」.)
    pub fn player_with_self_wind_in(self, variant: Variant, wind: Wind) -> Player {
        let n = variant.num_players();
        Player::new((self.button().to_u8() + wind.to_u8() % n) % n)
    }

    /// Index of the self wind (自風), in the given [`Variant`].
    ///
    /// # Why this is not `player.sub(button)`
    ///
    /// Self wind is the seat's *distance around the table* from the button, so it must be reduced
    /// modulo the number of **playing** seats, not modulo 4. With three seats and the button on
    /// seat 2, `P0.sub(P2) == P2` claims West where the seat is actually South.
    ///
    /// This corrects ADR 0006 / issue #108, both of which assert that `self_wind_for_player`
    /// "stays untouched and literally correct" in sanma alongside `button` and
    /// `prevailing_wind`. It is correct for those two — the button is `kyoku % 4` and sparse kyoku
    /// numbering keeps that on an active seat — but **not** for self wind, which is the one of the
    /// three that depends on a seat *difference*.
    pub fn self_wind_for_player_in(self, variant: Variant, player: Player) -> Wind {
        let n = variant.num_players();
        Wind::new((player.to_u8() + n - self.button().to_u8() % n) % n)
    }

    /// Returns the "real" actual round. This happens when the current round ends in a win, and the
    /// button player is not among the winner(s).
    ///
    /// **[`Variant::Yonma`] only**; see [`Self::next_kyoku_in`]. Sanma numbers Kyoku sparsely.
    pub const fn next_kyoku(self) -> Self {
        Self {
            kyoku: self.kyoku + 1,
            honba: 0,
        }
    }

    /// [`Self::next_kyoku`] for the given [`Variant`], skipping the Kyoku numbers that variant
    /// does not have.
    ///
    /// Sanma skips kyoku ≡ 3 (mod 4): the **absent seat** is never the button, so E1/E2/E3 are
    /// 0/1/2 and S1/S2/S3 are 4/5/6. That is what keeps [`Self::prevailing_wind`] and
    /// [`Self::button`] literally correct without threading a variant through them.
    pub const fn next_kyoku_in(self, variant: Variant) -> Self {
        Self {
            kyoku: variant.next_kyoku(self.kyoku),
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
    /// **[`Variant::Yonma`] only** when `renchan == false`; see [`Self::next_honba_in`].
    pub const fn next_honba(self, renchan: bool) -> Self {
        Self {
            kyoku: if renchan { self.kyoku } else { self.kyoku + 1 },
            honba: self.honba + 1,
        }
    }

    /// [`Self::next_honba`] for the given [`Variant`].
    ///
    /// Note that ADR 0006 says "only `next_kyoku` learns to skip". That is one call short:
    /// `next_honba(false)` also advances the Kyoku number (an abort that does not renchan), so it
    /// needs the same skip.
    pub const fn next_honba_in(self, renchan: bool, variant: Variant) -> Self {
        Self {
            kyoku: if renchan { self.kyoku } else { variant.next_kyoku(self.kyoku) },
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
    pub ruleset: Ruleset,

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
            ruleset: Default::default(),
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

    /// The variant-aware form must be identical to the historical one in yonma, for every
    /// kyoku and every seat.
    #[test]
    fn yonma_self_wind_is_unchanged() {
        for kyoku in 0..16u8 {
            let round_id = RoundId { kyoku, honba: 0 };
            for p in ALL_PLAYERS {
                assert_eq!(round_id.self_wind_for_player_in(Variant::Yonma, p),
                           round_id.self_wind_for_player(p));
            }
            for w in 0..4u8 {
                assert_eq!(round_id.player_with_self_wind_in(Variant::Yonma, Wind::new(w)),
                           round_id.player_with_self_wind(Wind::new(w)));
            }
            assert_eq!(round_id.next_kyoku_in(Variant::Yonma), round_id.next_kyoku());
            assert_eq!(round_id.next_honba_in(true, Variant::Yonma), round_id.next_honba(true));
            assert_eq!(round_id.next_honba_in(false, Variant::Yonma), round_id.next_honba(false));
        }
    }

    /// Sanma self wind must be the seat's distance from the button around a **three**-seat ring.
    /// The mod-4 form gets this wrong for every kyoku whose button is not seat 0.
    #[test]
    fn sanma_self_wind_uses_three_seats() {
        let v = Variant::Sanma;
        // E1 (kyoku 0): button = P0.
        let e1 = RoundId { kyoku: 0, honba: 0 };
        assert_eq!(e1.self_wind_for_player_in(v, P0), Wind::new(0));
        assert_eq!(e1.self_wind_for_player_in(v, P1), Wind::new(1));
        assert_eq!(e1.self_wind_for_player_in(v, P2), Wind::new(2));
        // E2 (kyoku 1): button = P1, so P2 is South and P0 is West (not North).
        let e2 = RoundId { kyoku: 1, honba: 0 };
        assert_eq!(e2.self_wind_for_player_in(v, P1), Wind::new(0));
        assert_eq!(e2.self_wind_for_player_in(v, P2), Wind::new(1));
        assert_eq!(e2.self_wind_for_player_in(v, P0), Wind::new(2));
        assert_ne!(e2.self_wind_for_player_in(v, P0), e2.self_wind_for_player(P0));
        // E3 (kyoku 2): button = P2, so P0 is South and P1 is West.
        let e3 = RoundId { kyoku: 2, honba: 0 };
        assert_eq!(e3.self_wind_for_player_in(v, P2), Wind::new(0));
        assert_eq!(e3.self_wind_for_player_in(v, P0), Wind::new(1));
        assert_eq!(e3.self_wind_for_player_in(v, P1), Wind::new(2));

        // North is never a self wind in sanma.
        for kyoku in [0u8, 1, 2, 4, 5, 6] {
            let r = RoundId { kyoku, honba: 0 };
            for p in v.active_seats() {
                assert_ne!(r.self_wind_for_player_in(v, *p), Wind::new(3));
            }
        }

        // Round-trip against `player_with_self_wind_in`.
        for kyoku in [0u8, 1, 2, 4, 5, 6] {
            let r = RoundId { kyoku, honba: 0 };
            for w in 0..3u8 {
                let p = r.player_with_self_wind_in(v, Wind::new(w));
                assert!(v.is_seat_active(p));
                assert_eq!(r.self_wind_for_player_in(v, p), Wind::new(w));
            }
        }
    }

    #[test]
    fn sanma_round_advance_skips_the_absent_button() {
        let v = Variant::Sanma;
        assert_eq!(RoundId { kyoku: 2, honba: 0 }.next_kyoku_in(v),
                   RoundId { kyoku: 4, honba: 0 });
        assert_eq!(RoundId { kyoku: 2, honba: 3 }.next_honba_in(false, v),
                   RoundId { kyoku: 4, honba: 4 });
        assert_eq!(RoundId { kyoku: 2, honba: 3 }.next_honba_in(true, v),
                   RoundId { kyoku: 2, honba: 4 });
    }
}
