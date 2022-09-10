//! Unordered multi-sets of tiles, represented as histograms.
//!
//! - When red 5's are counted separately, use [`TileSet37`].
//! - If red 5's are treated the same as normal 5's, use [`TileSet34`].
//! - If all 4 tiles of the same kind are treated the same (i.e. you only need to deal with distinct
//!   tiles), use [`TileMask34`].
//!
//! As the names suggest, the specific encoding of [`Tile`] is assumed.
//!
//! Both [`TileSet37`] and [`TileSet34`] can be indexed with [`Tile`] (red or normal, 37 or 34),
//! without first converting to encoding. A [`TileSet37`] can be converted to a [`TileSet34`] with
//! red 5's folded into normal 5's.

use std::fmt::{Display, Formatter};
use std::ops::{Index, IndexMut};

use derive_more::{
    Constructor, From, Into, IntoIterator, Index, IndexMut,
    BitAnd, BitOr, BitXor,
    BitAndAssign, BitOrAssign, BitXorAssign,
};

use super::Tile;

/// Histogram for all 37 kinds of tiles (including reds).
/// Can be directly indexed with [`Tile`].
#[derive(Clone, Debug, Eq, PartialEq, Constructor, From, Into, IntoIterator, Index, IndexMut)]
pub struct TileSet37(pub [u8; 37]);

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
    fn default() -> Self { TileSet37([0; 37]) }
}

impl Display for TileSet37 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for xs in [
            &self.0[0..9],
            &self.0[9..18],
            &self.0[18..27],
            &self.0[27..34],
            &self.0[34..37],
        ] {
            for x in xs {
                write!(f, "{}", x)?;
            }
            write!(f, ",")?;
        }
        Ok(())
    }
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
    /// An empty tile set.
    pub const fn empty_set() -> Self { TileSet37([0; 37]) }

    /// The complete set of tiles in a game, given the number of red 5's in play.
    /// Each red 5 replaces its corresponding normal 5; the total number of tiles remains 136
    /// (34 x 4).
    pub const fn complete_set(num_reds: [u8; 3]) -> Self {
        let mut a = [4; 37];
        a[34] = num_reds[0];
        a[35] = num_reds[1];
        a[36] = num_reds[2];
        a[4] = 4 - num_reds[0];
        a[13] = 4 - num_reds[1];
        a[22] = 4 - num_reds[2];
        TileSet37(a)
    }

    /// Same as [`TileSet34::packed`], but collapsing red 5's into normal 5's.
    pub fn packed(&self) -> [u32; 4] {
        let mut packed = [0u32; 4];
        let h = &self.0;
        for i in (0..34).rev() {
            let s = i / 9;
            packed[s] = (packed[s] << 3) | (h[i] as u32);
        }
        packed[0] += (h[34] as u32) << (3 * 4);
        packed[1] += (h[35] as u32) << (3 * 4);
        packed[2] += (h[36] as u32) << (3 * 4);
        packed
    }

    /// Iterate through all tiles in this tile set, in encoding order (i.e. reds come after honors).
    pub fn iter_tiles(&self) -> impl Iterator<Item=Tile> {
        self.0.into_iter().enumerate().flat_map(|(encoding, count)|
            itertools::repeat_n(
                Tile::from_encoding(encoding as u8).unwrap(),
                count as usize))
    }
}

/// Histogram for all 34 kinds of normal tiles (red 5's are treated as normal 5's).
/// Can be directly indexed with [`Tile`].
#[derive(Clone, Debug, Eq, PartialEq, Constructor, From, Into, IntoIterator, Index, IndexMut)]
pub struct TileSet34(pub [u8; 34]);

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

impl Display for TileSet34 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for xs in self.0.chunks(9) {
            for x in xs {
                write!(f, "{}", x)?;
            }
            write!(f, ",")?;
        }
        Ok(())
    }
}

// Conversion is one-way from 37 to 34 (count of red is lost).
impl From<&TileSet37> for TileSet34 {
    fn from(original: &TileSet37) -> Self {
        let mut result: [u8; 34] = (original[..34]).try_into().unwrap();
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
    /// An empty tile set.
    pub const fn empty_set() -> Self { TileSet34([0; 34]) }

    /// The complete set of tiles in a game, without differentiating red 5's from normal 5's.
    /// The total number of tiles is 136 (34 x 4).
    pub const fn complete_set() -> Self { TileSet34([4; 34]) }

    /// Reconstruct the histogram from its [packed](Self::packed) representation.
    pub fn from_packed(packed: [u32; 4]) -> Self {
        let mut ts34 = Self::default();
        let mut i = 0;
        for s in 0..3 {
            let mut m = packed[s];
            for _ in 0..9 {
                ts34[i] = (m & 0o7) as u8;
                i += 1;
                m >>= 3;
            }
        }
        let mut m = packed[3];
        for _ in 0..7 {
            ts34[i] = (m & 0o7) as u8;
            i += 1;
            m >>= 3;
        }
        ts34
    }

    /// Compress the histogram so that each element takes 3 bits (valid range `0..=4`).
    /// This results in 4 x 27-bit integers, one for each suit.
    ///
    /// Conveniently, this is 1 digit per element in octal.
    pub fn packed(&self) -> [u32; 4] {
        let mut packed = [0u32; 4];
        let h = &self.0;
        for i in (0..34).rev() {
            let s = i / 9;
            packed[s] = (packed[s] << 3) | (h[i] as u32);
        }
        packed
    }

    /// Iterate through all tiles in this tile set, in encoding order.
    pub fn iter_tiles(&self) -> impl Iterator<Item=Tile> {
        self.0.into_iter().enumerate().flat_map(|(encoding, count)|
            itertools::repeat_n(
                Tile::from_encoding(encoding as u8).unwrap(),
                count as usize))
    }
}

/// 1-bit-per-tile version of [`TileSet34`], i.e. non-multi set, set of tile kinds.
#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq,
    Constructor, From, Into,
    BitAnd, BitOr, BitXor,
    BitAndAssign, BitOrAssign, BitXorAssign,
)]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    use std::str::FromStr;
    use crate::common::tiles_from_str;

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
        assert_eq!(TileSet34::from_packed(h.packed()), h);
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
