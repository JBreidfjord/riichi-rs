use std::fmt::{Display, Formatter};

use crate::common::tile::Tile;
use crate::common::typedefs::*;
use crate::common::utils::*;
use super::packed::{PackedMeld, PackedMeldKind, normalize_kakan};
use super::Pon;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Kakan {
    pub pon: Pon,
    pub added: Tile,
}

impl Kakan {
    pub fn from_pon_added(pon: Pon, added: Tile) -> Option<Self> {
        if added.to_normal() != pon.called.to_normal() { return None; }
        Some(Kakan { pon, added })
    }
    pub const fn num(self) -> u8 { self.added.normal_num() }
    pub const fn suit(self) -> u8 { self.added.suit() }
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
