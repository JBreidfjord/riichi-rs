use ux::u2;

/// Points / point differences. Usually increments of 100.
pub type GamePoints = i32;

/// Wind index --- 0, 1, 2, 3 => east, south, west, north
///
/// This is forced to 2-bit modulo arithmetic.
pub type Wind = u2;

/// Player index -- 0, 1, 2, 3 => player initially assigned east, south, west, north
///
/// This is forced to 2-bit modulo arithmetic.
pub type Player = u2;

// Hack to add convenient conversions --- `ux` could have done this better for us...
pub trait U2Traits {
    fn to_u8(self) -> u8;
    fn to_usize(self) -> usize;
}
impl U2Traits for u2 {
    fn to_u8(self) -> u8 { u8::from(self) }
    fn to_usize(self) -> usize { self.to_u8() as usize }
}
