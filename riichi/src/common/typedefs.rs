use ux::u2;

/// Points / point differences. Usually increments of 100.
pub type GamePoints = i64;

/// Wind index --- 0, 1, 2, 3 => east, south, west, north.
///
/// Note that this is _identical_ to [`Player`] --- see its definition.
pub type Wind = u2;

/// Player index -- 0, 1, 2, 3 => player assigned east, south, west, north in the initial round.
///
/// This is forced to mod-4 arithmetic, and can represent both the absolute player index or
/// the difference between player indices ("relative player").
pub type Player = u2;

pub const P0: Player = Player::new(0);
pub const P1: Player = Player::new(1);
pub const P2: Player = Player::new(2);
pub const P3: Player = Player::new(3);

/// Returns the array of all players, in numerical order.
pub const fn all_players() -> [Player; 4] {
    [P0, P1, P2, P3]
}

/// Returns an array of all players, starting from the given player, in natural turn order.
///
/// Example:
/// ```
/// use riichi::common::*;
/// assert_eq!(all_players_from(P2), [P2, P3, P0, P1]);
/// ```
pub fn all_players_from(player: Player) -> [Player; 4] {
    [
        P0.wrapping_add(player),
        P1.wrapping_add(player),
        P2.wrapping_add(player),
        P3.wrapping_add(player),
    ]
}
/// Returns the next player after the given player in natural turn order, a.k.a. Successor.
/// In a physical game, this player would sit to the right of the given player (CCW).
pub fn player_succ(player: Player) -> Player { P1.wrapping_add(player) }

/// Returns the player 2 turns after the given player in natural turn order, a.k.a. Opposing.
/// In a physical game, this player would sit across the table from the given player.
pub fn player_oppo(player: Player) -> Player { P2.wrapping_add(player) }

/// Returns the previous player before the given player in natural turn order, a.k.a. Predecessor.
/// In a physical game, this player would sit to the left of the given player (CW).
pub fn player_pred(player: Player) -> Player { P3.wrapping_add(player) }

/// Returns an array of the 3 players after the given player, in natural turn order.
///
/// Example:
/// ```
/// use riichi::common::*;
/// assert_eq!(other_players_after(P2), [P3, P0, P1]);
/// ```
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

// Hack to provide serde impls for u2
#[cfg(feature = "serde")]
mod u2_serde {
    use serde::*;
    use super::*;
    #[derive(Serialize, Deserialize)]
    #[serde(remote = "u2")]
    pub struct U2Serde(
        #[serde(getter = "u2_to_u8")]
        u8
    );
    impl Into<u2> for U2Serde {
        fn into(self) -> u2 { u2::new(self.0) }
    }
    pub fn u2_to_u8(u2: &u2) -> u8 { u8::from(*u2) }
}
#[cfg(feature = "serde")]
pub use u2_serde::*;
