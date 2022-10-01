//! [`Tile`] 牌
//!
//! ## Ref
//! - <https://ja.wikipedia.org/wiki/%E9%BA%BB%E9%9B%80%E7%89%8C>
//! - <https://en.wikipedia.org/wiki/Mahjong_tiles>
//! - <https://riichi.wiki/Mahjong_equipment>

use core::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    str::FromStr
};

use crate::typedefs::*;

/// Represents one tile (牌).
///
/// Encoded as a 6-bit integer:
///
/// | Encoding   |  Shorthand  | Category (EN) | Category (JP) |
/// |------------|-------------|---------------|---------------|
/// | 0  ..= 8   |  1m ..= 9m  | characters    | 萬子          |
/// | 9  ..= 17  |  1p ..= 9p  | dots          | 筒子          |
/// | 18 ..= 26  |  1s ..= 9s  | bamboos       | 索子          |
/// | 27 ..= 30  |  1z ..= 4z  | winds         | 風牌          |
/// | 31, 32, 33 |  5z, 6z, 7z | dragons       | 三元牌        |
/// | 34, 35, 36 |  0m, 0p, 0s | reds          | 赤牌          |
///
/// Note that only red 5's can be represented (not other numbers or honors).
///
/// Details of this encoding is significant and implicitly assumed across the crate.
/// It should never be changed.
///
///
/// ## Optional `serde` support
///
/// The common string shorthand (e.g. `"1m"`, `"0p"`, `"7z"`) is used as the serialization format.
/// This ensures readability and interoperability.
///
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(all(feature = "serde", feature = "std"), serde(try_from = "String", into = "&str"))]
pub struct Tile(u8);

impl Tile {
    pub const MIN_ENCODING: u8 = 0;
    pub const MAX_ENCODING: u8 = 36;
    pub const MIN: Self = Self(Self::MIN_ENCODING);
    pub const MAX: Self = Self(Self::MAX_ENCODING);

    pub const fn from_encoding(encoding: u8) -> Option<Self> {
        if encoding <= Self::MAX_ENCODING { Some(Self(encoding)) } else { None }
    }

    pub const fn from_num_suit(num: u8, suit: u8) -> Option<Self> {
        if !(num <= 9 && suit <= 3) { return None; }
        if suit == 3 && !(1 <= num && num <= 7) { return None; }
        if num == 0 {
            Some(Self(34 + suit))
        } else {
            Some(Self(suit * 9 + num - 1))
        }
    }

    pub fn from_wind(wind: Wind) -> Self { Self(27 + wind.to_u8()) }

    pub const fn is_valid(self) -> bool { self.0 <= 36 }

    /// Not red 5
    pub const fn is_normal(self) -> bool { self.0 <= 33 }
    /// Red 5 赤牌
    pub const fn is_red(self) -> bool { 34 <= self.0 && self.0 <= 36 }

    pub const fn has_red(self) -> bool {
        self.0 == 4 || self.0 == 13 || self.0 == 22 || self.is_red()
    }

    /// Numerals := Characters + Dots + Bamboos ;
    /// 数牌 := 萬子 + 筒子 + 索子
    pub const fn is_numeral(self) -> bool {
        (self.0 <= 26) || (34 <= self.0 && self.0 <= 36)
    }
    /// Pure terminals := {1,9}{m,p,s} 老頭牌
    pub const fn is_pure_terminal(self) -> bool {
        matches!(self.0, 0 | 8 | 9 | 17 | 18 | 26)
    }
    /// Middle numerals := {2..=8}{m,p,s} ;
    /// 中張牌 := 数牌 - 老頭牌
    pub const fn is_middle(self) -> bool { self.is_numeral() && !self.is_pure_terminal() }

    /// Winds 風牌 := {1,2,3,4}z (correspond to {E,S,W,N})
    pub const fn is_wind(self) -> bool { 27 <= self.0 && self.0 <= 30 }
    /// Dragons 三元牌 := {5,6,7}z (correspond to {blue, green, red} dragons).
    pub const fn is_dragon(self) -> bool { 31 <= self.0 && self.0 <= 33 }
    /// Honors := Winds + Dragons ;
    /// 字牌 := 風牌 + 三元牌
    pub const fn is_honor(self) -> bool { 27 <= self.0 && self.0 <= 33 }

    /// Terminals := Pure terminals + Honors ;
    /// 么九牌 := 老頭牌 + 字牌
    pub const fn is_terminal(self) -> bool {
        self.is_pure_terminal() || self.is_honor()
    }

    pub const fn encoding(self) -> u8 {
        debug_assert!(self.is_valid());
        self.0
    }
    /// Encoding of this tile, except red 5 is converted to normal 5
    pub const fn normal_encoding(self) -> u8 {
        debug_assert!(self.is_valid());
        match self.0 {
            34 => 4,
            35 => 13,
            36 => 22,
            x => x,
        }
    }
    /// Encoding of this tile, except normal 5 is converted to red 5
    pub const fn red_encoding(self) -> u8 {
        debug_assert!(self.is_valid());
        match self.0 {
            4 => 34,
            13 => 35,
            22 => 36,
            x => x,
        }
    }

    /// Converts a red 5 to normal 5; otherwise no-op.
    pub const fn to_normal(self) -> Self { Self(self.normal_encoding()) }

    /// Converts normal 5 to red 5; otherwise no-op.
    pub const fn to_red(self) -> Self { Self(self.red_encoding()) }

    /// Converts to the corresponding wind (ESWN) if this is a wind tile.
    pub const fn wind(self) -> Option<Wind> {
        // Not using `Option::then` because it cannot be used in `const fn` (yet).
        if self.is_wind() { Some(Wind::new(self.0 - 27)) } else { None }
    }

    /// Converts tile to an internal ordering key where:
    /// 1m < ... < 4m < 0m < 5m < ... < 9m < 1p < ... < 9p < 1s < ... < 9s < 1z < ... < 7z
    ///
    /// This is implemented by doubling the encoding space and inserting the reds
    /// between 4 and 5 tiles.
    const fn to_ordering_key(self) -> u8 {
        debug_assert!(self.is_valid());
        if self.0 <= 33 { self.0 * 2 } else { 7 + (self.0 - 34) * 18 }
    }

    /// Returns the "number" part of the shorthand
    pub const fn num(self) -> u8 {
        debug_assert!(self.is_valid());
        if self.0 <= 33 { self.0 % 9 + 1 } else { 0 }
    }
    /// Returns the "number" part of the shorthand, with reds converted to non-red (i.e. 0 => 5).
    pub const fn normal_num(self) -> u8 {
        debug_assert!(self.is_valid());
        if self.0 <= 33 { self.0 % 9 + 1 } else { 5 }
    }
    /// Returns the "suit" part of the shorthand (0, 1, 2, 3 for m, p, s, z respectively)
    pub const fn suit(self) -> u8 {
        debug_assert!(self.is_valid());
        if self.0 <= 33 { self.0 / 9 } else { self.0 - 34 }
    }

    /// For numerals 1 to 8, returns 2 to 9 respectively. Otherwise None.
    pub const fn succ(self) -> Option<Self> {
        if self.is_numeral() && self.normal_num() <= 8 {
            Some(Self(self.normal_encoding() + 1))
        } else { None }
    }
    /// For numerals 1 to 7, returns 3 to 9 respectively. Otherwise None.
    pub const fn succ2(self) -> Option<Self> {
        if self.is_numeral() && self.normal_num() <= 7 {
            Some(Self(self.normal_encoding() + 2))
        } else { None }
    }
    /// For numerals 2 to 9, returns 1 to 8 respectively. Otherwise None.
    pub const fn pred(self) -> Option<Self> {
        if self.is_numeral() && self.normal_num() >= 2 {
            Some(Self(self.normal_encoding() - 1))
        } else { None }
    }
    /// For numerals 3 to 9, returns 1 to 7 respectively. Otherwise None.
    pub const fn pred2(self) -> Option<Self> {
        if self.is_numeral() && self.normal_num() >= 3 {
            Some(Self(self.normal_encoding() - 2))
        } else { None }
    }

    /// Given this tile as the dora-indicator (ドラ表示牌), returns the indicated dora tile (ドラ).
    ///
    /// Ref:
    /// - <https://ja.wikipedia.org/wiki/%E3%83%89%E3%83%A9_(%E9%BA%BB%E9%9B%80)>
    pub const fn indicated_dora(self) -> Self {
        debug_assert!(self.is_valid());
        Self([
            1, 2, 3, 4, 5, 6, 7, 8, 0, // m
            10, 11, 12, 13, 14, 15, 16, 17, 9, // p
            19, 20, 21, 22, 23, 24, 25, 26, 18, // s
            28, 29, 30, 27, // winds
            32, 33, 31, // dragons
            5, 14, 23u8, // reds indicate 6
        ][self.0 as usize])
    }
}

impl PartialOrd<Self> for Tile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_ordering_key().cmp(&other.to_ordering_key())
    }
}

// String/Char Conversions

/// Returns the tile suit represented by the shorthand suit char.
pub(crate) const fn suit_from_char(c: char) -> Option<u8> {
    match c {
        'm' => Some(0),
        'p' => Some(1),
        's' => Some(2),
        'z' => Some(3),
        _ => None
    }
}

/// Returns the shorthand char for tile suit.
pub(crate) const fn char_from_suit(suit: u8) -> Option<char> {
    match suit {
        0 => Some('m'),
        1 => Some('p'),
        2 => Some('s'),
        3 => Some('z'),
        _ => None
    }
}

// Concrete impls for conversion to/from strings.

impl Tile {
    /// Returns the "suit" part of the shorthand (0, 1, 2, 3 for m, p, s, z respectively)
    pub fn suit_char(self) -> char {
        debug_assert!(self.is_valid());
        char_from_suit(self.suit()).unwrap()
    }

    /// Returns the standard shorthand string of this tile.
    pub const fn as_str(self) -> &'static str {
        debug_assert!(self.is_valid());
        [
            "1m", "2m", "3m", "4m", "5m", "6m", "7m", "8m", "9m", //
            "1p", "2p", "3p", "4p", "5p", "6p", "7p", "8p", "9p", //
            "1s", "2s", "3s", "4s", "5s", "6s", "7s", "8s", "9s", //
            "1z", "2z", "3z", "4z", "5z", "6z", "7z", //
            "0m", "0p", "0s", //
        ][self.encoding() as usize]
    }

    /// Returns the corresponding codepoint in the Unicode Mahjong Tiles section (1F000 ~ 1F02F)
    ///
    /// NOTE: The ordering in Unicode is differerent from Japanese Riichi Mahjong conventions.
    pub const fn unicode(self) -> char {
        debug_assert!(self.is_valid());
        [
            '\u{1f007}', '\u{1f008}', '\u{1f009}', '\u{1f00a}', '\u{1f00b}', '\u{1f00c}', '\u{1f00d}', '\u{1f00e}', '\u{1f00f}',  // 1-9m
            '\u{1f019}', '\u{1f01a}', '\u{1f01b}', '\u{1f01c}', '\u{1f01d}', '\u{1f01e}', '\u{1f01f}', '\u{1f020}', '\u{1f021}',  // 1-9p
            '\u{1f010}', '\u{1f011}', '\u{1f012}', '\u{1f013}', '\u{1f014}', '\u{1f015}', '\u{1f016}', '\u{1f017}', '\u{1f018}',  // 1-9s
            '\u{1f000}', '\u{1f001}', '\u{1f002}', '\u{1f003}',  // 1-4z
            '\u{1f006}', '\u{1f005}', '\u{1f004}',  // 5-7z (this is the correct order!)
            '\u{1f00b}', '\u{1f01d}', '\u{1f014}',  // 0mps (char has no color, same as 5mps)
        ][self.encoding() as usize]
    }
}

pub const UNICODE_TILE_BACK: char = '\u{1f02B}';

impl FromStr for Tile {
    type Err = UnspecifiedError;
    fn from_str(pai_str: &str) -> Result<Self, Self::Err> {
        if pai_str.len() != 2 { return Err(UnspecifiedError); }
        let mut chars = pai_str.chars();
        if let (Some(num_char), Some(suit_char)) = (chars.next(), chars.next()) {
            let num = num_char.to_digit(10).ok_or(UnspecifiedError)? as u8;
            let suit = suit_from_char(suit_char).ok_or(UnspecifiedError)?;
            Self::from_num_suit(num, suit).ok_or(UnspecifiedError)
        } else { Err(UnspecifiedError) }
    }
}

// Blanket adaptors for various ways of converting to/from strings.

impl TryFrom<&str> for Tile {
    type Error = UnspecifiedError;
    fn try_from(value: &str) -> Result<Self, Self::Error> { value.parse() }
}

#[cfg(feature = "std")]
impl TryFrom<String> for Tile {
    type Error = UnspecifiedError;
    fn try_from(value: String) -> Result<Self, Self::Error> { value.parse() }
}

impl Into<&str> for Tile {
    fn into(self) -> &'static str { self.as_str() }
}

impl Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents `Some(tile)` as [`Tile::unicode()`] and `None` as [`UNICODE_TILE_BACK`]
pub const fn maybe_tile_unicode(tile: Option<Tile>) -> char {
    if let Some(tile) = tile { tile.unicode() } else { UNICODE_TILE_BACK }
}

/// Parse shorthand for a list of tiles.
/// Example:
/// ```
/// use riichi_elements::tile::*;
/// use itertools::assert_equal;
/// assert_equal(tiles_from_str("11123m8p8p777z"), [
///     t!("1m"), t!("1m"), t!("1m"), t!("2m"), t!("3m"),
///     t!("8p"), t!("8p"),
///     t!("7z"), t!("7z"), t!("7z"),
/// ]);
/// ```
pub fn tiles_from_str(s: &str) -> impl Iterator<Item = Tile> + '_ {
    let mut iter = TilesFromStr {
        iter_n: s.chars().peekable(),
        iter_s: s.chars().peekable(),
        suit_c: None,
        suit: None,
    };
    iter.find_next_suit();
    iter
}

// A `no_std` impl means we cannot cheat by buffering the numbers. Instead, we must find the next
// suit char in advance (two-pointer approach).

struct TilesFromStr<'a> {
    iter_n: core::iter::Peekable<core::str::Chars<'a>>,
    iter_s: core::iter::Peekable<core::str::Chars<'a>>,
    suit_c: Option<char>,
    suit: Option<u8>,
}

impl<'a> TilesFromStr<'a> {
    fn find_next_suit(&mut self) {
        while let Some(c) = self.iter_s.next() {
            if let Some(suit) = suit_from_char(c) {
                self.suit_c = Some(c);
                self.suit = Some(suit);
                return;
            }
        }
        self.suit_c = None;
        self.suit = None;
    }
}

impl<'a> Iterator for TilesFromStr<'a> {
    type Item = Tile;
    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_n.peek().copied() == self.suit_c {
            self.iter_n.next();
            self.find_next_suit();
        }
        self.suit.and_then(|suit|
            self.iter_n.next()
                .and_then(|num_char| num_char.to_digit(10))
                .and_then(|num| Tile::from_num_suit(num as u8, suit)))
    }
}

/// Shortcut for creating a tile literal through its string shorthand.
///
/// Example:
/// ```
/// use riichi_elements::tile::*;
/// assert_eq!(t!("3s"), Tile::from_encoding(20).unwrap());
/// ```
#[macro_export]
macro_rules! t {
    ($s:expr) => {{
        use core::str::FromStr;
        $crate::tile::Tile::from_str($s).unwrap()
    }};
}
pub use t;

#[cfg(test)]
mod tests {
    extern crate std;
    use std::{
        vec,
        string::*,
        print,
        println,
    };

    use itertools::Itertools;

    use super::*;

    #[test]
    fn tile_str_is_num_and_suite() {
        for encoding in Tile::MIN_ENCODING..=Tile::MAX_ENCODING {
            let tile = Tile::from_encoding(encoding).unwrap();
            let tile_str = tile.as_str();
            assert_eq!(tile_str.len(), 2);
            assert_eq!(tile_str[0..=0], tile.num().to_string());
            assert_eq!(tile_str[1..=1], tile.suit_char().to_string());
        }
    }

    #[test]
    fn tile_str_roundtrip() {
        for encoding in Tile::MIN_ENCODING..=Tile::MAX_ENCODING {
            let tile = Tile::from_encoding(encoding).unwrap();
            let tile_str = tile.as_str();
            let roundtrip: Tile = tile_str.parse().unwrap();
            assert_eq!(tile, roundtrip);
        }
    }

    #[test]
    fn tiles_from_str_examples() {
        assert_eq!(tiles_from_str("1m2p3s4z").collect_vec(), vec![
            t!("1m"), t!("2p"), t!("3s"), t!("4z"),
        ]);
    }

    #[test]
    fn tile_num_suite_roundtrip() {
        for encoding in Tile::MIN_ENCODING..=Tile::MAX_ENCODING {
            let tile = Tile::from_encoding(encoding).unwrap();
            let roundtrip: Tile = Tile::from_num_suit(tile.num(), tile.suit()).unwrap();
            assert_eq!(tile, roundtrip);
        }
    }

    #[test]
    fn tile_has_total_order() {
        use core::str::FromStr;
        let correct_order = [
            "1m", "2m", "3m", "4m", "0m", "5m", "6m", "7m", "8m", "9m", //
            "1p", "2p", "3p", "4p", "0p", "5p", "6p", "7p", "8p", "9p", //
            "1s", "2s", "3s", "4s", "0s", "5s", "6s", "7s", "8s", "9s", //
            "1z", "2z", "3z", "4z", "5z", "6z", "7z", //
        ];
        for window in correct_order.windows(2) {
            if let [a, b] = window {
                assert!(Tile::from_str(a).unwrap() < Tile::from_str(b).unwrap());
            } else { panic!() }
        }
    }

    #[test]
    fn tile_indicates_correct_dora() {
        // non-red numerals => wrapping successor in the same suit
        for num_indicator in 1..=9 {
            let num_dora = num_indicator % 9 + 1;
            for suit in 0..=2 {
                let indicator = Tile::from_num_suit(num_indicator, suit).unwrap();
                let dora = Tile::from_num_suit(num_dora, suit).unwrap();
                let indicated_dora = indicator.indicated_dora();
                assert_eq!(dora, indicated_dora);
            }
        }
        // red 5 => 6 in the same suit
        {
            let num_indicator = 0;
            let num_dora = 6;
            for suit in 0..=2 {
                let indicator = Tile::from_num_suit(num_indicator, suit).unwrap();
                let dora = Tile::from_num_suit(num_dora, suit).unwrap();
                let indicated_dora = indicator.indicated_dora();
                assert_eq!(dora, indicated_dora);
            }
        }
        // winds => wrapping successor among winds
        for num_indicator in 1..=4 {
            let num_dora = num_indicator % 4 + 1;
            let indicator = Tile::from_num_suit(num_indicator, 3).unwrap();
            let dora = Tile::from_num_suit(num_dora, 3).unwrap();
            let indicated_dora = indicator.indicated_dora();
            assert_eq!(dora, indicated_dora);
        }
        // dragons => wrapping successor among dragons
        for num_indicator in 5..=7 {
            let num_dora = (num_indicator - 4) % 3 + 5;
            let indicator = Tile::from_num_suit(num_indicator, 3).unwrap();
            let dora = Tile::from_num_suit(num_dora, 3).unwrap();
            let indicated_dora = indicator.indicated_dora();
            assert_eq!(dora, indicated_dora);
        }
    }

    #[test]
    fn wind_tile_indicates_correct_wind() {
        assert_eq!(t!("1z").wind(), Some(Wind::new(0)));
        assert_eq!(t!("2z").wind(), Some(Wind::new(1)));
        assert_eq!(t!("3z").wind(), Some(Wind::new(2)));
        assert_eq!(t!("4z").wind(), Some(Wind::new(3)));
        for enc in 0..27 {
            assert_eq!(Tile::from_encoding(enc).unwrap().wind(), None);
        }
        for enc in 31..37 {
            assert_eq!(Tile::from_encoding(enc).unwrap().wind(), None);
        }
    }

    #[test]
    fn print_tile_unicode() {
        for r in [0..9, 9..18, 18..27, 27..34, 34..37] {
            for enc in r {
                print!("{}", Tile(enc).unicode());
            }
            println!();
        }
        println!("{}", UNICODE_TILE_BACK);
    }
}
