//! [Hand Group](HandGroup), a.k.a. Mentsu 面子
//!
//! ## Ref
//! - <https://riichi.wiki/Mentsu>
//! - <https://ja.wikipedia.org/wiki/%E9%9D%A2%E5%AD%90_(%E9%BA%BB%E9%9B%80)>

use std::fmt::{Display, Formatter};
use super::Tile;

/// A group of 3 tiles within a player's _closed_ hand, a.k.a. Mentsu 面子.
///
/// Can be either:
/// - Koutsu (暗)刻子: 3 of a kind (ignoring red); e.g. `222z`, `055m`
/// - Shuntsu (暗)順子: 3 consecutives (ignoring red); e.g. `789m`, `406s`
///
/// These are like [`crate::common::Chii`] and [`crate::common::Pon`] respectively except concealed.
///
/// It can be encoded as a 6-bit integer (the same size as a [`Tile`]!), with 2 bitfields:
/// - `[3:0]`: `[111, 123, 222, 234, 333, 345, 444, 456, 555, 567, 666, 678, 777, 789, 888, 999]`.
///   Basically with `999` shifting 1 place to occupy the encoding for `89A` (invalid).
///   For suit 3 (honors), `123`, `234`, ..., `789`, `888`, `999` are all invalid.
///
/// - `[5:4]`: suit (0/1/2/3 = m/p/s/z)
///
/// ## Optional `serde` support
///
/// `{type, tile}` where `type` is `"Shuntsu"` or `"Koutsu"`.
///
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "tile"))]
pub enum HandGroup {
    /// Koutsu (暗)刻子: 3 of a kind (ignoring red); e.g. `222z`, `055m`.
    /// The tile argument is the repeated tile.
    Koutsu(Tile),

    /// Shuntsu (暗)順子: 3 consecutives (ignoring red); e.g. `789m`, `406s`.
    /// The tile argument is the minimum (normal) tile in the group.
    Shuntsu(Tile),
}

impl HandGroup {
    /// Parse from the 6-bit integer encoding. Higher 2 bits are ignored.
    pub fn from_packed(packed: u8) -> Option<Self> {
        let num = ((packed & 0b1111) >> 1) + 1;
        let suit = (packed >> 4) & 0b11;
        let tile = Tile::from_num_suit(num, suit)?;
        if (packed & 1) == 1 {
            if num == 8 {
                // What should have encoded [8, 9, 10] is reused to represent [9, 9, 9]
                Some(HandGroup::Koutsu(tile.succ().unwrap()))
            } else if suit < 3 {
                Some(HandGroup::Shuntsu(tile))
            } else {
                // Honors cannot form shuntsu
                None
            }
        } else {
            Some(HandGroup::Koutsu(tile))
        }
    }

    /// Encode as a 6-bit integer.
    pub fn packed(self) -> u8 {
        match self {
            HandGroup::Koutsu(tile) => {
                let n = tile.num() - 1;
                let s = tile.suit();
                (s << 4) | ((n << 1) - ((n == 8) as u8))
            }
            HandGroup::Shuntsu(tile) => {
                let n = tile.num() - 1;
                let s = tile.suit();
                (s << 4) | ((n << 1) + 1)
            }
        }
    }

    /// Returns the min tile in the group.
    pub fn min_tile(self) -> Tile {
        match self {
            HandGroup::Koutsu(tile) => tile,
            HandGroup::Shuntsu(tile) => tile,
        }
    }
}

impl Display for HandGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HandGroup::Koutsu(tile) => {
                let n = tile.normal_num();
                let s = tile.suit_char();
                write!(f, "{}{}{}{}", n, n, n, s)
            },
            HandGroup::Shuntsu(tile) => {
                let n = tile.normal_num();
                let s = tile.suit_char();
                write!(f, "{}{}{}{}", n, n + 1, n + 2, s)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn all_hand_groups_are_correctly_encoded() {
        let t = |enc| Tile::from_encoding(enc).unwrap();
        let k = |x| Some(HandGroup::Koutsu(t(x)));
        let s = |x| Some(HandGroup::Shuntsu(t(x)));
        let all = [
            // 111m, 123m, ..., 888m, 999m
            k(0), s(0), k(1), s(1), k(2), s(2), k(3), s(3),
            k(4), s(4), k(5), s(5), k(6), s(6), k(7), k(8),

            // 111p, 123p, ..., 888p, 999p
            k(9), s(9), k(10), s(10), k(11), s(11), k(12), s(12),
            k(13), s(13), k(14), s(14), k(15), s(15), k(16), k(17),

            // 111s, 123s, ..., 888s, 999s
            k(18), s(18), k(19), s(19), k(20), s(20), k(21), s(21),
            k(22), s(22), k(23), s(23), k(24), s(24), k(25), k(26),

            // 111z, x, 222z, x, ...777z, x, x, x
            k(27), None, k(28), None, k(29), None, k(30), None,
            k(31), None, k(32), None, k(33), None, None, None,
        ];
        for (i, ans) in all.into_iter().enumerate() {
            let i = i as u8;
            let unpacked = HandGroup::from_packed(i as u8);
            assert_eq!(unpacked, ans);
            if let Some(g) = unpacked {
                assert_eq!(g.packed(), i);
            }
        }
    }
}
