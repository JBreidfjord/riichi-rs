//! [`Action`] by the in-turn player.

use std::fmt::{Display, Formatter};
use crate::common::*;
use super::Discard;

/// Action by the in-turn player.
///
/// ## Optional `serde` support
///
/// [`Action`] adopts a custom layout `{type, tile?, riichi?, tsumokiri?}` to help with ergonomics.
///
/// - The `"tile"` field is defined the same as [`Action::tile()`].
/// - Only for [`Action::Discard`]: optionally add `"tsumokiri"` and `"riichi"` flags if set.
///
/// Examples:
///
/// - [`Action::Discard`] <=> `{"type": "Discard", "tile": "1m", "riichi": true, "tsumo": true}`
/// - [`Action::Ankan`], [`Action::Kakan`], [`Action::TsumoAgari`] <=>
///   `{"type": "TsumoAgari", "tile": "5z"}`
/// - [`Action::AbortNineKinds`] <=> `{"type": "AbortNineKinds"}`
///
/// Note that the `called_by` field of [`Discard`] is deliberately excluded.
///
#[derive(Copy, Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Action {
    /// Discard a tile (打牌). See [`Discard`].
    /// The `called_by` field is implied and can be safely ignored here.
    Discard(Discard),

    /// Declare an [`Ankan`] (暗槓; 4 in closed hand).
    Ankan(Tile),

    /// Declare a [`Kakan`] (加槓; 1 in closed hand, 3 in pon).
    Kakan(Tile),

    /// Win by self-draw (ツモ和ガリ).
    /// See [`super::ActionResult::Agari`], [`super::AgariKind::Tsumo`].
    TsumoAgari(Tile),

    /// Abort by Nine Kinds of Terminals (九種九牌).
    /// See [`super::ActionResult::Abort`], [`super::AbortReason::NineKinds`].
    AbortNineKinds,
}

impl Action {
    /// Construct the action corresponding to [`Meld::Kakan`] / [`Meld::Ankan`].
    pub fn from_meld(meld: &Meld) -> Option<Self> {
        match meld {
            Meld::Kakan(kakan) => Some(Action::Kakan(kakan.added)),
            Meld::Ankan(ankan) => Some(Action::Ankan(ankan.own[0].to_normal())),
            _ =>  None,
        }
    }

    /// Returns the tile argument of each kind of action, except [`Action::AbortNineKinds`] for
    /// which `None` is returned.
    pub fn tile(self) -> Option<Tile> {
        match self {
            Action::Discard(discard) => Some(discard.tile),
            Action::Ankan(tile) => Some(tile),
            Action::Kakan(tile) => Some(tile),
            Action::TsumoAgari(tile) => Some(tile),
            Action::AbortNineKinds => None,
        }
    }

    /// Does this action end the round?
    pub fn is_terminal(self) -> bool {
        matches!(self, Action::TsumoAgari(_) | Action::AbortNineKinds)
    }

    /// Returns whether this action produces a Kan.
    pub fn is_kan(self) -> bool {
        matches!(self, Action::Ankan(_)| Action::Kakan(_))
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Discard(discard) => write!(f, "{}", discard),
            Action::Ankan(tile) => write!(f, "Ankan({})", tile),
            Action::Kakan(tile) => write!(f, "Kakan({})", tile),
            Action::TsumoAgari(tile) => write!(f, "Tsumo({})", tile),
            Action::AbortNineKinds => write!(f, "NineKinds"),
        }
    }
}

#[cfg(feature = "serde")]
mod action_serde {
    use serde::{
        de::{Error, MapAccess},
        ser::{SerializeStruct},
        Deserializer,
        Serializer
    };
    use super::*;
    impl serde::Serialize for Action {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error> where S: Serializer {
            match self {
                Action::Discard(discard) => {
                    let mut st = s.serialize_struct("Action", 4)?;
                    st.serialize_field("type", "Discard")?;
                    st.serialize_field("tile", &discard.tile)?;
                    if discard.declares_riichi {
                        st.serialize_field("riichi", &true)?;
                    }
                    if discard.is_tsumokiri {
                        st.serialize_field("tsumokiri", &true)?;
                    }
                    st.end()
                }
                Action::Ankan(t) => {
                    let mut st = s.serialize_struct("Action", 2)?;
                    st.serialize_field("type", "Ankan")?;
                    st.serialize_field("tile", &t)?;
                    st.end()
                }
                Action::Kakan(t) => {
                    let mut st = s.serialize_struct("Action", 2)?;
                    st.serialize_field("type", "Kakan")?;
                    st.serialize_field("tile", &t)?;
                    st.end()
                }
                Action::TsumoAgari(t) => {
                    let mut st = s.serialize_struct("Action", 2)?;
                    st.serialize_field("type", "TsumoAgari")?;
                    st.serialize_field("tile", &t)?;
                    st.end()
                }
                Action::AbortNineKinds => {
                    let mut st = s.serialize_struct("Action", 1)?;
                    st.serialize_field("type", "AbortNineKinds")?;
                    st.end()
                }
            }
        }
    }

    impl<'de> serde::Deserialize<'de> for Action {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
            deserializer.deserialize_any(ActionVisitor)
        }
    }

    struct ActionVisitor;

    impl<'de> serde::de::Visitor<'de> for ActionVisitor {
        type Value = Action;

        fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
            write!(f, r#"{{"type": "Discard" or "Ankan" or "Kakan" or "TsumoAgari" or "AbortNineKinds", ...}}"#)
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
            let mut kind = String::new();
            let mut tile = Tile::MIN;
            let mut declares_riichi = false;
            let mut is_tsumokiri = false;

            while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                match key.as_str() {
                    "type" =>
                        kind = value.as_str()
                            .ok_or(Error::custom("invalid type"))?
                            .to_string(),
                    "tile" =>
                        tile = value.as_str().and_then(|str| str.parse().ok())
                            .ok_or(Error::custom("invalid tile"))?,
                    "tsumokiri" =>
                        is_tsumokiri = value.as_bool()
                            .ok_or(Error::custom("invalid tsumokiri"))?,
                    "riichi" =>
                        declares_riichi = value.as_bool()
                            .ok_or(Error::custom("invalid riichi"))?,
                    _ => {}
                }
            }
            match kind.as_str() {
                "Discard" => Ok(Action::Discard(Discard {
                    tile, declares_riichi, is_tsumokiri, called_by: P0
                })),
                "Ankan" => Ok(Action::Ankan(tile)),
                "Kakan" => Ok(Action::Kakan(tile)),
                "TsumoAgari" => Ok(Action::TsumoAgari(tile)),
                "AbortNineKinds" => Ok(Action::AbortNineKinds),
                _ => Err(Error::custom("invalid type")),
            }
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
        fn serde_discard() {
            let action = Action::Discard(Discard{
                tile: t!("1m"), called_by: P0, is_tsumokiri: false, declares_riichi: true});
            let json = serde_json::json!({
                "type": "Discard", "tile": "1m", "riichi": true
            });
            let serialized = serde_json::to_value(action).unwrap();
            let deserialized = serde_json::from_value(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(action, deserialized);
        }

        #[test]
        fn serde_one_arg() {
            let action = Action::TsumoAgari(t!("5z"));
            let json = serde_json::json!({
                "type": "TsumoAgari", "tile": "5z"
            });
            let serialized = serde_json::to_value(action).unwrap();
            let deserialized = serde_json::from_value(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(action, deserialized);
        }

        #[test]
        fn serde_zero_arg() {
            let action = Action::AbortNineKinds;
            let json = serde_json::json!({
                "type": "AbortNineKinds"
            });
            let serialized = serde_json::to_value(action).unwrap();
            let deserialized = serde_json::from_value(json.clone()).unwrap();
            assert_json_eq!(serialized, json);
            assert_eq!(action, deserialized);
        }
    }

}
