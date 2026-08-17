//! [`Variant`] --- which game is being played: [Yonma] (4-player) or [Sanma] (3-player).
//!
//! [Yonma]: Variant::Yonma
//! [Sanma]: Variant::Sanma
//!
//! # Why this lives in `riichi-elements`
//!
//! `riichi` depends on `riichi-elements` and never the reverse. [`Wall`](crate::wall::Wall),
//! [`Player`](crate::player::Player) and [`Meld`](crate::meld::Meld) are all in this crate, so a
//! variant flag living next to `Ruleset` (in the upper crate) could not be consulted by wall
//! construction or by [`Meld::Kita`](crate::meld::Meld::Kita).
//!
//! # Why one enum and not a bag of flags
//!
//! Every rule difference between the two games is *derived* from this one value rather than
//! configured independently, so a "half-sanma" ruleset --- three players with Chii legal, say ---
//! is not representable. Independent flags (`num_players` + `allow_chii` + ...) were rejected
//! because nothing would reject the illegal combinations.
//!
//! # The absent seat
//!
//! Sanma is modelled as **four seats with one absent**, not as three seats. Seat 3 is the
//! **absent seat**: present in every 4-wide array, never dealt, never acting, never the button,
//! holding 0 points, and excluded from every points scan. See [`Variant::is_seat_active`].
//!
//! This is an invariant the type system cannot enforce --- a missed active-seat filter is a
//! silent wrong answer, not a compile error. Where a scan over all seats has an identity element,
//! the absent seat holds it and the scan needs no special case; where none exists, callers must
//! ask for [`Variant::active_seats`] explicitly.

use crate::{
    player::*,
    tile::Tile,
    typedefs::GamePoints,
};

/// Which game is being played.
///
/// See [module-level docs](self).
///
/// ## Optional `serde` support
///
/// As a string (`"Yonma"` or `"Sanma"`).
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[derive(strum::IntoStaticStr, strum::EnumString, strum::Display)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Variant {
    /// 4-player riichi (四麻). The default.
    #[default]
    Yonma = 0,

    /// 3-player riichi (三麻), Tenhou-standard: no 2m--8m, no Chii, North is a nuki-dora,
    /// pure tsumo loss, and one absent seat.
    Sanma = 1,
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Seats
//////////////////////////////////////////////////////////////////////////////////////////////////

/// The seats a [`Variant::Yonma`] game plays: all four.
pub const YONMA_ACTIVE_SEATS: &[Player] = &[P0, P1, P2, P3];

/// The seats a [`Variant::Sanma`] game plays: 0--2. Seat 3 is the **absent seat**.
pub const SANMA_ACTIVE_SEATS: &[Player] = &[P0, P1, P2];

/// The **absent seat** in [`Variant::Sanma`]: seat 3.
///
/// Seat 3 rather than any other seat, because it is then never the button under the
/// kyoku-numbering scheme this crate uses (see [`Variant::next_kyoku`]), which is what lets
/// `RoundId::prevailing_wind` and `RoundId::button` stay untouched.
pub const ABSENT_SEAT: Player = P3;

impl Variant {
    /// How many seats actually play.
    pub const fn num_players(self) -> u8 {
        match self {
            Variant::Yonma => 4,
            Variant::Sanma => 3,
        }
    }

    /// Does this seat play at all?
    ///
    /// Always `true` in [`Variant::Yonma`]. In [`Variant::Sanma`], `false` for the
    /// **absent seat** ([`ABSENT_SEAT`]) only.
    ///
    /// Every points scan --- buttobi, placement, noten-bappu, pot, ranks --- must filter through
    /// this (or rely on the absent seat holding a scan's identity element).
    pub const fn is_seat_active(self, player: Player) -> bool {
        match self {
            Variant::Yonma => true,
            Variant::Sanma => player.to_u8() != ABSENT_SEAT.to_u8(),
        }
    }

    /// The seats that actually play, in natural turn order from seat 0.
    pub const fn active_seats(self) -> &'static [Player] {
        match self {
            Variant::Yonma => YONMA_ACTIVE_SEATS,
            Variant::Sanma => SANMA_ACTIVE_SEATS,
        }
    }

    /// The **absent seat**, if this variant has one.
    pub const fn absent_seat(self) -> Option<Player> {
        match self {
            Variant::Yonma => None,
            Variant::Sanma => Some(ABSENT_SEAT),
        }
    }

    /// The next seat to act after `player`, skipping the **absent seat**.
    ///
    /// In [`Variant::Sanma`] this is `P2 -> P0` (never `P2 -> P3`).
    pub const fn succ(self, player: Player) -> Player {
        let next = player.succ();
        if self.is_seat_active(next) { next } else { next.succ() }
    }

    /// The active seats after `player` in natural turn order, i.e. everyone who may react to
    /// `player`'s action. Length is `num_players() - 1`.
    ///
    /// This is the active-seat replacement for
    /// [`other_players_after`](crate::player::other_players_after).
    pub fn other_active_players_after(self, player: Player) -> impl Iterator<Item = Player> {
        let n = self.num_players();
        (1..n).map(move |i| {
            // Walk forward `i` *active* steps from `player`.
            let mut p = player;
            for _ in 0..i {
                p = self.succ(p);
            }
            p
        })
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Calls
//////////////////////////////////////////////////////////////////////////////////////////////////

impl Variant {
    /// Is Chii (チー) a legal call?
    ///
    /// Sanma: no (「チーはできない」). Note that sequences remain legal *in the hand* --- only the
    /// call is removed. (With 2m--8m absent, manzu cannot form a run at all, so Sanshoku Doujun
    /// simply never occurs.)
    pub const fn allows_chii(self) -> bool {
        matches!(self, Variant::Yonma)
    }

    /// Is Kita (北抜き) --- the North extraction --- a legal turn action?
    pub const fn allows_kita(self) -> bool {
        matches!(self, Variant::Sanma)
    }

    /// How many Kita extractions can happen in one round, across all seats.
    ///
    /// Bounded by the number of North tiles in the wall.
    pub const fn max_num_kita(self) -> u8 {
        match self {
            Variant::Yonma => 0,
            Variant::Sanma => 4,
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Tiles
//////////////////////////////////////////////////////////////////////////////////////////////////

/// The tile filling the unused tail of a [`Variant::Sanma`] wall buffer.
///
/// The wall stays a 136-slot buffer in both variants (so `[Tile; 136]` signatures across
/// downstream crates are structural and unaffected); sanma fills slots 0..108 and this value pads
/// the rest.
///
/// It is deliberately **2m**, a tile that cannot legally exist anywhere in a sanma game: if a
/// sentinel ever leaks into a hand, the sanma tile-legality checks trip on it too. The primary
/// guard is nevertheless the index bound in [`Variant::wall_size`] --- see
/// [`crate::wall::tile_at`], which panics rather than returning a sentinel.
pub const SANMA_WALL_SENTINEL_ENCODING: u8 = 1;

impl Variant {
    /// How many tiles are actually in the wall: 136, or 108 for sanma (2m--8m removed).
    ///
    /// The wall *buffer* is 136 slots in both variants; this is how much of it is live.
    pub const fn wall_size(self) -> usize {
        match self {
            Variant::Yonma => 136,
            Variant::Sanma => 108,
        }
    }

    /// Does this tile kind exist in this variant at all?
    ///
    /// Sanma removes 2m--8m, and with them red 5m.
    pub const fn has_tile(self, tile: Tile) -> bool {
        match self {
            Variant::Yonma => true,
            Variant::Sanma => {
                let e = tile.encoding();
                // 1..=7 are 2m..8m; 34 is red 5m.
                !(e >= 1 && e <= 7) && e != 34
            }
        }
    }

    /// How many copies of the given 34-encoded tile kind the full wall holds: 4, or 0 for a kind
    /// this variant does not have.
    ///
    /// Red-5 accounting is *not* included here --- the number of red tiles is implied by the wall
    /// array, not by the variant. Callers that need a copies table split by normal/red must
    /// subtract their own red counts, exactly as they do in yonma.
    pub const fn num_copies_34(self, encoding_34: u8) -> u8 {
        match self {
            Variant::Yonma => 4,
            Variant::Sanma => {
                if encoding_34 >= 1 && encoding_34 <= 7 { 0 } else { 4 }
            }
        }
    }

    /// The dora indicated by the given indicator, under this variant's chain.
    ///
    /// Yonma delegates to [`Tile::indicated_dora`] unchanged.
    ///
    /// Sanma differs in manzu only: with 2m--8m absent the chain is **1m <-> 9m**
    /// (「ドラ表示「一萬」のドラは「九萬」、「九萬」のドラは「一萬」」), where
    /// [`Tile::indicated_dora`]'s `n % 9 + 1` would wrongly say 1m -> 2m.
    pub fn indicated_dora(self, indicator: Tile) -> Tile {
        match self {
            Variant::Yonma => indicator.indicated_dora(),
            Variant::Sanma => match indicator.to_normal().encoding() {
                // 1m -> 9m, 9m -> 1m. 2m..8m cannot be an indicator in sanma (they are not in
                // the wall), so the rest of the manzu chain is unreachable and left alone.
                0 => Tile::from_encoding(8).unwrap(),
                8 => Tile::from_encoding(0).unwrap(),
                _ => indicator.indicated_dora(),
            },
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Wall geometry
//
// These replace what used to be `pub const`s in `crate::wall`. They are deliberately *data*: the
// exact sanma dead-wall layout is derived below from the physical procedure rather than taken from
// a primary source, so it should be adjustable without a redesign.
//////////////////////////////////////////////////////////////////////////////////////////////////

/// For each player starting from the button player, which wall tiles to take as the initial draw?
/// (Yonma: 13 x 4 = 52 tiles.)
pub const YONMA_DEAL_INDEX: &[[usize; 13]] = &[
    [0x00, 0x01, 0x02, 0x03, 0x10, 0x11, 0x12, 0x13, 0x20, 0x21, 0x22, 0x23, 0x30],
    [0x04, 0x05, 0x06, 0x07, 0x14, 0x15, 0x16, 0x17, 0x24, 0x25, 0x26, 0x27, 0x31],
    [0x08, 0x09, 0x0a, 0x0b, 0x18, 0x19, 0x1a, 0x1b, 0x28, 0x29, 0x2a, 0x2b, 0x32],
    [0x0c, 0x0d, 0x0e, 0x0f, 0x1c, 0x1d, 0x1e, 0x1f, 0x2c, 0x2d, 0x2e, 0x2f, 0x33],
];

/// Sanma initial deal: the same 4-4-4-1 taking order with one fewer seat, so 13 x 3 = 39 tiles
/// from slots 0..39. The **absent seat** is not dealt to at all.
pub const SANMA_DEAL_INDEX: &[[usize; 13]] = &[
    [0, 1, 2, 3, 12, 13, 14, 15, 24, 25, 26, 27, 36],
    [4, 5, 6, 7, 16, 17, 18, 19, 28, 29, 30, 31, 37],
    [8, 9, 10, 11, 20, 21, 22, 23, 32, 33, 34, 35, 38],
];

/// Yonma dora indicators, in order of revealing first-to-last: tops of the 3rd..7th stack
/// counted from the tail.
pub const YONMA_DORA_INDICATOR_INDEX: &[usize] = &[130, 128, 126, 124, 122];
/// Yonma ura-dora indicators; order corresponds to [`YONMA_DORA_INDICATOR_INDEX`].
pub const YONMA_URA_DORA_INDICATOR_INDEX: &[usize] = &[131, 129, 127, 125, 123];
/// Yonma Kan replacement draws, first-to-last: the last two stacks from the tail, top then bottom
/// within each stack.
pub const YONMA_KAN_DRAW_INDEX: &[usize] = &[134, 135, 132, 133];

/// Sanma dora indicators: same construction as yonma, but the replacement section is twice as
/// wide (8 tiles, not 4), so the indicators start one stack further from the tail --- tops of the
/// 5th..9th stack counted from the tail of the 108-tile wall.
pub const SANMA_DORA_INDICATOR_INDEX: &[usize] = &[98, 96, 94, 92, 90];
/// Sanma ura-dora indicators; order corresponds to [`SANMA_DORA_INDICATOR_INDEX`].
pub const SANMA_URA_DORA_INDICATOR_INDEX: &[usize] = &[99, 97, 95, 93, 91];
/// Sanma replacement draws, first-to-last: **8** of them
/// (「嶺上牌は8枚」) --- up to 4 Kans plus up to 4 Kita all draw from the tail.
pub const SANMA_KAN_DRAW_INDEX: &[usize] = &[106, 107, 104, 105, 102, 103, 100, 101];

impl Variant {
    /// Which wall slots each seat takes as its initial deal, indexed from the button player.
    ///
    /// Length is [`Self::num_players`]; the **absent seat** has no entry, so this array is
    /// *not* 4-wide in sanma. This is deliberate: an absent seat with an empty deal would be a
    /// silently-13-tile-short hand rather than no hand at all.
    pub const fn deal_index(self) -> &'static [[usize; 13]] {
        match self {
            Variant::Yonma => YONMA_DEAL_INDEX,
            Variant::Sanma => SANMA_DEAL_INDEX,
        }
    }

    /// Wall slots holding the dora indicators, in order of revealing.
    pub const fn dora_indicator_index(self) -> &'static [usize] {
        match self {
            Variant::Yonma => YONMA_DORA_INDICATOR_INDEX,
            Variant::Sanma => SANMA_DORA_INDICATOR_INDEX,
        }
    }

    /// Wall slots holding the ura-dora indicators; order corresponds to
    /// [`Self::dora_indicator_index`].
    pub const fn ura_dora_indicator_index(self) -> &'static [usize] {
        match self {
            Variant::Yonma => YONMA_URA_DORA_INDICATOR_INDEX,
            Variant::Sanma => SANMA_URA_DORA_INDICATOR_INDEX,
        }
    }

    /// Wall slots holding the replacement draws taken from the tail, first-to-last.
    ///
    /// Yonma: 4 (one per possible Kan). Sanma: 8 --- Kans *and* Kita both draw here.
    pub const fn kan_draw_index(self) -> &'static [usize] {
        match self {
            Variant::Yonma => YONMA_KAN_DRAW_INDEX,
            Variant::Sanma => SANMA_KAN_DRAW_INDEX,
        }
    }

    /// How many tiles the dead wall (王牌) holds. 14 in both variants
    /// (「王牌は常に１４枚残し」): each replacement draw is repaid from the tail of the live wall,
    /// which is why the live wall shortens by one per Kan *or* Kita.
    pub const fn dead_wall_size(self) -> usize { 14 }

    /// How many tiles are dealt in total: 13 per playing seat.
    pub const fn num_dealt(self) -> u8 { 13 * self.num_players() }

    /// Total tiles that may be consumed from the wall (deal + head draws + tail draws).
    ///
    /// `num_drawn_head + num_drawn_tail` can never exceed this. Because each tail draw is repaid
    /// out of the head's allowance, the sum is invariant: it is the wall size minus the dead wall.
    pub const fn max_num_draws(self) -> u8 {
        (self.wall_size() - self.dead_wall_size()) as u8
    }

    /// How many normal (head) self-draws a round has when no Kan or Kita occurs.
    ///
    /// Yonma: 136 - 14 - 52 = **70**. Sanma: 108 - 14 - 39 = **55**, confirmed against houou 3p
    /// logs (every exhaustive-draw kyoku contains exactly 55 draw events, regardless of how many
    /// Kans and Kita occurred).
    pub const fn num_live_wall_draws(self) -> u8 {
        self.max_num_draws() - self.num_dealt()
    }

    /// `num_drawn_head` at the very start of a round: the initial deal plus the button player's
    /// first self draw.
    ///
    /// Yonma: 53. Sanma: 40.
    pub const fn initial_num_drawn_head(self) -> u8 { self.num_dealt() + 1 }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Round flow
//////////////////////////////////////////////////////////////////////////////////////////////////

impl Variant {
    /// The next Kyoku number, skipping the numbers that do not exist in this variant.
    ///
    /// Sanma uses **sparse** kyoku numbering: seat 3 is the **absent seat** and is never the
    /// button, so kyoku ≡ 3 (mod 4) simply does not exist. E1/E2/E3 are kyoku 0/1/2 and
    /// S1/S2/S3 are 4/5/6.
    ///
    /// This is what keeps `RoundId::prevailing_wind` (`kyoku / 4`) and `RoundId::button`
    /// (`kyoku % 4`) untouched and literally correct in both variants, at the cost of the
    /// numbering not being dense.
    pub const fn next_kyoku(self, kyoku: u8) -> u8 {
        let next = kyoku + 1;
        match self {
            Variant::Yonma => next,
            Variant::Sanma => if next % 4 == 3 { next + 1 } else { next },
        }
    }

    /// Does this Kyoku number exist in this variant?
    pub const fn is_valid_kyoku(self, kyoku: u8) -> bool {
        self.is_seat_active(Player::new(kyoku % 4))
    }

    /// The Kyoku number of the last round of the given prevailing wind (0 = East, 1 = South, ...).
    ///
    /// Useful for setting the game-length caps: yonma East-South ends at `kyoku_max_soft == 7`
    /// (South 4), sanma East-South at `6` (South 3).
    pub const fn last_kyoku_of_wind(self, wind: u8) -> u8 {
        wind * 4 + (self.num_players() - 1)
    }

    /// Highest `seq` (turn counter) that still counts as the first uninterrupted go-around.
    ///
    /// Affects Kyuushuu Kyuuhai eligibility, Double Riichi, and the first-chance yaku
    /// (Tenhou/Chiihou/Renhou). Yonma: 3 (four turns). Sanma: 2 (three turns) --- using 3 here
    /// would let the dealer's *second* discard declare Double Riichi.
    pub const fn first_chance_max_seq(self) -> u8 { self.num_players() - 1 }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Abortive draws
//////////////////////////////////////////////////////////////////////////////////////////////////

impl Variant {
    /// Can Suufon Renda (四風連打, four identical wind discards) abort a round?
    ///
    /// Sanma: no --- there are only 3 first discards, and **there is no three-wind variant**.
    pub const fn allows_four_wind_abort(self) -> bool {
        matches!(self, Variant::Yonma)
    }

    /// Can Suucha Riichi (四家立直, all players under riichi) abort a round?
    ///
    /// Sanma: no. This is explicit rather than derived --- 「三人打ちの三人立直は流局にならない」
    /// says all *three* players under riichi is **not** an abort, so the trigger must not be
    /// rewritten as "all players riichi".
    pub const fn allows_all_riichi_abort(self) -> bool {
        matches!(self, Variant::Yonma)
    }

    /// Can Sanchahou (三家和, triple ron) abort a round?
    ///
    /// Sanma: no --- with 3 seats, double ron is the maximum.
    pub const fn allows_triple_ron_abort(self) -> bool {
        matches!(self, Variant::Yonma)
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Scoring
//////////////////////////////////////////////////////////////////////////////////////////////////

impl Variant {
    /// Total pot moved by the Noten Bappu (ノーテン罰符) settlement at an exhaustive draw.
    ///
    /// Yonma: 3000. Sanma: **2000**, verified against houou 3p logs --- 1 tenpai is
    /// +2000/-1000/-1000 and 2 tenpai is +1000/+1000/-2000. Note this is *not* the yonma
    /// schedule with a seat removed: the pot itself shrinks.
    pub const fn noten_penalty_total(self) -> GamePoints {
        match self {
            Variant::Yonma => 3000,
            Variant::Sanma => 2000,
        }
    }

    /// Honba surcharge paid by each payer on a Tsumo win, per honba stick.
    ///
    /// 100 in both variants; the per-payer form generalizes untouched, which is why sanma's total
    /// works out to 200 per stick.
    pub const fn honba_points_per_payer(self) -> GamePoints { 100 }

    /// Honba surcharge paid by the discarder on a Ron win, per honba stick.
    ///
    /// `(num_players - 1) x 100`: 300 in yonma, **200** in sanma (verified on 3,265 houou 3p
    /// wins).
    pub const fn honba_points_on_ron(self) -> GamePoints {
        (self.num_players() as GamePoints - 1) * self.honba_points_per_payer()
    }

    /// Does this variant apply **pure tsumo loss** (純正ツモ損)?
    ///
    /// Sanma: yes. Every per-payer amount is exactly the yonma amount; the absent seat's
    /// non-dealer share is simply never paid, so a non-dealer tsumo collects 3/4 of the yonma
    /// total and a dealer tsumo 2/3. This falls out of a per-payer loop over the *active* seats,
    /// so it is a property to assert rather than a branch to write.
    pub const fn has_tsumo_loss(self) -> bool {
        matches!(self, Variant::Sanma)
    }

    /// Points each seat starts a game with.
    ///
    /// Yonma 25000, sanma **35000** (「35000開始の40000点返し」). *Informational*: the engine takes
    /// starting points from `RoundBegin::points`, so nothing here is enforced.
    pub const fn starting_points(self) -> GamePoints {
        match self {
            Variant::Yonma => 25000,
            Variant::Sanma => 35000,
        }
    }

    /// The return point (返し点) used for final settlement.
    ///
    /// Yonma 30000, sanma **40000**. *Informational* in the same sense as
    /// [`Self::starting_points`]; the engine's own all-last qualification threshold is the
    /// separately configurable `Ruleset::points_min_qualify`.
    pub const fn return_points(self) -> GamePoints {
        match self {
            Variant::Yonma => 30000,
            Variant::Sanma => 40000,
        }
    }

    /// Placement bonus (ウマ) by rank, in points (i.e. already x1000).
    ///
    /// Yonma +20/+10/-10/-20, sanma **+20/0/-20** with the **absent seat**'s 4th slot
    /// unreachable. *Informational*: uma and oka are a game-level settlement the engine does not
    /// compute; this is here so that every consumer derives them from the one variant value.
    pub const fn uma(self) -> [GamePoints; 4] {
        match self {
            Variant::Yonma => [20_000, 10_000, -10_000, -20_000],
            Variant::Sanma => [20_000, 0, -20_000, 0],
        }
    }

    /// Oka (オカ) awarded to 1st place: `(return - start) x num_players`.
    ///
    /// Yonma +20000, sanma **+15000**. *Informational*, as [`Self::uma`].
    pub const fn oka(self) -> GamePoints {
        (self.return_points() - self.starting_points()) * self.num_players() as GamePoints
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    #[test]
    fn yonma_geometry_matches_the_historical_consts() {
        let v = Variant::Yonma;
        assert_eq!(v.wall_size(), 136);
        assert_eq!(v.num_dealt(), 52);
        assert_eq!(v.max_num_draws(), 136 - 14);
        assert_eq!(v.num_live_wall_draws(), 70);
        assert_eq!(v.initial_num_drawn_head(), 53);
        assert_eq!(v.dora_indicator_index(), &[130, 128, 126, 124, 122]);
        assert_eq!(v.ura_dora_indicator_index(), &[131, 129, 127, 125, 123]);
        assert_eq!(v.kan_draw_index(), &[134, 135, 132, 133]);
        assert_eq!(v.deal_index().len(), 4);
    }

    #[test]
    fn sanma_geometry() {
        let v = Variant::Sanma;
        assert_eq!(v.wall_size(), 108);
        assert_eq!(v.num_dealt(), 39);
        assert_eq!(v.max_num_draws(), 94);
        // The one number the houou 3p corpus pins exactly.
        assert_eq!(v.num_live_wall_draws(), 55);
        assert_eq!(v.initial_num_drawn_head(), 40);
        assert_eq!(v.kan_draw_index().len(), 8);
        assert_eq!(v.deal_index().len(), 3);
    }

    /// The deal, the indicators and the replacement draws never share a slot, and none of them
    /// reaches past the end of the wall.
    ///
    /// The *head draw* range is deliberately excluded here: in sanma the last four head slots
    /// (90--93) double as the outer dora indicators, which is sound only because of the reveal
    /// condition proved in [`revealed_indicators_are_never_also_drawn`]. What must hold
    /// unconditionally is that the head can never reach them once enough tail draws have happened
    /// to reveal them --- asserted below at maximum tail consumption.
    #[test]
    fn wall_slots_are_partitioned() {
        for v in [Variant::Yonma, Variant::Sanma] {
            let mut seen = [0u8; 136];
            let bump = |seen: &mut [u8; 136], i: usize, what: &str| {
                assert!(i < v.wall_size(), "{:?}: {} slot {} is past the wall", v, what, i);
                seen[i] += 1;
                assert!(seen[i] <= 1, "{:?}: wall slot {} claimed twice (by {})", v, i, what);
            };
            for row in v.deal_index() {
                for &i in row { bump(&mut seen, i, "deal"); }
            }
            for &i in v.dora_indicator_index() { bump(&mut seen, i, "dora"); }
            for &i in v.ura_dora_indicator_index() { bump(&mut seen, i, "ura"); }
            for &i in v.kan_draw_index() { bump(&mut seen, i, "kan draw"); }

            // The deal always occupies exactly the head of the wall.
            for i in 0..(v.num_dealt() as usize) {
                assert_eq!(seen[i], 1, "{:?}: slot {} should be dealt", v, i);
            }

            // At maximum tail consumption, head draws and the dead wall are disjoint and together
            // account for every live slot.
            let max_tail = v.kan_draw_index().len();
            let head_end = v.max_num_draws() as usize - max_tail;
            for &i in v.dora_indicator_index() { assert!(i >= head_end); }
            for &i in v.ura_dora_indicator_index() { assert!(i >= head_end); }
            for &i in v.kan_draw_index() { assert!(i >= head_end); }
            assert_eq!(v.wall_size() - head_end, v.dead_wall_size() + max_tail);
        }
    }

    /// The reason the sanma layout works despite `8 rinshan + 5 dora + 5 ura = 18 > 14`:
    /// dora indicator `i` is only revealed once at least `i` Kans have happened, and every Kan
    /// shortens the head's reach by one. So no revealed indicator is ever also drawn.
    #[test]
    fn revealed_indicators_are_never_also_drawn() {
        for v in [Variant::Yonma, Variant::Sanma] {
            let max_kan = 4usize;
            let max_tail = v.kan_draw_index().len();
            for kans in 0..=max_kan {
                for kita in 0..=(v.max_num_kita() as usize) {
                    let tail = kans + kita;
                    if tail > max_tail { continue; }
                    // Head reaches this exclusive bound.
                    let head_end = v.max_num_draws() as usize - tail;
                    // `1 + kans` indicators are revealed (initial + one per kan).
                    for i in 0..=kans {
                        let d = v.dora_indicator_index()[i];
                        let u = v.ura_dora_indicator_index()[i];
                        assert!(d >= head_end,
                            "{:?}: kans={} kita={}: dora indicator {} (slot {}) is inside the \
                             head draws (< {})", v, kans, kita, i, d, head_end);
                        assert!(u >= head_end,
                            "{:?}: kans={} kita={}: ura indicator {} (slot {}) is inside the \
                             head draws (< {})", v, kans, kita, i, u, head_end);
                        // ... and never inside the tail draws either.
                        for &t in &v.kan_draw_index()[0..tail] {
                            assert_ne!(d, t, "{:?}: dora indicator {} collides with a tail draw",
                                       v, i);
                            assert_ne!(u, t, "{:?}: ura indicator {} collides with a tail draw",
                                       v, i);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sanma_skips_kyoku_3_mod_4() {
        let v = Variant::Sanma;
        // E1 E2 E3 -> S1 S2 S3 -> W1
        assert_eq!(v.next_kyoku(0), 1);
        assert_eq!(v.next_kyoku(1), 2);
        assert_eq!(v.next_kyoku(2), 4);
        assert_eq!(v.next_kyoku(4), 5);
        assert_eq!(v.next_kyoku(5), 6);
        assert_eq!(v.next_kyoku(6), 8);
        assert!(!v.is_valid_kyoku(3));
        assert!(!v.is_valid_kyoku(7));
        assert!(v.is_valid_kyoku(6));
        // East-South caps.
        assert_eq!(v.last_kyoku_of_wind(0), 2);
        assert_eq!(v.last_kyoku_of_wind(1), 6);
        assert_eq!(v.last_kyoku_of_wind(2), 10);
    }

    #[test]
    fn yonma_kyoku_numbering_is_dense() {
        let v = Variant::Yonma;
        for k in 0..15 { assert_eq!(v.next_kyoku(k), k + 1); }
        for k in 0..16 { assert!(v.is_valid_kyoku(k)); }
        assert_eq!(v.last_kyoku_of_wind(1), 7);
        assert_eq!(v.last_kyoku_of_wind(3), 15);
    }

    #[test]
    fn absent_seat_is_never_the_button() {
        let v = Variant::Sanma;
        let mut kyoku = 0u8;
        for _ in 0..12 {
            assert!(v.is_seat_active(Player::new(kyoku % 4)),
                    "kyoku {} makes the absent seat the button", kyoku);
            kyoku = v.next_kyoku(kyoku);
        }
    }

    #[test]
    fn turn_order_skips_the_absent_seat() {
        let v = Variant::Sanma;
        assert_eq!(v.succ(P0), P1);
        assert_eq!(v.succ(P1), P2);
        assert_eq!(v.succ(P2), P0);

        itertools::assert_equal(v.other_active_players_after(P0), [P1, P2]);
        itertools::assert_equal(v.other_active_players_after(P2), [P0, P1]);

        // In yonma this must agree exactly with the historical helper.
        let y = Variant::Yonma;
        for p in ALL_PLAYERS {
            itertools::assert_equal(y.other_active_players_after(p), other_players_after(p));
        }
    }

    #[test]
    fn sanma_dora_chain_wraps_1m_to_9m() {
        let v = Variant::Sanma;
        let m1 = Tile::from_encoding(0).unwrap();
        let m9 = Tile::from_encoding(8).unwrap();
        assert_eq!(v.indicated_dora(m1), m9);
        assert_eq!(v.indicated_dora(m9), m1);
        // Yonma is untouched.
        assert_eq!(Variant::Yonma.indicated_dora(m1), Tile::from_encoding(1).unwrap());
        assert_eq!(Variant::Yonma.indicated_dora(m9), m1);
        // Every non-manzu indicator agrees between the two variants.
        for e in 9..34u8 {
            let t = Tile::from_encoding(e).unwrap();
            assert_eq!(v.indicated_dora(t), Variant::Yonma.indicated_dora(t));
        }
    }

    #[test]
    fn sanma_tile_set_omits_2m_to_8m() {
        let v = Variant::Sanma;
        let mut total = 0u32;
        for e in 0..34u8 {
            let n = v.num_copies_34(e);
            total += n as u32;
            assert_eq!(v.has_tile(Tile::from_encoding(e).unwrap()), n > 0);
        }
        assert_eq!(total as usize, v.wall_size());
        // Red 5m does not exist either.
        assert!(!v.has_tile(Tile::from_encoding(34).unwrap()));
        assert!(v.has_tile(Tile::from_encoding(35).unwrap()));
    }

    #[test]
    fn scoring_constants() {
        assert_eq!(Variant::Yonma.noten_penalty_total(), 3000);
        assert_eq!(Variant::Sanma.noten_penalty_total(), 2000);
        assert_eq!(Variant::Yonma.honba_points_on_ron(), 300);
        assert_eq!(Variant::Sanma.honba_points_on_ron(), 200);
        assert_eq!(Variant::Sanma.oka(), 15_000);
        assert_eq!(Variant::Yonma.oka(), 20_000);
    }
}
