//! [`Ruleset`] of a game.
//!
//! Even though the Japanese Riichi Mahjong is more standardized than other variants of Mahjong,
//! there are detailed rules that may be interpreted in different ways, affecting the validity of
//! certain actions and/or the outcome of the game. There are also naturally "flexible" variations,
//! such as the number of each red 5 tiles in the wall, and how many "normal" kyoku's are allowed
//! (i.e. "East Only" vs. "East-South").
//!
//! This crate attempts to handle most commonly used variations on an arbitrarily-decided "standard"
//! interpretation.
//!

use std::collections::HashSet;

use semver::Version;
#[cfg(feature = "serde")]
use serde_with::{
    serde_as, skip_serializing_none,
    As, DisplayFromStr
};

use crate::{
    common::*,
    model::Yaku,
};

/// Ruleset of a game.
///
/// The [`Default::default()`] ruleset represents our "standard" rules.
///
///
/// ## Key exceptions
///
/// - The number of red tiles is implied by the wall composition, therefore excluded from Ruleset.
/// - The starting points of each player is encoded in [`crate::model::RoundBegin`], therefore
///   excluded from Ruleset.
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
/// ## Optional `serde` support
///
/// Straightforward struct mapping of fields.
/// The SemVer is serialized as a string.
///
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub version: Version,

    /// The Kyoku index of the "all-last" round.
    ///
    /// - For "East Only" games, `kyoku_max_soft == 3` (i.e. East 4 Kyoku, any Honba).
    /// - For "East-South" games, `kyoku_max_soft == 7` (i.e. South 4 Kyoku, any Honba).
    ///
    /// Other fields control the behavior of game beyond the "all-last" round.
    ///
    /// See [`crate::model::RoundId`] for definitions of "Kyoku" and "Honba".
    pub kyoku_max_soft: u8,

    /// The absolute maximum Kyoku a game allows.
    /// At the next attempt to increment the Kyoku number, the game must end.
    pub kyoku_max_hard: u8,

    /// The minimum points any player need to hold for the game to end at or after the "all-last"
    /// round (see `kyoku_max_soft`).
    pub points_min_qualify: GamePoints,

    /// Extra non-standard [`Yaku`]'s to enable (in addition to the standard ones).
    pub yaku_extra: HashSet<Yaku>,

    /// Standard [`Yaku`]'s to disable.
    pub yaku_block: HashSet<Yaku>,

    // TODO
}

impl Default for Ruleset {
    fn default() -> Self {
        Self {
            version: crate::VERSION.clone(),

            kyoku_max_soft: 7,  // East-South (South 4 Kyoku)
            kyoku_max_hard: 15,  // North 4 Kyoku
            points_min_qualify: 30000,

            yaku_extra: Default::default(),
            yaku_block: Default::default(),
            // TODO
        }
    }
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
                yaku_block: HashSet::from([Yaku::Iipeikou]),
                kyoku_max_soft: 3,
                kyoku_max_hard: 7,
                ..Ruleset::default()
            };
            let json = serde_json::json!({
                "version": crate::VERSION_STR,
                "kyoku_max_soft": 3,
                "kyoku_max_hard": 7,
                "points_min_qualify": 30000,
                "yaku_extra": [],
                "yaku_block": ["Iipeikou"]
            });
            let serialized = serde_json::to_value(ruleset.clone()).unwrap();
            let deserialized: Ruleset = serde_json::from_value(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, ruleset);
        }
    }
}
