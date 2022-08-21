use std::fmt::{Display, Formatter};

use crate::common::tile::Tile;
use crate::common::typedefs::*;
use crate::common::utils::*;
use super::packed::{PackedMeld, PackedMeldKind, normalize_daiminkan};

/// "Big Open Kan" formed by calling 1 with 3 of the same kind in the closed hand (大明槓).
/// Similar to [Pon](super::Pon), may be called from any other player's discard.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Daiminkan {
    /// The calling player's own 3 tiles.
    pub own: [Tile; 3],

    /// The called tile.
    pub called: Tile,

    /// (discarding player - self) mod 4
    pub dir: Player,
}

impl Daiminkan {
    pub fn from_tiles_dir(own0: Tile, own1: Tile, own2: Tile, called: Tile, dir: Player) -> Option<Self> {
        if own0.to_normal() != called.to_normal() ||
            own1.to_normal() != called.to_normal() ||
            own2.to_normal() != called.to_normal() ||
            dir.to_u8() == 0 { return None; }
        let (own0, own1, own2) = sort3(own0, own1, own2);
        Some(Daiminkan { own: [own0, own1, own2], called, dir })
    }
    pub const fn num(self) -> u8 { self.called.normal_num() }
    pub const fn suit(self) -> u8 { self.called.suit() }
}

impl Display for Daiminkan {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (n0, n1, n2, nc, s) = (
            self.own[0].num(),
            self.own[1].num(),
            self.own[2].num(),
            self.called.num(),
            self.called.suit_char(),
        );
        match self.dir.to_u8() {
            1 => write!(f, "{}{}{}D{}{}", n0, n1, n2, nc, s),
            2 => write!(f, "{}D{}{}{}{}", n0, nc, n1, n2, s),
            3 => write!(f, "D{}{}{}{}{}", nc, n0, n1, n2, s),
            _ => Err(std::fmt::Error::default()),
        }
    }
}

impl TryFrom<PackedMeld> for Daiminkan {
    type Error = ();

    fn try_from(raw: PackedMeld) -> Result<Self, Self::Error> {
        if raw.kind() != PackedMeldKind::Daiminkan.into() { return Err(()); }
        let t = raw.get_tile().ok_or(())?;
        let (mut own0, mut own1, mut own2, mut called) = (t, t, t, t);
        let (r0, r1, r2, r3) = unpack4(normalize_daiminkan(raw.red()));
        if r0 { own0 = own0.to_red(); }
        if r1 { own1 = own1.to_red(); }
        if r2 { own2 = own2.to_red(); }
        if r3 { called = called.to_red(); }
        Daiminkan::from_tiles_dir(
            own0, own1, own2, called, Player::new(raw.dir())).ok_or(())
    }
}

impl From<Daiminkan> for PackedMeld {
    fn from(daiminkan: Daiminkan) -> Self {
        let [own0, own1, own2] = daiminkan.own;
        PackedMeld::new()
            .with_tile(own0.normal_encoding())
            .with_dir(daiminkan.dir.to_u8())
            .with_red(pack4(own0.is_red(),
                            own1.is_red(),
                            own2.is_red(),
                            daiminkan.called.is_red()))
            .with_kind(PackedMeldKind::Daiminkan.into())
    }
}
