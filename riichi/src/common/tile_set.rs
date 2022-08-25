//! Unordered multi-sets of tiles, represented as histograms.
//!
//! - When red 5's are counted separately, use [`TileSet37`].
//! - If red 5's are treated the same as normal 5's, use [`TileSet34`].
//!
//! Both can be directly indexed with [`Tile`] (red or normal, 37 or 34).
//!
//! A [`TileSet37`] can be converted to a [`TileSet34`] with red 5's folded into normal 5's.

use std::ops::{Index, IndexMut};

use derive_more::{
    Constructor, From, Into, Index, IndexMut,
    BitAnd, BitOr, BitXor,
    BitAndAssign, BitOrAssign, BitXorAssign,
};

use crate::Tile;

/// Histogram for all 37 kinds of tiles (including red).
/// Can be directly indexed with [`Tile`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Constructor, From, Into, Index, IndexMut)]
pub struct TileSet37([u8; 37]);

impl Index<Tile> for TileSet37 {
    type Output = u8;
    fn index(&self, tile: Tile) -> &Self::Output {
        &self.0[tile.encoding() as usize]
    }
}

impl IndexMut<Tile> for TileSet37 {
    fn index_mut(&mut self, tile: Tile) -> &mut Self::Output {
        &mut self.0[tile.encoding() as usize]
    }
}

impl Default for TileSet37 {
    fn default() -> Self { TileSet37([0u8; 37]) }
}

impl FromIterator<Tile> for TileSet37 {
    fn from_iter<T: IntoIterator<Item=Tile>>(tiles: T) -> Self {
        let mut ts = Self::default();
        for tile in tiles {
            ts[tile] += 1;
        }
        ts
    }
}

impl TileSet37 {
    pub fn to_sorted_vec(&self) -> Vec<Tile> {
        let mut tiles: Vec<Tile> = vec![];
        tiles.reserve_exact(self.0.into_iter().sum::<u8>() as usize);
        for (encoding, count) in self.0.into_iter().enumerate() {
            for _ in 0..count {
                tiles.push(Tile::from_encoding(encoding as u8).unwrap());
            }
        }
        tiles
    }
}

/// Histogram for all 34 kinds of normal tiles (red 5's are treated as normal 5's).
/// Can be directly indexed with [`Tile`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Constructor, From, Into, Index, IndexMut)]
pub struct TileSet34([u8; 34]);

impl Index<Tile> for TileSet34 {
    type Output = u8;
    fn index(&self, tile: Tile) -> &Self::Output {
        &self.0[tile.normal_encoding() as usize]  // NOTE: different
    }
}

impl IndexMut<Tile> for TileSet34 {
    fn index_mut(&mut self, tile: Tile) -> &mut Self::Output {
        &mut self.0[tile.normal_encoding() as usize]  // NOTE: different
    }
}

impl Default for TileSet34 {
    fn default() -> Self { TileSet34([0u8; 34]) }
}

// Conversion is one-way from 37 to 34 (count of red is lost).
impl From<TileSet37> for TileSet34 {
    fn from(original: TileSet37) -> Self {
        let mut result: [u8; 34] = (&original[..34]).try_into().unwrap();
        result[4] += original[34];
        result[13] += original[35];
        result[22] += original[36];
        Self(result)
    }
}

impl FromIterator<Tile> for TileSet34 {
    fn from_iter<T: IntoIterator<Item=Tile>>(tiles: T) -> Self {
        let mut ts = Self::default();
        for tile in tiles {
            ts[tile.to_normal()] += 1;
        }
        ts
    }
}

impl TileSet34 {
    pub fn to_sorted_vec(&self) -> Vec<Tile> {
        let mut tiles: Vec<Tile> = vec![];
        tiles.reserve_exact(self.0.into_iter().sum::<u8>() as usize);
        for (encoding, count) in self.0.into_iter().enumerate() {
            for _ in 0..count {
                tiles.push(Tile::from_encoding(encoding as u8).unwrap());
            }
        }
        tiles
    }

    /// Compress the histogram so that each element takes 3 bits (valid range `0..=4`).
    /// This results in 4 x 27-bit integers, one for each suit.
    ///
    /// Conveniently this is 1 digit per element in octal.
    pub fn packed(&self) -> [u32; 4] {
        let mut packed = [0u32; 4];
        let h = &self.0;
        for i in (0..34).rev() {
            let s = i / 9;
            packed[s] = (packed[s] << 3) | (h[i] as u32);
        }
        packed
    }
}

/// 1-bit-per-tile version of [`TileSet34`], i.e. non-multi set.
#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq,
    Constructor, From, Into,
    BitAnd, BitOr, BitXor,
    BitAndAssign, BitOrAssign, BitXorAssign,
)]
pub struct TileMask34(pub u64);

impl FromIterator<Tile> for TileMask34 {
    fn from_iter<T: IntoIterator<Item=Tile>>(tiles: T) -> Self {
        let mut mask = 0u64;
        for tile in tiles {
            mask |= 1u64 << tile.normal_encoding() as u64;
        }
        Self(mask)
    }
}

impl TileMask34 {
    pub fn has(self, tile: Tile) -> bool {
        (self.0 >> (tile.normal_encoding() as u64)) & 1 == 1
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use crate::tiles_from_str;

    #[test]
    fn histogram_can_be_indexed_with_tile() {
        let mut h = TileSet37::from_iter(tiles_from_str("1112345678999m"));
        h[Tile::from_str("9m").unwrap()] -= 2;
        h[Tile::from_str("7z").unwrap()] += 2;
        assert_eq!(h, [
            3, 1, 1, 1, 1, 1, 1, 1, 1,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 2,
            0, 0, 0u8,
        ].into());
    }

    #[test]
    fn ts34_treats_red_as_normal() {
        let mut h = TileSet34::default();
        h[Tile::from_str("5m").unwrap()] = 1;
        h[Tile::from_str("0p").unwrap()] = 2;
        h[Tile::from_str("6s").unwrap()] = 3;
        assert_eq!(h, [
            0, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 2, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 3, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ].into());
    }

    #[test]
    fn ts34_packs_correctly() {
        let h = TileSet34::from_iter(tiles_from_str("147m258p369s77z"));
        assert_eq!(h.packed(), [
            0o001001001,
            0o010010010,
            0o100100100,
            0o2000000,
        ]);
    }

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
