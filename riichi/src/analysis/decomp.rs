use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

use itertools::Itertools;
use once_cell::sync::OnceCell;

use crate::common::*;
use riichi_decomp_table::{
    c_entry_iter, make_c_table, make_w_table, w_entry_iter, CTable, CompleteGrouping, WTable,
    WaitingKind, WaitingPattern,
};

/// TODO(summivox): doc
#[derive(Copy, Clone, Debug)]
pub struct RegularWait {
    // TODO(summivox): reduce these into a "small vec" type --- decomp table could use these too
    raw_groups: u32, // (suited) groups: u8 x 4, with current len;
    pub num_groups: u8,

    pub pair: Option<Tile>, // only the complete pair (not Tanki)

    pub waiting_kind: WaitingKind,
    pub pattern_tile: Tile,
    pub waiting_tile: Tile,
}

impl RegularWait {
    pub fn new(groups: &[HandGroup], pair: Option<Tile>,
               waiting_kind: WaitingKind, pattern_tile: Tile, waiting_tile: Tile) -> Self {
        let raw_groups = groups.iter().enumerate()
            .map(|(i, g)| (g.packed() as u32) << (i as u32 * 8)).sum();
        Self {
            raw_groups,
            num_groups: groups.len() as u8,
            pair,
            waiting_kind,
            pattern_tile,
            waiting_tile,
        }
    }
    /// TODO(summivox): doc
    pub fn groups(&self) -> impl Iterator<Item = HandGroup> {
        self.raw_groups
            .to_le_bytes()
            .into_iter()
            .take(self.num_groups as usize)
            .map(|x| HandGroup::from_packed(x).unwrap())
    }

    pub fn has_pair_or_tanki(&self) -> bool {
        self.pair.is_some() || self.waiting_kind == WaitingKind::Tanki
    }

    pub fn pair_or_tanki(&self) -> Option<Tile> {
        self.pair.or_else(||
            (self.waiting_kind == WaitingKind::Tanki).then_some(self.waiting_tile))
    }

    pub fn is_true_ryanmen(&self) -> bool {
        matches!((self.waiting_kind, self.pattern_tile.normal_num()),
            (WaitingKind::RyanmenLow, 2..=7) |  // (1)23 ..= (6)78 ; excluding (7)89
            (WaitingKind::RyanmenHigh, 2..=7) |  // 23(4) ..= 78(9) ; excluding 12(3)
            (WaitingKind::RyanmenBoth, _)
        )
    }
}

impl PartialEq<Self> for RegularWait {
    fn eq(&self, other: &Self) -> bool {
        self.groups().sorted().collect_vec() == other.groups().sorted().collect_vec()
            && self.pair == other.pair
            && self.waiting_kind == other.waiting_kind
            && self.pattern_tile == other.pattern_tile
            && self.waiting_tile == other.waiting_tile
    }
}

impl Eq for RegularWait {}

impl PartialOrd<Self> for RegularWait {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RegularWait {
    fn cmp(&self, other: &Self) -> Ordering {
        let o =
            self.groups().sorted().collect_vec().cmp(&other.groups().sorted().collect_vec());
        if o != Ordering::Equal { return o; }
        let o = self.pair.cmp(&other.pair);
        if o != Ordering::Equal { return o; }
        let o = self.waiting_kind.cmp(&other.waiting_kind);
        if o != Ordering::Equal { return o; }
        let o = self.pattern_tile.cmp(&other.pattern_tile);
        if o != Ordering::Equal { return o; }
        self.waiting_tile.cmp(&other.waiting_tile)
    }
}

impl Display for RegularWait {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use WaitingKind::*;
        write!(f, "{}", self.groups().sorted().map(|g| g.to_string()).join(" "))?;
        if let Some(pair) = self.pair {
            write!(f, " {}{}", pair.num(), pair)?;
        }
        let p = self.pattern_tile;
        let t = self.waiting_tile;
        match self.waiting_kind {
            Tanki =>
                write!(f, " {}+{}", p.num(), t),
            Shanpon =>
                write!(f, " {}{}+{}", p.num(), p.num(), t),
            Kanchan =>
                write!(f, " {}{}+{}", p.num(), p.succ2().unwrap().num(), t),
            RyanmenHigh | RyanmenLow | RyanmenBoth =>
                write!(f, " {}{}+{}", p.num(), p.succ().unwrap().num(), t),
        }
    }
}

pub struct Decomposer<'a> {
    c_table: &'a CTable,
    w_table: &'a WTable,
    keys: [u32; 4],
    c_for_suit: [Vec<CompleteGrouping>; 4],
}

impl Decomposer<'_> {
    /// TODO(summivox): doc
    pub fn new() -> Decomposer<'static> {
        Decomposer {
            c_table: get_c_table(),
            w_table: get_w_table(),
            keys: Default::default(),
            c_for_suit: Default::default(),
        }
    }

    /// TODO(summivox): doc
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

    /// TODO(summivox): doc
    pub fn with_tile_set(&mut self, tile_set: TileSet34) -> &Self {
        self.with_keys(tile_set.packed())
    }

    /// TODO(summivox): doc
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

impl RegularWait {
    fn from_waiting_pattern(suit: u8, w: WaitingPattern) -> Option<Self> {
        if suit == 3 && w.waiting_kind.is_shuntsu() { return None }
        Some(Self {
            raw_groups: 0,
            num_groups: 0,

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
            num_groups: self.num_groups + c.num_groups,
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

fn extend_groups(groups: u32, suit: u8, c: &CompleteGrouping) -> u32 {
    c.groups().fold(groups, |gs, g| (gs << 8) | ((g | (suit << 4)) as u32))
}

fn extend_pair(pair: Option<Tile>, suit: u8, c: &CompleteGrouping) -> Option<Tile> {
    pair.or_else(|| c.pair().and_then(|m| Tile::from_encoding(m + suit * 9)))
}

fn get_c_table() -> &'static CTable {
    static C_TABLE: OnceCell<CTable> = OnceCell::new();
    C_TABLE.get_or_init(make_c_table)
}

fn get_w_table() -> &'static WTable {
    static W_TABLE: OnceCell<WTable> = OnceCell::new();
    W_TABLE.get_or_init(|| make_w_table(get_c_table()))
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

    use std::str::FromStr;

    use pretty_assertions::assert_eq;

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
    fn debug_1() {
        print_decomp([2, 0, 0, 0o0110]);
    }

    #[test]
    fn check_decomp_examples() {
        // shorthands for building decomp "literals"
        use HandGroup::{Koutsu, Shuntsu};
        use WaitingKind::*;
        use RegularWait as W;
        let t = |str| Tile::from_str(str).unwrap();
        let k = |str| Koutsu(t(str));
        let s = |str| Shuntsu(t(str));

        // no ten
        check_decomp([3, 2, 1, 0], &[]);
        check_decomp([0, 0, 0, 0o0122000], &[]);
        check_decomp([0, 0, 0, 0o0011000], &[]);

        // 2 x 2 independent
        check_decomp([0o000000333, 0, 0, 0o2200000], &[
            W::new(&[k("1m"), k("2m"), k("3m")], Some(t("6z")),
                   Shanpon, t("7z"), t("7z")),
            W::new(&[k("1m"), k("2m"), k("3m")], Some(t("7z")),
                   Shanpon, t("6z"), t("6z")),
            W::new(&[s("1m"), s("1m"), s("1m")], Some(t("6z")),
                   Shanpon, t("7z"), t("7z")),
            W::new(&[s("1m"), s("1m"), s("1m")], Some(t("7z")),
                   Shanpon, t("6z"), t("6z")),
        ]);

        // 2x2 Ryanmen + Shanpon
        check_decomp([0, 0o011300000, 0, 0o0002000], &[
            W::new(&[k("6p")], Some(t("4z")),
                   RyanmenBoth, t("7p"), t("6p")),
            W::new(&[k("6p")], Some(t("4z")),
                   RyanmenBoth, t("7p"), t("9p")),
            W::new(&[s("6p")], Some(t("6p")),
                   Shanpon, t("4z"), t("4z")),
            W::new(&[s("6p")], Some(t("4z")),
                   Shanpon, t("6p"), t("6p")),
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
