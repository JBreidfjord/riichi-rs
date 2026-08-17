//! [`Meld`] (副露) = one of [`Chii`], [`Pon`], [`Kakan`], [`Daiminkan`], [`Ankan`], [`Kita`].
//!
//! ## Ref
//!
//! - <https://riichi.wiki/Naki>
//! - <https://ja.wikipedia.org/wiki/副露>

use core::fmt::{Display, Formatter};

use crate::{hand_group::HandGroup, player::*, tile::Tile, tile_set::*};

mod ankan;
mod chii;
mod daiminkan;
mod kakan;
mod kita;
mod packed;
mod pon;
mod utils;

pub use ankan::Ankan;
pub use chii::Chii;
pub use daiminkan::Daiminkan;
pub use kakan::Kakan;
pub use kita::{Kita, NORTH};
pub use pon::Pon;

/// Sum type of all kinds of melds (副露).
///
/// This is one of: [`Chii`], [`Pon`], [`Kakan`], [`Daiminkan`], [`Ankan`], [`Kita`].
///
///
/// ## Optional `serde` support
///
/// `{type, ...}` where `...` represents the flattened fields of the actual meld.
///
/// Examples:
///
/// - `{"type": "Chii", "own": ["4s", "6s"], "called": "0s", "min": "4s"}`
/// - `{"type": "Pon", "own": ["0p", "5p"], "called": "0p", "dir": 2}`
/// - `{"type": "Kakan", "own": ["0p", "5p"], "called": "0p", "dir": 1, "added": "5p"}`
/// - `{"type": "Daiminkan", "own": ["0s", "5s", "5s"], "called": "0s", "dir": 3}`
/// - `{"type": "Ankan", "own": ["4z", "4z", "4z", "4z"]}`
/// - `{"type": "Kita", "own": ["4z"]}`
///
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum Meld {
    /// See [`Chii`].
    Chii(Chii),
    /// See [`Pon`].
    Pon(Pon),
    /// See [`Kakan`].
    Kakan(Kakan),
    /// See [`Daiminkan`].
    Daiminkan(Daiminkan),
    /// See [`Ankan`].
    Ankan(Ankan),
    /// See [`Kita`]. Sanma only.
    Kita(Kita),
}

impl Meld {
    /// [`Ankan`] or [`Kita`] --- i.e. this meld does not open the hand.
    ///
    /// [`Kita`] counts as closed because extracting a North leaves the hand menzen: riichi,
    /// menzen tsumo and the closed-hand yaku all remain available afterwards. (An extraction is
    /// "treated like a call" only for the purposes that kill Ippatsu / Chiihou / Kyuushuu /
    /// Double Riichi --- see the Ippatsu handling in the engine, not here.)
    pub fn is_closed(&self) -> bool {
        matches!(self, Meld::Ankan(_) | Meld::Kita(_))
    }

    /// [`Kita`] --- is this the North extraction rather than a called/formed group?
    ///
    /// A Kita is *not* part of the winning hand shape, contributes no fu, and must be excluded
    /// from every group-shaped scan over a seat's meld list.
    pub fn is_kita(&self) -> bool {
        matches!(self, Meld::Kita(_))
    }

    /// [`Kakan`], [`Daiminkan`], or [`Ankan`]
    pub fn is_kan(&self) -> bool {
        matches!(self, Meld::Kakan(_) | Meld::Daiminkan(_) | Meld::Ankan(_))
    }

    /// Returns the called tile for [`Chii`], [`Pon`], [`Daiminkan`], or [`Kakan`].
    pub fn called(&self) -> Option<Tile> {
        match self {
            Self::Chii(chii) => Some(chii.called),
            Self::Pon(pon) => Some(pon.called),
            Self::Daiminkan(daiminkan) => Some(daiminkan.called),
            Self::Kakan(kakan) => Some(kakan.pon.called),
            Self::Ankan(_) => None,
            Self::Kita(_) => None,
        }
    }

    /// Returns where this meld is called from (relative to this [`Player`]).
    /// - [`Chii`]: Always the previous player (+3).
    /// - [`Ankan`]: Not called from anyone, so `None`.
    pub fn dir(&self) -> Option<Player> {
        match self {
            Self::Chii(_) => Some(P3),
            Self::Pon(pon) => Some(pon.dir),
            Self::Daiminkan(daiminkan) => Some(daiminkan.dir),
            Self::Kakan(kakan) => Some(kakan.pon.dir),
            Self::Ankan(_) => None,
            Self::Kita(_) => None,
        }
    }

    /// Maps to the equivalent closed-hand group. Useful for e.g. winning condition calculations.
    /// - [`Chii`] => [`HandGroup::Shuntsu`]
    /// - [`Pon`]/Kan => [`HandGroup::Koutsu`] (ignoring the 4th tile)
    /// - [`Kita`] => `None`; an extracted North is not a group at all.
    ///
    /// Prefer [`Self::to_group`] at any site that must not silently treat a Kita as a triplet.
    /// This method panics on [`Kita`] deliberately: every historical caller assumed a meld is
    /// always a group, and a wrong answer there is a silently-inflated hand.
    pub fn to_equivalent_group(&self) -> HandGroup {
        self.to_group().expect("Kita is not a hand group; use `to_group`")
    }

    /// Maps to the equivalent closed-hand group, or `None` for [`Kita`].
    pub fn to_group(&self) -> Option<HandGroup> {
        use HandGroup::*;
        Some(match self {
            Meld::Chii(chii) => Shuntsu(chii.min),
            Meld::Pon(pon) => Koutsu(pon.called.to_normal()),
            Meld::Kakan(kakan) => Koutsu(kakan.added.to_normal()),
            Meld::Daiminkan(daiminkan) => Koutsu(daiminkan.called.to_normal()),
            Meld::Ankan(ankan) => Koutsu(ankan.own[0].to_normal()),
            Meld::Kita(_) => return None,
        })
    }

    /// Removes this meld's "own tile(s)" from the closed hand.
    /// For [`Kakan`], the added tile is removed, since the underlying [`Pon`] already exists.
    pub fn consume_from_hand(&self, hand: &mut TileSet37) {
        match self {
            Meld::Chii(chii) => chii.consume_from_hand(hand),
            Meld::Pon(pon) => pon.consume_from_hand(hand),
            Meld::Daiminkan(daiminkan) => daiminkan.consume_from_hand(hand),
            Meld::Kakan(kakan) => kakan.consume_from_hand(hand),
            Meld::Ankan(ankan) => ankan.consume_from_hand(hand),
            Meld::Kita(kita) => kita.consume_from_hand(hand),
        }
    }

    pub fn to_tiles(&self) -> Vec<Tile> {
        match self {
            Meld::Chii(chii) => vec![chii.own[0], chii.own[1], chii.called],
            Meld::Pon(pon) => vec![pon.own[0], pon.own[1], pon.called],
            Meld::Kakan(kakan) => {
                vec![kakan.pon.own[0], kakan.pon.own[1], kakan.pon.called, kakan.added]
            }
            Meld::Daiminkan(daiminkan) => {
                vec![daiminkan.own[0], daiminkan.own[1], daiminkan.own[2], daiminkan.called]
            }
            Meld::Ankan(ankan) => ankan.own.to_vec(),
            // The extracted North *is* set aside face-up and therefore visible, so it belongs
            // here for wall-accounting purposes even though it is not part of the hand shape.
            Meld::Kita(kita) => kita.own.to_vec(),
        }
    }
}

impl Display for Meld {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        // Different melds' string representations are already distinct; simply pass through.
        match self {
            Meld::Chii(chii) => write!(f, "{}", chii),
            Meld::Pon(pon) => write!(f, "{}", pon),
            Meld::Kakan(kakan) => write!(f, "{}", kakan),
            Meld::Daiminkan(daiminkan) => write!(f, "{}", daiminkan),
            Meld::Ankan(ankan) => write!(f, "{}", ankan),
            Meld::Kita(kita) => write!(f, "{}", kita),
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use std::string::ToString;

    use super::*;
    use crate::t;

    #[test]
    fn chii_example() {
        let chii = Chii::from_tiles(t!("4s"), t!("6s"), t!("0s")).unwrap();
        let meld = Meld::Chii(chii);
        let packed = 0x1155;
        assert_eq!(Meld::from_packed(packed), Some(meld));
        assert_eq!(meld.packed(), packed);
        assert_eq!(chii.to_string(), "C046s");
        assert_eq!(meld.to_string(), "C046s");

        assert_eq!(meld.called(), Some(t!("0s")));
        assert_eq!(meld.dir(), Some(P3));
        assert_eq!(meld.to_equivalent_group(), HandGroup::Shuntsu(t!("4s")));
    }

    #[test]
    fn pon_example() {
        let pon = Pon::from_tiles_dir(t!("5p"), t!("0p"), t!("0p"), P2).unwrap();
        let meld = Meld::Pon(pon);
        let packed = 0x258D;
        assert_eq!(Meld::from_packed(packed), Some(meld));
        assert_eq!(meld.packed(), packed);
        assert_eq!(pon.to_string(), "0P05p");
        assert_eq!(meld.to_string(), "0P05p");

        assert_eq!(meld.called(), Some(t!("0p")));
        assert_eq!(meld.dir(), Some(P2));
        assert_eq!(meld.to_equivalent_group(), HandGroup::Koutsu(t!("5p")));
    }

    #[test]
    fn kakan_example() {
        let kakan = Kakan::from_pon_added(
            Pon::from_tiles_dir(t!("5p"), t!("0p"), t!("0p"), P1).unwrap(),
            t!("5p"),
        )
        .unwrap();
        let meld = Meld::Kakan(kakan);
        let packed = 0x354D;
        assert_eq!(Meld::from_packed(packed), Some(meld));
        assert_eq!(meld.packed(), packed);
        assert_eq!(kakan.to_string(), "05K(5/0)p");
        assert_eq!(meld.to_string(), "05K(5/0)p");

        assert_eq!(meld.called(), Some(t!("0p")));
        assert_eq!(meld.dir(), Some(P1));
        assert_eq!(meld.to_equivalent_group(), HandGroup::Koutsu(t!("5p")));
    }

    #[test]
    fn daiminkan_example() {
        let daiminkan =
            Daiminkan::from_tiles_dir([t!("5s"), t!("0s"), t!("5s")], t!("0s"), P3).unwrap();
        let meld = Meld::Daiminkan(daiminkan);
        let packed = 0x49D6;
        assert_eq!(Meld::from_packed(packed), Some(meld));
        assert_eq!(meld.packed(), packed);
        assert_eq!(daiminkan.to_string(), "D0055s");
        assert_eq!(meld.to_string(), "D0055s");

        assert_eq!(meld.called(), Some(t!("0s")));
        assert_eq!(meld.dir(), Some(P3));
        assert_eq!(meld.to_equivalent_group(), HandGroup::Koutsu(t!("5s")));
    }

    #[test]
    fn ankan_example() {
        let ankan = Ankan::from_tiles([t!("4z"), t!("4z"), t!("4z"), t!("4z")]).unwrap();
        let meld = Meld::Ankan(ankan);
        let packed = 0x501E;
        assert_eq!(Meld::from_packed(packed), Some(meld));
        assert_eq!(meld.packed(), packed);
        assert_eq!(ankan.to_string(), "A4444z");
        assert_eq!(meld.to_string(), "A4444z");

        assert_eq!(meld.called(), None);
        assert_eq!(meld.dir(), None);
        assert_eq!(meld.to_equivalent_group(), HandGroup::Koutsu(t!("4z")));
    }

    #[test]
    fn kita_example() {
        let kita = Kita::new();
        let meld = Meld::Kita(kita);
        assert_eq!(Meld::from_packed(meld.packed()), Some(meld));
        assert_eq!(kita.to_string(), "N4z");
        assert_eq!(meld.to_string(), "N4z");

        assert_eq!(meld.called(), None);
        assert_eq!(meld.dir(), None);
        // A Kita is not a hand group, and does not open the hand.
        assert_eq!(meld.to_group(), None);
        assert!(meld.is_closed());
        assert!(meld.is_kita());
        assert!(!meld.is_kan());
        assert_eq!(meld.to_tiles(), vec![t!("4z")]);
        assert_eq!(kita.tile(), t!("4z"));
        assert_eq!(kita.own, [t!("4z")]);

        // Only a North can be extracted.
        assert_eq!(Kita::from_tile(t!("4z")), Some(kita));
        assert_eq!(Kita::from_tile(t!("1z")), None);
        assert_eq!(Kita::from_tile(t!("3z")), None);
    }

    #[test]
    #[should_panic]
    fn kita_is_not_a_hand_group() {
        let _ = Meld::Kita(Kita::new()).to_equivalent_group();
    }

    /// Every existing meld kind must keep its exact packed encoding: the packed `u16` is
    /// persisted downstream, so adding `Kita = 6` must not perturb kinds 1..=5.
    #[test]
    fn packed_encodings_are_stable() {
        assert_eq!(Meld::Chii(Chii::from_tiles(t!("4s"), t!("6s"), t!("0s")).unwrap()).packed(),
                   0x1155);
        assert_eq!(Meld::Pon(Pon::from_tiles_dir(t!("5p"), t!("0p"), t!("0p"), P2)
                       .unwrap()).packed(), 0x258D);
        assert_eq!(Meld::Ankan(Ankan::from_tiles(
                       [t!("4z"), t!("4z"), t!("4z"), t!("4z")]).unwrap()).packed(), 0x501E);
    }

    #[test]
    fn null_example() {
        assert_eq!(Meld::from_packed(0), None);
    }

    #[test]
    fn sizeof() {
        std::println!(
            "Meld={} (align={}), Option<Meld>={} (align={})",
            core::mem::size_of::<Meld>(),
            core::mem::align_of::<Meld>(),
            core::mem::size_of::<Option<Meld>>(),
            core::mem::align_of::<Option<Meld>>(),
        );
    }

    #[cfg(all(feature = "serde", feature = "std"))]
    mod serde_tests {
        use super::*;
        use assert_json_diff::assert_json_eq;
        #[test]
        fn serde_chii() {
            let meld = Meld::Chii(Chii::from_tiles(t!("4s"), t!("6s"), t!("0s")).unwrap());
            let json = serde_json::json!(
                {"type": "Chii", "own": ["4s", "6s"], "called": "0s", "min": "4s"});
            let serialized = serde_json::to_value(meld).unwrap();
            let deserialized = serde_json::from_value::<Meld>(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, meld);
        }

        #[test]
        fn serde_pon() {
            let meld = Meld::Pon(Pon::from_tiles_dir(t!("5p"), t!("0p"), t!("0p"), P2).unwrap());
            let json = serde_json::json!(
                {"type": "Pon", "own": ["0p", "5p"], "called": "0p", "dir": 2});
            let serialized = serde_json::to_value(meld).unwrap();
            let deserialized = serde_json::from_value::<Meld>(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, meld);
        }

        #[test]
        fn serde_kakan() {
            let meld = Meld::Kakan(
                Kakan::from_pon_added(
                    Pon::from_tiles_dir(t!("5p"), t!("0p"), t!("0p"), P1).unwrap(),
                    t!("5p"),
                )
                .unwrap(),
            );
            let json = serde_json::json!(
                {"type": "Kakan", "own": ["0p", "5p"], "called": "0p", "dir": 1, "added": "5p"});
            let serialized = serde_json::to_value(meld).unwrap();
            let deserialized = serde_json::from_value::<Meld>(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, meld);
        }

        #[test]
        fn serde_daiminkan() {
            let meld = Meld::Daiminkan(
                Daiminkan::from_tiles_dir([t!("5s"), t!("0s"), t!("5s")], t!("0s"), P3).unwrap(),
            );
            let json = serde_json::json!(
                {"type": "Daiminkan", "own": ["0s", "5s", "5s"], "called": "0s", "dir": 3});
            let serialized = serde_json::to_value(meld).unwrap();
            let deserialized = serde_json::from_value::<Meld>(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, meld);
        }

        #[test]
        fn serde_ankan() {
            let meld =
                Meld::Ankan(Ankan::from_tiles([t!("4z"), t!("4z"), t!("4z"), t!("4z")]).unwrap());
            let json = serde_json::json!(
                {"type": "Ankan", "own": ["4z", "4z", "4z", "4z"]});
            let serialized = serde_json::to_value(meld).unwrap();
            let deserialized = serde_json::from_value::<Meld>(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(deserialized, meld);
        }
    }
}
