use log::log_enabled;

use crate::{
    analysis::{Decomposer, WaitingInfo},
    common::*,
    model::*,
};

pub struct EngineCache {
    /// Local decomposer instance for simplifying ownership.
    /// All regular hand decompositions are performed through this cache anyway.
    pub decomposer: Decomposer<'static>,

    /// Pending meld declared by each player, either action or reaction.
    pub meld: [Option<Meld>; 4],

    /// Pending wins declared by each player, either action (tsumo) or reaction (ron).
    /// Note that _all_ win candidates are cached; optimization for points is deferred.
    pub win: [Vec<AgariCandidate>; 4],

    /// Full (3N + 1) hand waiting decomposition cache for each player.
    /// - Initialized when jumped to a new state.
    /// - Updated when a player's hand returns to (3N + 1) form.
    pub wait: [WaitingInfo; 4],
}

impl EngineCache {
    pub fn new() -> Self {
        Self {
            decomposer: Decomposer::new(),

            meld: Default::default(),
            win: Default::default(),
            wait: Default::default(),
        }
    }

    pub fn init_wait_cache(&mut self, hands: &[TileSet37; 4]) {
        for player in ALL_PLAYERS {
            self.wait[player.to_usize()] = WaitingInfo::from_keys(
                &mut self.decomposer,
                &hands[player.to_usize()].packed_34());
        }
    }

    pub fn update_wait_cache(&mut self, player: Player, hand: &TileSet37) {
        self.wait[player.to_usize()] = WaitingInfo::from_keys(
            &mut self.decomposer, &hand.packed_34());

        if log_enabled!(log::Level::Trace) {
            // This is very noisy --- called every turn. Please turn on with care.
            log::debug!("updated waiting cache for P{} (hand={}): {}",
                player.to_usize(), hand, self.wait[player.to_usize()]);
        }
    }
}

impl Default for EngineCache {
    fn default() -> Self { Self::new() }
}
