// TODO(summivox): If we are already doing so much to write down the entire API surface, then
// perhaps we should simply re-implement this?

use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Sub};
use derive_more::{From, Into};
use ux::u2;

/// Player index -- 0, 1, 2, 3 => player assigned east, south, west, north in the initial round.
///
/// This is forced to mod-4 arithmetic, and can represent both the absolute player index or
/// the difference between player indices ("relative player").
///
/// ## Optional `serde` support
///
/// Serializes as the underlying number. When deserialized, automatically takes mod 4.
///
#[derive(Copy, Clone, Default, Eq, PartialEq, Hash, From, Into)]
pub struct Player(pub u2);

impl Player {
    pub const fn new(x: u8) -> Self { Player(u2::new(x % 4)) }
    
    pub fn wrapping_add(self, other: Player) -> Player {
        Player(self.0.wrapping_add(other.0))
    }

    pub fn wrapping_sub(self, other: Player) -> Player {
        Player(self.0.wrapping_sub(other.0))
    }

    pub fn to_u8(self) -> u8 { u8::from(self.0) }
    pub fn to_usize(self) -> usize { self.to_u8() as usize }
}

impl From<u8> for Player {
    fn from(x: u8) -> Self { Self::new(x) }
}
impl From<usize> for Player {
    fn from(x: usize) -> Self { Self::new(x as u8) }
}

impl Add for Player {
    type Output = Player;
    fn add(self, rhs: Self) -> Self::Output { Player(self.0.wrapping_add(rhs.0)) }
}

impl Sub for Player {
    type Output = Player;
    fn sub(self, rhs: Self) -> Self::Output { Player(self.0.wrapping_sub(rhs.0)) }
}

impl Debug for Player {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Player({})", self.to_u8())
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_u8())
    }
}

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

#[cfg(feature = "serde")]
mod player_serde {
    use std::fmt::Formatter;
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

                fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
                    write!(f, "0..=3")
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: Error {
                    if (0..=3).contains(&v) {
                        Ok(Player(u2::new(v as u8)))
                    } else {
                        Err(E::custom("out of range"))
                    }
                }
            }
            deserializer.deserialize_u8(PlayerVisitor)
        }
    }
}

#[cfg(feature = "serde")]
pub use player_serde::*;
