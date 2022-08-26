//! State-Action representation of the game.
//!
//! This module mainly provides data model definitions and some straightforward helpers.
//! Game logic belongs to [`crate::engine`].
mod actions;
mod agari;
mod boundary;
mod state;

pub use actions::*;
pub use agari::*;
pub use boundary::*;
pub use state::*;

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
    pub called_by: Player,
    /// Whether this tile was discarded as a part of declaring riichi.
    pub declares_riichi: bool,
    /// Whether this tile was discarded immediately after being drawn (ツモ切り).
    pub is_tsumokiri: bool,
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
