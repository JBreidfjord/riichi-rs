//! The wall of tiles (牌山/山/壁牌) and utils.
//!
//! ## How we represent the wall
//!
//! ```ascii_art
//!                          _______________      ______________
//!                         <--- TAIL (CCW) |    / HEAD (CW) --->
//!  self draws |  dora indicators  |rinshan|   |            initial deal             | self draws
//!      118 120|122 124 126 128 130|132 134|   | 0   2   4   6   8  10        48  50 |52  54
//! ... +---+---*---+---+---+---+###*---+---+   +---+---+---+---+---+---+ ... +---+---*---+---+ ...
//!     |#66|#68| D4| D3| D2| D1| D0|RS2|RS0|   |E0 |E2 |S0 |S2 |W0 |W2 |     |E12|W12|#00|#02|      TOP
//! ... +===+===*===+===+===+===+===*===+===+   +===+===+===+===+===+===+ ... +===+===*===+===+ ...
//!     |#67|#69|UD4|UD3|UD2|UD1|UD0|RS3|RS1|   |E1 |E3 |S1 |S3 |W1 |W3 |     |S12|N12|#01|#03|      BOTTOM
//! ... +---+---*---+---+---+---+---*---+---+   +---+---+---+---+---+---+ ... +---+---*---+---+ ...
//!      119 121|123 125 127 129 131|133 135|   | 1   3   5   7   9  11        49  51 |53  55
//!  self draws |ura-dora indicators|rinshan|   |            initial deal             | self draws
//! ```
//!
//! In a physical game, the following procedure is used to prepare the wall:
//!
//! 1.  Shuffle: 136 tiles => 4 sides x 17 stacks x 2 tiles per stack, treated as a ring.
//!
//! 2.  Randomly decide the splitting point on this ring (rules vary on this).
//!
//! 3.  From the splitting point: clockwise => head, counterclockwise => tail.
//!     Now the ring can be treated as a linear 68 x 2 array of tiles.
//!
//! 4.  Reveal the top tile of the 3rd stack from tail; this is the first Dora Indicator.
//!     (figure: `###`)
//!
//! 5.  Initial deal: Players take turns (E->S->W->N->E->...) to draw 2 stacks (= 4 tiles) from the
//!     head until everyone has 12. Each player then draws one more tile.
//!     (figure: `E0`~`E3`, `S0`~`S3`, (...), `W8`~`W11`, `N8`~`N11`; `E12`, `S12`, `W12`, `N12`)
//!
//! 6.  The button player takes his first self-draw and the round starts.
//!     (figure: `#00`)
//!
//! 7.  Additional draw after each Kan is taken from the tail.
//!     (figure: `RS0`, `RS1`, `RS2`, `RS3`)
//!
//! 8.  Additional Dora Indicators are flipped further from tail since the initial one.
//!     (figure: `D1`, `D2`, `D3`, `D4`)
//!
//! In this crate, we assign an index to each of the 136 tiles in the "linear" wall after splitting.
//! The split wall is indexed head-to-tail (major), top-to-bottom (minor). In the figure, this index
//! is annotated as numbers next to the boxes (above/below).
//!
//! ## Ref
//!
//! - <https://ja.wikipedia.org/wiki/配牌>
//! - <https://ja.wikipedia.org/wiki/壁牌>
//! - <https://riichi.wiki/Yama>

use core::fmt::{Display, Formatter};

use crate::{
    tile::Tile,
    tile_set::*,
    player::*,
    variant::*,
};

/// The wall of tiles.
/// See [mod-level docs](self).
///
/// This is a fixed **136-slot buffer** in both [`Variant`]s. A [`Variant::Sanma`] wall fills slots
/// `0..108` and pads the rest with [`SANMA_WALL_SENTINEL_ENCODING`]; the live length is
/// [`Variant::wall_size`]. Keeping the buffer 136 wide is what makes `[Tile; 136]` signatures in
/// downstream crates structural and unaffected by the variant.
pub type Wall = [Tile; 136];

/// Wall with some tiles unknown.
pub type PartialWall = [Option<Tile>; 136];

/// Constructor for an obviously invalid wall. Useful for mutating it later.
pub const fn make_dummy_wall() -> Wall { [Tile::MIN; 136] }

/// Make a sorted wall of the standard 136-tile set, including specified number of red-5's for each
/// (numeral) suit.
pub fn make_sorted_wall(num_reds: [u8; 3]) -> Wall {
    let mut wall = [Tile::MIN; 136];
    for encoding in 0u8..34u8 {
        let tile = Tile::from_encoding(encoding).unwrap();
        let suit = tile.suit();
        let num = tile.num();
        if num == 5 && suit <= 2 {
            for i in 0..num_reds[suit as usize] {
                wall[(encoding * 4 + i) as usize] = tile.to_red();
            }
            for i in num_reds[suit as usize]..4 {
                wall[(encoding * 4 + i) as usize] = tile;
            }
        } else {
            for i in 0..4 {
                wall[(encoding * 4 + i) as usize] = tile;
            }
        }
    }
    wall
}

/// Make sure that a wall is valid --- 34 kinds x 4 each = 136
pub fn is_valid_wall(wall: Wall) -> bool {
    TileSet34::from_iter(wall).into_iter().all(|n| n == 4)
}

/// For each player starting from the button player, which wall tiles to take as the initial draw?
///
/// **[`Variant::Yonma`] only.** Use [`Variant::deal_index`] to get the table for the variant in
/// play; nothing in the engine reads this constant any more. It is retained because it is part of
/// this crate's published surface.
pub const DEAL_INDEX: [[usize; 13]; 4] = [
    [0x00, 0x01, 0x02, 0x03, 0x10, 0x11, 0x12, 0x13, 0x20, 0x21, 0x22, 0x23, 0x30],
    [0x04, 0x05, 0x06, 0x07, 0x14, 0x15, 0x16, 0x17, 0x24, 0x25, 0x26, 0x27, 0x31],
    [0x08, 0x09, 0x0a, 0x0b, 0x18, 0x19, 0x1a, 0x1b, 0x28, 0x29, 0x2a, 0x2b, 0x32],
    [0x0c, 0x0d, 0x0e, 0x0f, 0x1c, 0x1d, 0x1e, 0x1f, 0x2c, 0x2d, 0x2e, 0x2f, 0x33],
];
/// Index of dora indicators in the wall, by their order of revealing first-to-last.
///
/// **[`Variant::Yonma`] only**; see [`Variant::dora_indicator_index`].
pub const DORA_INDICATOR_INDEX: [usize; 5] = [130, 128, 126, 124, 122];
/// Index of ura-dora indicators in the wall; order corresponding to dora indicators.
///
/// **[`Variant::Yonma`] only**; see [`Variant::ura_dora_indicator_index`].
pub const URA_DORA_INDICATOR_INDEX: [usize; 5] = [131, 129, 127, 125, 123];
/// Index of kan draws in the wall, first-to-last.
///
/// **[`Variant::Yonma`] only**; see [`Variant::kan_draw_index`]. Sanma has **8** of these, not 4.
pub const KAN_DRAW_INDEX: [usize; 4] = [134, 135, 132, 133];

/// Total number of draws (front + back) cannot exceed this.
///
/// **[`Variant::Yonma`] only**; see [`Variant::max_num_draws`] (94 in sanma).
pub const MAX_NUM_DRAWS: u8 = 136 - 14;

/// Draws the initial 13 tiles for each of the 4 players, according to standard rules.
/// See [module-level docs](self).
pub fn deal(wall: &Wall, button: Player) -> [TileSet37; 4] {
    let mut hists = [
        TileSet37::default(),
        TileSet37::default(),
        TileSet37::default(),
        TileSet37::default(),
    ];
    for i in 0..4 {
        for wall_index in DEAL_INDEX[i] {
            let p = button.add(Player::new(i as u8));
            hists[p.to_usize()][wall[wall_index].encoding() as usize] += 1;
        }
    }
    hists
}

/// Returns the indexed (0..=4) dora indicator.
pub fn dora_indicator(wall: &Wall, i: usize) -> Tile {
    wall[DORA_INDICATOR_INDEX[i]]
}

/// Returns the indexed (0..=4) ura-dora indicator.
pub fn ura_dora_indicator(wall: &Wall, i: usize) -> Tile {
    wall[URA_DORA_INDICATOR_INDEX[i]]
}

/// Returns the entire dora indicator section of the wall as an array.
/// Note that this does not handle the gradual revealing of dora indicators.
pub fn dora_indicators(wall: &Wall) -> [Tile; 5] {
    DORA_INDICATOR_INDEX.map(|i| wall[i])
}

/// Returns the entire ura-dora indicator section of the wall as an array.
/// Note that this does not handle the final revealing of ura-dora indicators.
pub fn ura_dora_indicators(wall: &Wall) -> [Tile; 5] {
    URA_DORA_INDICATOR_INDEX.map(|i| wall[i])
}

/// Returns the indexed (0..=3) Kan draw.
///
/// **[`Variant::Yonma`] only**; see [`kan_draw_in`].
pub fn kan_draw(wall: &Wall, i: usize) -> Tile {
    wall[KAN_DRAW_INDEX[i]]
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Variant-aware wall access
//
// Every wall read the engine performs goes through one of these. The index bound in `tile_at` is
// the "panicking sentinel" that guards the unused tail of a sanma wall: a `Tile` has no invalid
// representation, so the panic has to live in the accessor rather than in the stored value.
//////////////////////////////////////////////////////////////////////////////////////////////////

/// Reads one wall slot, panicking if the index is past the end of this variant's wall.
///
/// This is the sanma sentinel. Slots `108..136` of a sanma wall hold
/// [`SANMA_WALL_SENTINEL_ENCODING`] (2m --- a tile that cannot exist in a sanma game), but the
/// real guard is here: a mis-derived draw index is a loud panic rather than a plausible tile.
#[track_caller]
pub fn tile_at(variant: Variant, wall: &Wall, index: usize) -> Tile {
    assert!(index < variant.wall_size(),
            "wall slot {} is past the end of a {:?} wall ({} tiles) --- \
             this is the absent-tail sentinel firing",
            index, variant, variant.wall_size());
    wall[index]
}

/// Make a sorted wall for the given variant, including the specified number of red-5's for each
/// (numeral) suit.
///
/// For [`Variant::Yonma`] this is exactly [`make_sorted_wall`]. For [`Variant::Sanma`] the 2m--8m
/// kinds are omitted (and with them red 5m, whose count is ignored), the remaining 108 tiles are
/// packed into slots `0..108`, and the tail is filled with the sentinel.
pub fn make_sorted_wall_in(variant: Variant, num_reds: [u8; 3]) -> Wall {
    if variant == Variant::Yonma {
        return make_sorted_wall(num_reds);
    }
    let sentinel = Tile::from_encoding(SANMA_WALL_SENTINEL_ENCODING).unwrap();
    let mut wall = [sentinel; 136];
    let mut w = 0usize;
    for encoding in 0u8..34u8 {
        if variant.num_copies_34(encoding) == 0 { continue; }
        let tile = Tile::from_encoding(encoding).unwrap();
        let suit = tile.suit();
        let num = tile.num();
        // Red 5m cannot exist in sanma; its requested count is dropped rather than silently
        // shifted onto another suit.
        let reds = if num == 5 && suit <= 2 && variant.has_tile(tile.to_red()) {
            num_reds[suit as usize]
        } else { 0 };
        for i in 0..reds { let _ = i; wall[w] = tile.to_red(); w += 1; }
        for _ in reds..4 { wall[w] = tile; w += 1; }
    }
    debug_assert_eq!(w, variant.wall_size());
    wall
}

/// Make sure that a wall is valid for the given variant: every kind the variant has appears
/// exactly 4 times within the live prefix, and the tail is all sentinel.
pub fn is_valid_wall_in(variant: Variant, wall: Wall) -> bool {
    if variant == Variant::Yonma {
        return is_valid_wall(wall);
    }
    let sentinel = Tile::from_encoding(SANMA_WALL_SENTINEL_ENCODING).unwrap();
    let live = variant.wall_size();
    if wall[live..].iter().any(|t| *t != sentinel) { return false; }
    let counts = TileSet34::from_iter(wall[..live].iter().copied());
    (0u8..34).all(|e| counts[Tile::from_encoding(e).unwrap()] == variant.num_copies_34(e))
}

/// Draws the initial 13 tiles for each **active seat**, according to this variant's deal table.
///
/// The returned array stays 4-wide in both variants; in sanma the **absent seat**'s entry is left
/// empty, which is the one representation that cannot be mistaken for a real (short) hand.
pub fn deal_in(variant: Variant, wall: &Wall, button: Player) -> [TileSet37; 4] {
    let mut hists = [
        TileSet37::default(),
        TileSet37::default(),
        TileSet37::default(),
        TileSet37::default(),
    ];
    let table = variant.deal_index();
    // Walk the seats in *turn order for this variant*, not `button.add(i)`: with three seats the
    // mod-4 step from P2 lands on the absent seat.
    let mut p = button;
    for row in table.iter() {
        assert!(variant.is_seat_active(p),
                "{:?}: deal reached the absent seat --- the button must be active", variant);
        for &wall_index in row {
            hists[p.to_usize()][tile_at(variant, wall, wall_index).encoding() as usize] += 1;
        }
        p = variant.succ(p);
    }
    hists
}

/// Returns the indexed (0..=4) dora indicator for this variant.
pub fn dora_indicator_in(variant: Variant, wall: &Wall, i: usize) -> Tile {
    tile_at(variant, wall, variant.dora_indicator_index()[i])
}

/// Returns the indexed (0..=4) ura-dora indicator for this variant.
pub fn ura_dora_indicator_in(variant: Variant, wall: &Wall, i: usize) -> Tile {
    tile_at(variant, wall, variant.ura_dora_indicator_index()[i])
}

/// Returns the entire dora indicator section for this variant.
/// Note that this does not handle the gradual revealing of dora indicators.
pub fn dora_indicators_in(variant: Variant, wall: &Wall) -> [Tile; 5] {
    let idx = variant.dora_indicator_index();
    [0, 1, 2, 3, 4].map(|i| tile_at(variant, wall, idx[i]))
}

/// Returns the entire ura-dora indicator section for this variant.
/// Note that this does not handle the final revealing of ura-dora indicators.
pub fn ura_dora_indicators_in(variant: Variant, wall: &Wall) -> [Tile; 5] {
    let idx = variant.ura_dora_indicator_index();
    [0, 1, 2, 3, 4].map(|i| tile_at(variant, wall, idx[i]))
}

/// Returns the indexed replacement draw (嶺上牌) for this variant: `0..=3` in yonma, `0..=7` in
/// sanma, where Kans *and* Kita share the sequence.
pub fn kan_draw_in(variant: Variant, wall: &Wall, i: usize) -> Tile {
    let idx = variant.kan_draw_index();
    assert!(i < idx.len(),
            "{:?}: replacement draw #{} does not exist (only {} available)",
            variant, i, idx.len());
    tile_at(variant, wall, idx[i])
}

/// Deduces the set of unknown tiles from the given partially-known wall, and the known total number
/// of red tiles.
///
/// **[`Variant::Yonma`] only**; see [`get_missing_tiles_in_partial_wall_in`]. A sanma wall has no
/// 2m--8m, so assuming a complete 136-tile set here would invent 28 tiles.
///
/// Panics when the partially-known wall is inconsistent with the assumed complete set of tiles. 
pub fn get_missing_tiles_in_partial_wall(partial_wall: &PartialWall, num_reds: [u8; 3]) -> TileSet37 {
    let mut missing = TileSet37::complete_set(num_reds);
    for tile_or_hole in partial_wall {
        if let &Some(tile) = tile_or_hole {
            if missing[tile] == 0 {
                panic!("More {} in the partial wall than expected.", tile)
            }
            missing[tile] -= 1;
        }
    }
    missing
}

/// Deduces the set of unknown tiles from the given partially-known wall, for the given
/// [`Variant`].
///
/// Only the live prefix (`variant.wall_size()`) is consulted; the sanma tail is sentinel padding
/// and is neither a known tile nor a hole to fill.
///
/// Panics when the partially-known wall is inconsistent with the variant's complete tile set.
pub fn get_missing_tiles_in_partial_wall_in(
    variant: Variant, partial_wall: &PartialWall, num_reds: [u8; 3],
) -> TileSet37 {
    let mut missing = TileSet37::complete_set_in(variant, num_reds);
    for tile_or_hole in &partial_wall[..variant.wall_size()] {
        if let &Some(tile) = tile_or_hole {
            if missing[tile] == 0 {
                panic!("More {} in the partial wall than expected for {:?}.", tile, variant)
            }
            missing[tile] -= 1;
        }
    }
    missing
}

/// Combine the partially-known wall and (reordered) unknown tiles to form a fully-known wall,
/// for the given [`Variant`].
///
/// Holes in the live prefix are filled from `missing_tiles`; the tail past `variant.wall_size()`
/// is filled with the sentinel, so a mis-derived draw index still trips [`tile_at`].
pub fn fill_missing_tiles_in_partial_wall_in(
    variant: Variant,
    partial_wall: &PartialWall,
    missing_tiles: impl IntoIterator<Item=Tile>,
) -> Wall {
    let sentinel = Tile::from_encoding(SANMA_WALL_SENTINEL_ENCODING).unwrap();
    let mut missing_iter = missing_tiles.into_iter();
    let live = variant.wall_size();
    let mut wall = [sentinel; 136];
    for i in 0..live {
        wall[i] = partial_wall[i].or_else(|| missing_iter.next())
            .expect("not enough tiles to fill the partial wall");
    }
    wall
}

/// Combine the partially-known wall and (reordered) unknown tiles to form a fully-known wall.
/// 
/// Panics when there are not enough tiles in `missing_tiles` to fill the "holes" in `partial_wall`.
/// 
/// Does not check validity of the resulting wall.
pub fn fill_missing_tiles_in_partial_wall(
    partial_wall: &PartialWall, missing_tiles: impl IntoIterator<Item=Tile>) -> Wall {
    let mut missing_iter = missing_tiles.into_iter();
    partial_wall.map(|tile_or_hole|
        tile_or_hole.or_else(|| missing_iter.next()).unwrap())
}

// Hack to impl `Display` for both `Wall` and `PartialWall`

pub struct WallDisplay<'a>(&'a Wall);
pub trait WallDisplayMethod {
    fn display(&self) -> WallDisplay;
}
impl WallDisplayMethod for Wall {
    fn display(&self) -> WallDisplay { WallDisplay(self) }
}
impl<'a> Display for WallDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for (i, tile) in self.0.iter().enumerate() {
            write!(f, "{} ", tile)?;
            if i % 8 == 7 {
                writeln!(f)?;
            }
        }
        writeln!(f)
    }
}

pub struct PartialWallDisplay<'a>(&'a PartialWall);
pub trait PartialWallDisplayMethod {
    fn display(&self) -> PartialWallDisplay;
}
impl PartialWallDisplayMethod for PartialWall {
    fn display(&self) -> PartialWallDisplay { PartialWallDisplay(self) }
}
impl<'a> Display for PartialWallDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for (i, maybe_tile) in self.0.iter().enumerate() {
            if let Some(tile) = maybe_tile {
                write!(f, "{} ", tile)?;
            } else {
                write!(f, "?? ")?;
            }
            if i % 8 == 7 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::tile::tiles_from_str;

    #[test]
    fn sorted_wall_is_correct() {
        let ans = concat!(
            "111122223333444405556666777788889999m",
            "111122223333444400556666777788889999p",
            "111122223333444455556666777788889999s",
            "1111222233334444555566667777z");
        let wall = make_sorted_wall([1, 2, 0]);
        itertools::assert_equal(wall, tiles_from_str(ans));
        assert!(is_valid_wall(wall));
    }

    #[test]
    fn sanma_sorted_wall_is_valid_and_yonma_is_untouched() {
        // Yonma goes through the historical path byte-for-byte.
        assert_eq!(make_sorted_wall_in(Variant::Yonma, [1, 2, 0]), make_sorted_wall([1, 2, 0]));
        assert!(is_valid_wall_in(Variant::Yonma, make_sorted_wall([1, 1, 1])));

        let v = Variant::Sanma;
        let wall = make_sorted_wall_in(v, [1, 1, 1]);
        assert!(is_valid_wall_in(v, wall));
        // A yonma wall is not a valid sanma wall, and vice versa.
        assert!(!is_valid_wall_in(v, make_sorted_wall([1, 1, 1])));
        assert!(!is_valid_wall(wall));
        // 2m--8m and red 5m are absent; everything else is present 4 times.
        let counts = TileSet37::from_iter(wall[..v.wall_size()].iter().copied());
        for e in 1..=7u8 { assert_eq!(counts[Tile::from_encoding(e).unwrap()], 0); }
        assert_eq!(counts[Tile::from_encoding(34).unwrap()], 0, "no red 5m in sanma");
        assert_eq!(counts[Tile::from_encoding(35).unwrap()], 1, "red 5p survives");
    }

    #[test]
    #[should_panic(expected = "absent-tail sentinel")]
    fn sanma_wall_tail_panics_if_drawn() {
        let wall = make_sorted_wall_in(Variant::Sanma, [1, 1, 1]);
        let _ = tile_at(Variant::Sanma, &wall, 108);
    }

    #[test]
    fn sanma_deal_leaves_the_absent_seat_empty() {
        let v = Variant::Sanma;
        let wall = make_sorted_wall_in(v, [1, 1, 1]);
        for button in [P0, P1, P2] {
            let hands = deal_in(v, &wall, button);
            for p in v.active_seats() {
                assert_eq!(hands[p.to_usize()].0.iter().map(|&n| n as u32).sum::<u32>(), 13);
            }
            let absent = v.absent_seat().unwrap();
            assert_eq!(hands[absent.to_usize()].0.iter().map(|&n| n as u32).sum::<u32>(), 0);
        }
        // Yonma deal is unchanged.
        let ywall = make_sorted_wall([1, 1, 1]);
        assert_eq!(deal_in(Variant::Yonma, &ywall, P1), deal(&ywall, P1));
    }

    #[test]
    fn yonma_accessors_agree_with_the_historical_consts() {
        let wall = make_sorted_wall([1, 1, 1]);
        let v = Variant::Yonma;
        assert_eq!(v.max_num_draws(), MAX_NUM_DRAWS);
        assert_eq!(v.dora_indicator_index(), &DORA_INDICATOR_INDEX);
        assert_eq!(v.ura_dora_indicator_index(), &URA_DORA_INDICATOR_INDEX);
        assert_eq!(v.kan_draw_index(), &KAN_DRAW_INDEX);
        assert_eq!(v.deal_index(), &DEAL_INDEX);
        assert_eq!(dora_indicators_in(v, &wall), dora_indicators(&wall));
        assert_eq!(ura_dora_indicators_in(v, &wall), ura_dora_indicators(&wall));
        for i in 0..4 { assert_eq!(kan_draw_in(v, &wall, i), kan_draw(&wall, i)); }
        for i in 0..5 {
            assert_eq!(dora_indicator_in(v, &wall, i), dora_indicator(&wall, i));
            assert_eq!(ura_dora_indicator_in(v, &wall, i), ura_dora_indicator(&wall, i));
        }
    }

    #[test]
    fn sorted_wall_deals_correctly() {
        let wall = make_sorted_wall([1, 1, 1]);
        assert_eq!(deal(&wall, P1), [
            TileSet37::new([
                0, 0, 0, 4, 0, 0, 0, 4, 0,
                0, 0, 4, 1, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0,
                0, 0, 0,
            ]),  // N: 4444m 8888m 3333p 4p
            TileSet37::new([
                4, 0, 0, 0, 3, 0, 0, 0, 4,
                0, 0, 0, 1, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0,
                1, 0, 0,
            ]),  // E: 1111m 0555m 9999m 4p
            TileSet37::new([
                0, 4, 0, 0, 0, 4, 0, 0, 0,
                4, 0, 0, 1, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0,
                0, 0, 0,
            ]),  // S: 2222m 6666m 1111p 4p
            TileSet37::new([
                0, 0, 4, 0, 0, 0, 4, 0, 0,
                0, 4, 0, 1, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0,
                0, 0, 0,
            ]),  // W: 3333m 7777m 2222p 4p
        ]);
    }
}
