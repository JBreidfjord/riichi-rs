use bitfield_struct::bitfield;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::common::Tile;
use super::{Meld, Chii, Pon, Kakan, Daiminkan, Ankan};

/// Defines the bit-fields for packing `Meld` into `u16`:
///
/// - `[5:0]` -- tile
/// - `[7:6]` -- dir
/// - `[11:8]` -- red
/// - `[14:12]` -- kind
///
#[bitfield(u16)]
pub(crate) struct PackedMeld {
    /// The lowest normal num (no red) tile
    #[bits(6)]
    pub tile: u8,

    /// - Chii: The normal number of the called tile is `min_num + dir`
    /// - Pon/Kakan/Daiminkan: The tile is called from `this_player + dir` (mod 4)
    #[bits(2)]
    pub dir: u8,

    /// Packed red fields; see comments below
    #[bits(4)]
    pub red: u8,

    #[bits(3)]
    pub kind: u8,

    #[bits(1)]
    pub _reserved0: u8,
}

impl PackedMeld {
    pub fn get_tile(self) -> Option<Tile> {
        Tile::from_encoding(self.tile()).map(|t| t.to_normal())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum PackedMeldKind {
    Chii = 0,
    Pon = 1,
    Kakan = 2,
    Daiminkan = 3,
    Ankan = 4,
}

impl TryFrom<PackedMeld> for Meld {
    type Error = ();

    fn try_from(raw: PackedMeld) -> Result<Self, Self::Error> {
        match PackedMeldKind::try_from(raw.kind()).map_err(|_| ())? {
            PackedMeldKind::Chii =>
                Chii::try_from(raw).map(|chii| Meld::Chii(chii)),
            PackedMeldKind::Pon =>
                Pon::try_from(raw).map(|pon| Meld::Pon(pon)),
            PackedMeldKind::Kakan =>
                Kakan::try_from(raw).map(|kakan| Meld::Kakan(kakan)),
            PackedMeldKind::Daiminkan =>
                Daiminkan::try_from(raw).map(|daiminkan| Meld::Daiminkan(daiminkan)),
            PackedMeldKind::Ankan =>
                Ankan::try_from(raw).map(|ankan| Meld::Ankan(ankan)),
        }
    }
}

impl From<Meld> for PackedMeld {
    fn from(meld: Meld) -> Self {
        match meld {
            Meld::Chii(chii) => PackedMeld::from(chii),
            Meld::Pon(pon) => PackedMeld::from(pon),
            Meld::Kakan(kakan) => PackedMeld::from(kakan),
            Meld::Daiminkan(daiminkan) => PackedMeld::from(daiminkan),
            Meld::Ankan(ankan) => PackedMeld::from(ankan),
        }
    }
}

impl Meld {
    /// Parse from the packed representation.
    pub fn from_packed(packed: u16) -> Option<Self> {
        Meld::try_from(PackedMeld::try_from(packed).ok()?).ok()
    }
    /// Convert to the packed representation.
    pub fn packed(self) -> u16 {
        u16::from(PackedMeld::from(self))
    }
}

// Definition of `PackedMeld::red` as a sub-bitfield.
//
// | bit | Chii    | Pon     | Kakan   | Daiminkan | Ankan   |
// |-----|---------|---------|---------|-----------|---------|
// | 0   | any     | own0    | own0    | own0      | own0    |
// | 1   | 0       | own1    | own1    | own1      | own1    |
// | 2   | 0       | called  | called  | own2      | own2    |
// | 3   | 0       | 0       | added   | called    | own3    |
//
// Note that the order of "own" tiles does not matter, but we will always normalize to use the
// smallest bit representation; e.g. (`0b0011` instead of `0b1010`).
//
// Examples:
// - **Chii**: If _any_ tile is red, `0b0001`; otherwise `0b0000`.
// - **Pon**:
//     - Use 55 to call 0 => `0b0100`
//     - Use 05 to call 5 => `0b0001`
//     - Use 05 to call 0 => `0b0101`
// - **Kakan**:
//     - Add 0 to (55 pon 0) => `0b1100`
//     - Add 5 to (55 pon 0) => `0b0100` (unchanged)
//     - Add 0 to (05 pon 0) => `0b1101`
// - **Daiminkan**:
//     - Use 055 to call 5 => `0b0001`
//     - Use 005 to call 0 => `0b1011`
// - **Ankan**: 0005 => `0b0111`
//

// Any function from u4 to u4 can be represented as a u64 (2^4 x 4).
// Here we take advantage of this to efficiently normalize (a slice of) red bits.

const fn normalize_bits(x: u8, n: u8) -> u8 {
    let lsbs = x & ((1 << n) - 1);
    let msbs = x & !((1 << n) - 1);
    let new_lsbs = (1u8 << lsbs.count_ones()) - 1u8;
    msbs | new_lsbs
}

const fn normalize_mask(n: u8) -> u64 {
    let mut mask = 0u64;
    let mut x = 0u8;
    while x < 16u8 {
        mask |= (normalize_bits(x, n) as u64) << (x * 4);
        x += 1;
    }
    mask
}

const MASK_PON_KAKAN: u64 = normalize_mask(2);
const MASK_DAIMINKAN: u64 = normalize_mask(3);
const MASK_ANKAN: u64 = normalize_mask(4);

pub const fn normalize_pon(x: u8) -> u8 { ((MASK_PON_KAKAN >> (x * 4)) & 0b0111) as u8 }
pub const fn normalize_kakan(x: u8) -> u8 { ((MASK_PON_KAKAN >> (x * 4)) & 0b1111) as u8 }
pub const fn normalize_daiminkan(x: u8) -> u8 { ((MASK_DAIMINKAN >> (x * 4)) & 0b1111) as u8 }
pub const fn normalize_ankan(x: u8) -> u8 { ((MASK_ANKAN >> (x * 4)) & 0b1111) as u8 }

