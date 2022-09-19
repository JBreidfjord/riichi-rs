//! Complete hand decomposition

use nanovec::{NanoArray, NanoArrayBit, NanoDeque, NanoStackBit};
use rustc_hash::FxHashMap as HashMap;

use crate::utils::*;
use details::*;

pub type CTable = HashMap<u32, CAlts>;
pub type CTableStatic = phf::Map<u32, u64>;

/// Since the CTable is statically determinable, we can check its number of keys to make sure that
/// we have generated the correct table.
pub const C_TABLE_NUM_KEYS: usize = 21743;

/// Each entry of the C-Table is 1..=4 "alternatives", each consists of 1..=4 [`Groups`].
///
/// Each alternative is stored as the bitwise inverse of the bit-packed groups array.
/// Since `0xFFFF` is not a valid alternative, we can avoid storing its inverse, which is zero.
/// This allows us to use [`NanoStackBit`] to encode the number of alternatives implicitly.
pub type CAlts = NanoStackBit<u64, u16, 16>;

/// Each group is in 0..16 (prefix of suited HandGroup encoding in the main package).
/// The number of groups is implicit by the hand (len == num_tiles / 3).
pub type CGroups = NanoArrayBit<u16, u8, 4>;

pub fn make_c_table() -> CTable {
    let mut table = CTable::with_capacity_and_hasher(
        C_TABLE_NUM_KEYS, Default::default());
    let jantou_only_alts = CAlts::new().with(!0);
    table.insert(0, jantou_only_alts);
    dfs_koutsu(&mut table, 0, 0, 0, CGroups::new());
    dfs_shuntsu(&mut table, 0, 0, 0, CGroups::new());
    for j in 0..=8 {
        let j_key = 2 << ((j as u32) * 3);
        table.insert(j_key, jantou_only_alts);
        dfs_koutsu(&mut table, 0, 0, j_key, CGroups::new());
        dfs_shuntsu(&mut table, 0, 0, j_key, CGroups::new());
    }
    table
}

fn dfs_koutsu(table: &mut CTable, i: u8, pos0: u8, key: u32, groups: CGroups) {
    for pos in pos0..=8 {
        let new_key = key + k_key(pos);
        if key_is_overflow(new_key) { continue; }
        let new_groups = groups.with(i as usize, k_to_ks(pos));

        table.entry(new_key).or_insert(CAlts::new()).push(!new_groups.packed());

        if i < 3 {
            dfs_koutsu(table, i + 1, pos + 1, new_key, new_groups);
            dfs_shuntsu(table, i + 1, 0, new_key, new_groups);
        }
    }
}

fn dfs_shuntsu(table: &mut CTable, i: u8, pos0: u8, key: u32, groups: CGroups) {
    for pos in pos0..=6 {
        let new_key = key + s_key(pos);
        if key_is_overflow(new_key) { continue; }
        let new_groups = groups.with(i as usize, s_to_ks(pos));

        table.entry(new_key).or_default().push(!new_groups.packed());

        if i < 3 {
            dfs_shuntsu(table, i + 1, pos, new_key, new_groups);
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CompleteGrouping {
    pub groups: NanoDeque<CGroups>,
    raw_pair: u8,
}

impl CompleteGrouping {
    pub fn has_shuntsu(self) -> bool {
        any_group_is_shuntsu(self.groups.packed())
    }
    pub fn pair(self) -> Option<u8> {
        (self.raw_pair != u8::MAX).then_some(self.raw_pair)
    }
}

pub fn c_entry_iter(key: u32, alts_packed: u64) -> impl Iterator<Item = CompleteGrouping> {
    c_entry_iter_alts(key, CAlts::from_packed(alts_packed))
}

pub fn c_entry_iter_alts(key: u32, alts: CAlts) -> impl Iterator<Item = CompleteGrouping> {
    let n = key_sum(key);
    let num_groups = n / 3;
    let has_pair = n % 3 == 2;
    alts.map(move |inv_groups_packed| {
        let groups = CGroups::from_packed(!inv_groups_packed);
        let raw_pair = if has_pair {
            recover_known_pair(key, groups, num_groups as usize)
        } else {
            u8::MAX
        };
        CompleteGrouping {
            groups: NanoDeque::<CGroups>::from_packed_len(
                !inv_groups_packed, num_groups as u8),
            raw_pair,
        }
    })

}

pub(crate) mod details {
    use nanovec::NanoArray;
    use super::{CGroups};

    /// Encodes a Koutsu(0..=8) into a Group(0..=0xF)
    pub const fn k_to_ks(i: u8) -> u8 { if i == 8 { 0xF } else { i * 2 } }
    /// Encodes a Shuntsu(0..=6) into a Group(1..=0xD)
    pub const fn s_to_ks(i: u8) -> u8 { i * 2 + 1 }

    /// Packed hand of a Koutsu at `i`.
    pub const fn k_key(i: u8) -> u32 { 3 << ((i as u32) * 3) }
    /// Packed hand of a Shuntsu at `i`.
    pub const fn s_key(i: u8) -> u32 { 0o111 << ((i as u32) * 3) }
    /// Packed hand of an encoded Group (either Koutsu or Shuntsu).
    pub const fn ks_key(ks: u8) -> u32 {
        const TABLE: [u32; 16] = [
            // 111, 123, 222, 234,
            k_key(0), s_key(0), k_key(1), s_key(1),
            // 333, 345, 444, 456,
            k_key(2), s_key(2), k_key(3), s_key(3),
            // 555, 567, 666, 678,
            k_key(4), s_key(4), k_key(5), s_key(5),
            // 777, 789, 888, 999,
            k_key(6), s_key(6), k_key(7), k_key(8),
        ];
        TABLE[ks as usize]
    }

    pub fn any_group_is_shuntsu(packed_groups: u16) -> bool {
        let mask = !((((packed_groups >> 1) & 0x7777) + 0x1111) >> 3) & 0x1111 & packed_groups;
        mask != 0
    }

    pub fn groups_to_key(mid: CGroups, num_groups: usize) -> u32 {
        (0..num_groups).map(|i| mid.get(i)).map(ks_key).sum()
    }

    pub fn recover_known_pair(key: u32, groups: CGroups, num_groups: usize) -> u8 {
        let pair_key = key - groups_to_key(groups, num_groups);
        (pair_key.trailing_zeros() / 3) as u8
    }
}
