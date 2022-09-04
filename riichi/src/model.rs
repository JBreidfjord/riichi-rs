//! State-Action representation of the game.
//!
//! This module mainly provides data model definitions and some straightforward helpers.
//! Game logic belongs to [`crate::engine`].
mod actions;
mod agari;
mod boundary;
mod state;
mod yaku;

pub use actions::*;
pub use agari::*;
pub use boundary::*;
pub use state::*;
pub use yaku::*;

use crate::common::*;

trait PartiallyObservable {
    fn observe_by(&self, player: Player) -> Self;
}

/// A discarded tile.
/// This has the same representation in action and state.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Discard {
    /// The discarded tile.
    pub tile: Tile,
    /// If called by another player, that player; otherwise the player who discarded this tile.
    /// This is explicitly ignored when wrapped in [`Action::Discard`].
    pub called_by: Player,
    /// Whether this tile was discarded as a part of declaring riichi.
    pub declares_riichi: bool,
    /// Whether this tile was discarded immediately after being drawn (ツモ切り).
    pub is_tsumokiri: bool,
}

impl RoundBegin {
    /// Returns the initial state of a round, with all 4 players' initial hand dealt (13 x 4).
    pub fn to_initial_state(&self) -> State {
        let wall = &self.wall;
        let button = self.round_id.button();
        State {
            seq: 0,
            action_player: button,
            num_drawn_head: 53,
            num_drawn_tail: 0,
            num_dora_indicators: 0,
            draw: Some(self.wall[52]),
            incoming_meld: None,
            closed_hands: wall::deal(wall, button),
            discards: [vec![], vec![], vec![], vec![]],
            furiten: Default::default(),
            riichi: Default::default(),
            melds: [vec![], vec![], vec![], vec![]],
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert2::check;
    use itertools::Itertools;

    #[test]
    fn print_state_size() {
        dbg!(std::mem::size_of::<State>());
    }

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
