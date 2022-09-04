//! [`Action`]s, [`Reaction`]s, and the [result](`ActionResult`) of an action-reaction cycle.

use crate::common::*;
use super::Discard;

/// Action by the in-turn player.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Action {
    /// Discard a tile. See [`Discard`].
    /// The `called_by` field is implied and can be safely ignored here.
    Discard(Discard),
    /// Declare an [`Ankan`] (4 in closed hand).
    Ankan(Tile),
    /// Declare a [`Kakan`] (1 in closed hand, 3 in pon).
    Kakan(Tile),
    /// Win by self-draw. See [`ActionResult::TsumoAgari`].
    TsumoAgari(Tile),
    /// Abort by Nine Kinds of Terminals. See [`ActionResult::AbortNineKinds`].
    AbortNineKinds,
}

impl Action {
    pub fn from_meld(meld: Meld) -> Option<Self> {
        match meld {
            Meld::Kakan(kakan) => Some(Action::Kakan(kakan.added)),
            Meld::Ankan(ankan) => Some(Action::Ankan(ankan.own[0].to_normal())),
            _ =>  None,
        }
    }

    pub fn tile(self) -> Option<Tile> {
        match self {
            Action::Discard(discard) => Some(discard.tile),
            Action::Ankan(tile) => Some(tile),
            Action::Kakan(tile) => Some(tile),
            Action::TsumoAgari(tile) => Some(tile),
            Action::AbortNineKinds => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        match self {
            Self::TsumoAgari(_) | Self::AbortNineKinds => true,
            _ => false,
        }
    }
}

/// Reaction from an out-of-turn player.
/// The lack of reaction / "pass" / unknown reaction can be represented by `Option<Reaction>`.
/// Variants are ordered by their priority (`Chii` is the lowest, `RonAgari` the highest).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Reaction {
    /// Declare a [`crate::Chii`] (チー) on the recent discard with the specified own tiles.
    Chii(Tile, Tile),
    /// Declare a [`crate::Pon`] (ポン) on the recent discard with the specified own tiles.
    Pon(Tile, Tile),
    /// Declare a [`crate::Daiminkan`] (大明槓) on the recent discard; own tiles are implicit.
    Daiminkan,
    /// Declare win-by-steal (ロン和ガリ) on the recent action, which can be
    /// [`Action::Discard`], [`Action::Kakan`] (rare), or [`Action::ankan`] (very rare).
    RonAgari,
}

impl Reaction {
    pub fn from_meld(meld: Meld) -> Option<Self> {
        match meld {
            Meld::Chii(chii) => Some(Self::Chii(chii.own[0], chii.own[1])),
            Meld::Pon(pon) => Some(Self::Pon(pon.own[0], pon.own[1])),
            Meld::Daiminkan(_) => Some(Self::Daiminkan),
            _ => None,
        }
    }
}

/// Conclusion of an action-reaction cycle.
/// Unknown state can be represented by `Option<PostReactionState>`, just like `Reaction`.
/// However, an explicit `Pass` is included to represent "nothing has happened; move on".
#[allow(unused_qualifications)]
#[derive(Copy, Clone, Debug, num_enum::Default, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ActionResult {
    #[num_enum(default)]
    /// The action has successfully taken place without any reaction.
    Pass = 0,

    /// A [`crate::Chii`] has been called (チー).
    Chii,
    /// A [`crate::Pon`] has been called (ポン).
    Pon,
    /// A [`crate::Daiminkan`] has been called (大明槓).
    Daiminkan,

    /// At least one player has won by steal (ロン和ガリ).
    /// Multiple players (but not too many) may call Ron on the same tile (discard/kakan/ankan).
    RonAgari,

    /// The player in action has won by self-draw (ツモ和ガリ).
    ///
    /// Resolution:
    /// - Determined by in-turn action.
    /// - No reaction allowed.
    TsumoAgari,

    /// The round has been aborted due to the player in action declaring "nine kinds of terminals"
    /// (九種九牌).
    ///
    /// Resolution:
    /// - Determined by in-turn action.
    /// - No reaction allowed.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Kyuushu_kyuuhai>
    AbortNineKinds,

    /// The round has ended because no more tiles can be drawn from the wall (荒牌).
    /// Penalties payments may apply (不聴罰符), including sub-type [`Self::AbortNagashiMangan`].
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by [`Self::RonAgari`] and all other aborts.
    ///
    /// <https://riichi.wiki/Ryuukyoku>
    AbortWallExhausted,

    /// Special case of [`Self::AbortWallExhausted`] (流し満貫).
    /// Treated as penalties payments.
    ///
    /// <https://riichi.wiki/Nagashi_mangan>
    AbortNagashiMangan,

    /// Four Kan's (四開槓).
    /// - A player has attemped to call the 5th Kan of the round; all 5 are by the same player.
    /// - A player has attempted to call the 4th Kan of the round; all 4 are NOT by the same player.
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by [`Self::RonAgari`].
    ///
    /// Note that kakan and ankan may also be preempted due to Chankan (搶槓).
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suukaikan>
    AbortFourKan,

    /// The round has been aborted because four of the same kind of wind tile has been discarded
    /// consecutively since the game starts (四風連打).
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Cannot be preempted.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suufon_renda>
    AbortFourWind,

    /// The round has been aborted because all four players are under active riichi (四家立直).
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by [`Self::RonAgari`].
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suucha_riichi>
    AbortFourRiichi,

    /// The round has been aborted because 2 players called Ron on the same tile.
    /// This is a rare rule variation of [`Self::AbortTripleRon`].
    AbortDoubleRon,

    /// The round has been aborted because 3 players called Ron on the same tile.
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Pre-empts all others.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Sanchahou>
    AbortTripleRon,
}

impl ActionResult {
    pub const fn is_meld(self) -> bool {
        use ActionResult::*;
        match self {
            Chii | Pon | Daiminkan => true,
            _ => false,
        }
    }
    pub const fn is_agari(self) -> bool {
        use ActionResult::*;
        match self {
            TsumoAgari | RonAgari => true,
            _ => false,
        }
    }
    pub const fn is_abort(self) -> bool {
        use ActionResult::*;
        match self {
            AbortNineKinds | AbortWallExhausted | AbortNagashiMangan |
            AbortFourKan | AbortFourWind | AbortFourRiichi | AbortTripleRon => true,
            _ => false,
        }
    }
    pub const fn is_terminal(self) -> bool { self.is_agari() || self.is_abort() }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use assert2::assert;
    use itertools::Itertools;
    use super::*;

    #[test]
    fn reaction_order_by_priority() {
        use Reaction::*;
        let reactions = [
            Chii(Tile::from_str("1s").unwrap(), Tile::from_str("2s").unwrap()),
            Pon(Tile::from_str("3s").unwrap(), Tile::from_str("3s").unwrap()),
            Daiminkan,
            RonAgari,
        ];
        for (low, high) in reactions.into_iter().tuple_windows() {
            assert!(low < high);
        }
    }
}
