#![doc = include_str!("../README.md")]

pub mod decomposer;
pub mod irregular;
pub mod regular;

use std::fmt::{Display, Formatter};

use itertools::Itertools;

use riichi_elements::prelude::*;

pub use riichi_decomp_table::WaitingKind;
pub use self::{
    decomposer::Decomposer,
    regular::RegularWait,
    irregular::{IrregularWait, detect_irregular_wait},
};

/// One waiting pattern, either [`RegularWait`] or [`IrregularWait`].
///
/// ## Optional `serde` support
///
/// Serialization only. `{type, wait}` (adjacently tagged).
///
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "wait"))]
pub enum Wait {
    Regular(RegularWait),
    Irregular(IrregularWait),
}

/// All the ways a player's closed hand can be considered waiting, regular and/or irregular.
///
/// ## Optional `serde` support
///
/// Serialization only.
/// Straightforward struct mapping of fields.
///
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WaitSet {
    /// The set of all waiting tiles in all different ways of waiting.
    pub waiting_tiles: TileMask34,

    /// Regular waiting patterns (groups and a pair).
    pub regular: Vec<RegularWait>,

    /// Irregular waiting pattern (seven pairs, thirteen orphans).
    pub irregular: Option<IrregularWait>,
}

impl WaitSet {
    pub fn from_keys(decomposer: &mut Decomposer, keys: &[u32; 4]) -> Self {
        let mut waiting_tiles = TileMask34::default();
        let regular = decomposer.with_keys(*keys).iter().collect_vec();
        for wait in regular.iter() {
            waiting_tiles.0 |= 1 << wait.waiting_tile.encoding() as u64;
        }
        let irregular = detect_irregular_wait(*keys);
        if let Some(irregular) = irregular {
            waiting_tiles |= irregular.to_waiting_set();
        }
        Self { waiting_tiles, regular, irregular }
    }

    pub fn from_tile_set(decomposer: &mut Decomposer, tile_set: &TileSet34) -> Self {
        Self::from_keys(decomposer, &tile_set.packed_34())
    }
}

impl Display for WaitSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{waiting_tiles={}", self.waiting_tiles)?;
        if let Some(irregular) = self.irregular {
            write!(f, " irregular={}", irregular)?;
        }
        write!(f, " regular=[")?;
        for w in &self.regular {
            write!(f, "({}),", w)?;
        }
        write!(f, "]}}")
    }
}
