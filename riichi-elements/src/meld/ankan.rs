use core::fmt::{Display, Formatter};

use crate::{
    tile::Tile,
    tile_set::*,
    utils::{pack4, unpack4},
};

use super::{
    packed::{normalize_ankan, PackedMeld, PackedMeldKind},
    utils::{ankan_tiles, count_for_kan},
};

/// Closed Kan, formed by setting aside 4 tiles of the same kind in a player's closed hand (暗槓).
/// This can be done during this player's own turn.
///
/// Declaring Ankan does not _technically_ open one's hand, although it _is_ revealed to others.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Ankan {
    pub own: [Tile; 4],
}

impl Ankan {
    pub const fn num(self) -> u8 {
        self.own[0].normal_num()
    }
    pub const fn suit(self) -> u8 {
        self.own[0].suit()
    }

    /// Constructs from 4 own tiles.
    pub fn from_tiles(mut own: [Tile; 4]) -> Option<Self> {
        if own[0].to_normal() != own[1].to_normal()
            || own[0].to_normal() != own[2].to_normal()
            || own[0].to_normal() != own[3].to_normal()
        {
            return None;
        }
        own.sort();
        Some(Ankan { own })
    }

    /// Constructs from the closed hand for the specified tile.
    pub fn from_hand(hand: &TileSet37, tile: Tile) -> Option<Self> {
        let normal = tile.to_normal();
        let (num_normal, num_red) = count_for_kan(hand, normal);
        if num_normal + num_red != 4 {
            return None;
        }
        Self::from_tiles(ankan_tiles(normal, num_red))
    }

    /// Removes all own tiles from the hand (where this was constructed from).
    pub fn consume_from_hand(self, hand: &mut TileSet37) {
        hand[self.own[0]] = 0;
        hand[self.own[3]] = 0;
    }
}

impl Display for Ankan {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let (n0, n1, n2, n3, s) = (
            self.own[0].num(),
            self.own[1].num(),
            self.own[2].num(),
            self.own[3].num(),
            self.own[0].suit_char(),
        );
        write!(f, "A{}{}{}{}{}", n0, n1, n2, n3, s)
    }
}

// Parse from the unpacked bitfields
impl TryFrom<PackedMeld> for Ankan {
    type Error = ();

    fn try_from(raw: PackedMeld) -> Result<Self, Self::Error> {
        if raw.kind() != PackedMeldKind::Ankan as u8 {
            return Err(());
        }
        let t = raw.get_tile().ok_or(())?;
        let (mut own0, mut own1, mut own2, mut own3) = (t, t, t, t);
        let (r0, r1, r2, r3) = unpack4(normalize_ankan(raw.red()));
        if r0 { own0 = own0.to_red(); }
        if r1 { own1 = own1.to_red(); }
        if r2 { own2 = own2.to_red(); }
        if r3 { own3 = own3.to_red(); }
        Ankan::from_tiles([own0, own1, own2, own3]).ok_or(())
    }
}

impl From<Ankan> for PackedMeld {
    fn from(ankan: Ankan) -> Self {
        let [own0, own1, own2, own3] = ankan.own;
        PackedMeld::new()
            .with_tile(own0.normal_encoding())
            .with_dir(0)
            .with_red(pack4(own0.is_red(), own1.is_red(), own2.is_red(), own3.is_red()))
            .with_kind(PackedMeldKind::Ankan as u8)
    }
}
