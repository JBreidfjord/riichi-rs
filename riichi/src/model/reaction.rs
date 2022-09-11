//! [`Reaction`] from an out-of-turn player.

use std::fmt::{Display, Formatter};
use crate::common::*;

/// Reaction from an out-of-turn player.
///
/// The lack of reaction / "pass" / unknown reaction can be represented by `Option<Reaction>`.
///
/// Variants are ordered by their priority (`Chii` is the lowest, `RonAgari` the highest).
///
/// ## Optional `serde` support
///
/// `{type, tiles?}` (adjacently tagged, in serde terms)
///
/// - [`Reaction::Chii`], [`Reaction::Pon`] <=> `{"type": "Chii", "tiles": ["1s", "2s"]}`
/// - [`Reaction::Daiminkan`], [`Reaction::RonAgari`] <=> `{"type": "RonAgari"}`
///
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "tiles"))]
pub enum Reaction {
    /// Declare a [`Chii`] (チー) on the recent discard with the specified own tiles.
    Chii(Tile, Tile),

    /// Declare a [`Pon`] (ポン) on the recent discard with the specified own tiles.
    Pon(Tile, Tile),

    /// Declare a [`Daiminkan`] (大明槓) on the recent discard; own tiles are implicit.
    Daiminkan,

    /// Declare win-by-steal (ロン和ガリ) on the recent action.
    ///
    /// The action can be:
    /// - [`super::Action::Discard`] (common)
    /// - [`super::Action::Kakan`] (rare)
    /// - [`super::Action::Ankan`] (very rare)
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

impl Display for Reaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Reaction::Chii(a, b) => write!(f, "Chii({}{})", a.num(), b),
            Reaction::Pon(a, b) => write!(f, "Pon({}{})", a.num(), b),
            Reaction::Daiminkan => write!(f, "Daiminkan"),
            Reaction::RonAgari => write!(f, "Ron"),
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use super::*;

    #[test]
    fn reaction_order_by_priority() {
        use Reaction::*;
        let reactions = [
            Chii(t!("1s"), t!("2s")),
            Chii(t!("2s"), t!("3s")),
            Pon(t!("0p"), t!("5p")),
            Pon(t!("8p"), t!("8p")),
            Daiminkan,
            RonAgari,
        ];
        for (low, high) in reactions.into_iter().tuple_windows() {
            assert!(low < high);
        }
    }

    #[cfg(feature = "serde")]
    mod serde_tests {
        use assert_json_diff::assert_json_eq;
        use super::*;

        #[test]
        fn serde_two_args() {
            let reaction = Reaction::Chii(t!("1s"), t!("2s"));
            let json = serde_json::json!({"type": "Chii", "tiles": ["1s", "2s"]});
            let serialized = serde_json::to_value(reaction).unwrap();
            let deserialized: Reaction = serde_json::from_value(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, reaction);
        }

        #[test]
        fn serde_no_args() {
            let reaction = Reaction::RonAgari;
            let json = serde_json::json!({"type": "RonAgari"});
            let serialized = serde_json::to_value(reaction).unwrap();
            let deserialized: Reaction = serde_json::from_value(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, reaction);
        }
    }
}
