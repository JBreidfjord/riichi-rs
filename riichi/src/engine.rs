//! Core game logic, i.e. state transitions.

pub mod agari;
pub mod wait_calc;
pub mod errors;
pub mod utils;

use agari::*;
pub use errors::*;
use utils::*;
pub use wait_calc::WaitingInfo;

use std::cell::{Cell, RefCell};

use itertools::Itertools;

use crate::analysis::Decomposer;
use crate::common::*;
use crate::model::*;
use crate::wall::is_valid_wall;

pub struct Engine {
    /// Keep a ref to the decomposer.
    decomposer: RefCell<Decomposer<'static>>,

    begin: RoundBegin,
    state: State,
    action: Option<Action>,
    reactions: [Option<Reaction>; 4],
    end: Option<RoundEnd>,

    /// Pending meld declared by each player, either action or reaction.
    meld_cache: [Cell<Option<Meld>>; 4],

    /// Pending win declared by each player, either action (tsumo) or reaction (ron).
    win_cache: [Cell<Option<AgariResult>>; 4],

    /// Full (3N + 1) hand waiting decomposition cache for each player.
    /// - Initialized when jumped to a new state.
    /// - Updated when a player's hand returns to (3N + 1) form.
    wait_cache: [RefCell<WaitingInfo>; 4],
}

impl Engine {
    // TODO(summivox): rules (riichi sticks)
    const RIICHI_POT: GamePoints = 1000;

    pub fn new() -> Self {
        Self {
            decomposer: RefCell::new(Decomposer::new()),

            begin: Default::default(),
            state: Default::default(),
            action: Default::default(),
            reactions: Default::default(),
            end: None,

            meld_cache: Default::default(),
            win_cache: Default::default(),
            wait_cache: Default::default(),
        }
    }

    pub fn state(&self) -> &State { &self.state }

    fn init_wait_cache(&self) {
        let mut decomposer = self.decomposer.borrow_mut();
        for player in all_players() {
            self.wait_cache[player.to_usize()].replace(WaitingInfo::from_keys(
                &mut decomposer,
                &self.state.closed_hands[player.to_usize()].packed()));
        }
    }

    fn update_wait_cache(&self, player: Player, hand: &TileSet37) {
        self.wait_cache[player.to_usize()].replace(WaitingInfo::from_keys(
            &mut self.decomposer.borrow_mut(), &hand.packed()));
    }

    pub fn begin_round(&mut self, begin: RoundBegin) -> &mut Self {
        self.meld_cache = Default::default();
        self.win_cache = Default::default();
        self.begin = begin;
        self.state = self.begin.to_initial_state();
        self.action = Default::default();
        self.reactions = Default::default();
        self.end = None;
        self.init_wait_cache();
        self
    }

    pub fn jump_to_state(&mut self, state: State) -> &mut Self {
        // sanity check: must have valid begin
        debug_assert!(is_valid_wall(self.begin.wall));

        self.meld_cache = Default::default();
        self.win_cache = Default::default();
        self.state = state;
        self.action = Default::default();
        self.reactions = Default::default();
        self.end = None;
        self.init_wait_cache();
        self
    }

    pub fn register_action(&mut self, action: Action) -> Result<&mut Self, ActionError> {
        // sanity check: must have valid state
        assert!(self.state.num_drawn_head >= 52);

        self.meld_cache = Default::default();
        self.win_cache = Default::default();
        self.reactions = Default::default();
        self.check_action(action)?;
        self.action = Some(action);
        Ok(self)
    }

    fn check_action(&self, action: Action) -> Result<(), ActionError> {
        use ActionError::*;

        let state = &self.state;
        let actor = state.action_player;
        let actor_i = actor.to_usize();

        // Make a copy of `actor`'s hand; this will be used to determine its
        let mut hand = state.closed_hands[actor.to_usize()];
        if let Some(draw) = state.draw {
            hand[draw] += 1;
        };

        let under_riichi = state.riichi[actor_i].is_active;

        match action {
            Action::Discard(discard) => {
                // D'oh!
                if under_riichi && discard.declares_riichi { return Err(DeclareRiichiAgain); }

                // Discarded tile must be either just drawn, or already in our closed hand.
                if discard.is_tsumokiri {
                    if state.draw != Some(discard.tile) {
                        return Err(TsumokiriMismatch(discard.tile, state.draw));
                    }
                } else {
                    if under_riichi {
                        return Err(DiscardClosedHandUnderRiichi);
                    }
                }
                if hand[discard.tile] == 0 { return Err(TileNotExist(discard.tile)); }
                hand[discard.tile] -= 1;
                self.update_wait_cache(actor, &hand);

                // Declaring riichi requires a closed 3N+1 ready (tenpai) hand after discarding.
                if discard.declares_riichi {
                    if self.begin.points[actor_i] < Self::RIICHI_POT {
                        // TODO(summivox): rules (riichi sticks)
                        return Err(DeclareRiichiWithoutPoints);
                    }
                    // Ankan is considered closed; all other melds are not ok.
                    if state.melds[actor_i]
                        .iter()
                        .any(|meld| !matches!(meld, &Meld::Ankan(_)))
                    {
                        return Err(DeclareRiichiWithOpenMeld);
                    }
                    if self.wait_cache[actor_i].borrow().waiting_set.is_empty() {
                        return Err(DeclareRiichiWhileNotReady);
                    }
                }

                if let Some(meld) = state.incoming_meld {
                    if is_forbidden_swap_call(meld, discard.tile) {
                        return Err(NoSwapCalling(discard.tile, meld));
                    }
                }
            }

            Action::Ankan(tile) => {
                let tile = tile.to_normal();
                if under_riichi && !is_ankan_ok_under_riichi(
                    &self.wait_cache[actor_i].borrow().regular, tile) {
                    return Err(InvalidAnkanUnderRiichi(tile));
                }
                if let Some(ankan) = Ankan::from_hand(&hand, tile) {
                    ankan.consume_from_hand(&mut hand);
                    self.meld_cache[actor_i].set(Some(Meld::Ankan(ankan)));
                    self.update_wait_cache(actor, &hand);
                } else {
                    return Err(NotEnoughForAnkan(tile));
                }
            }
            Action::Kakan(added) => {
                let &pon = state.melds[actor_i]
                    .iter()
                    .filter_map(|meld| {
                        if let Meld::Pon(pon) = meld {
                            if pon.called.to_normal() == added.to_normal() {
                                return Some(pon);
                            }
                        }
                        None
                    })
                    .exactly_one()
                    .map_err(|_| NoPonForKakan(added))?;
                if let Some(kakan) = Kakan::from_pon_hand(pon, &hand) {
                    kakan.consume_from_hand(&mut hand);
                    self.meld_cache[actor_i].set(Some(Meld::Kakan(kakan)));
                    self.update_wait_cache(actor, &hand);
                } else {
                    return Err(TileNotExist(added));
                }
            }

            Action::TsumoAgari(tile) => {
                if state.draw != Some(tile) { return Err(MustDeclareTsumoAgariOnDraw); }
                // TODO(summivox): agari
            }
            Action::AbortNineKinds => {
                if !is_init_abortable(state) { return Err(NotInitAbortable); }
                let n = hand.terminal_kinds();
                if n < 9 {
                    return Err(NotEnoughKindsForNineKinds(n));
                }
            }
        }
        Ok(())
    }

    pub fn register_reaction(&mut self, reactor: Player, reaction: Reaction)
        -> Result<&mut Self, ReactionError> {
        assert!(self.action.is_some());
        self.reactions[reactor.to_usize()] = None;
        self.check_reaction(reactor, reaction)?;
        self.reactions[reactor.to_usize()] = Some(reaction);
        Ok(self)
    }

    fn check_reaction(&self, reactor: Player, reaction: Reaction) -> Result<(), ReactionError> {
        use ReactionError::*;

        let action = self.action.ok_or(NoAction)?;
        if action.is_terminal() { return Err(TerminalAction); }

        let state = &self.state;
        let actor = state.action_player;
        let reactor_i = reactor.to_usize();
        let hand = &state.closed_hands[reactor_i];

        match reaction {
            Reaction::Chii(own0, own1) => {
                if state.riichi[reactor_i].is_active { return Err(MeldUnderRiichi); }
                if player_succ(actor) != reactor {
                    return Err(CanOnlyChiiPrevPlayer);
                }
                if let Action::Discard (discard) = action {
                    let called = discard.tile;
                    if let Some(chii) = Chii::from_tiles(own0, own1, called) {
                        if chii.is_in_hand(hand) {
                            self.meld_cache[reactor_i].set(Some(Meld::Chii(chii)));
                        } else {
                            return Err(InvalidChii(own0, own1, called));
                        }
                    } else {
                        return Err(InvalidChii(own0, own1, called));
                    }
                } else {
                    return Err(NotDiscard(action));
                }
            }

            Reaction::Pon(own0, own1) => {
                if state.riichi[reactor_i].is_active { return Err(MeldUnderRiichi); }
                if let Action::Discard(discard) = action {
                    let called = discard.tile;
                    let dir = actor.wrapping_sub(reactor);
                    if let Some(pon) =Pon::from_tiles_dir(own0, own1, called, dir) {
                        if pon.is_in_hand(hand) {
                            self.meld_cache[reactor_i].set(Some(Meld::Pon(pon)));
                        } else {
                            return Err(InvalidPon(own0, own1, called));
                        }
                    } else {
                        return Err(InvalidPon(own0, own1, called));
                    }
                } else {
                    return Err(NotDiscard(action));
                }
            }

            Reaction::Daiminkan => {
                if state.riichi[reactor_i].is_active { return Err(MeldUnderRiichi); }
                if let Action::Discard(discard) = action {
                    let called = discard.tile;
                    let dir = actor.wrapping_sub(reactor);
                    if let Some(daiminkan) = Daiminkan::from_hand_dir(hand, called, dir) {
                        self.meld_cache[reactor_i].set(Some(Meld::Daiminkan(daiminkan)));
                    } else {
                        return Err(InvalidDaiminkan);
                    }
                } else {
                    return Err(NotDiscard(action));
                }
            }

            Reaction::RonAgari => {
                // TODO(summivox): all kinds of fun stuff here:
                // furiten(done), agari, chankan, kokushi-ankan, ...
                // Also the "agari summary" should be cached.
                if state.furiten[reactor_i].any() { return Err(Furiten(state.furiten[reactor_i])); }
            }
        }
        Ok(())
    }

    pub fn next(&mut self) -> ActionResult {
        let action = self.action.unwrap();
        let action_result = self.resolve_reaction();
        if action_result.is_abort() {
            self.end = Some(self.next_abort(action_result));
        } else if action_result.is_agari() {
            self.end = Some(self.next_agari(action_result));
        } else {
            self.next_normal(action, action_result);
        }
        action_result
    }

    fn resolve_reaction(&self) -> ActionResult {
        let state = &self.state;
        let actor = state.action_player;
        let action = self.action.unwrap();

        // Handle in-turn voluntary termination.
        match action {
            Action::TsumoAgari(_) => return ActionResult::TsumoAgari,
            Action::AbortNineKinds => return ActionResult::AbortNineKinds,
            _ => {}
        }

        let highest_priority_reaction = other_players_after(actor).into_iter()
            .flat_map(|reactor| self.reactions[reactor.to_usize()]).max();
        let result = match highest_priority_reaction {
            // Meld can be preempted by:
            // - four riichi
            // - four kan
            // - wall exhausted (see <https://riichi.wiki/Haitei_raoyue_and_houtei_raoyui>)
            //
            // Meld does not conflict with:
            // - four wind: 4th wind cannot be called
            Some(Reaction::Chii(_, _)) => ActionResult::Chii,
            Some(Reaction::Pon(_, _)) => ActionResult::Pon,
            Some(Reaction::Daiminkan) => ActionResult::Daiminkan,

            // Ron takes precedence over everything else at this point.
            Some(Reaction::RonAgari) => {
                // Triple win => Abort
                // TODO(summivox): rules (double/triple ron)
                let num_rons = self.reactions.into_iter()
                    .filter(|&reaction| reaction == Some(Reaction::RonAgari))
                    .count();
                return if num_rons == 3 {
                    ActionResult::AbortMultiRon
                } else {
                    ActionResult::RonAgari
                }
            }
            None => ActionResult::Pass
        };

        if is_aborted_four_wind(state, action) { return ActionResult::AbortFourWind; }
        if is_aborted_four_riichi(state, action) { return ActionResult::AbortFourRiichi; }
        if is_aborted_four_kan(state, action, result) {
            return ActionResult::AbortFourKan;
        }
        if result == ActionResult::Pass && is_wall_exhausted(state) {
            return if is_any_player_nagashi_mangan(state) {
                ActionResult::AbortNagashiMangan
            } else {
                ActionResult::AbortWallExhausted
            }
        }

        result
    }

    /// Process normal end-of-turn flow (no abort, no win).
    /// Each change to the state is processed in chronological order, gradually morphing the current
    /// state to the next.
    fn next_normal(&mut self, action: Action, action_result: ActionResult) {
        let state = &mut self.state;
        let actor = state.action_player;
        let actor_i = actor.to_usize();

        // Special case: Deferred revealing of new dora indicators due to Kakan/Daiminkan.
        // Why this is special:
        //
        // - **Timing**: The player who did a Kakan/Daiminkan has finished their turn after drawing
        //   from the tail of the wall. Revealing now makes sure that the player did not know what
        //   the new dora indicator is right after making the call.
        //
        // - **Information**: This solely relies on the result of the previous turn. Actions and
        //   reactions during this turn has no effect on this.
        //
        // TODO(summivox): rules (kan-dora)
        match state.incoming_meld {
            Some(Meld::Kakan(_)) | Some(Meld::Daiminkan(_)) => {
                state.num_dora_indicators += 1;
            }
            _ => {}
        }

        // After handling the only special case, we can start mutating the current state from
        // begin of this turn to begin of the next turn.

        state.seq += 1;

        // Commit the draw to the player's closed hand.
        if let Some(draw) = state.draw {
            state.closed_hands[actor_i][draw] += 1;
        };

        // Commit the action.
        // Note that the round has not ended. This means if there's an reaction, it must be a call
        // (Chii/Pon/Daiminkan) on this turn's Discard. Therefore we can merge reaction handling
        // into [`Action::Discard`].
        match action {
            Action::Discard(discard) => {
                // If there is indeed a call (according to `action_result`), find who called.
                let caller =
                    other_players_after(actor).into_iter().filter(|reactor|
                        match self.reactions[reactor.to_usize()] {
                            Some(Reaction::Chii(_, _)) => action_result == ActionResult::Chii,
                            Some(Reaction::Pon(_, _)) => action_result == ActionResult::Pon,
                            Some(Reaction::Daiminkan) => action_result == ActionResult::Daiminkan,
                            _ => false
                        }
                    ).exactly_one().unwrap_or(actor);

                // Commit the discard first.
                state.closed_hands[actor_i][discard.tile] -= 1;
                state.discards[actor_i].push(Discard { called_by: caller, ..discard });

                // Handle both existing and new riichi.
                if state.riichi[actor_i].is_active {
                    // Ippatsu naturally expires after the first discard since declaring riichi.
                    state.riichi[actor_i].is_ippatsu = false;
                } else if discard.declares_riichi {
                    // Round has not ended => the new riichi is successful.
                    state.riichi[actor_i] = RiichiFlags {
                        is_active: true,
                        is_ippatsu: caller == actor,  // no ippatsu if immediately called
                        is_double: is_init_abortable(state),
                    }
                }

                if caller == actor {
                    // No one called. Next turn is the next player (surprise!).
                    state.action_player = player_succ(actor);
                    state.incoming_meld = None;
                    // state.melds => no-op
                    state.draw = Some(self.begin.wall[state.num_drawn_head as usize]);
                    state.num_drawn_head += 1;
                } else {
                    // Someone called and will take the next turn instead.
                    let meld = self.meld_cache[caller.to_usize()].get().unwrap();
                    meld.consume_from_hand(&mut state.closed_hands[caller.to_usize()]);

                    state.action_player = caller;
                    state.incoming_meld = Some(meld);
                    state.melds[caller.to_usize()].push(meld);
                    state.draw = None;
                    // state.num_drawn_* => no-op
                }

                // Check Furiten status for the discarding player.
                // furiten-by-discard == some tile in the waiting set exists in the discard set
                if !state.furiten[actor_i].by_discard && !state.furiten[actor_i].miss_permanent {
                    let discard_set = TileMask34::from_iter(
                        state.discards[actor_i].iter().map(|discard| discard.tile));
                    let waiting_set = self.wait_cache[actor_i].borrow().waiting_set;

                    state.furiten[actor_i].by_discard = discard_set.0 & waiting_set.0 > 0
                }
                // Temporary miss expires after discarding.
                state.furiten[actor_i].miss_temporary = false;

            }

            Action::Ankan(_) | Action::Kakan(_) => {
                // The current player has made an Ankan/Kakan and is entitled to a bonus turn.
                // The round has not ended => no reaction is possible on this.
                let ankan_or_kakan = self.meld_cache[actor_i].get().unwrap();
                ankan_or_kakan.consume_from_hand(&mut state.closed_hands[actor_i]);

                state.action_player = actor;
                state.incoming_meld = Some(ankan_or_kakan);
                state.melds[actor_i].push(ankan_or_kakan);
                state.draw = Some(wall::kan_draw(
                    &self.begin.wall, state.num_drawn_tail as usize));
                state.num_drawn_tail += 1;

                // Only for Ankan: reveal the next dora indicator immediately.
                // For Kakan, it will only be revealed at the end of the next turn, in the same way
                // as Daiminkan (see above).
                // TODO(summivox): rules (kan-dora)
                if let Action::Ankan(_) = action {
                    state.num_dora_indicators += 1;
                }
            }

            Action::TsumoAgari(_) | Action::AbortNineKinds => panic!()
        }

        // Any kind of meld will forcefully break riichi ippatsu.
        if state.incoming_meld.is_some() {
            for player in all_players() {
                state.riichi[player.to_usize()].is_ippatsu = false;
            }
        }

        // Check Furiten status for other players.
        // For another player who misses the chance to win (discard in waiting set):
        // - Immediately enters temporary miss state
        // - Immediately enters permanent miss state if under riichi
        // TODO(summivox): ankan should only affects kokushi-tenpai here, although kakan is treated
        //     the same as Ron.
        // TODO(summivox): rules (kokushi-ankan)
        for other_player in other_players_after(actor) {
            let other_player_i = other_player.to_usize();
            let furiten = &mut state.furiten[other_player_i];

            if furiten.miss_permanent { continue; }
            if self.wait_cache[other_player_i].borrow().waiting_set.has(action.tile().unwrap()) {
                furiten.miss_temporary = true;
                furiten.miss_permanent = state.riichi[other_player_i].is_active;
            }
        }
    }

    fn next_agari(&self, action_result: ActionResult) -> RoundEnd {
        // TODO(summivox): agari
        RoundEnd {
            round_result: action_result,
            pot: 0,
            points: [0; 4],
            points_delta: [0; 4],
            renchan: false,
            next_round_id: None,
            agari_result: Default::default(),
        }
    }

    fn next_abort(&self, action_result: ActionResult) -> RoundEnd {
        // TODO(summivox): rules (riichi sticks)
        let riichi_pot = Self::RIICHI_POT;

        let mut end = RoundEnd {
            round_result: action_result,
            pot: self.begin.pot + (num_active_riichi(&self.state) as GamePoints * riichi_pot),
            points: self.begin.points,
            ..RoundEnd::default()
        };

        let round_id = self.begin.round_id;
        // ugly syntax gets around array::map moving the Vec value
        let waiting = [0, 1, 2, 3].map(|i| self.wait_cache[i].borrow().waiting_set.any() as u8);
        match action_result {
            ActionResult::AbortWallExhausted | ActionResult::AbortNagashiMangan => {
                // The latter is only a special case of the former, with points delta being the
                // only real distinction. Therefore, we merge the handling.
                (end.points_delta, end.renchan) =
                    resolve_wall_exhausted(&self.state, waiting, round_id.button());
                end.next_round_id = Some(round_id.next_honba(end.renchan));
            }

            ActionResult::AbortFourKan | ActionResult::AbortFourWind |
            ActionResult::AbortFourRiichi | ActionResult::AbortMultiRon => {
                // force renchan with honba + 1
                end.renchan = true;
                end.next_round_id = Some(round_id.next_honba(true));
            }

            _ => panic!()
        }

        for i in 0..4 { end.points[i] += end.points_delta[i]; }

        end
    }
}
