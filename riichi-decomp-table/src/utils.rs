pub const fn key_is_overflow(key: u32) -> bool {
    (((key & 0o333333333) + 0o333333333) & key & 0o444444444) != 0
}

pub const fn key_sum(key: u32) -> u32 {
    let key = (key & 0o707070707) + ((key & 0o070707070) >> 3);
    let key = (key & 0o700770077) + ((key & 0o077007700) >> 6);
    let key = (key & 0o700007777) + ((key & 0o077770000) >> 12);
    (key & 0o077777777) + (key >> 24)
}
