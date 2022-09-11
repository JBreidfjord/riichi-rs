use std::fmt::{Display, Formatter};
use std::iter::zip;
use crate::common::*;

/// Represents one of the irregular waiting hand patterns.
///
/// Note that they are mutually exclusive --- one hand can fit at most one of these patterns.
///
/// ## Optional `Serde` support
///
/// `{type, wait?}` (adjacently tagged, in serde terms).
/// Examples: `{"type": "SevenPairs", "wait": "9s"}`, `{"type": "ThirteenOrphansAll"}`
///
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "wait"))]
pub enum IrregularWait {
    /// Seven Pairs (七対子)
    ///
    /// The associated tile is the waiting tile, i.e. the final (7th) incomplete pair.
    ///
    /// <https://riichi.wiki/Chiitoitsu>
    SevenPairs(Tile),

    /// Thirteen Orphans (十三幺), more commonly known as Kokushi-Musou (国士無双).
    ///
    /// The associated tile is the waiting tile, i.e. the "missing"
    /// [terminal tile](Tile::is_terminal).
    ///
    /// <https://riichi.wiki/Kokushi_musou>
    ThirteenOrphans(Tile),

    /// 13-way waiting version of Thirteen Orphans (国士無双１３面待ち).
    ///
    /// Any [terminal tile](Tile::is_terminal) will complete this hand.
    ThirteenOrphansAll,
}

impl IrregularWait {
    pub fn to_waiting_set(self) -> TileMask34 {
        match self {
            IrregularWait::SevenPairs(t) | IrregularWait::ThirteenOrphans(t)
                => TileMask34(1u64 << (t.encoding() as u64)),
            IrregularWait::ThirteenOrphansAll
                => TileMask34(0b1111111_100000001_100000001_100000001),
        }
    }
}

// This is necessary to pretty-print the wrapped tile.
impl Display for IrregularWait {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IrregularWait::SevenPairs(t) => write!(f, "SevenPairs({})", t),
            IrregularWait::ThirteenOrphans(t) => write!(f, "ThirteenOrphans({})", t),
            IrregularWait::ThirteenOrphansAll => write!(f, "ThirteenOrphansAll"),
        }
    }
}

/// Detect which irregular waiting patterns match the supplied hand (octal-[packed]).
///
/// [packed]: TileSet34::packed_34
pub fn detect_irregular_wait(keys: [u32; 4]) -> Option<IrregularWait> {
    if let Some(tile) = detect_seven_pairs(keys) {
        Some(IrregularWait::SevenPairs(tile))
    } else {
        detect_thirteen_orphans(keys)
    }
}

/// Exactly 6 pairs and 1 single
fn detect_seven_pairs(keys: [u32; 4]) -> Option<Tile> {
    let d = keys.map(one_two);
    let num_twos = d[0].3 + d[1].3 + d[2].3 + d[3].3;
    if num_twos != 6 { return None; }
    let num_ones = d[0].1 + d[1].1 + d[2].1 + d[3].1;
    if num_ones != 1 { return None; }
    for i in 0..4 {
        let ones = d[i].0;
        if ones > 0 {
            return Tile::from_encoding(ones.trailing_zeros() as u8 / 3 + (i as u8) * 9);
        }
    }
    panic!()
}

/// - No middle numerals
/// - No trips/quads
/// - If 0 pair and 13 singles: 13-wait version
/// - If 1 pair and 11 singles: 1-wait version; the "hole" is the waiting tile
fn detect_thirteen_orphans(keys: [u32; 4]) -> Option<IrregularWait> {
    const MASK: [u32; 4] = [
        0o700000007,
        0o700000007,
        0o700000007,
        0o7777777,
    ];
    if zip(keys, MASK).any(|(key, mask)| key & !mask > 0) {
        return None;
    }
    let d = keys.map(one_two);
    let num_twos = d[0].3 + d[1].3 + d[2].3 + d[3].3;
    let num_ones = d[0].1 + d[1].1 + d[2].1 + d[3].1;
    match (num_ones, num_twos) {
        (13, 0) => Some(IrregularWait::ThirteenOrphansAll),
        (11, 1) => {
            for i in 0..4 {
                let k = (MASK[i] & 0o111111111) - (d[i].0 | d[i].2);
                if k > 0 {
                    return Some(IrregularWait::ThirteenOrphans(Tile::from_encoding(
                        k.trailing_zeros() as u8 / 3 + (i as u8) * 9
                    ).unwrap()))
                }
            }
            None
        }
        _ => None,
    }
}

/// Bit hack to obtain {bit mask, count} of {isolated tiles, pairs} in one octal-packed suit.
/// If there are any trips / quads, return an absurdly big count for both.
fn one_two(x: u32) -> (u32, u32, u32, u32) {
    // detect trips/quads
    let over = (x + 0o111111111) & 0o444444444;
    if over > 0 { return (0, 20, 0, 20); }
    // now everything is 0/1/2; 2 has one more bit.
    let twos = (x >> 1) & 0o111111111;
    let num_twos = twos.count_ones();
    // 0/1/2 with 2 removed becomes 0/1
    let ones = x - twos * 2;
    let num_ones = ones.count_ones();
    (ones, num_ones, twos, num_twos)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;

    #[test]
    fn just_seven_pairs() {
        assert_eq!(
            detect_seven_pairs([0o202020202, 0o000000002, 0o000000000, 0o0100000]),
            Some(Tile::from_str("6z").unwrap()));
        assert_eq!(
            detect_seven_pairs([0o201020202, 0o000000002, 0o000000000, 0o0200000]),
            Some(Tile::from_str("7m").unwrap()));
        assert_eq!(
            detect_seven_pairs([0o200020202, 0o000000002, 0o000020001, 0o0000000]),
            Some(Tile::from_str("1s").unwrap()));
        assert_eq!(
            detect_seven_pairs([0o202000202, 0o000000002, 0o000000000, 0o0100000]),
            None);
        assert_eq!(
            detect_seven_pairs([0o202040202, 0o000000001, 0o000000000, 0o0000000]),
            None);
        assert_eq!(
            detect_seven_pairs([0o202020202, 0o000000002, 0o000000000, 0o0000110]),
            None);
        assert_eq!(
            detect_seven_pairs([0o202020202, 0o001000000, 0o000001000, 0o0000100]),
            None);
    }

    #[test]
    fn just_thirteen_orphans() {
        assert_eq!(
            detect_thirteen_orphans([0o100000001, 0o100000001, 0o100000001, 0o1111111]),
            Some(IrregularWait::ThirteenOrphansAll));
        assert_eq!(
            detect_thirteen_orphans([0o100000000, 0o100000001, 0o100000001, 0o1111112]),
            Some(IrregularWait::ThirteenOrphans(Tile::from_str("1m").unwrap())));
        assert_eq!(
            detect_thirteen_orphans([0o000000002, 0o100000001, 0o100000001, 0o1111111]),
            Some(IrregularWait::ThirteenOrphans(Tile::from_str("9m").unwrap())));
        assert_eq!(
            detect_thirteen_orphans([0o100000001, 0o100000000, 0o100000001, 0o1211111]),
            Some(IrregularWait::ThirteenOrphans(Tile::from_str("1p").unwrap())));
        assert_eq!(
            detect_thirteen_orphans([0o200000001, 0o000000001, 0o100000001, 0o1111111]),
            Some(IrregularWait::ThirteenOrphans(Tile::from_str("9p").unwrap())));
        assert_eq!(
            detect_thirteen_orphans([0o100000002, 0o100000001, 0o100000000, 0o1111111]),
            Some(IrregularWait::ThirteenOrphans(Tile::from_str("1s").unwrap())));
        assert_eq!(
            detect_thirteen_orphans([0o100000002, 0o100000001, 0o000000001, 0o1111111]),
            Some(IrregularWait::ThirteenOrphans(Tile::from_str("9s").unwrap())));
        assert_eq!(
            detect_thirteen_orphans([0o100000001, 0o100000001, 0o100000001, 0o1102111]),
            Some(IrregularWait::ThirteenOrphans(Tile::from_str("5z").unwrap())));

        assert_eq!(
            detect_thirteen_orphans([0o100000010, 0o100000001, 0o100000001, 0o1102111]),
            None);
        assert_eq!(
            detect_thirteen_orphans([0o100000003, 0o100000001, 0o100000001, 0o1102111]),
            None);
    }
}
