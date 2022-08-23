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
pub struct FullHandWaitingPattern {
    raw_groups: u32, // (suited) groups: u8 x 4, with current len;
    pub num_groups: u8,

    has_pair_or_tanki: bool, // complete or waiting, have we allocated a pair
    pub pair: Option<Tile>, // only the complete pair (not Tanki)

    pattern_tile: Tile,
    pub waiting_tile: Tile,
    pub waiting_kind: WaitingKind,
}

impl FullHandWaitingPattern {
    /// TODO(summivox): doc
    pub fn groups(&self) -> impl Iterator<Item = HandGroup> {
        self.raw_groups
            .to_le_bytes()
            .into_iter()
            .take(self.num_groups as usize)
            .map(|x| HandGroup::from_packed(x).unwrap())
    }
}

impl PartialEq<Self> for FullHandWaitingPattern {
    fn eq(&self, other: &Self) -> bool {
        self.groups().sorted().collect_vec() == other.groups().sorted().collect_vec()
            && self.has_pair_or_tanki == other.has_pair_or_tanki
            && self.pair == other.pair
            && self.pattern_tile == other.pattern_tile
            && self.waiting_tile == other.waiting_tile
            && self.waiting_kind == other.waiting_kind
    }
}

impl Eq for FullHandWaitingPattern {}

impl Display for FullHandWaitingPattern {
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
    pub fn keys(&mut self, keys: [u32; 4]) -> &Self {
        self.keys = keys;
        self.c_for_suit = [
            c_iter(self.c_table, keys[0]).collect_vec(),
            c_iter(self.c_table, keys[1]).collect_vec(),
            c_iter(self.c_table, keys[2]).collect_vec(),
            c_iter_z(self.c_table, keys[3]).collect_vec(),
        ];
        return self;
    }
    /// TODO(summivox): doc
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = FullHandWaitingPattern> + 'a {
        let suit_x =
            self.c_for_suit
                .iter()
                .map(|v| v.len())
                .enumerate()
                .fold(4, |suit_x, (suit, len)| {
                    if len != 0 { suit_x }
                    else if suit_x == 4 { suit as u8 }
                    else { 5 }
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

            chain.flat_map(move |partial| FullHandWaitingPattern::complete(partial))
        })
    }
}

impl FullHandWaitingPattern {
    fn new(suit: u8, w: WaitingPattern) -> Self {
        Self {
            raw_groups: 0,
            num_groups: 0,

            has_pair_or_tanki: w.waiting_kind == WaitingKind::Tanki,
            pair: None,

            pattern_tile: Tile::from_encoding(w.pattern_pos + 9 * suit).unwrap(),
            waiting_tile: Tile::MIN,  // dummy

            waiting_kind: w.waiting_kind,
        }
    }

    fn try_extend(&self, suit: u8, c: &CompleteGrouping) -> Option<Self> {
        if (self.has_pair_or_tanki && c.pair().is_some()) || (suit == 3 && c.has_shuntsu()) {
            return None;
        }
        Some(FullHandWaitingPattern {
            raw_groups: extend_groups(self.raw_groups, suit, &c),
            num_groups: self.num_groups + c.num_groups,
            has_pair_or_tanki: self.has_pair_or_tanki || c.pair().is_some(),
            pair: extend_pair(self.pair, suit, &c),
            ..*self
        })
    }

    fn complete(self) -> impl Iterator<Item = Self> {
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
) -> impl Iterator<Item =FullHandWaitingPattern> + 'a {
    w_iter(w_table, key).flat_map(move |w| {
        c_iter(c_table, w.complete_key)
            .filter_map(move |c|
                FullHandWaitingPattern::new(suit, w).try_extend(suit, &c))
    })
}

/// Includes all decomps of another (complete) suit into each of the current partial decompositions.
///
/// In iterator comprehension notation:
/// ```python
///   (extended
///    for partial_decomp in partial_iter
///    for complete_grouping in c
///    if extended := partial_decomp.extend(complete_grouping))
/// ```
fn extend_partial_iter<'a>(
    partial_iter: impl Iterator<Item =FullHandWaitingPattern> + 'a,
    suit: u8,
    c: &'a [CompleteGrouping],
) -> impl Iterator<Item =FullHandWaitingPattern> + 'a {
    partial_iter.flat_map(move |partial|
        c.iter().filter_map(move |c|
            partial.try_extend(suit, c)))
}

fn extend_groups(groups: u32, suit: u8, c: &CompleteGrouping) -> u32 {
    c.groups().fold(groups, |gs, g| (gs << 8) | ((g | (suit << 4)) as u32))
}

fn extend_pair(pair: Option<Tile>, suit: u8, c: &CompleteGrouping) -> Option<Tile> {
    pair.or(c.pair().and_then(|m| Tile::from_encoding(m + suit * 9)))
}

fn get_c_table() -> &'static CTable {
    static C_TABLE: OnceCell<CTable> = OnceCell::new();
    C_TABLE.get_or_init(make_c_table)
}

fn get_w_table() -> &'static WTable {
    static W_TABLE: OnceCell<WTable> = OnceCell::new();
    W_TABLE.get_or_init(|| make_w_table(get_c_table()))
}

fn c_iter<'a>(c_table: &'a CTable, key: u32) -> impl Iterator<Item = CompleteGrouping> + 'a {
    c_table
        .get(&key)
        .into_iter()
        .flat_map(move |&value| c_entry_iter(key, value))
}

fn w_iter<'a>(w_table: &'a WTable, key: u32) -> impl Iterator<Item = WaitingPattern> + 'a {
    w_table
        .get(&key)
        .into_iter()
        .flat_map(move |&value| w_entry_iter(key, value))
}

fn c_iter_z<'a>(c_table: &'a CTable, key: u32) -> impl Iterator<Item = CompleteGrouping> + 'a {
    c_iter(c_table, key).filter(|x| !x.has_shuntsu())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print_decomp(keys: [u32; 4]) {
        println!(
            "[0o{:09o}, 0o{:09o}, 0o{:09o}, 0o{:07o}]",
            keys[0], keys[1], keys[2], keys[3]
        );
        for x in Decomposer::new().keys(keys).iter() {
            println!("{}", x);
        }
        println!();
    }

    #[test]
    fn debug_print_some_decomp() {
        print_decomp([0o000000333, 0, 0, 0o2200000]);
        print_decomp([0o111222202, 0, 0, 0]);
        print_decomp([0, 0, 0, 0o0122000]);
        print_decomp([0, 0, 0o000122000, 0]);
        print_decomp([0, 0o011300000, 0, 0o0002000]);
    }
}
