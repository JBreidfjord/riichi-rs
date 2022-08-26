use std::fmt::{Display, Formatter};

use crate::common::Tile;
use crate::common::TileSet37;
use crate::common::typedefs::*;
use crate::common::utils::*;
use crate::count_for_kan;
use super::packed::{PackedMeld, PackedMeldKind, normalize_kakan};
use super::Pon;

/// A Kan formed by existing [Pon](super::Pon) + the 1 last identical tile from closed hand
/// (加槓 / 小明槓). This can be formed when the owner of the [Pon](super::Pon) is in action.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Kakan {
    /// The original Pon.
    pub pon: Pon,

    /// The extra tile from the player's closed hand.
    pub added: Tile,
}

impl Kakan {
    pub const fn num(self) -> u8 { self.added.normal_num() }
    pub const fn suit(self) -> u8 { self.added.suit() }

    /// Constructs from an existing Pon and the (last) added tile.
    pub fn from_pon_added(pon: Pon, added: Tile) -> Option<Self> {
        if added.to_normal() != pon.called.to_normal() { return None; }
        Some(Kakan { pon, added })
    }

    /// Constructs from an existing Pon and the closed hand.
    /// If the closed hand does not have the last remaining tile, returns `None`.
    pub fn from_pon_hand(pon: Pon, hand: &TileSet37) -> Option<Self> {
        let added = pon.called.to_normal();
        let (num_normal, num_red) = count_for_kan(hand, added);
        match (num_normal, num_red) {
            (1, 0) => Some(Kakan { pon, added }),
            (0, 1) => Some(Kakan { pon, added: added.to_red() }),
            _ => None,
        }
    }

    /// Removes the added tile from the hand (where this was constructed from).
    pub fn consume_from_hand(self, hand: &mut TileSet37) {
        hand[self.added] -= 1;
    }
}

impl Display for Kakan {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (n0, n1, nc, na, suit) = (
            self.pon.own[0].num(),
            self.pon.own[1].num(),
            self.pon.called.num(),
            self.added.num(),
            self.added.suit_char(),
        );
        match self.pon.dir.to_u8() {
            1 => write!(f, "{}{}K({}/{}){}", n0, n1, na, nc, suit),
            2 => write!(f, "{}K({}/{}){}{}", n0, na, nc, n1, suit),
            3 => write!(f, "K({}/{}){}{}{}", na, nc, n0, n1, suit),
            _ => Err(std::fmt::Error::default()),
        }
    }
}

impl TryFrom<PackedMeld> for Kakan {
    type Error = ();

    fn try_from(raw: PackedMeld) -> Result<Self, Self::Error> {
        if raw.kind() != PackedMeldKind::Kakan.into() { return Err(()); }
        let t = raw.get_tile().ok_or(())?;
        let (mut own0, mut own1, mut called, mut added) = (t, t, t, t);
        let (r0, r1, r2, r3) = unpack4(normalize_kakan(raw.red()));
        if r0 { own0 = own0.to_red(); }
        if r1 { own1 = own1.to_red(); }
        if r2 { called = called.to_red(); }
        if r3 { added = added.to_red(); }
        let pon = Pon::from_tiles_dir(own0, own1, called, Player::new(raw.dir()))
            .ok_or(())?;
        Kakan::from_pon_added(pon, added).ok_or(())
    }
}

impl From<Kakan> for PackedMeld {
    fn from(kakan: Kakan) -> Self {
        let [own0, own1] = kakan.pon.own;
        PackedMeld::new()
            .with_tile(own0.normal_encoding())
            .with_dir(kakan.pon.dir.to_u8())
            .with_red(pack4(own0.is_red(),
                            own1.is_red(),
                            kakan.pon.called.is_red(),
                            kakan.added.is_red()))
            .with_kind(PackedMeldKind::Kakan.into())
    }
}
