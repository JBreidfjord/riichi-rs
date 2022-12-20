//! Configurable rules and interpretations of rules for a game, bundled as [`Ruleset`].

use derivative::Derivative;
use rustc_hash::FxHashSet as HashSet;
use semver::Version;
#[cfg(feature = "serde")]
use serde_with::{
    serde_as, skip_serializing_none,
    As, DisplayFromStr
};

use riichi_elements::prelude::*;

use crate::{
    yaku::Yaku,
};

/// Bundle of configurable rules and interpretations of rules for a game.
///
/// A [`Default::default()`] ruleset is provided that closely matches [Tenhou] defaults for
/// current 4-player public lobbies (as of 2022-10-01).
///
/// See the documentation on each field for what can be configured.
///
/// ## Key exceptions (what cannot be configured here)
///
/// - The number of red tiles --- implied by the wall array.
/// - The starting points of each player --- already specified in [`crate::model::RoundBegin`].
///
///
/// ## Semantic versioning
///
/// The Ruleset is part of the SemVer guarantee of this crate.
///
/// - Ruleset may not be changed across Patch increments.
/// - New variations may be added with a Minor increment. The default behavior of any newly added
///   variation must be the same as before. The outcome of the game will not be affected (except for
///   any bug fixes).
/// - Any changes in the interpretation of existing variations, or removal of a variation, must be
///   done with a Major increment.
///
/// This ensures that any persisted games are reproduceable by a compatibly-versioned game engine.
///
///
/// ## Background
///
/// Even though the Japanese Riichi Mahjong is more standardized than other variants of Mahjong,
/// there are detailed rules that may be interpreted in different ways, affecting the validity of
/// certain actions and/or the outcome of the game. There are also naturally "flexible" variations,
/// such as the number of each red 5 tiles in the wall, and how many "normal" kyoku's are allowed
/// (i.e. "East Only" vs. "East-South").
///
/// This crate attempts to handle most commonly used variations on an arbitrarily-decided "standard"
/// interpretation.
///
///
/// [Tenhou]: https://riichi.wiki/Tenhou.net_rules
///
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of fields.
/// The SemVer is serialized as a string.
///
#[derive(Derivative)]
#[derive(Clone, Debug)]
#[derivative(Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", serde_as)]
#[cfg_attr(feature = "serde", skip_serializing_none)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Ruleset {
    /// The SemVer of this ruleset.
    ///
    /// When constructed, this should always be [`crate::VERSION`]. This is in contrast with any
    /// deserialization and/or external source.
    ///
    /// See [struct-level doc](Self) for how SemVer is enforced.
    #[cfg_attr(feature = "serde", serde(with = "As::<DisplayFromStr>"))]
    #[derivative(Default(value = "crate::VERSION.clone()"))]
    pub version: Version,


    /////////////////////////////////////////////
    // How many rounds

    /// The Kyoku index of the "all-last" round.
    ///
    /// - For "East Only" games, `kyoku_max_soft == 3` (i.e. East 4 Kyoku, any Honba).
    /// - For "East-South" games, `kyoku_max_soft == 7` (i.e. South 4 Kyoku, any Honba).
    ///
    /// **Default: East-South**.
    ///
    /// Other fields control the behavior of game beyond the "all-last" round.
    ///
    /// See [`crate::model::RoundId`] for definitions of "Ba", "Kyoku", and "Honba".
    ///
    /// <https://riichi.wiki/Ba>
    #[derivative(Default(value = "7"))]
    pub kyoku_max_soft: u8,

    /// The absolute maximum Kyoku a game allows.
    /// At the next attempt to increment the Kyoku number, the game must end.
    ///
    /// **Default: North 4 Kyoku**.
    ///
    /// <https://riichi.wiki/Ba>
    #[derivative(Default(value = "15"))]
    pub kyoku_max_hard: u8,

    /// The minimum points any player need to hold for the game to end at or after the "all-last"
    /// round (see `kyoku_max_soft`).
    ///
    /// **Default: 30000**. (assuming starting point is usually 25000)
    ///
    /// <https://riichi.wiki/Japanese_mahjong_scoring_rules>
    #[derivative(Default(value = "30000"))]
    pub points_min_qualify: GamePoints,


    /////////////////////////////////////////////
    // Ron

    /// Max num of players that can simultaneously win by Ron over the same tile.
    /// More players than allowed declaring Ron would result in an forced abortion instead
    /// ([DoubleRon] or [TripleRon]).
    ///
    /// **Default: Double ok; Triple abort**.
    ///
    /// [DoubleRon]: crate::model::AbortReason::DoubleRon
    /// [TripleRon]: crate::model::AbortReason::TripleRon
    ///
    /// <https://riichi.wiki/Multiple_ron>
    #[derivative(Default(value = "2"))]
    pub ron_max_num_players: u8,

    /// When multiple players declare Ron over the same tile, after `ron_max_num_players` check:
    /// - `ron_first_only == false`: They all win. (most common)
    /// - `ron_first_only == true`: Only the first player (CCW from the contributor) wins.
    ///
    /// **Default: all can win (`false`)**.
    ///
    /// - <https://riichi.wiki/Multiple_ron>
    /// - <https://riichi.wiki/Atamahane>
    #[derivative(Default(value = "false"))]
    pub ron_first_only: bool,


    /////////////////////////////////////////////
    // Dora

    /// Do we count Ura-Dora's when a player wins under Riichi?
    /// **Default: yes**.
    ///
    /// <https://riichi.wiki/Dora_variations>
    #[derivative(Default(value = "true"))]
    pub dora_allow_ura: bool,

    /// Do we reveal new Dora's after making a Kan (subject to deferred revealing rules)?
    /// **Default: yes**.
    ///
    /// <https://riichi.wiki/Dora_variations>
    #[derivative(Default(value = "true"))]
    pub dora_allow_kan: bool,

    /// Do we count Ura-Dora's for newly revealed Kan-Dora's?
    /// **Default: yes**.
    ///
    /// <https://riichi.wiki/Dora_variations>
    #[derivative(Default(value = "true"))]
    pub dora_allow_kan_ura: bool,

    // TODO(summivox): Configurable deferred reveal for Kakan/Daiminkan?


    /////////////////////////////////////////////
    // Yaku

    /// Do we award [`Yaku::Tanyaochuu`] to an open hand?
    /// This is known as "open Tanyao" or "kui-tan" (喰い断), one of the most common rule variations.
    ///
    /// **Default: yes**.
    ///
    /// <https://riichi.wiki/Tanyao#Kuitan>
    #[derivative(Default(value = "true"))]
    pub yaku_allow_open_tanyao: bool,

    /// Extra non-standard [`Yaku`]'s to enable (in addition to the standard ones).
    /// See [`crate::yaku::STANDARD_YAKU`] for the list of Yaku's considered "standard".
    ///
    /// **Default: (none)**.
    pub yaku_extra: HashSet<Yaku>,

    /// Standard [`Yaku`]'s to disable.
    /// See [`crate::yaku::STANDARD_YAKU`] for the list of Yaku's considered "standard".
    ///
    /// **Default: (none)**.
    pub yaku_block: HashSet<Yaku>,


    /////////////////////////////////////////////
    // Corner case interactions

    /// Can a player with a waiting [Kokushi-Musou][kokushi] (a.k.a. [Thirteen Orphans]) hand
    /// declare a [`Yaku::Chankan`] Ron over [Ankan], in addition to [Kakan] which is already
    /// allowed under all rules?
    ///
    /// **Default: no**.
    ///
    /// Notes:
    ///
    /// - This affects Furiten rules as well --- if enabled, then `Ankan` may trigger Furiten for a
    ///   player with a Kokushi wait.
    ///
    /// - Due to the Kan, the waiting player cannot possibly be on a [13-way wait][kokushi13].
    ///
    /// <https://riichi.wiki/Chankan#Kokushi_musou>
    ///
    /// [Ankan]: crate::model::Action::Ankan
    /// [Kakan]: crate::model::Action::Kakan
    /// [kokushi]: Yaku::Kokushi
    /// [Thirteen Orphans]: riichi_decomp::IrregularWait::ThirteenOrphans
    /// [kokushi13]: riichi_decomp::IrregularWait::ThirteenOrphansAll
    #[derivative(Default(value = "false"))]
    pub kokushi_chankan_allow_ankan: bool,

    /// Swap calling, a.k.a. Kui-kae (喰い替え).
    ///
    /// After calling Chii/Pon on a certain tile, is it okay to immediately discard another copy of
    /// the same tile? (Red 5's are considered the same as normal 5's).
    ///
    /// **Default: no**.
    ///
    /// See [`Self::swap_call_allow_other`].
    ///
    /// <https://riichi.wiki/Kuikae>
    #[derivative(Default(value = "false"))]
    pub swap_call_allow_same: bool,

    /// Swap calling, a.k.a. Kui-kae (喰い替え).
    ///
    /// After calling Chii on a certain tile, e.g. 45m + 6m, is it okay to immediately discard a
    /// tile that would have formed a group on the other end, e.g. 3m?
    ///
    /// **Default: no**
    ///
    /// See [`Self::swap_call_allow_same`].
    ///
    /// <https://riichi.wiki/Kuikae>
    #[derivative(Default(value = "false"))]
    pub swap_call_allow_other: bool,

    /// Do we use the most strict rules to judge whether an [Ankan] is allowed while under Riichi?
    ///
    /// **Default: The strict rule (`true`)**:
    ///
    /// - The [Ankan] must be from the self draw immediately before it.
    ///   This prevents the situation known as Okuri-Kan (送り槓).
    ///
    /// - Must not change the waiting pattern. An equivalent statement: In all (standard) waiting
    ///   decompositions of the closed hand, there must be a [Koutsu] with the same tile as the
    ///   self draw.
    ///
    /// - Special: Must not destroy the [`Yaku::Chuurenpoutou`] form.
    ///
    /// The relaxed rule (`false`):
    ///
    /// - Must not change the _waiting set_ of the waiting hand.
    ///
    /// [Ankan]: crate::model::Action::Ankan
    /// [Koutsu]: riichi_elements::prelude::HandGroup::Koutsu
    ///
    /// <https://ja.wikipedia.org/wiki/立直#立直後の暗槓が認められないケース> (Japanese only)
    #[derivative(Default(value = "true"))]
    pub riichi_ankan_strict_mode: bool,

    // TODO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "serde")]
    mod serde_tests {
        use assert_json_diff::assert_json_eq;
        use super::*;

        #[test]
        fn ruleset_example() {
            let ruleset = Ruleset {
                yaku_block: HashSet::from_iter([Yaku::Iipeikou]),
                kyoku_max_soft: 3,
                kyoku_max_hard: 7,
                ..Ruleset::default()
            };
            let json = serde_json::json!({
                "version": crate::VERSION_STR,

                "kyoku_max_soft": 3,
                "kyoku_max_hard": 7,
                "points_min_qualify": 30000,

                "ron_max_num_players": 2,
                "ron_first_only": false,

                "dora_allow_ura": true,
                "dora_allow_kan": true,
                "dora_allow_kan_ura": true,

                "yaku_allow_open_tanyao": true,

                "yaku_extra": [],
                "yaku_block": ["Iipeikou"],

                "kokushi_chankan_allow_ankan": true,
                "swap_call_allow_same": false,
                "swap_call_allow_other": false,
                "riichi_ankan_strict_mode": true,
            });
            let serialized = serde_json::to_value(ruleset.clone()).unwrap();
            let deserialized: Ruleset = serde_json::from_value(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, ruleset);
        }
    }
}
