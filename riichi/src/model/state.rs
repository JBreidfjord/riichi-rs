//! Main game state bundle.
//! Due to its central importance, it is simply known as The [`State`].

// use arrayvec::ArrayVec;  // TODO(summivox): use ArrayVec

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use itertools::Itertools;
#[cfg(feature = "serde")]
use serde_with::{
    serde_as, As, DisplayFromStr,
};

use riichi_elements::prelude::*;

use super::{
    Discard,
    RoundBegin,
};

/// State of a round of game.
///
/// This is sampled before a player takes action, but after drawing and/or meld is taken in.
/// Note that any newly drawn tile is deliberately kept separate from `closed_hands`.
///
/// See [mod-level docs](crate::model) for the details of modeling.
#[cfg_attr(test, derive(type_layout::TypeLayout))]
#[derive(Clone, Debug, Default)]
pub struct State {
    /// Core state variables.
    /// **Visibility varies.**
    pub core: StateCore,

    /// Melds / open hands of each player.
    /// **Publicly visible.**
    pub melds: [Vec<Meld>; 4],

    /// The concealed/closed hand of each player, represented as a [`TileSet37`].
    /// This does not include any newly drawn tile, i.e. [`StateCore::draw`].
    /// **A player can only observe their own hand.**
    pub closed_hands: [TileSet37; 4],

    /// The discard stream of each player, a.k.a. "River" (河).
    /// **Publicly visible.**
    ///
    /// Tiles that are called by other players are explicitly marked so, unlike the physical
    /// 6-in-a-row representation on table, where the tile cannot exist both in the discard stream
    /// and the corresponding meld.
    ///
    /// See [`Discard`].
    pub discards: [Vec<Discard>; 4],

    /// The set of discarded tiles for each player.
    /// **Publicly visible.**
    ///
    /// This makes it easier to answer queries of whether a player had discarded a certain tile, no
    /// matter how many times and when. Specifically, this is used to calculate [`FuritenFlags`].
    ///
    /// Any red 5 is treated the same as its corresponding normal 5.
    pub discard_sets: [TileMask34; 4],
}

/// Essential state variables.
///
/// Reasons for separating these out: Hands, discards, and melds are big arrays. If we are to store
/// all states in a round, they take up a lot of space. Fortunately, all 3 can be reconstructed by
/// aggregating actions and "core" variables from the initial full state. This means we can instead
/// only store `{action, next state core}` for each turn and obtain a more space-efficient
/// representation of the same information. Reconstruction of any full state is effectively a `fold`
/// operation ([`State::evolve`], [`State::apply_step`], [`State::apply_steps`]).
///
/// See [mod-level docs](crate::model) for the details of modeling.
///
/// ## Optional `serde` support
///
/// Straightforward struct mapping of all fields.
///
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(test, derive(type_layout::TypeLayout))]
#[cfg_attr(feature = "serde", serde_as)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StateCore {
    /// Sequence number of this state, defined as the total number of closed actions since the
    /// beginning of this round.
    /// **Publicly visible.**
    pub seq: u8,

    /// The player in action.
    /// **Publicly visible.**
    pub actor: Player,

    /// Number of tiles drawn from the head of the double-stacked cut wall.
    /// **Publicly visible.**
    ///
    /// This includes the initial deal (13 x 4 = 52), the current player's normal self draw, and
    /// everything in between.
    ///
    /// A normal draw is from `wall[num_drawn_head - 1]`. As an example, before the first action of
    /// any round, `num_drawn_head == 53` and the tile drawn will be `wall[52]`.
    pub num_drawn_head: u8,

    /// Number of tiles drawn from the tail of the double-stacked cut wall, as a result of forming
    /// (any kind of) Kan's. Same as the number of completed Kan's.
    /// **Publicly visible.**
    ///
    /// Due to the double-stacking (see [`wall`]), the order of tiles drawing from the tail
    /// is NOT the same as the reverse order of the wall array. See [`wall::KAN_DRAW_INDEX`].
    pub num_drawn_tail: u8,

    /// Number of revealed dora indicators (see [`Tile::indicated_dora`]).
    /// **Publicly visible.**
    pub num_dora_indicators: u8,

    // The player will always gain one tile before action. Possibilities:
    // - `{draw=Some, meld=None}` Normal draw: from the head of the wall
    // - `{draw=Some, meld=Some}` Kan draw: from the tail of the wall
    // - `{draw=None, meld=Some}` Chii/Pon: from another player; combined into the meld list
    //   (not the closed hand!)

    /// If the player has drawn a tile from the wall (normal or kan), this is it.
    /// **A player can only observe their own draw.**
    pub draw: Option<Tile>,

    /// If the player called a meld during the last action-reaction cycle, this is it.
    /// Note that this is not mutually exclusive with `draw`; an incoming Kan => both draw and meld.
    /// **Publicly visible.**
    pub incoming_meld: Option<Meld>,

    /// Furiten status for each player before action.
    /// **A player can only observe their own status.**
    pub furiten: [FuritenFlags; 4],

    /// Riichi status for each player.
    /// **Publicly visible.**
    #[cfg_attr(feature = "serde", serde(with = "As::<[Option<DisplayFromStr>; 4]>"))]
    pub riichi: [Option<Riichi>; 4],
}

impl State {
    /// Returns the initial state of a round, with all 4 players' initial hands dealt (13 x 4),
    /// and the button player's first self draw added.
    pub fn new(begin: &RoundBegin) -> Self {
        let button = begin.round_id.button();
        Self {
            core: StateCore::new(begin),

            closed_hands: wall::deal(&begin.wall, button),
            melds: Default::default(),
            discards: Default::default(),
            discard_sets: Default::default(),
        }
    }
}

impl StateCore {
    /// Returns the initial state of a round, with all 4 players' initial hands dealt (13 x 4),
    /// and the button player's first self draw added.
    pub fn new(begin: &RoundBegin) -> Self {
        let button = begin.round_id.button();
        Self {
            seq: 0,
            actor: button,
            num_drawn_head: 53,  // 13 x 4 + 1
            num_drawn_tail: 0,
            num_dora_indicators: 1,
            draw: Some(begin.wall[52]),
            incoming_meld: None,
            furiten: Default::default(),
            riichi: Default::default(),
        }
    }
}

impl Display for StateCore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{#{} P{} draw[{}|{}]={} meld={:?} dora={} riichi=[{}] furiten=[{}]}}",
               self.seq,
               self.actor.to_usize(),
               self.num_drawn_head,
               self.num_drawn_tail,
               self.draw.map(|t| t.as_str()).unwrap_or("NA"),
               self.incoming_meld.map(|x| x.to_string()),
               self.num_dora_indicators,
               self.riichi.into_iter().map(maybe_riichi_as_str).join(","),
               self.furiten.into_iter().map(|f| f.any() as u8).join(","),
        )
    }
}

/// Status regarding whether a player is under riichi (リーチ).
///
/// <https://riichi.wiki/Riichi>
///
/// ## Optional `serde` support
///
/// Deliberately not provided; [`StateCore`] uses [`DisplayFromStr`] to serialize this as a string.
///
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Riichi {
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

impl Riichi {
    pub fn as_str(self) -> &'static str {
        match (self.is_double, self.is_ippatsu) {
            (false, false) => "r",
            (false, true) => "R",
            (true, false) => "d",
            (true, true) => "D",
        }
    }
}

pub fn maybe_riichi_as_str(r: Option<Riichi>) -> &'static str {
    match r {
        Some(r) => r.as_str(),
        None => "_",
    }
}

impl FromStr for Riichi {
    type Err = UnspecifiedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = match s {
            "r" => Riichi { is_double: false, is_ippatsu: false },
            "R" => Riichi { is_double: false, is_ippatsu: true },
            "d" => Riichi { is_double: true, is_ippatsu: false },
            "D" => Riichi { is_double: true, is_ippatsu: true },
            _ => return Err(UnspecifiedError),
        };
        Ok(r)
    }
}

impl Display for Riichi {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Status regarding whether a player is under the penalty of Furiten (振聴) and cannot declare Ron,
/// either temporarily (by discard or missed chance) or permanently (by missed chance under riichi).
///
/// <https://riichi.wiki/Furiten>
///
/// ## Optional `serde` support
///
/// To reduce verbosity of outputs, this is serialized as `[discard, temp, permanent]` (untagged).
///
/// TODO(summivox): FuritenFlags serialization format is too verbose.
///
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde_tuple::Serialize_tuple, serde_tuple::Deserialize_tuple))]
pub struct FuritenFlags {
    /// At least one tile in the player's waiting set had been discarded by the player before.
    /// This penalty is considered temporary as the player's waiting set might change.
    pub by_discard: bool,

    /// Another player discarded, or made kakan/ankan, on a tile in the player's waiting set, but
    /// this player did not (including if this player was not able to) declare Ron on it.
    /// This penalty is temporary, as it will be lifted once this player discards, unless this
    /// player is also under riichi, which will mark `miss_permanent` instead.
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
        println!("{}", State::type_layout());
        println!("{}", StateCore::type_layout());
    }
}
