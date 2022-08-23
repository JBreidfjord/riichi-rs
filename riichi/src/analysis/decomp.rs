use itertools::Itertools;
use once_cell::sync::OnceCell;

use crate::common::*;
use riichi_decomp_table::{
    c_entry_iter, make_c_table, make_w_table, w_entry_iter, CTable, CompleteGrouping, WTable,
    WaitingKind, WaitingPattern,
};

#[derive(Copy, Clone, Debug)]
pub struct FullHandWaitingPattern {
    raw_groups: u32,
    pub num_groups: u8,
    pub pair: Option<Tile>,
    pub waiting_kind: WaitingKind,
    pub waiting_tile: Tile,
}

impl FullHandWaitingPattern {
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
            && self.pair == other.pair
            && self.waiting_kind == other.waiting_kind
            && self.waiting_tile == other.waiting_tile
    }
}

impl Eq for FullHandWaitingPattern {}

pub struct Decomposer<'a> {
    c_table: &'a CTable,
    w_table: &'a WTable,
    keys: [u32; 4],
    c_for_suit: [Vec<CompleteGrouping>; 4],
}

impl Decomposer<'_> {
    pub fn new() -> Decomposer<'static> {
        Decomposer {
            c_table: get_c_table(),
            w_table: get_w_table(),
            keys: Default::default(),
            c_for_suit: Default::default(),
        }
    }
    pub fn keys(&mut self, keys: [u32; 4]) -> &Self {
        self.keys = keys;
        self.c_for_suit = dbg!([
            c_iter(self.c_table, keys[0]).collect_vec(),
            c_iter(self.c_table, keys[1]).collect_vec(),
            c_iter(self.c_table, keys[2]).collect_vec(),
            c_iter_z(self.c_table, keys[3]).collect_vec(),
        ]);
        return self;
    }
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = FullHandWaitingPattern> + 'a {
        let suit_x =
            self.c_for_suit
                .iter()
                .map(|v| v.len())
                .enumerate()
                .fold(4, |suit_x, (suit, len)| {
                    if len != 0 {
                        suit_x
                    } else if suit_x == 4 {
                        suit as u8
                    } else {
                        5
                    }
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

            chain.flat_map(move |partial| PartialDecomp::to_full_hand(partial, suit_w))
        })
    }
}

#[derive(Debug)]
struct PartialDecomp {
    // carried from the outermost layer to the innermost layer;
    pub waiting_kind: WaitingKind,
    pub pattern_pos: u8,

    // (suited) groups: u8 x 4, with current len;
    groups: u32,
    num_groups: u8,

    // complete or waiting, have we allocated a pair
    has_pair_or_tanki: bool,

    // only the complete pair
    pair: Option<Tile>,
}

impl PartialDecomp {
    fn new(w: WaitingPattern) -> Self {
        Self {
            waiting_kind: w.waiting_kind,
            pattern_pos: w.pattern_pos,
            groups: 0,
            num_groups: 0,
            has_pair_or_tanki: w.waiting_kind == WaitingKind::Tanki,
            pair: None,
        }
    }

    fn try_extend(&self, suit: u8, c: &CompleteGrouping) -> Option<PartialDecomp> {
        if (self.has_pair_or_tanki && c.pair().is_some()) || (suit == 3 && c.has_shuntsu()) {
            return None;
        }
        Some(PartialDecomp {
            groups: extend_groups(self.groups, suit, &c),
            num_groups: self.num_groups + c.num_groups,
            has_pair_or_tanki: self.has_pair_or_tanki || c.pair().is_some(),
            pair: extend_pair(self.pair, suit, &c),
            ..*self
        })
    }

    fn to_full_hand(self, suit: u8) -> impl Iterator<Item = FullHandWaitingPattern> {
        [self.waiting_pos_low(), self.waiting_pos_high()]
            .into_iter()
            .filter_map(move |pos| pos.and_then(|pos| Tile::from_encoding(pos + suit * 9)))
            .map(move |waiting_tile| FullHandWaitingPattern {
                raw_groups: self.groups,
                num_groups: self.num_groups,
                pair: self.pair,
                waiting_kind: self.w.waiting_kind,
                waiting_tile,
            })
    }

    fn waiting_pos_low(&self) -> Option<u8> {
        use WaitingKind::*;
        match self.waiting_kind {
            Tanki | Shanpon => Some(self.pattern_pos),
            Kanchan => Some(self.pattern_pos + 1),
            RyanmenHigh => None,
            RyanmenLow | RyanmenBoth => Some(self.pattern_pos - 1), // guaranteed to not wrap
        }
    }

    fn waiting_pos_high(&self) -> Option<u8> {
        use WaitingKind::*;
        match self.waiting_kind {
            RyanmenHigh | RyanmenBoth => Some(self.pattern_pos + 2),
            _ => None,
        }
    }
}

/// Starting from all waiting patterns for the waiting suit, make it
fn new_partial_iter<'a>(
    c_table: &'a CTable,
    w_table: &'a WTable,
    suit: u8,
    key: u32,
) -> impl Iterator<Item = PartialDecomp> + 'a {
    w_iter(w_table, key).flat_map(move |w| {
        c_iter(c_table, w.complete_key)
            .filter_map(move |c| PartialDecomp::new(w).try_extend(suit, &c))
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
    partial_iter: impl Iterator<Item = PartialDecomp> + 'a,
    suit: u8,
    c: &'a [CompleteGrouping],
) -> impl Iterator<Item = PartialDecomp> + 'a {
    partial_iter.flat_map(move |partial| c.iter().filter_map(move |c| partial.try_extend(suit, c)))
}

fn extend_groups(full_groups: u32, suit: u8, c: &CompleteGrouping) -> u32 {
    c.groups()
        .fold(full_groups, |gs, g| (gs << 8) | ((g | (suit << 4)) as u32))
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
            "=== {:09o} {:09o} {:09o} {:07o} ===",
            keys[0], keys[1], keys[2], keys[3]
        );
        for x in Decomposer::new().keys(keys).iter() {
            println!("{:?}  -- groups: {:?}", x, x.groups().collect_vec());
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
