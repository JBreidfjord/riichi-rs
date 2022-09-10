//! Driver of the main game logic.

mod action;
mod agari;
mod cache;
mod step;
mod reaction;
mod scoring;
pub mod utils;

use std::default::Default;

use crate::{
    common::*,
    model::*,
};
use self::{
    action::check_action,
    cache::EngineCache,
    reaction::{check_reaction, resolve_reaction},
    step::{next_normal, next_agari, next_abort}
};
pub use self::{
    action::ActionError,
    reaction::ReactionError,
    scoring::*,
};

// TODO(summivox): rules (riichi sticks)
const RIICHI_POT: GamePoints = 1000;

/// Driver of the main game logic.
///
/// This has the following main functions:
///
/// - Given [`State`], is some [`Action`] valid?
/// - Given [`State`] and a valid [`Action`], is some [`Reaction`] valid?
/// - Given [`State`] and valid [`Action`] and any valid [`Reaction`]s, proceed to the next state.
///
/// For improved efficiency, valid [`Action`] and [`Reaction`]s, together with cached info from
/// their validation, are cached here for deriving the next state.
///
/// Example:
/// ```
/// use riichi::prelude::*;  // includes `Engine`
/// let mut engine = Engine::new();
///
/// engine.begin_round(RoundBegin {
///     rules: Default::default(),
///     round_id: RoundId { kyoku: 0, honba: 0 },
///     wall: wall::make_sorted_wall([1, 1, 1]),
///     pot: 0,
///     points: [25000, 25000, 25000, 25000],
/// });
/// assert_eq!(engine.state().core.seq, 0);
/// assert_eq!(engine.state().core.action_player, P0);
///
/// engine.register_action(Action::Discard(Discard {
///     tile: t!("1m"), ..Discard::default()})).unwrap();
///
/// assert_eq!(engine.step(), ActionResult::Pass);
///
/// assert_eq!(engine.state().core.seq, 1);
/// assert_eq!(engine.state().core.action_player, P1);
/// /* ... */
/// ```
#[derive(Default)]
pub struct Engine {
    begin: RoundBegin,
    state: State,
    action: Option<Action>,
    reactions: [Option<Reaction>; 4],
    end: Option<RoundEnd>,

    cache: EngineCache,
}

impl Engine {
    /// Creates an empty engine.
    pub fn new() -> Self { Default::default() }

    /// Returns the current game state.
    pub fn state(&self) -> &State { &self.state }

    /// Returns the end-of-round conclusions if the round has ended.
    pub fn end(&self) -> &Option<RoundEnd> { &self.end }

    /// Set up the engine for the specified round.
    /// The state is initialized to the beginning of this round, ready for the button player to
    /// take action.
    pub fn begin_round(&mut self, begin: RoundBegin) -> &mut Self {
        self.begin = begin;
        self.state = State::new(&self.begin);
        self.action = Default::default();
        self.reactions = Default::default();
        self.end = None;
        self.cache.init_wait_cache(&self.state.closed_hands);
        self
    }

    /// Within the same round, resets the engine to start from the given state.
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

    /// Validates the given action against the current state, then caches it in the engine if valid.
    pub fn register_action(&mut self, action: Action) -> Result<&mut Self, ActionError> {
        // sanity check: must have valid state
        assert!(self.state.core.num_drawn_head >= 53);

        self.action = None;
        self.reactions = Default::default();
        self.cache.meld = Default::default();
        self.cache.win = Default::default();
        check_action(&self.begin, &self.state, action, &mut self.cache)?;
        self.action = Some(action);
        Ok(self)
    }

    /// Validates the given reaction against the current state and cached action, then caches it if
    /// valid.
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

    /// Resolves the cached actions and reactions into the conclusion of this turn, then updates
    /// the state to the beginning of the next turn, or determines the end-of-round conclusions if
    /// the round has ended.
    pub fn step(&mut self) -> ActionResult {
        let actor = self.state.core.action_player;
        let action = self.action.unwrap();
        let (action_result, reactor_reaction) =
            resolve_reaction(&self.state, action, &self.reactions);
        match action_result {
            ActionResult::Pass | ActionResult::CalledBy(_) => {
                let next_core = next_normal(
                    &self.begin, &self.state, action, action_result, &self.cache);
                self.state.apply_step(&GameStep {
                    actor,
                    action,
                    reactor_reaction,
                    action_result,
                    next: Some(next_core),
                });
            }
            ActionResult::Agari(agari_kind) => {
                self.end = Some(next_agari(
                    &self.begin, &self.state, action, &self.reactions, agari_kind, &self.cache));
            }
            ActionResult::Abort(abort_reason) => {
                self.end = Some(next_abort(&self.begin, &self.state, abort_reason, &self.cache));
            }
        }
        action_result
    }
}
