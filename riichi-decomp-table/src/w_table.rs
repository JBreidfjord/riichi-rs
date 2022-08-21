
use num_enum::{TryFromPrimitive, IntoPrimitive};
use rustc_hash::FxHashMap as HashMap;

use super::utils::*;

#[derive(Copy, Clone, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum TenpaiType {
    #[num_enum(default)]
    Tanki = 0,   // e.g. 1222333444555z wait 1z
    Shanpon,     // e.g. 4477s wait 4s/7s
    Kanchan,     // e.g. 13m wait 2m
    RyanmenHigh, // e.g. 12m wait 3m, 12333345m77z wait 6m (because 3m is used up)
    RyanmenLow,  // e.g. 89m wait 7m, ditto
    RyanmenBoth, // e.g. 34m wait 2m/5m
}

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
pub const W_TABLE_NUM_KEYS: usize = 66913;

pub fn make_w_table(c_table: &super::c_table::CTable) -> WTable {
    let mut w_table = WTable::with_capacity_and_hasher(W_TABLE_NUM_KEYS, Default::default());
    for (&key, &big) in c_table.iter() {
        make_waiting_for_c_entry(&mut w_table, key, big);
    }
    w_table
}

pub fn make_waiting_for_c_entry(w_table: &mut WTable, key: u32, _big: u64) {

    let num_tiles = key_sum(key);
    let mid_len = num_tiles / 3;
    let has_pair = (num_tiles % 3) == 2;

    let mut push = |new_key, i: i8, tenpai_type: TenpaiType, _has_pair: bool| {
        let x = w_table.entry(new_key).or_default();
        *x = x.checked_mul(56).unwrap() + ((i as u8 + 1 + 9 * u8::from(tenpai_type)) as u64);
    };

    if !has_pair {
        for i in 0..=8 {
            if let Some(new_key) = check_pattern(key, 0o1, i, 0) {
                push(new_key, i, TenpaiType::Tanki, true);
            }
        }
    }
    if mid_len <= 3 {
        // only 3 or less mentsu in complete part
        // try add mentsu-based tenpai pattern
        for i in 0..=8 {
            if let Some(new_key) = check_pattern(key, 0o2, i, 0) {
                push(new_key, i, TenpaiType::Shanpon, has_pair);
            }
        }
        for i in 0..=6 {
            if let Some(new_key) = check_pattern(key, 0o101, i, 1) {
                push(new_key, i, TenpaiType::Kanchan, has_pair);
            }
        }
        for i in 0..=7 {
            let key_low = check_pattern(key, 0o11, i, -1);
            let key_high = check_pattern(key, 0o11, i, 2);
            if key_low.is_some() && key_high.is_some() {
                push(key_low.unwrap(), i, TenpaiType::RyanmenBoth, has_pair);
            } else if let Some(key) = key_low {
                push(key, i, TenpaiType::RyanmenLow, has_pair);
            } else if let Some(key) = key_high {
                push(key, i, TenpaiType::RyanmenHigh, has_pair);
            }
        }
    }
}

fn check_pattern(key: u32, pat: u8, pat_pos: i8, tenpai_offset: i8) -> Option<u32> {
    let new_key = key + ((pat as u32) << ((pat_pos as u32) * 3));
    let tenpai = pat_pos + tenpai_offset;
    if (0 <= tenpai && tenpai <= 8) &&
        !key_is_overflow(new_key) &&
        ((new_key >> ((tenpai as u32) * 3)) & 0o7) < 4 {
        Some(new_key)
    } else {
        None
    }
}
