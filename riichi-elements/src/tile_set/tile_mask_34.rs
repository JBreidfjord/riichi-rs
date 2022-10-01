use core::fmt::{Display, Formatter};

use derive_more::{
    Constructor, From, Into,
    BitAnd, BitOr, BitXor,
    BitAndAssign, BitOrAssign, BitXorAssign,
};

use crate::tile::Tile;
use super::{TileSet37, TileSet34};

/// 1-bit-per-tile version of [`TileSet34`], i.e. non-multi set, set of tile kinds.
#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq,
    Constructor, From, Into,
    BitAnd, BitOr, BitXor,
    BitAndAssign, BitOrAssign, BitXorAssign,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]  // TODO(summivox): don't cheat
pub struct TileMask34(pub u64);

impl TileMask34 {
    /// An empty tile set.
    pub const fn empty_set() -> Self { Self(0) }

    /// The complete set of tiles (all 34 kinds).
    pub const fn complete_set() -> Self { Self((1 << 34) - 1) }

    /// Returns if this is an empty set.
    pub fn is_empty(self) -> bool { self.0 == 0 }

    /// Returns if there is at least one kind of tile in the set.
    pub fn any(self) -> bool { self.0 > 0 }

    /// Tests if this set contains the given tile.
    pub fn has(self, tile: Tile) -> bool {
        (self.0 >> (tile.normal_encoding() as u64)) & 1 == 1
    }

    /// Tests if this set contains the given tile encoding.
    pub fn has_i(self, i: u8) -> bool {
        (self.0 >> (i as u64)) & 1 == 1
    }

    /// Mutates the set to include the given tile.
    pub fn set(&mut self, tile: Tile) {
        self.0 |= 1 << (tile.normal_encoding() as u64);
    }

    /// Mutates the set to exclude the given tile.
    pub fn clear(&mut self, tile: Tile) {
        self.0 &= !(1 << (tile.normal_encoding() as u64));
    }
}

impl FromIterator<Tile> for TileMask34 {
    fn from_iter<T: IntoIterator<Item=Tile>>(tiles: T) -> Self {
        let mut mask = 0u64;
        for tile in tiles {
            mask |= 1u64 << tile.normal_encoding() as u64;
        }
        Self(mask)
    }
}

impl From<TileSet37> for TileMask34 {
    fn from(ts37: TileSet37) -> Self {
        let mut mask = 0u64;
        for i in 0..34 {
            if ts37[i] > 0 {
                mask |= 1 << i;
            }
        }
        if ts37[34] > 0 { mask |= 1 << 4; }
        if ts37[35] > 0 { mask |= 1 << 13; }
        if ts37[36] > 0 { mask |= 1 << 22; }
        Self(mask)
    }
}

impl From<TileSet34> for TileMask34 {
    fn from(ts34: TileSet34) -> Self {
        let mut mask = 0u64;
        for i in 0..34 {
            if ts34[i] > 0 {
                mask |= 1 << i;
            }
        }
        Self(mask)
    }
}

impl Display for TileMask34 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for r in [0..9, 9..18, 18..27, 27..34] {
            for i in r {
                write!(f, "{}", (self.0 >> i) & 1)?;
            }
            write!(f, ",")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::*;

    #[test]
    fn mask34_examples() {
        let tiles1 = tiles_from_str("147m208p369s77z");
        let mask1 = TileMask34::from_iter(tiles1);
        assert_eq!(u64::from(mask1), 0b1000000100100100010010010001001001u64);

        let tiles2 = tiles_from_str("1112345678999m");
        let mask2 = TileMask34::from_iter(tiles2);
        assert_eq!(u64::from(mask2), 0b111111111u64);

        let mask_and = mask1 & mask2;
        assert_eq!(u64::from(mask_and), 0b001001001u64);
    }
}
