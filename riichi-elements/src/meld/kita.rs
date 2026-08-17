use core::fmt::{Display, Formatter};

use crate::{
    tile::Tile,
    tile_set::*,
};

use super::packed::{PackedMeld, PackedMeldKind};

/// The North tile (北), 4z --- the only tile a [`Kita`] can ever set aside.
pub const NORTH: Tile = match Tile::from_encoding(30) {
    Some(t) => t,
    None => panic!("4z must be a valid tile encoding"),
};

/// Kita (北抜き) --- the North extraction, a sanma-only turn action that sets a North tile aside
/// as a nuki-dora (抜きドラ) and draws a replacement from the tail of the wall (嶺上牌).
///
/// # Why this is a [`Meld`](super::Meld) and not a flagged Discard
///
/// It matches Tenhou's own mjlog encoding (a nuki is an `<N>` meld), so it lands in the per-seat
/// meld list that the wire format and the observation encoder already carry. Modelling it as a
/// discard was rejected: it would make the extracting player Furiten on North, which Tenhou
/// explicitly rules out --- 「フリテンは河に捨てられた牌でのみ判定(加槓・**抜き**でさらした非純手牌
/// を除く)」 ("furiten is judged only on tiles discarded into the river, excluding non-concealed
/// tiles exposed via added-kan or *nuki*").
///
/// # What it is *not*
///
/// An extracted North is **not part of the winning hand shape** and contributes **no fu**
/// (「何枚使っても0符で、和了形にはカウントされず」). It contributes exactly `+1` han each at win
/// time, for the extractor only, stacking with an ordinary West dora indicator. It also does not
/// open the hand: riichi remains available, so [`Meld::is_closed`](super::Meld::is_closed) is
/// `true` for a Kita.
///
/// A ron on the exposed North (搶北) is legal for any yaku-bearing hand, but grants **no**
/// [Chankan] --- 搶北 falls outside the definition of 搶槓.
///
/// [Chankan]: https://ja.wikipedia.org/wiki/搶槓
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Kita {
    /// The extracted tile. Always [`NORTH`] --- there is no red North --- but carried explicitly
    /// so that `Meld`'s accessors and the packed representation stay uniform.
    pub tile: Tile,
}

impl Kita {
    /// The one and only Kita.
    pub const fn new() -> Self { Kita { tile: NORTH } }

    pub const fn num(self) -> u8 { self.tile.normal_num() }
    pub const fn suit(self) -> u8 { self.tile.suit() }

    /// Constructs from the extracted tile; `None` unless it is a North.
    pub fn from_tile(tile: Tile) -> Option<Self> {
        if tile.to_normal() == NORTH { Some(Kita { tile: NORTH }) } else { None }
    }

    /// Constructs from the closed hand, if it holds at least one North.
    ///
    /// Kita takes **any** North in the concealed hand, not only a just-drawn one: of 23,104
    /// extractions in the houou 3p sample, only 10,499 followed the extractor's own North draw.
    /// Callers that want to extract a just-drawn North must merge the draw into the hand first,
    /// exactly as they do for Ankan and Kakan.
    pub fn from_hand(hand: &TileSet37) -> Option<Self> {
        if hand[NORTH] > 0 { Some(Kita::new()) } else { None }
    }

    /// Removes the extracted North from the hand.
    pub fn consume_from_hand(self, hand: &mut TileSet37) {
        hand[NORTH] -= 1;
    }
}

impl Default for Kita {
    fn default() -> Self { Self::new() }
}

impl Display for Kita {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "N{}{}", self.tile.num(), self.tile.suit_char())
    }
}

// Parse from the unpacked bitfields
impl TryFrom<PackedMeld> for Kita {
    type Error = ();

    fn try_from(raw: PackedMeld) -> Result<Self, Self::Error> {
        if raw.kind() != PackedMeldKind::Kita as u8 {
            return Err(());
        }
        let t = raw.get_tile().ok_or(())?;
        Kita::from_tile(t).ok_or(())
    }
}

impl From<Kita> for PackedMeld {
    fn from(kita: Kita) -> Self {
        PackedMeld::new()
            .with_tile(kita.tile.normal_encoding())
            .with_dir(0)
            .with_red(0)
            .with_kind(PackedMeldKind::Kita as u8)
    }
}
