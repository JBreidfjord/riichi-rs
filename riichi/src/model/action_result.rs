//! Conclusion of an action-reaction cycle ([`ActionResult`]).

use riichi_elements::prelude::*;

use super::agari::AgariKind;

/// Conclusion of an action-reaction cycle.
///
/// ## Optional `serde` support
///
/// `{type, details?}` (adjacently tagged, in serde terms).
///
/// Examples:
///
/// - `{"type": "Pass"}`
/// - `{"type": "CalledBy", "details": 3}`
/// - `{"type": "Agari", "details": "Tsumo"}`
/// - `{"type": "Abort", "details": "NagashiMangan"}`
///
#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "details"))]
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

impl ActionResult {
    pub fn as_str(self) -> &'static str {
        match self {
            ActionResult::Pass => "",
            ActionResult::CalledBy(_) => "鳴き",
            ActionResult::Agari(agari) => agari.into(),
            ActionResult::Abort(abort) => abort.into(),
        }
    }
}

impl From<ActionResult> for &'static str {
    fn from(value: ActionResult) -> Self {
        value.as_str()
    }
}

/// The reason why the round has ended without a win.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, strum::IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AbortReason {
    /// The round has been aborted due to the player in action declaring "nine kinds of terminals"
    /// (九種九牌).
    /// This terminates the round. No reactions are possible.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Kyuushu_kyuuhai>
    #[strum(to_string = "九種九牌")]
    NineKinds,

    /// The round has ended because no more tiles can be drawn from the wall (荒牌).
    /// Penalties payments may apply (不聴罰符), including sub-type [`Self::NagashiMangan`].
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by Ron and all other aborts.
    ///
    /// <https://riichi.wiki/Ryuukyoku>
    #[strum(to_string = "流局")]
    WallExhausted,

    /// Special case of [`Self::WallExhausted`] (流し満貫).
    /// Treated as penalties payments.
    ///
    /// <https://riichi.wiki/Nagashi_mangan>
    #[strum(to_string = "流し満貫")]
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
    #[strum(to_string = "四開槓")]
    FourKan,

    /// The round has been aborted because four of the same kind of wind tile has been discarded
    /// consecutively since the game starts (四風連打).
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Cannot be preempted.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suufon_renda>
    #[strum(to_string = "四風連打")]
    FourWind,

    /// The round has been aborted because all four players are under active riichi (四家立直).
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by Ron.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suucha_riichi>
    #[strum(to_string = "四家立直")]
    FourRiichi,

    /// The round has been aborted because 2 players called Ron on the same tile.
    /// This is a rare rule variation of [`Self::TripleRon`].
    #[strum(to_string = "二家和")]
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
    #[strum(to_string = "三家和")]
    TripleRon,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_result_str() {
        let pass: &'static str = ActionResult::Pass.into();
        assert_eq!(pass, "");
        let call: &'static str = ActionResult::CalledBy(P0).into();
        assert_eq!(call, "鳴き");
    }
}
