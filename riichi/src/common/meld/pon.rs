use std::fmt::{Display, Formatter};

use crate::common::*;
use super::packed::*;

/// An open group of 3 identical (ignoring red) tiles (ポン / 明刻).
/// May be called from any other player's discard.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Pon {
    /// The calling player's own 2 tiles.
    pub own: [Tile; 2],

    /// The called tile.
    pub called: Tile,

    /// (discarding player - self) mod 4
    pub dir: Player,
}

impl Pon {
    pub const fn num(self) -> u8 { self.called.normal_num() }
    pub const fn suit(self) -> u8 { self.called.suit() }

    /// Constructs from own tiles, called tile, and the relative pos of the discarding player.
    pub fn from_tiles_dir(own0: Tile, own1: Tile, called: Tile, dir: Player) -> Option<Self> {
        if own0.to_normal() != called.to_normal() ||
            own1.to_normal() != called.to_normal() ||
            dir.to_u8() == 0 { return None; }
        let (own0, own1) = sort2(own0, own1);
        Some(Pon { own: [own0, own1], called, dir })
    }

    /// Checks whether own tiles exist in player's closed hand.
    pub fn is_in_hand(self, hand: &TileSet37) -> bool {
        if self.own[0] != self.own[1] { hand[self.own[0]] >= 1 && hand[self.own[1]] >= 1 }
        else { hand[self.own[0]] >= 2 }
    }

    /// Removes own tiles from player's closed hand (assuming they exist).
    pub fn consume_from_hand(self, hand: &mut TileSet37) {
        hand[self.own[0]] -= 1;
        hand[self.own[1]] -= 1;
    }
}

impl Display for Pon {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (n0, n1, nc, s) = (
            self.own[0].num(),
            self.own[1].num(),
            self.called.num(),
            self.called.suit_char(),
        );
        match self.dir.to_u8() {
            1 => write!(f, "{}{}P{}{}", n0, n1, nc, s),
            2 => write!(f, "{}P{}{}{}", n0, nc, n1, s),
            3 => write!(f, "P{}{}{}{}", nc, n0, n1, s),
            _ => Err(std::fmt::Error::default()),
        }
    }
}

impl TryFrom<PackedMeld> for Pon {
    type Error = ();

    fn try_from(raw: PackedMeld) -> Result<Self, Self::Error> {
        if raw.kind() != u8::from(PackedMeldKind::Pon) { return Err(()); }
        let t = raw.get_tile().ok_or(())?;
        let (mut own0, mut own1, mut called) = (t, t, t);
        let (r0, r1, r2, _) = unpack4(normalize_pon(raw.red()));
        if r0 { own0 = own0.to_red(); }
        if r1 { own1 = own1.to_red(); }
        if r2 { called = called.to_red(); }
        Pon::from_tiles_dir(own0, own1, called, Player::new(raw.dir())).ok_or(())
    }
}

impl From<Pon> for PackedMeld {
    fn from(pon: Pon) -> Self {
        let [own0, own1] = pon.own;
        PackedMeld::new()
            .with_tile(own0.normal_encoding())
            .with_dir(pon.dir.to_u8())
            .with_red(pack4(own0.is_red(),
                            own1.is_red(),
                            pon.called.is_red(),
                            false))
            .with_kind(PackedMeldKind::Pon.into())
    }
}
