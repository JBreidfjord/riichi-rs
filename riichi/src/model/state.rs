//! Main game state bundle.
//! Due to its central importance, it is simply known as The [`State`].

// use arrayvec::ArrayVec;  // TODO(summivox): use ArrayVec

use crate::common::*;
use super::Discard;
use super::PartiallyObservable;

/// State variables sampled right before a player's action.
/// Note that the effects of drawing (if any) is included in the state.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub core: StateCore,

    /// Melds / open hands of each player.
    pub melds: [Vec<Meld>; 4],

    /// The concealed/closed hand of each player, represented as a [`TileSet37`].
    /// Note that this does NOT include any newly drawn tile.
    /// **A player can only observe their own hand.**
    pub closed_hands: [TileSet37; 4],

    /// The discard stream of each player.
    /// Tiles that are called by other players are explicitly marked so, not excluded.
    /// See [`Discard`].
    pub discards: [Vec<Discard>; 4],
}

/// TODO
#[derive(Clone, Debug, Default)]
pub struct StateCore {
    /// Sequence number of this action, defined as the total number of closed actions since the
    /// beginning of this round.
    pub seq: u8,

    /// The player in action.
    pub action_player: Player,

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

    /// Furiten status for each player before action.
    /// **A player can only observe their own status.**
    pub furiten: [FuritenFlags; 4],

    /// Riichi status for each player.
    pub riichi: [RiichiFlags; 4],
}

impl PartiallyObservable for State {
    fn observe_by(&self, player: Player) -> Self {
        let mut observed = self.clone();
        if player != observed.core.action_player {
            observed.core.draw = None;
        }
        for i in 0..4 {
            if i != player.to_usize() {
                observed.closed_hands[i] = TileSet37::default();
                observed.core.furiten[i] = FuritenFlags::default();
            }
        }
        observed
    }
}

/// Status regarding whether a player is under riichi (リーチ).
///
/// <https://riichi.wiki/Riichi>
// TODO(summivox): represent with `Option<(bool, bool)>` instead?
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

