//! State-Action representation of the game.
//!
//! This module mainly provides data model definitions and some straightforward helpers.
//! Game logic belongs to [`crate::engine`].

use std::cmp::Ordering;

use derive_more::{Constructor, From, Into};
use num_enum::Default;

use crate::common::tile_set::TileSet37;
use crate::common::typedefs::*;
use crate::common::tile::Tile;
use crate::common::meld::Meld;
use crate::common::wall::{make_dummy_wall, Wall};
use crate::tiles_from_str;

trait PartiallyObservable {
    fn observe_by(&self, player: Player) -> Self;
}

/// Kyoku-Honba (局-本場) pair that identifies a round in a game.
///
/// Ref:
/// - <https://riichi.wiki/Kyoku>
/// - <https://riichi.wiki/Honba>
/// - <https://ja.wikipedia.org/wiki/%E9%80%A3%E8%8D%98>
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct RoundId {
    /// Index of the wind-round (局), enumerated in combination with the prevailing wind:
    ///
    /// - 0 => east 1 (東1局) -- min
    /// - 3 => east 4 (東4局)
    /// - 4 => south 1 (南1局)
    /// - 7 => south 4 (南4局)
    /// - 8 => west 1 (西1局)
    /// - 15 => north 4 (北4局) -- max
    ///
    /// NOTE: The theoretical max value is not enforced here.
    pub kyoku: u8,

    /// The "sub round" number (本場数), commonly represented as the number of 100-pt sticks placed
    /// on the table.
    ///
    /// NOTE: There are no real limits in the rules, so theoretically this can grow towards +inf.
    /// Saturation arithmetic should be used to ensure sanity.
    pub honba: u8,
}

impl RoundId {
    /// Index of the prevailing wind (場風).
    ///
    /// This is shared by all players (unlike "self wind").
    pub const fn prevailing_wind(self) -> Wind {
        Wind::new(self.kyoku / 4)
    }

    /// Index of the dealer/button/east-wind player (荘家).
    ///
    /// NOTE: "button" refers to the similar concept in Texas Hold'em, a.k.a. dealer
    pub const fn button(self) -> Player { Player::new(self.kyoku % 4) }

    /// Index of the player with given self wind.
    /// - east-wind player == button
    /// - south-wind player == button + 1
    /// - west-wind player == button + 2
    /// - north-wind player == button + 3
    pub fn player_with_self_wind(self, wind: Wind) -> Player {
        Player::new(self.kyoku + wind.to_u8())
    }

    /// Index of the self wind (自風).
    pub fn self_wind_for_player(self, player: Player) -> Wind {
        Wind::from(player.wrapping_sub(self.button()))
    }

    /// TODO(summivox): doc
    pub const fn next_kyoku(self) -> Self {
        Self {
            kyoku: self.kyoku + 1,
            honba: 0,
        }
    }

    /// TODO(summivox): doc
    pub const fn next_honba(self, renchan: bool) -> Self {
        Self {
            kyoku: if renchan { self.kyoku } else { self.kyoku + 1 },
            honba: self.honba + 1,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RoundBeginState {
    pub rules: (),  // TODO(summivox): define Rules

    /// Kyoku-honba that identifies this round.
    pub round_id: RoundId,

    /// The tile wall right after shuffling and cutting (full 136 tiles).  Drawing and revealing
    /// (of dora indicators) are "virtual", always referring to this original wall.
    pub wall: Wall,

    /// Points left on the table (供託), up for grabs by the next winner.
    /// Commonly 1000-pt sticks from riichi.
    ///
    /// Ref:
    /// - <https://ja.wikipedia.org/wiki/%E9%BA%BB%E9%9B%80%E3%81%AE%E7%82%B9#%E4%BE%9B%E8%A8%97>
    pub pot: GamePoints,

    /// Points for each player.
    pub points: [GamePoints; 4],
}

impl PartiallyObservable for RoundBeginState {
    fn observe_by(&self, _player: Player) -> Self {
        let mut observed = self.clone();
        observed.wall = make_dummy_wall();
        observed
    }
}

/// Status regarding whether a player is under riichi (リーチ).
///
/// <https://riichi.wiki/Riichi>
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct RiichiFlags {
    /// Player is under active riichi (リーチ).
    pub is_active: bool,

    /// Player declared riichi in one of the first 4 uninterrupted turns of the game (両立直).
    ///
    /// <https://riichi.wiki/Daburu_riichi>
    pub is_double: bool,

    /// It has been less than 4 uninterrupted turns since the player declared riichi (一発).
    /// This includes the player's first turn after riichi.
    ///
    /// <https://riichi.wiki/Ippatsu>
    pub is_ippatsu: bool,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct FuritenFlags {
    pub by_discard: bool,
    pub miss_temporary: bool,
    pub miss_permanent: bool,
}

impl FuritenFlags {
    pub const fn any(self) -> bool { self.by_discard || self.miss_temporary || self.miss_permanent }
}

/// State variables known right before a player's action (after tile-drawing, if any).
#[derive(Clone, Debug, Default)]
pub struct PreActionState {
    /// The player in action.
    pub action_player: Player,

    /// Sequence number of this action, defined as the total number of closed actions since the
    /// beginning of this round.
    pub seq: u8,

    /// Number of tiles drawn from the head of the double-stacked cut wall. This includes the
    /// initial deal (13 x 4 = 52), the current player's normal self draw, and everything in between.
    ///
    /// A normal draw is from `wall[num_drawn_head - 1]`. As an example, before the first action of
    /// any round, `num_drawn_head == 53` and the tile drawn will be `wall[52]`.
    pub num_drawn_head: u8,

    /// Number of tiles drawn from the tail of the double-stacked cut wall, as a result of forming
    /// (any kind of) kan. Same as the number of completed kan.
    ///
    /// Due to the double-stacking (see [`crate::wall`]), the order of tiles drawing from the tail
    /// is NOT the same as the reverse order of the wall array. See [`crate::wall::KAN_DRAW_INDEX`].
    pub num_drawn_tail: u8,

    /// Number of revealed dora indicators (see [`Tile::indicated_dora`]).
    pub num_dora_indicators: u8,

    // The player will always gain one tile before action. Possibilities:
    // - Normal draw: from the head of the wall
    // - Kan draw: from the tail of the wall
    // - Chii/Pon: from another player; combined into the meld list (not the closed hand!)

    /// If the player has drawn a tile from the wall (normal or kan), this is it.
    /// **A player can only observe their own draw.**
    pub draw: Option<Tile>,
    /// If the player called a meld during the last action-reaction cycle, this is it.
    /// Note that this is not mutually exclusive with `draw`; kan => both draw and meld.
    pub incoming_meld: Option<Meld>,

    /// The concealed/closed hand of each player, represented as a [`TileSet37`].
    /// Note that this does NOT include any newly drawn tile.
    /// **A player can only observe their own hand.**
    pub closed_hands: [TileSet37; 4],

    /// The discard stream of each player.
    /// Tiles that are called by other players are explicitly marked so, not excluded.
    /// All other tiles will have the "called player" field equal to the player itself.
    pub discards: [Vec<(Tile, Player)>; 4],

    /// Furiten status for each player before action.
    /// **A player can only observe their own status.**
    pub furiten: [FuritenFlags; 4],

    /// Riichi status for each player.
    pub riichi: [RiichiFlags; 4],

    /// Melds / open hands of each player.
    pub melds: [Vec<Meld>; 4],
}

impl PartiallyObservable for PreActionState {
    fn observe_by(&self, player: Player) -> Self {
        let mut observed = self.clone();
        if player != observed.action_player {
            observed.draw = None;
        }
        for i in 0..4 {
            if i != player.to_usize() {
                observed.closed_hands[i] = TileSet37::default();
                observed.furiten[i] = FuritenFlags::default();
            }
        }
        observed
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Discard {
        tile: Tile,
        riichi: bool,
        tsumokiri: bool,
    },
    Ankan(Tile),
    Kakan(Tile),
    TsumoAgari(Tile),
    AbortKyuushuukyuuhai,
}

impl Action {
    pub fn tile(self) -> Option<Tile> {
        match self {
            Action::Discard { tile, .. } => Some(tile),
            Action::Ankan(tile) => Some(tile),
            Action::Kakan(tile) => Some(tile),
            Action::TsumoAgari(tile) => Some(tile),
            Action::AbortKyuushuukyuuhai => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        match self {
            Self::TsumoAgari(_) | Self::AbortKyuushuukyuuhai => true,
            _ => false,
        }
    }
}

/// Reaction from an out-of-turn player.
/// The lack of reaction / "pass" / unknown reaction can be represented by `Option<Reaction>`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Reaction {
    // NOTE: Variant order matters --- used by `derive(Ord)` to comprare priority.
    Chii(Tile, Tile),
    Pon(Tile, Tile),
    Daiminkan,
    // tile is implicit
    RonAgari,
}

/// Conclusion of an action-reaction cycle.
/// Unknown state can be represented by `Option<PostReactionState>`, just like `Reaction`.
/// However, an explicit `Pass` is included to represent "nothing has happened; move on".
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum ActionResult {
    #[num_enum(default)]
    Pass = 0,

    // reactions
    Chii,
    Pon,
    Daiminkan,

    /// Win by steal (ロン和ガリ).
    /// Multiple players (but not too many) may call Ron on the same tile (discard/kakan/ankan).
    RonAgari,

    /// Win by self draw (ツモ和ガリ).
    ///
    /// Resolution:
    /// - Determined by in-turn action.
    /// - No reaction allowed.
    TsumoAgari,

    /// Nine kinds of terminals (九種九牌).
    ///
    /// Resolution:
    /// - Determined by in-turn action.
    /// - No reaction allowed.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Kyuushu_kyuuhai>
    AbortKyuushuukyuuhai,

    /// No more tiles to draw from the wall (荒牌).
    /// Penalties payments may apply (不聴罰符), including sub-type [`Self::AbortNagashiMangan`].
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by [`Self::RonAgari`] and all other aborts.
    ///
    /// <https://riichi.wiki/Ryuukyoku>
    AbortWallExhausted,

    /// Special case of [`Self::AbortWallExhausted`] (流し満貫).
    /// Treated as penalties payments.
    ///
    /// <https://riichi.wiki/Nagashi_mangan>
    AbortNagashiMangan,

    /// Four Kan's (四開槓), triggered by:
    /// - the 5th kan by the same player, or
    /// - the 4th kan if all 4 are not by the same player
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by [`Self::RonAgari`].
    ///
    /// Note that kakan and ankan may also be preempted due to Chankan (搶槓).
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suukaikan>
    AbortFourKan,
    
    /// Four of the same wind discarded consecutively since the game starts (四風連打).
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Cannot be preempted.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suufon_renda>
    AbortFourWind,

    /// All four players under active riichi (四家立直)
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Can be preempted by [`Self::RonAgari`].
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Suucha_riichi>
    AbortFourRiichi,

    /// Too many players calling Ron on the same tile; usually 3 (三家和)
    ///
    /// Resolution:
    /// - Determined by end-of-turn resolution.
    /// - Pre-empts all others.
    ///
    /// <https://riichi.wiki/Tochuu_ryuukyoku#Sanchahou>
    AbortMultiRon,
}

impl ActionResult {
    pub const fn is_meld(self) -> bool {
        use ActionResult::*;
        match self {
            Chii | Pon | Daiminkan => true,
            _ => false,
        }
    }
    pub const fn is_agari(self) -> bool {
        use ActionResult::*;
        match self {
            TsumoAgari | RonAgari => true,
            _ => false,
        }
    }
    pub const fn is_abort(self) -> bool {
        use ActionResult::*;
        match self {
            AbortKyuushuukyuuhai | AbortWallExhausted | AbortNagashiMangan |
            AbortFourKan | AbortFourWind | AbortFourRiichi | AbortMultiRon => true,
            _ => false,
        }
    }
    pub const fn is_terminal(self) -> bool { self.is_agari() || self.is_abort() }
}

#[derive(Clone, Debug, Default)]
pub struct RoundEndState {
    /// The result of the round; equal to the last `ActionResult` before round ended.
    /// Guaranteed to be "terminal" (see [`ActionResult::is_terminal`]).
    pub round_result: ActionResult,

    /// Same definition as [`RoundBeginState::pot`] but at round end.
    pub pot: GamePoints,
    /// Points for each player at round end.
    pub points: [GamePoints; 4],
    /// Point increments for each player (end - begin)
    pub points_delta: [GamePoints; 4],

    /// Whether the next round is "this round + 1 honba".
    pub renchan: bool,
    /// Id of the next round; `None` if the game ends.
    pub next_round_id: Option<RoundId>,

    /// If at least 1 player has won this round, how they did so.
    pub agari_summary: Option<()>,  // TODO(summivox): implement `AgariSummary`
}

#[derive(Clone, Debug)]
pub enum NextOrEnd {
    Next(PreActionState),
    End(RoundEndState),
}

#[cfg(test)]
mod test {
    use super::*;
    use assert2::check;
    use itertools::Itertools;

    #[test]
    fn reaction_ordering_is_correct() {
        let reactions = [
            Reaction::Chii(Tile::MIN, Tile::MIN.succ().unwrap()),
            Reaction::Chii(Tile::MIN.succ().unwrap(), Tile::MIN.succ2().unwrap()),
            Reaction::Pon(Tile::MIN, Tile::MIN),
            Reaction::Pon(Tile::MIN.succ().unwrap(), Tile::MIN.succ().unwrap()),
            Reaction::Daiminkan,
            Reaction::RonAgari,
        ];
        for (low, high) in reactions.iter().tuple_windows() {
            check!(low < high);
        }
    }

    #[test]
    fn round_id_computes_correct_self_wind() {
        let round_id = RoundId { kyoku: 6, honba: 0 };
        check!(round_id.self_wind_for_player(Player::new(2)) == Wind::new(0));
        check!(round_id.self_wind_for_player(Player::new(3)) == Wind::new(1));
        check!(round_id.self_wind_for_player(Player::new(0)) == Wind::new(2));
        check!(round_id.self_wind_for_player(Player::new(1)) == Wind::new(3));
    }
}
