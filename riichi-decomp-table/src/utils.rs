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
