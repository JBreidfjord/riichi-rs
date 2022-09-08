//! Complete hand decomposition

use rustc_hash::FxHashMap as HashMap;

use crate::utils::*;
use details::*;

pub type CTable = HashMap<u32, u64>;
pub type CTableStatic = phf::Map<u32, u64>;

/// Since the CTable is statically determinable, we can check its number of keys to make sure that
/// we have generated the correct table.
pub const C_TABLE_NUM_KEYS: usize = 21743;

pub fn make_c_table() -> CTable {
    let mut table = CTable::with_capacity_and_hasher(
        C_TABLE_NUM_KEYS, Default::default());
    table.insert(0, 0);
    dfs_koutsu(&mut table, 1, 0, 0, 0);
    dfs_shuntsu(&mut table, 1, 0, 0, 0);
    for j in 0..=8 {
        let j_key = 2 << ((j as u32) * 3);
        table.insert(j_key, 0);
        dfs_koutsu(&mut table, 1, 0, j_key, 0);
        dfs_shuntsu(&mut table, 1, 0, j_key, 0);
    }
    table
}

fn dfs_koutsu(table: &mut CTable, n: u8, pos0: u8, key: u32, value: u16) {
    for pos in pos0..=8 {
        let new_key = key + k_key(pos);
        if key_is_overflow(new_key) { continue; }
        let new_value = mid_set(value, n - 1, k_to_ks(pos));

        table.entry(new_key)
            .and_modify(|v| *v = big_push(*v, new_value))
            .or_insert(new_value as u64);

        if n < 4 {
            dfs_koutsu(table, n + 1, pos + 1, new_key, new_value);
            dfs_shuntsu(table, n + 1, 0, new_key, new_value);
        }
    }
}

fn dfs_shuntsu(table: &mut CTable, n: u8, pos0: u8, key: u32, value: u16) {
    for pos in pos0..=6 {
        let new_key = key + s_key(pos);
        if key_is_overflow(new_key) { continue; }
        let new_value = mid_set(value, n - 1, s_to_ks(pos));

        table.entry(new_key)
            .and_modify(|v| *v = big_push(*v, new_value))
            .or_insert(new_value as u64);

        if n < 4 {
            dfs_shuntsu(table, n + 1, pos, new_key, new_value);
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CompleteGrouping {
    pub num_groups: u8,
    raw_groups: u16,
    raw_pair: u8,
}

impl CompleteGrouping {
    pub fn has_shuntsu(self) -> bool {
        mid_has_any_s(self.raw_groups)
    }
    pub fn groups(self) -> impl Iterator<Item = u8> {
        mid_to_smalls(self.raw_groups).into_iter().take(self.num_groups as usize)
    }
    pub fn pair(self) -> Option<u8> {
        if self.raw_pair == u8::MAX { None } else { Some(self.raw_pair) }
    }
}

pub fn c_entry_iter(key: u32, value: u64) -> impl Iterator<Item = CompleteGrouping> {
    CTableEntryIterator::new(key, value)
}

#[derive(Debug)]
struct CTableEntryIterator {
    key: u32,
    value: u64,

    num_groups: u8,
    has_pair: bool,
    once: bool,
}

impl CTableEntryIterator {
    fn new(key: u32, value: u64) -> Self {
        let n = key_sum(key) as u8;
        debug_assert!(n % 3 != 1);
        CTableEntryIterator {
            key,
            value,
            num_groups: n / 3,
            has_pair: n % 3 == 2,
            once: false,
        }
    }
}

impl Iterator for CTableEntryIterator {
    type Item = CompleteGrouping;

    fn next(&mut self) -> Option<Self::Item> {
        if self.once && self.value == 0 { return None; }
        self.once = true;

        let mid = big_top(self.value);
        let raw_pair = if self.has_pair {
            recover_known_pair(self.key, self.num_groups, mid)
        } else {
            u8::MAX
        };
        self.value = big_pop(self.value);

        Some(CompleteGrouping{
            num_groups: self.num_groups,
            raw_groups: mid,
            raw_pair,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = big_len(self.value) as usize;
        (n, Some(n))
    }
}

pub(crate) mod details {
    pub const fn big_push(big: u64, mid: u16) -> u64 { (big << 16) | (mid as u64) }
    pub const fn big_top(big: u64) -> u16 { big as u16 }
    pub const fn big_pop(big: u64) -> u64 { big >> 16 }
    pub const fn big_len(big: u64) -> u32 { 4 - (big.leading_zeros() / 16) }

    pub const fn mid_set(mid: u16, i: u8, small: u8) -> u16 {
        mid | ((small as u16) << (i * 4) as u16)
    }
    pub const fn mid_to_smalls(mid: u16) -> [u8; 4] {
        [
            (mid & 0xF) as u8,
            ((mid >> 4) & 0xF) as u8,
            ((mid >> 8) & 0xF) as u8,
            ((mid >> 12) & 0xF) as u8,
        ]
    }

    /// default = 0 will not trigger
    pub const fn mid_has_any_s(mid: u16) -> bool {
        let mask = !((((mid >> 1) & 0x7777) + 0x1111) >> 3) & 0x1111 & mid;
        mask != 0
    }

    /// 0..=8 => 0..=0xF
    pub const fn k_to_ks(i: u8) -> u8 { if i == 8 { 0xF } else { i * 2 } }
    /// 0..=6 => 0..=0xE
    pub const fn s_to_ks(i: u8) -> u8 { i * 2 + 1 }

    // value space back to key space
    pub const fn k_key(i: u8) -> u32 { 3 << ((i as u32) * 3) }
    pub const fn s_key(i: u8) -> u32 { 0o111 << ((i as u32) * 3) }
    /// one mid => its contribution to key
    pub const fn ks_key(ks: u8) -> u32 {
        const TABLE: [u32; 16] = [
            // 111m, 123m, ..., 888m, 999m
            k_key(0), s_key(0), k_key(1), s_key(1),
            k_key(2), s_key(2), k_key(3), s_key(3),
            k_key(4), s_key(4), k_key(5), s_key(5),
            k_key(6), s_key(6), k_key(7), k_key(8),
        ];
        TABLE[ks as usize]
    }

    pub fn mid_to_key(mid: u16, n: u8) -> u32 {
        mid_to_smalls(mid).into_iter().take(n as usize).map(ks_key).sum()
    }

    pub fn recover_known_pair(key: u32, num_groups: u8, mid: u16) -> u8 {
        let pair_key = key - mid_to_key(mid, num_groups);
        (pair_key.trailing_zeros() / 3) as u8
    }
}
