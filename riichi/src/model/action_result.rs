//! Conclusion of an action-reaction cycle ([`ActionResult`]).

use crate::common::*;
use super::agari::AgariKind;

/// Conclusion of an action-reaction cycle.
#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum ActionResult {
    #[default]
    /// The action has successfully taken place without any reaction.
    Pass,

    /// A meld (Chii/Pon/Daiminkan) has been called by another player.
    /// Note that Kakan/Ankan does not count.
    CalledBy(Player),

    /// At least one player has won, either by steal (ロン和ガリ) or by self-draw (ツモ和ガリ).
    Agari(AgariKind),

    /// The round has ended without a win. See [`AbortReason`].
    Abort(AbortReason),
}

/// The reason why the round has ended without a win.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum AbortReason {
    /// The round has been aborted due to the player in action declaring "nine kinds of terminals"
    /// (九種九牌).
    /// This terminates the round. No reactions are possible.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Kyuushu_kyuuhai>
    NineKinds,

    /// The round has ended because no more tiles can be drawn from the wall (荒牌).
    /// Penalties payments may apply (不聴罰符), including sub-type [`Self::NagashiMangan`].
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by Ron and all other aborts.
    ///
    /// <https://riichi.wiki/Ryuukyoku>
    WallExhausted,

    /// Special case of [`Self::WallExhausted`] (流し満貫).
    /// Treated as penalties payments.
    ///
    /// <https://riichi.wiki/Nagashi_mangan>
    NagashiMangan,

    /// Four Kan's (四開槓).
    /// - A player has attemped to call the 5th Kan of the round; all 5 are by the same player.
    /// - A player has attempted to call the 4th Kan of the round; all 4 are NOT by the same player.
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by Ron.
    ///
    /// Note that Kakan and Ankan may also be preempted due to Chankan (搶槓).
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suukaikan>
    FourKan,

    /// The round has been aborted because four of the same kind of wind tile has been discarded
    /// consecutively since the game starts (四風連打).
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Cannot be preempted.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suufon_renda>
    FourWind,

    /// The round has been aborted because all four players are under active riichi (四家立直).
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by Ron.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suucha_riichi>
    FourRiichi,

    /// The round has been aborted because 2 players called Ron on the same tile.
    /// This is a rare rule variation of [`Self::TripleRon`].
    DoubleRon,

    /// The round has been aborted because 3 players called Ron on the same tile.
    /// This is a common rule variation. Other variations include:
    /// - Accept triple-Ron.
    /// - Only "the first player" (in turn order after the current) calling Ron passes.
    /// - Even double-Ron results in an abortion.
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Pre-empts all others.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Sanchahou>
    TripleRon,
}
