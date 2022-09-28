//! Regular waiting hand decomposition (LUT-accelerated).
//!
//! See doc on [`Decomposer`].

use itertools::Itertools;

use crate::common::*;
use super::regular::{RegularWait, RegularWaitGroups};

use riichi_decomp_table::{
    CompleteGrouping, WaitingKind, WaitingPattern,
};

#[cfg(not(feature = "static-lut"))]
use riichi_decomp_table::{CTable, WTable};
#[cfg(feature = "static-lut")]
use riichi_decomp_table::{CTableStatic as CTable, WTableStatic as WTable};

#[cfg(not(feature = "static-lut"))]
mod tables {
    //! On-demand generated lookup tables.

    use once_cell::sync::Lazy;
    use riichi_decomp_table::{CTable, WTable, make_c_table, make_w_table};
    pub static C_TABLE: Lazy<CTable> = Lazy::new(make_c_table);
    pub static W_TABLE: Lazy<WTable> = Lazy::new(|| make_w_table(&C_TABLE));
    pub use riichi_decomp_table::{
        c_entry_iter_alts as c_entry_iter,
        w_entry_iter_alts as w_entry_iter,
    };
}
#[cfg(feature = "static-lut")]
mod tables {
    //! Statically generated lookup tables (using the `phf` crate).
    //! See `build.rs` for how this is generated.
    include!(concat!(env!("OUT_DIR"), "/decomp_tables.rs"));
    pub use riichi_decomp_table::{
        c_entry_iter,
        w_entry_iter,
    };
}
use tables::{c_entry_iter, w_entry_iter};

/// Helper for iterating all regular decompositions (i.e. [`RegularWait`]) of a waiting hand.
///
/// Example:
/// ```
/// use riichi::analysis::*;
/// use riichi::common::*;
/// let mut decomposer = Decomposer::new();
/// let hand = TileSet34::from_iter(tiles_from_str("1112345678999m"));
/// for wait in decomposer.with_tile_set(&hand).iter() {
///     println!("{wait}");
/// }
/// ```
///
/// The reason this is needed: The iterator needs to reference some intermediate cached results,
/// with lifetimes ultimated tied to the lookup tables. An object must be used to concretely
/// represent these lifetimes.
pub struct Decomposer<'a> {
    c_table: &'a CTable,
    w_table: &'a WTable,
    keys: [u32; 4],
    c_for_suit: [Vec<CompleteGrouping>; 4],
}

impl Decomposer<'_> {
    /// Creates a new decomposer. Note that this instance can be reused across multiple hands.
    pub fn new() -> Decomposer<'static> {
        Decomposer {
            c_table: &tables::C_TABLE,
            w_table: &tables::W_TABLE,
            keys: Default::default(),
            c_for_suit: Default::default(),
        }
    }

    /// Loads the decomposer with a hand, represented as an [octal-packed][packed] tile set.
    /// After loading, the decomposer is ready to be iterated.
    ///
    /// [packed]: TileSet34::packed_34
    pub fn with_keys(&mut self, keys: [u32; 4]) -> &Self {
        self.keys = keys;
        self.c_for_suit = [
            c_iter(self.c_table, keys[0]).collect_vec(),
            c_iter(self.c_table, keys[1]).collect_vec(),
            c_iter(self.c_table, keys[2]).collect_vec(),
            c_iter_z(self.c_table, keys[3]).collect_vec(),
        ];
        self
    }

    /// Loads the decomposer with a hand, represented as a plain tile set.
    /// After loading, the decomposer is ready to be iterated.
    pub fn with_tile_set(&mut self, tile_set: &TileSet34) -> &Self {
        self.with_keys(tile_set.packed_34())
    }

    /// Iterates through all regular hand decompositions.
    pub fn iter(&self) -> impl Iterator<Item=RegularWait> + '_ {
        let suit_x =
            self.c_for_suit
                .iter()
                .map(|v| v.len())
                .enumerate()
                .fold(4, |suit_x, (suit, len)| {
                    if len != 0 { suit_x } else if suit_x == 4 { suit as u8 } else { 5 }
                });
        [
            (0, [1, 2, 3]),
            (1, [0, 2, 3]),
            (2, [0, 1, 3]),
            (3, [0, 1, 2]),
        ]
            .into_iter()
            .filter(move |(suit_w, _)| suit_x == 4 || (suit_x < 4 && *suit_w == suit_x))
            .flat_map(move |(suit_w, suits_c)| {
                let chain = new_partial_iter(
                    self.c_table,
                    self.w_table,
                    suit_w,
                    self.keys[suit_w as usize],
                );

                let chain =
                    extend_partial_iter(chain, suits_c[0], &self.c_for_suit[suits_c[0] as usize]);
                let chain =
                    extend_partial_iter(chain, suits_c[1], &self.c_for_suit[suits_c[1] as usize]);
                let chain =
                    extend_partial_iter(chain, suits_c[2], &self.c_for_suit[suits_c[2] as usize]);

                chain.flat_map(RegularWait::complete)
            })
    }
}

/*
// TODO(summivox): rust (impl Trait in type aliases)
impl<'a> IntoIterator for &Decomposer<'a> {
    type Item = RegularWait;
    type IntoIter = impl Iterator<Item=RegularWait> + 'a;

    fn into_iter(self) -> Self::IntoIter { self.iter() }
}
*/

// Note: Below are all implementation details. Conveniently, `RegularWait` can be directly used to
// represent intermediate results of a hand decomposition.

impl RegularWait {
    fn from_waiting_pattern(suit: u8, w: WaitingPattern) -> Option<Self> {
        if suit == 3 && w.waiting_kind.is_shuntsu() { return None }
        Some(Self {
            raw_groups: Default::default(),

            pair: None,

            pattern_tile: Tile::from_encoding(w.pattern_pos + 9 * suit).unwrap(),
            waiting_tile: Tile::MIN,  // dummy
            waiting_kind: w.waiting_kind,
        })
    }

    fn try_extend(&self, suit: u8, c: &CompleteGrouping) -> Option<Self> {
        if (self.has_pair_or_tanki() && c.pair().is_some()) || (suit == 3 && c.has_shuntsu()) {
            return None;
        }
        Some(RegularWait {
            raw_groups: extend_groups(self.raw_groups, suit, c),
            pair: extend_pair(self.pair, suit, c),
            ..*self
        })
    }

    fn complete(self) -> impl Iterator<Item=Self> {
        [self.waiting_tile_low(), self.waiting_tile_high()]
            .into_iter()
            .flatten()
            .map(move |waiting_tile| Self { waiting_tile, ..self })
    }

    fn waiting_tile_low(&self) -> Option<Tile> {
        use WaitingKind::*;
        match self.waiting_kind {
            Tanki | Shanpon => Some(self.pattern_tile),
            Kanchan => self.pattern_tile.succ(),
            RyanmenHigh => None,
            RyanmenLow | RyanmenBoth => self.pattern_tile.pred(),
        }
    }

    fn waiting_tile_high(&self) -> Option<Tile> {
        use WaitingKind::*;
        match self.waiting_kind {
            RyanmenHigh | RyanmenBoth => self.pattern_tile.succ2(),
            _ => None,
        }
    }
}

/// Starting from all waiting patterns for the waiting suit...
fn new_partial_iter<'a>(
    c_table: &'a CTable,
    w_table: &'a WTable,
    suit: u8,
    key: u32,
) -> impl Iterator<Item=RegularWait> + 'a {
    w_iter(w_table, key).flat_map(move |w| {
        c_iter(c_table, w.complete_key)
            .filter_map(move |c|
                RegularWait::from_waiting_pattern(suit, w)?.try_extend(suit, &c))
    })
}

/// Includes all decomps of another (complete) suit into each of the current partial decompositions.
///
/// In iterator comprehension notation:
/// ```python
///   (extended
///    for partial in partial_iter
///    for complete in c
///    if extended := partial.extend(complete))
/// ```
fn extend_partial_iter<'a>(
    partial_iter: impl Iterator<Item=RegularWait> + 'a,
    suit: u8,
    c: &'a [CompleteGrouping],
) -> impl Iterator<Item=RegularWait> + 'a {
    partial_iter.flat_map(move |partial|
        c.iter().filter_map(move |complete|
            partial.try_extend(suit, complete)))
}

fn extend_groups(groups: RegularWaitGroups, suit: u8, c: &CompleteGrouping) -> RegularWaitGroups {
    c.groups.fold(groups, |gs, g| gs.with_back(g + suit * 16))
}

fn extend_pair(pair: Option<Tile>, suit: u8, c: &CompleteGrouping) -> Option<Tile> {
    pair.or_else(|| c.pair().and_then(|m| Tile::from_encoding(m + suit * 9)))
}

fn c_iter(c_table: &CTable, key: u32) -> impl Iterator<Item=CompleteGrouping> + '_ {
    c_table
        .get(&key)
        .into_iter()
        .flat_map(move |&value| c_entry_iter(key, value))
}

fn w_iter(w_table: &WTable, key: u32) -> impl Iterator<Item=WaitingPattern> + '_ {
    w_table
        .get(&key)
        .into_iter()
        .flat_map(move |&value| w_entry_iter(key, value))
}

fn c_iter_z(c_table: &CTable, key: u32) -> impl Iterator<Item=CompleteGrouping> + '_ {
    c_iter(c_table, key).filter(|x| !x.has_shuntsu())
}



#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    use HandGroup::{Koutsu, Shuntsu};
    use WaitingKind::*;
    use RegularWait as W;

    #[allow(unused)]
    fn k(str: &str) -> HandGroup { Koutsu(t!(str)) }
    #[allow(unused)]
    fn s(str: &str) -> HandGroup { Shuntsu(t!(str)) }

    fn print_decomp(keys: [u32; 4]) {
        println!(
            "[0o{:09o}, 0o{:09o}, 0o{:09o}, 0o{:07o}]",
            keys[0], keys[1], keys[2], keys[3]
        );
        for x in Decomposer::new().with_keys(keys).iter() {
            println!("{}", x);
        }
        println!();
    }

    fn check_decomp(keys: [u32; 4], expected_decomp: &[RegularWait]) {
        let result = Decomposer::new().with_keys(keys).iter().sorted().collect_vec();
        let mut expected = expected_decomp.to_vec();
        expected.sort();
        if result != expected {
            let r = result.iter().map(|r| r.to_string()).join("\n");
            let e = expected.iter().map(|r| r.to_string()).join("\n");
            assert_eq!(r, e);
        }
    }

    #[test]
    fn debug_print_some_decomp() {
        print_decomp([0o111222202, 0, 0, 0]);
        print_decomp([0, 0, 0, 0o0122000]);
        print_decomp([0, 0, 0o000122000, 0]);
        print_decomp([0, 0o011300000, 0, 0o0002000]);
        print_decomp([0o000031100, 0o000022220, 0, 0]);
        print_decomp([3, 2, 1, 0]);
    }

    #[test]
    fn check_decomp_examples() {
        // not waiting
        check_decomp([3, 2, 1, 0], &[]);
        check_decomp([0, 0, 0, 0o0122000], &[]);
        check_decomp([0, 0, 0, 0o0011000], &[]);

        // 2 x 2 independent
        check_decomp([0o000000333, 0, 0, 0o2200000], &[
            W::new(&[k("1m"), k("2m"), k("3m")], Some(t!("6z")),
                   Shanpon, t!("7z"), t!("7z")),
            W::new(&[k("1m"), k("2m"), k("3m")], Some(t!("7z")),
                   Shanpon, t!("6z"), t!("6z")),
            W::new(&[s("1m"), s("1m"), s("1m")], Some(t!("6z")),
                   Shanpon, t!("7z"), t!("7z")),
            W::new(&[s("1m"), s("1m"), s("1m")], Some(t!("7z")),
                   Shanpon, t!("6z"), t!("6z")),
        ]);

        // 2x2 Ryanmen + Shanpon
        check_decomp([0, 0o011300000, 0, 0o0002000], &[
            W::new(&[k("6p")], Some(t!("4z")),
                   RyanmenBoth, t!("7p"), t!("6p")),
            W::new(&[k("6p")], Some(t!("4z")),
                   RyanmenBoth, t!("7p"), t!("9p")),
            W::new(&[s("6p")], Some(t!("6p")),
                   Shanpon, t!("4z"), t!("4z")),
            W::new(&[s("6p")], Some(t!("4z")),
                   Shanpon, t!("6p"), t!("6p")),
        ]);

        // random shit that failed due to stupid bit-packing reasons
        check_decomp([0o001130000,0o000000000,0o000011000,0o0000003], &[
            W::new(&[s("5m"), k("1z")], Some(t!("5m")),
                   RyanmenBoth, t!("4s"), t!("3s")),
            W::new(&[s("5m"), k("1z")], Some(t!("5m")),
                   RyanmenBoth, t!("4s"), t!("6s")),
        ]);

        // junsei chuuren poutou 「純正九蓮宝燈」
        // https://riichi.wiki/Chuuren_poutou
        // TODO(summivox): complete transcribing all 15...

        //     111m 999m 234m 567m 8+8m
        //     111m 999m 234m 678m 5+5m
        //     111m 999m 345m 678m 2+2m
        //     111m 234m 567m 99m 89+7m
        //     111m 234m 789m 99m 56+4m
        //     111m 234m 789m 99m 56+7m
        //     111m 456m 789m 99m 23+1m
        //     111m 456m 789m 99m 23+4m
        //     999m 123m 456m 11m 78+6m
        //     999m 123m 456m 11m 78+9m
        //     999m 123m 678m 11m 45+3m
        //     999m 123m 678m 11m 45+6m
        //     999m 345m 678m 11m 12+3m
        //     123m 456m 789m 11m 99+9m
        //     123m 456m 789m 99m 11+1m

        /*
        check_decomp([0o311111113, 0, 0, 0], &[

            W::new(&[s("1m"), s("4m"), s("7m")], Some(t("9m")),
            Shanpon, t("1m"), t("1m")),

            W::new(&[k("1m"), s("4m"), s("7m")], Some(t("9m")),
                   RyanmenBoth, t("2m"), t("1m")),
            W::new(&[k("1m"), s("4m"), s("7m")], Some(t("9m")),
                   RyanmenBoth, t("2m"), t("4m")),

        ]);
         */
    }
}
