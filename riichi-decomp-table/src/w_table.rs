use num_enum::{TryFromPrimitive, IntoPrimitive};
use rustc_hash::FxHashMap as HashMap;

use crate::utils::*;
use details::*;

/*
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct WEntry {
    pub big: u64,
    pub tenpai_type: u8,
    pub tenpai_num: u8,
    pub anchor_num: u8,
    pub has_pair: bool,
}
// pub type WTable = HashMap<u32, Vec<WEntry>>;
 */

pub type WTable = HashMap<u32, u64>;
pub type WTableStatic = phf::Map<u32, u64>;

/// Since the WTable is statically determinable, we can check its number of keys to make sure that
/// we have generated the correct table.
pub const W_TABLE_NUM_KEYS: usize = 66913;

pub fn make_w_table(c_table: &super::c_table::CTable) -> WTable {
    let mut w_table = WTable::with_capacity_and_hasher(
        W_TABLE_NUM_KEYS, Default::default());
    for (&key, &value) in c_table.iter() {
        make_waiting_for_c_entry(&mut w_table, key, value);
    }
    w_table
}

fn make_waiting_for_c_entry(w_table: &mut WTable, key: u32, _c_value: u64) {
    let num_tiles = key_sum(key);
    let mid_len = num_tiles / 3;
    let has_pair = (num_tiles % 3) == 2;

    let mut push = |new_key, pos: i8, waiting_kind: WaitingKind, _has_pair: bool| {
        let x = w_table.entry(new_key).or_default();
        *x = w_push(*x, pos, waiting_kind);
    };

    if !has_pair {
        for pos in 0..=8 {
            if let Some(new_key) = check_pattern(key, 0o1, pos, 0) {
                push(new_key, pos, WaitingKind::Tanki, true);
            }
        }
    }
    if mid_len <= 3 {
        // only 3 or less mentsu in complete part
        // try add mentsu-based tenpai pattern
        for pos in 0..=8 {
            if let Some(new_key) = check_pattern(key, 0o2, pos, 0) {
                push(new_key, pos, WaitingKind::Shanpon, has_pair);
            }
        }
        for pos in 0..=6 {
            if let Some(new_key) = check_pattern(key, 0o101, pos, 1) {
                push(new_key, pos, WaitingKind::Kanchan, has_pair);
            }
        }
        for pos in 0..=7 {
            let key_low = check_pattern(key, 0o11, pos, -1);
            let key_high = check_pattern(key, 0o11, pos, 2);
            if key_low.is_some() && key_high.is_some() {
                push(key_low.unwrap(), pos, WaitingKind::RyanmenBoth, has_pair);
            } else if let Some(key) = key_low {
                push(key, pos, WaitingKind::RyanmenLow, has_pair);
            } else if let Some(key) = key_high {
                push(key, pos, WaitingKind::RyanmenHigh, has_pair);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct WaitingPattern {
    pub complete_key: u32,
    pub waiting_kind: WaitingKind,
    pub pattern_pos: u8,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum WaitingKind {
    #[num_enum(default)]
    Tanki = 0,   // e.g. 1222333444555z wait 1z
    Shanpon,     // e.g. 4477s wait 4s/7s
    Kanchan,     // e.g. 13m wait 2m
    RyanmenHigh, // e.g. 12m wait 3m, 12333345m77z wait 6m (because 3m is used up)
    RyanmenLow,  // e.g. 89m wait 7m, ditto
    RyanmenBoth, // e.g. 34m wait 2m/5m
}

pub fn w_entry_iter(key: u32, value: u64) -> impl Iterator<Item = WaitingPattern> {
    WaitingPatternIterator { key, value }
}

struct WaitingPatternIterator {
    key: u32,
    value: u64,
}

impl Iterator for WaitingPatternIterator {
    type Item = WaitingPattern;

    fn next(&mut self) -> Option<Self::Item> {
        use WaitingKind::*;

        let (new_value, pos, waiting_kind) = w_pop(self.value)?;
        let pattern = match waiting_kind {
            Tanki => 0o1,
            Shanpon => 0o2,
            Kanchan => 0o101,
            RyanmenHigh | RyanmenLow | RyanmenBoth => 0o11,
        };
        let complete_key = self.key - (pattern << ((pos as u32) * 3));
        self.value = new_value;
        Some(WaitingPattern{
            complete_key,
            waiting_kind,
            pattern_pos: pos,
        })
    }
}

pub(crate) mod details {
    use crate::utils::*;
    use super::*;

    pub fn check_pattern(key: u32, pattern: u8, pos: i8, waiting_offset: i8) -> Option<u32> {
        let new_key = key + ((pattern as u32) << ((pos as u32) * 3));
        let waiting_pos = pos + waiting_offset;
        if (0 <= waiting_pos && waiting_pos <= 8) &&
            !key_is_overflow(new_key) &&
            ((new_key >> ((waiting_pos as u32) * 3)) & 0o7) < 4 {
            Some(new_key)
        } else {
            None
        }
    }

    pub fn w_push(w: u64, pos: i8, waiting_kind: WaitingKind) -> u64 {
        w.checked_mul(56).unwrap() + (((pos as u8) + 1 + 9 * u8::from(waiting_kind)) as u64)
    }

    pub fn w_pop(w: u64) -> Option<(u64, u8, WaitingKind)> {
        if w == 0 { return None; }
        let x = ((w % 56) as u8) - 1;
        Some((w / 56, x % 9, WaitingKind::try_from(x / 9).unwrap()))
    }
}
