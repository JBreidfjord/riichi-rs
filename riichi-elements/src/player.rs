//! [`Player`] newtype (mod-4 arithmetic).

use core::fmt::{Debug, Display, Formatter};
use core::ops::{Add, Sub};
use derive_more::{From, Into};

/// Player index -- 0, 1, 2, 3 => player assigned east, south, west, north in the initial round.
///
/// This is forced to mod-4 arithmetic, and can represent both the absolute player index or
/// the difference between player indices ("relative player").
///
/// Reason for reinventing the wheel instead of using `ux`, `bare_metal_modulo` etc.:
/// This is trivial, and these don't support `serde`.
///
/// ## Optional `serde` support
///
/// Serializes as the player index number (0/1/2/3). Deserialization will check the range.
///
#[derive(Copy, Clone, Default, Eq, PartialEq, Hash, From, Into)]
pub struct Player(u8);

pub const P0: Player = Player(0);
pub const P1: Player = Player(1);
pub const P2: Player = Player(2);
pub const P3: Player = Player(3);
pub const ALL_PLAYERS: [Player; 4] = [P0, P1, P2, P3];

impl Player {
    pub const fn new(x: u8) -> Self { Player(x & 3) }
    
    pub const fn add(self, other: Player) -> Player {
        Player(self.0.wrapping_add(other.0) & 3)
    }

    pub const fn add_u8(self, other: u8) -> Player {
        Player(self.0.wrapping_add(other) & 3)
    }

    pub const fn sub(self, other: Player) -> Player {
        Player(self.0.wrapping_sub(other.0) & 3)
    }

    pub const fn sub_u8(self, other: u8) -> Player {
        Player(self.0.wrapping_sub(other) & 3)
    }

    pub const fn to_u8(self) -> u8 { self.0 }
    pub const fn to_usize(self) -> usize { self.0 as usize }

    /// Returns the player 1 turn after me, a.k.a. Successor, Shimocha (下家).
    /// In a physical game, said player would sit to the right of me (CCW).
    pub const fn succ(self) -> Self { self.add(P1) }

    /// Returns the player 2 turns after me, a.k.a. Opposing, Toimen (対面).
    /// In a physical game, said player would sit across the table from me.
    pub const fn oppo(self) -> Self { self.add(P2) }

    /// Returns the player 1 turn before me, a.k.a. Predecessor, Kamicha (上家).
    /// In a physical game, said player would sit to the left of me (CW).
    pub const fn pred(self) -> Self { self.add(P3) }
}

impl From<usize> for Player {
    fn from(x: usize) -> Self { Self::new(x as u8) }
}

impl Into<usize> for Player {
    fn into(self) -> usize { self.0 as usize }
}

impl Add for Player {
    type Output = Player;
    fn add(self, rhs: Self) -> Self::Output { self.add(rhs) }
}

impl Add<u8> for Player {
    type Output = Player;
    fn add(self, rhs: u8) -> Self::Output { self.add_u8(rhs) }
}

impl Sub for Player {
    type Output = Player;
    fn sub(self, rhs: Self) -> Self::Output { self.sub(rhs) }
}

impl Sub<u8> for Player {
    type Output = Player;
    fn sub(self, rhs: u8) -> Self::Output { self.sub_u8(rhs) }
}

impl Debug for Player {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Player({})", self.0)
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Shorthand for [`Player::new`].
pub const fn player(i: u8) -> Player { Player::new(i) }

/// Returns an array of all players, starting from the given player, in natural turn order.
///
/// Example:
/// ```
/// use riichi_elements::player::*;
/// assert_eq!(all_players_from(P2), [P2, P3, P0, P1]);
/// ```
pub const fn all_players_from(player: Player) -> [Player; 4] {
    [player.add(P0), player.add(P1), player.add(P2), player.add(P3)]
}

/// Returns an array of the 3 players after the given player, in natural turn order.
///
/// Example:
/// ```
/// use riichi_elements::player::*;
/// assert_eq!(other_players_after(P2), [P3, P0, P1]);
/// ```
pub const fn other_players_after(player: Player) -> [Player; 3] {
    [player.add(P1), player.add(P2), player.add(P3)]
}

#[cfg(feature = "serde")]
mod player_serde {
    use core::fmt::Formatter;
    use serde::{*};
    use serde::de::{Error, Visitor};
    use super::*;

    impl Serialize for Player {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error> where S: Serializer {
            s.serialize_u8(self.0.into())
        }
    }

    impl<'de> Deserialize<'de> for Player {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
            struct PlayerVisitor;
            impl<'a> Visitor<'a> for PlayerVisitor {
                type Value = Player;

                fn expecting(&self, f: &mut Formatter) -> core::fmt::Result {
                    write!(f, "0..=3")
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: Error {
                    if (0..=3).contains(&v) {
                        Ok(Player(v as u8))
                    } else {
                        Err(E::custom("out of range"))
                    }
                }
            }
            deserializer.deserialize_u8(PlayerVisitor)
        }
    }
}
