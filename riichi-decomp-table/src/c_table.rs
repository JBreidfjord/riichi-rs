
use rustc_hash::FxHashMap as HashMap;
use itertools::Itertools;

use crate::utils::*;

pub type CTable = HashMap<u32, u64>;

/// Since the CTable is statically determinable, we can check its number of keys to make sure that
/// we have generated the correct table.
pub const C_TABLE_NUM_KEYS: usize = 21743;

pub fn make_c_table() -> CTable {
    let mut table = CTable::with_capacity_and_hasher(
        C_TABLE_NUM_KEYS, Default::default());
    table.insert(0, 0);
    dfs_kou(&mut table, 1, 0, 0, 0);
    dfs_shun(&mut table, 1, 0, 0, 0);
    for j in 0..=8 {
        let j_key = 2 << ((j as u32) * 3);
        table.insert(j_key, 0);
        dfs_kou(&mut table, 1, 0, j_key, 0);
        dfs_shun(&mut table, 1, 0, j_key, 0);
    }
    table
}

fn dfs_kou(table: &mut CTable, n: i32, i0: u8, key: u32, value: u16) {
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

fn dfs_shun(table: &mut CTable, n: i32, i0: u8, key: u32, value: u16) {
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
