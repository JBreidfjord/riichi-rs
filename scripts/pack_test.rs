#!/usr/bin/env rust-script
// cargo-deps: packed_struct="0.10"

use packed_struct::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, PackedStruct)]
#[packed_struct(size_bytes="4", bit_numbering="lsb0", endian="lsb")]
pub struct Pack {
    #[packed_field(bits="0..4")]
    pub a: u8,
    #[packed_field(bits="4..12")]
    pub bc: u8,
    #[packed_field(bits="12..16")]
    pub d: u8,
    #[packed_field(bits="16..32")]
    pub e: u16,
}

pub fn main() {
    {
        let packed = 0x76543210u32;

        let packed_bytes = packed.to_le_bytes();
        assert_eq!(packed_bytes, [0x10, 0x32, 0x54, 0x76u8]);

        let fields = Pack::unpack(&packed_bytes).unwrap();
        println!("{}", fields);
        assert_eq!(fields, Pack { a: 0x0, bc: 0x21, d: 0x3, e: 0x7654 });
    }
    /*
    {
        let fields = Pack { a: 0x8, b: 0x7, c: 0x6, d: 0x5, e: 0x1234 };
        let packed_bytes = fields.pack().unwrap();
        let packed = u32::from_le_bytes(packed_bytes);
        println!("{}", fields);
        assert_eq!(packed_bytes, [0x78u8, 0x56u8, 0x34u8, 0x12u8]);
        assert_eq!(packed, 0x12345678u32);
    }
    */
}
