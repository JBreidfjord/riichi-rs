//! Main game state bundle.
//! Due to its central importance, it is simply known as The [`State`].

// use arrayvec::ArrayVec;  // TODO(summivox): use ArrayVec

use std::fmt::{Display, Formatter};
use itertools::Itertools;
use crate::common::*;
use super::Discard;
use super::PartiallyObservable;

/// State of a round of game.
/// This is sampled before a player takes action, but after drawing and/or meld is taken in.
///
/// NOTE: The newly drawn tile is deliberately kept separate from `closed_hands`.
#[derive(Clone, Debug, Default)]
pub struct State {
    /// Core variables; see [`StateCore`].
    pub core: StateCore,

    /// Melds / open hands of each player.
    pub melds: [Vec<Meld>; 4],

    /// The concealed/closed hand of each player, represented as a [`TileSet37`].
    /// This does not include any newly drawn tile.
    /// **A player can only observe their own hand.**
    pub closed_hands: [TileSet37; 4],

    /// The discard stream of each player.
    /// Tiles that are called by other players are explicitly marked so, not excluded.
    /// See [`Discard`].
    pub discards: [Vec<Discard>; 4],

    /// The set of discarded tiles for each player. This makes it easier to answer queries of
    /// whether a player had discarded a certain tile, no matter how many times and when.
    /// Specifically, this is used to calculate [`FuritenFlags`].
    ///
    /// Any red 5 is treated the same as its corresponding normal 5.
    pub discard_sets: [TileMask34; 4],
}

/// Essential state variables.
/// These variables, together with the previous turn's actions and reactions, imply the delta of
/// all other variables from the previous state to the current.
///
/// Expressed in forward form: `state + {action, reactions, next state core} => next state`.
///
/// This means that the history of full states can be derived by folding the initial state with
/// each consecutive `{action, reactions, next state core}` triplet, which is effectively a
/// more space-efficient representation of the same information.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(test, derive(type_layout::TypeLayout))]
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

impl Display for StateCore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{#{} P{} draw[{}|{}]={} meld={:?} dora={} riichi=[{}] furiten=[{}]}}",
               self.seq,
               self.action_player.to_usize(),
               self.num_drawn_head,
               self.num_drawn_tail,
               self.draw.map(|t| t.as_str()).unwrap_or("NA"),
               self.incoming_meld.map(|x| x.to_string()),
               self.num_dora_indicators,
               self.riichi.into_iter().map(RiichiFlags::as_str).join(","),
               self.furiten.into_iter().map(|f| f.any() as u8).join(","),
        )
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

impl RiichiFlags {
    /// Shorthand to help debugging.
    pub fn as_str(self) -> &'static str {
        match (self.is_active, self.is_double, self.is_ippatsu) {
            (false, _, _) => "_",
            (true, false, false) =>  "r",
            (true, false, true) => "R",
            (true, true, false) => "d",
            (true, true, true) => "D",
        }
    }
}

impl Display for RiichiFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Status regarding whether a player is under the penalty of Furiten (振聴) and cannot declare Ron,
/// either temporarily (by discard or missed chance) or permanently (by missed chance under riichi).
///
/// <https://riichi.wiki/Furiten>
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct FuritenFlags {
    /// At least one tile in the player's waiting set had been discarded by the player before.
    /// This penalty is considered temporary as the player's waiting set might change.
    pub by_discard: bool,

    /// Another player discarded, or made kakan/ankan, on a tile in the player's waiting set, but
    /// this player did not (including if this player was not able to) declare Ron on it.
    /// This penalty is temporary, as it will be lifted once this player discards, unless this
    /// player is also under riichi, which will mark [`miss_permanent`] instead.
    pub miss_temporary: bool,

    /// Same trigger as `miss_temporary` while under riichi.
    /// This penalty is permanent (for the rest of the round).
    pub miss_permanent: bool,
}

impl FuritenFlags {
    /// Returns whether any Furiten penalty is currently active.
    pub const fn any(self) -> bool { self.by_discard || self.miss_temporary || self.miss_permanent }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_layout() {
        use type_layout::TypeLayout;
        println!("{}", StateCore::type_layout());
    }
}
