use ux::u2;

/// Points / point differences. Usually increments of 100.
pub type GamePoints = i64;

/// Wind index --- 0, 1, 2, 3 => east, south, west, north
///
/// This is forced to 2-bit modulo arithmetic.
pub type Wind = u2;

/// Player index -- 0, 1, 2, 3 => player initially assigned east, south, west, north
///
/// This is forced to 2-bit modulo arithmetic.
pub type Player = u2;

pub const P0: Player = Player::new(0);
pub const P1: Player = Player::new(1);
pub const P2: Player = Player::new(2);
pub const P3: Player = Player::new(3);

pub fn all_players() -> [Player; 4] {
    [P0, P1, P2, P3]
}
pub fn all_players_from(player: Player) -> [Player; 4] {
    [
        P0.wrapping_add(player),
        P1.wrapping_add(player),
        P2.wrapping_add(player),
        P3.wrapping_add(player),
    ]
}
pub fn player_succ(player: Player) -> Player { P1.wrapping_add(player) }
pub fn player_oppo(player: Player) -> Player { P2.wrapping_add(player) }
pub fn player_pred(player: Player) -> Player { P3.wrapping_add(player) }
pub fn other_players_after(player: Player) -> [Player; 3] {
    [
        P1.wrapping_add(player),
        P2.wrapping_add(player),
        P3.wrapping_add(player),
    ]
}

// Hack to add convenient conversions --- `ux` could have done this better for us...
pub trait U2Traits {
    fn to_u8(self) -> u8;
    fn to_usize(self) -> usize;
}
impl U2Traits for u2 {
    fn to_u8(self) -> u8 { u8::from(self) }
    fn to_usize(self) -> usize { self.to_u8() as usize }
}
