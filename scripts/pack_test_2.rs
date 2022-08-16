#!/usr/bin/env rust-script
// cargo-deps: bitfield-struct = "0.1"

use bitfield_struct::bitfield;

#[bitfield(u32)]
pub struct Pack {
    #[bits(4)]
    pub a: u8,
    #[bits(8)]
    pub bc: u8,
    #[bits(4)]
    pub d: u8,

    pub e: u16,
}

pub fn main() {
    {
        let packed = 0x76543210u32;
        let fields = Pack::from(packed);
        assert_eq!(fields.a(), 0x0);
        assert_eq!(fields.bc(), 0x21);
        assert_eq!(fields.d(), 0x3);
        assert_eq!(fields.e(), 0x7654);
        assert_eq!(u32::from(fields), packed);
        assert_eq!(u32::from(fields), u32::from(Pack::new()
            .with_a(0x0)
            .with_bc(0x21)
            .with_d(0x3)
            .with_e(0x7654)));
        println!("OK");
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
