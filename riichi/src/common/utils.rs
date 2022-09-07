//! Collection of misc utilities...

use std::ops::*;

pub fn sort2<T: Ord>(a: T, b: T) -> (T, T) {
    if a < b { (a, b) } else { (b, a) }
}

pub fn sort3<T: Ord>(a: T, b: T, c: T) -> (T, T, T) {
    let (a1, b1) = sort2(a, b);
    let (b2, c2) = sort2(b1, c);
    let (a3, b3) = sort2(a1, b2);
    (a3, b3, c2)
}

pub const fn unpack4(x: u8) -> (bool, bool, bool, bool) {
    (
        (x & 0b0001) > 0,
        (x & 0b0010) > 0,
        (x & 0b0100) > 0,
        (x & 0b1000) > 0,
    )
}

pub const fn pack4(a: bool, b: bool, c: bool, d: bool) -> u8 {
    ((a as u8) << 0) | ((b as u8) << 1) | ((c as u8) << 2) | ((d as u8) << 3)
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use super::*;

    #[test]
    fn sort3_test() {
        for x in [1, 2, 3].into_iter().permutations(3) {
            assert_eq!(sort3(x[0], x[1], x[2]), (1, 2, 3));
        }
    }
}
