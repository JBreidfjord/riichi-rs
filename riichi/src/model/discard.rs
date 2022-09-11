use std::fmt::{Display, Formatter};

use crate::{
    common::*,
};

/// A discarded tile with extra info.
///
/// This is both an [`Action`] and an entry in a player's discard stream (see [`State`]).
/// See `called_by` for the difference between the two usages.
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of all fields _with name remapping_.
///
/// Note that [`Action`] will take over the serde format if part of it, so this impl is only for
/// the discard stream in [`State`].
///
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Discard {
    /// The discarded tile.
    pub tile: Tile,

    /// If called by another player, that player; otherwise the player who discarded this tile.
    /// Since this is unknown at the time the action is made, it is ignored in [`Action::Discard`].
    pub called_by: Player,

    /// Whether this tile was discarded as a part of declaring Riichi (立直, リーチ).
    #[cfg_attr(feature = "serde", serde(rename = "riichi"))]
    pub declares_riichi: bool,

    /// Whether this tile was discarded immediately after being drawn (ツモ切り).
    #[cfg_attr(feature = "serde", serde(rename = "tsumokiri"))]
    pub is_tsumokiri: bool,
}

impl Display for Discard {
    // NOTE: we won't be showing `called_by` here; most of the time it's redundant
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.declares_riichi {
            write!(f, "RIICHI!({}{})",
                   self.tile,
                   if self.is_tsumokiri { "*" } else { " " })
        } else {
            write!(f, "discard({}{})",
                   self.tile,
                   if self.is_tsumokiri { "*" } else { " " })
        }
    }
}
