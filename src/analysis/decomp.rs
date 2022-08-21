
use once_cell::sync::OnceCell;

use crate::common::*;

/// One way to break down (all suits, 3N+1 total tiles) as a waiting hand.
pub struct FullHandRem1Decomp {
    /// 0..=4 x 6-bit [encoded hand groups](HandGroup).
    pub groups: u32,

    ///
    pub full_pair: Option<Tile>,
}

pub mod c_table {
    use rustc_hash::FxHashMap as HashMap;
    use itertools::Itertools;

    // what we want: map from key (27-bit) to 0..=4 x (0..=4 x KS)
    // what we do here:
    // - ks := koutsu or shuntsu (4-bit) , a.k.a. "small"
    // - pack 0..=4 x small => mid (16-bit, len is deduced externally by key)
    // - pack 0..=4 x mid => big (64-bit, len is deduced by null termination)

    pub const fn big_push(big: u64, mid: u16) -> u64 {
        (big << 16) | (mid as u64)
    }
    pub const fn big_top(big: u64) -> u16 { big as u16 }
    pub const fn big_pop(big: u64) -> u64 { big >> 16 }
    pub const fn big_to_mids(big: u64) -> [u16; 4] {
        [
            ((big >> 0) & 0xFFFF) as u16,
            ((big >> 16) & 0xFFFF) as u16,
            ((big >> 32) & 0xFFFF) as u16,
            ((big >> 48) & 0xFFFF) as u16,
        ]
    }
    pub const fn big_len(big: u64) -> u32 { 4 - (big.leading_zeros() / 16) }

    pub const fn mid_push(mid: u16, small: u8) -> u16 {
        (mid << 4) | ((small as u16) & 0xF)
    }
    pub const fn mid_top(mid: u16) -> u8 { (mid as u8) & 0xF }
    pub const fn mid_pop(mid: u16) -> u16 { mid >> 4 }
    pub const fn mid_to_smalls(mid: u16) -> [u8; 4] {
        [
            ((mid >> 0) & 0xF) as u8,
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
    pub const fn key_is_overflow(key: u32) -> bool {
        (((key & 0o333333333) + 0o333333333) & key & 0o444444444) != 0
    }
    pub const fn key_sum(key: u32) -> u32 {
        let key = (key & 0o707070707) + ((key & 0o070707070) >> 3);
        let key = (key & 0o700770077) + ((key & 0o077007700) >> 6);
        let key = (key & 0o700007777) + ((key & 0o077770000) >> 12);
        (key & 0o077777777) + (key >> 24)
    }

    pub fn mid_to_key(mid: u16, n: u32) -> u32 {
        mid_to_smalls(mid).into_iter().take(n as usize).map(ks_key).sum()
    }

    pub type CTable = HashMap<u32, u64>;
    pub fn dfs_kou(table: &mut CTable, n: i32, i0: u8, key: u32, value: u16) {
        for i in i0..=8 {
            let new_key = key + k_key(i);
            if key_is_overflow(new_key) { continue; }
            let new_value = mid_push(value, k_to_ks(i));

            table.entry(new_key)
                .and_modify(|v| *v = big_push(*v, new_value))
                .or_insert(new_value as u64);

            if n < 4 {
                dfs_kou(table, n + 1, i + 1, new_key, new_value);
                dfs_shun(table, n + 1, 0, new_key, new_value);
            }
        }
    }
    pub fn dfs_shun(table: &mut CTable, n: i32, i0: u8, key: u32, value: u16) {
        for i in i0..=6 {
            let new_key = key + s_key(i);
            if key_is_overflow(new_key) { continue; }
            let new_value = mid_push(value, s_to_ks(i));

            table.entry(new_key)
                .and_modify(|v| *v = big_push(*v, new_value))
                .or_insert(new_value as u64);

            if n < 4 {
                dfs_shun(table, n + 1, i, new_key, new_value);
            }
        }
    }

    pub fn make_complete() -> CTable {
        let mut table = CTable::with_capacity_and_hasher(21743, Default::default());
        table.insert(0, 0);
        dfs_kou(&mut table, 1, 0, 0, 0);
        dfs_shun(&mut table, 1, 0, 0, 0);
        for j in 0..=8 {
            let j_key = 2 << ((j as u32) * 3);
            table.insert(j_key, 0);
            dfs_kou(&mut table, 1, 0, j_key, 0);
            dfs_shun(&mut table, 1, 0, j_key, 0);
        }
        assert_eq!(table.len(), 21743);
        table
    }

    pub fn table_to_string(table: &CTable) -> String {
        const SMALLS: [&'static str; 16] = [
           "10", "00", "11", "01", "12", "02", "13", "03", "14", "04", "15", "05", "16", "06", "17", "18",
        ];
        let mut lines: Vec<String> = vec![];
        for (&key, &value) in table.iter() {
            let n = key_sum(key);
            let mut big = value;
            loop {
                let mid = big_top(big);
                let mid_str = mid_to_smalls(mid)
                    .into_iter().take((n / 3) as usize)
                    .map(|ks| SMALLS[ks as usize])
                    .sorted().join(",");

                let pair = key - mid_to_key(mid, n / 3);
                if pair > 0 {
                    let pair_i = pair.trailing_zeros() / 3;
                    if mid_str.is_empty() {
                        lines.push(format!("{:09o},2{}", key, pair_i));
                    } else {
                        lines.push(format!("{:09o},{},2{}", key, mid_str, pair_i));
                    }
                } else {
                    if mid_str.is_empty() {
                        lines.push(format!("{:09o}", key));
                    } else {
                        lines.push(format!("{:09o},{}", key, mid_str));
                    }
                }
                big = big_pop(big);
                if big == 0 { break; }
            }
        }
        assert_eq!(lines.len(), 23533);
        lines.sort();
        lines.join("\n")
    }
}

pub mod w_table_naive {
    use rustc_hash::FxHashMap as HashMap;
    use crate::analysis::decomp::c_table::key_is_overflow;
    use super::c_table::{mid_to_key, key_sum};

    pub const TANKI: u8 = 0;
    pub const SHANPON: u8 = 1;
    pub const KANCHAN: u8 = 2;
    pub const PENCHAN: u8 = 3;
    pub const RYANMEN: u8 = 4;

    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    pub struct WEntry {
        pub big: u64,
        pub tenpai_type: u8,
        pub tenpai_num: u8,
        pub anchor_num: u8,
        pub has_pair: bool,
    }

    // pub type WTable = HashMap<u32, Vec<WEntry>>;
    pub type WTable = HashMap<u32, u64>;

    pub fn make_waiting(c_table: &super::c_table::CTable) -> WTable {
        let mut w_table = WTable::with_capacity_and_hasher(66913, Default::default());
        for (&key, &big) in c_table.iter() {
            make_waiting_for_c_entry(&mut w_table, key, big);
        }
        assert_eq!(w_table.len(), 66913);
        w_table
    }

    pub fn make_waiting_for_c_entry(w_table: &mut WTable, key: u32, _big: u64) {
        let num_tiles = key_sum(key);
        let mid_len = num_tiles / 3;
        let has_pair = (num_tiles % 3) == 2;

        let mut expand = |tenpai_type: u8, pat: u8, d_tenpai: i8, d_anchor: i8, i: i8, has_pair: bool| {
            let new_key = key + ((pat as u32) << ((i as u32) * 3));
            let tenpai = i + d_tenpai;
            let anchor = i + d_anchor;
            if !key_is_overflow(new_key) && ((new_key >> ((tenpai as u32) * 3)) & 0o7) < 4 {
                /*
                w_table.entry(new_key).or_default().push(WEntry {
                    big,
                    tenpai_type,
                    tenpai_num: tenpai as u8,
                    anchor_num: anchor as u8,
                    has_pair,
                });
                 */
                let x = w_table.entry(new_key).or_default();
                *x = *x * 45 + ((i as u8 + 1 + 9 * tenpai_type) as u64);
                true
            } else {
                false
            }
        };

        if !has_pair {
            for i in 0..=8 { expand(TANKI, 0o1, 0, 0, i, true); }
        }
        if mid_len <= 3 {
            // only 3 or less mentsu in complete part
            // try add mentsu-based tenpai pattern

            for i in 0..=8 { expand(SHANPON, 0o2, 0, 0, i, has_pair); }
            for i in 0..=6 { expand(KANCHAN, 0o101, 1, 0, i, has_pair); }

            // 12 and 89 are penchan; the rest are ryanmen.
            expand(PENCHAN, 0o11, 2, 0, 0, has_pair);
            for i in 1..=6 {
                if !expand(RYANMEN, 0o11, -1, -1, i, has_pair) {
                    expand(RYANMEN, 0o11, 2, 0, i, has_pair);
                }
            }
            expand(PENCHAN, 0o11, -1, -1, 7, has_pair);
        }
    }
}

struct FullHandFacts {
    m147: u8,
    m258: u8,
    m369: u8,
    mn: u8,
    p147: u8,
    p258: u8,
    p369: u8,
    pn: u8,
    // do we have to do this?
}
