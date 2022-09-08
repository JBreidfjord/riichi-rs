//! Core game logic, i.e. state transitions.

mod action;
mod agari;
mod step;
mod reaction;
mod scoring;
pub mod utils;
mod wait_calc;

use std::default::Default;

use crate::{
    analysis::Decomposer,
    common::*,
    model::*,
};
use self::{
    reaction::{check_reaction, resolve_reaction},
    action::check_action,
    step::{next_normal, next_agari, next_abort}
};
pub use self::{
    action::ActionError,
    reaction::ReactionError,
    scoring::*,
    wait_calc::WaitingInfo,
};

// TODO(summivox): rules (riichi sticks)
const RIICHI_POT: GamePoints = 1000;

#[derive(Default)]
pub struct Engine {
    begin: RoundBegin,
    state: State,
    action: Option<Action>,
    reactions: [Option<Reaction>; 4],
    end: Option<RoundEnd>,

    cache: EngineCache,
}

pub(crate) struct EngineCache {
    /// Local decomposer instance for simplifying ownership.
    /// All regular hand decomposition is performed through this cache anyway.
    decomposer: Decomposer<'static>,

    /// Pending meld declared by each player, either action or reaction.
    meld: [Option<Meld>; 4],

    /// Pending wins declared by each player, either action (tsumo) or reaction (ron).
    /// Note that _all_ win candidates are cached; optimization for points is deferred.
    win: [Vec<AgariCandidate>; 4],

    /// Full (3N + 1) hand waiting decomposition cache for each player.
    /// - Initialized when jumped to a new state.
    /// - Updated when a player's hand returns to (3N + 1) form.
    wait: [WaitingInfo; 4],
}

impl EngineCache {
    fn new() -> Self {
        Self {
            decomposer: Decomposer::new(),

            meld: Default::default(),
            win: Default::default(),
            wait: Default::default(),
        }
    }

    fn init_wait_cache(&mut self, hands: &[TileSet37; 4]) {
        for player in all_players() {
            self.wait[player.to_usize()] = WaitingInfo::from_keys(
                &mut self.decomposer,
                &hands[player.to_usize()].packed());
        }
    }

    fn update_wait_cache(&mut self, player: Player, hand: &TileSet37) {
        self.wait[player.to_usize()] = WaitingInfo::from_keys(
            &mut self.decomposer, &hand.packed());
    }
}

impl Default for EngineCache {
    fn default() -> Self { Self::new() }
}

impl Engine {
    pub fn state(&self) -> &State { &self.state }
    pub fn end(&self) -> &Option<RoundEnd> { &self.end }

    pub fn begin_round(&mut self, begin: RoundBegin) -> &mut Self {
        self.begin = begin;
        self.state = self.begin.to_initial_state();
        self.action = Default::default();
        self.reactions = Default::default();
        self.end = None;
        self.cache.init_wait_cache(&self.state.closed_hands);
        self
    }

    pub fn jump_to_state(&mut self, state: State) -> &mut Self {
        // sanity check: must have valid begin
        debug_assert!(wall::is_valid_wall(self.begin.wall));

        self.state = state;
        self.action = Default::default();
        self.reactions = Default::default();
        self.end = None;
        self.cache.init_wait_cache(&self.state.closed_hands);
        self
    }

    pub fn register_action(&mut self, action: Action) -> Result<&mut Self, ActionError> {
        // sanity check: must have valid state
        assert!(self.state.num_drawn_head >= 52);

        self.action = None;
        self.reactions = Default::default();
        self.cache.meld = Default::default();
        self.cache.win = Default::default();
        check_action(&self.begin, &self.state, action, &mut self.cache)?;
        self.action = Some(action);
        Ok(self)
    }

    pub fn register_reaction(&mut self, reactor: Player, reaction: Reaction)
        -> Result<&mut Self, ReactionError> {
        self.reactions[reactor.to_usize()] = None;
        check_reaction(
            &self.begin,
            &self.state,
            self.action.unwrap(),
            reactor,
            reaction,
            &mut self.cache)?;
        self.reactions[reactor.to_usize()] = Some(reaction);
        Ok(self)
    }

    pub fn step(&mut self) -> ActionResult {
        let action = self.action.unwrap();
        let action_result = resolve_reaction(&self.state, action, &self.reactions);
        if action_result.is_abort() {
            self.end = Some(
                next_abort(&self.begin, &self.state, action_result, &self.cache));
        } else if action_result.is_agari() {
            self.end = Some(
                next_agari(&self.begin, &self.state, &self.reactions, action_result, &self.cache));
        } else {
            next_normal(
                &self.begin, &mut self.state, action, &self.reactions, action_result, &self.cache);
        }
        action_result
    }
}
