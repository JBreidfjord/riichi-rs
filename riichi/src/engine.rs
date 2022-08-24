//! Core game logic, i.e. state transitions.

mod utils;
use utils::*;

use itertools::Itertools;
use thiserror::Error;

use crate::analysis::{Decomposer, FullHandWaitingPattern};
use crate::common::*;
use crate::model::*;

#[derive(Error, Debug)]
pub enum ActionError {
    #[error("Tsumokiri discard tile {0} != drawn tile {1:?}")]
    TsumokiriMismatch(Tile, Option<Tile>),

    #[error("Discarding from the closed hand while under riichi.")]
    DiscardClosedHandUnderRiichi,

    #[error("Tile {0} does not exist in the closed hand.")]
    TileNotExist(Tile),

    #[error("Attempting to declare riichi with an open hand.")]
    DeclareRiichiWithOpenMeld,

    #[error("Attempting to declare riichi with a hand not ready after discarding.")]
    DeclareRiichiWhileNotReady,

    #[error("Attempting invalid ankan on {0} under riichi.")]
    InvalidAnkanUnderRiichi(Tile),

    #[error("Cannot ankan on {0} with only {} in hand.")]
    NotEnoughForAnkan(Tile, u8),

    #[error("Attempting kakan on {0} without corresponding pon.")]
    NoPonForKakan(Tile),

    #[error("Cannot kyuushuukyuuhai with only {0} kinds of terminals in hand.")]
    NotEnoughForKyuushuukyuuhai(u8),

    #[error("Cannot abort after the first go-around.")]
    NotInitAbortable,
}

#[derive(Error, Debug)]
pub enum ReactionError {
    #[error("No action to react to.")]
    NoAction,

    #[error("The action is terminal; no reactions possible.")]
    TerminalAction,

    #[error("Cannot declare an open meld under riichi.")]
    MeldUnderRiichi,

    #[error("Tile {0} does not exist in the closed hand.")]
    TileNotExist(Tile),

    #[error("You can only call a discarded tile (is actually {0:?})")]
    NotDiscard(Action),

    #[error("Can only Chii on the previous player's discard.")]
    CanOnlyChiiPrevPlayer,

    #[error("Cannot Chii {2} with own {0}{1}.")]
    InvalidChii(Tile, Tile, Tile),

    #[error("Cannot Pon {2} with own {0}{1}.")]
    InvalidPon(Tile, Tile, Tile),

    #[error("Cannot Daiminkan.")]
    InvalidDaiminkan,

    #[error("No ron when you are furiten: {0:?}")]
    Furiten(FuritenFlags),
}

pub struct Engine {
    /// Keep a ref to the decomposer.
    decomposer: Decomposer<'static>,

    begin: RoundBeginState,
    s: PreActionState,
    action: Option<Action>,
    reactions: [Option<Reaction>; 4],

    /// The closed hand after the player takes action --- including draw and discard.
    hand_after_action: TileSet37,

    /// Full hand waiting decomposition cache for each player.
    /// Computed after the action is registered.
    wait_cache: [Vec<FullHandWaitingPattern>; 4],

    /// Target meld made by the player, either action or reaction.
    meld_cache: [Option<Meld>; 4],
}

impl Engine {
    const RIICHI_POT: GamePoints = 1000;
    const NO_WAIT_PENALTY_TOTAL: GamePoints = 3000;

    pub fn new() -> Self {
        Self {
            decomposer: Decomposer::new(),

            begin: Default::default(),
            s: Default::default(),
            action: Default::default(),
            reactions: Default::default(),

            hand_after_action: Default::default(),
            wait_cache: Default::default(),
            meld_cache: Default::default(),
        }
    }

    fn clear_cache(&mut self) {
        self.action = Default::default();
        self.reactions = Default::default();

        self.hand_after_action = Default::default();
        self.wait_cache = Default::default();
        self.meld_cache = Default::default();
    }

    pub fn pre_action(&mut self, pre_action: PreActionState) -> &mut Self {
        self.clear_cache();
        self.s = pre_action;
        self
    }

    pub fn action(&mut self, action: Action) -> Result<&mut Self, ActionError> {
        self.clear_cache();  // not redundant
        self.check_action(action)?;
        self.action = Some(action);
        self.cache_wait_for_all();
        Ok(self)
    }

    fn check_action(&mut self, action: Action) -> Result<(), ActionError> {
        use ActionError::*;

        let s = &self.s;
        let p = s.action_player;
        let pp = p.to_usize();

        let mut hand = s.closed_hands[p.to_usize()];
        if let Some(draw) = s.draw {
            hand[draw] += 1;
        };

        match action {
            Action::Discard {
                tile,
                riichi,
                tsumokiri,
            } => {
                // Discarded tile must be either just drawn, or already in our closed hand.
                if tsumokiri {
                    if s.draw != Some(tile) {
                        return Err(TsumokiriMismatch(tile, s.draw));
                    }
                } else {
                    if s.riichi[pp].is_active {
                        return Err(DiscardClosedHandUnderRiichi);
                    }
                }
                if hand[tile] == 0 { return Err(TileNotExist(tile)); }
                hand[tile] -= 1;

                // Declaring riichi requires a closed 3N+1 ready (tenpai) hand after discarding.
                if riichi {
                    // Ankan is considered closed; all other melds are not ok.
                    if s.melds[pp]
                        .iter()
                        .any(|meld| !matches!(meld, &Meld::Ankan(_)))
                    {
                        return Err(DeclareRiichiWithOpenMeld);
                    }
                    if self.decomposer.with_tile_set(hand.into())
                        .iter().next().is_none() {
                        return Err(DeclareRiichiWhileNotReady);
                    }
                }

                if let Some(Meld::Chii(chii)) = s.incoming_meld {
                    // TODO(summivox): check kui-kae
                }
            }
            Action::Ankan(tile) => {
                let tile = tile.to_normal();
                if s.riichi[pp].is_active {
                    // TODO(summivox): check ankan-riichi conflict using 3N+1 tenpai
                    if false {
                        return Err(InvalidAnkanUnderRiichi(tile));
                    }
                }

                let (num_normal, num_red) = count_for_kan(&hand, tile);
                if num_normal + num_red < 4 {
                    return Err(NotEnoughForAnkan(tile, hand[tile]));
                }
                hand[tile] = 0;
                hand[tile.to_red()] = 0;
                let tiles = ankan_tiles(tile, num_red);
                self.meld_cache[pp] =
                    Ankan::from_tiles(tiles[0], tiles[1], tiles[2], tiles[3])
                        .map(|ankan| Meld::Ankan(ankan));
            }
            Action::Kakan(added) => {
                if hand[added] == 0 { return Err(TileNotExist(added)); }
                hand[added] -= 1;

                let (_i, pon) = s.melds[pp]
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &meld)| {
                        if let Meld::Pon(pon) = meld {
                            if pon.called.to_normal() == added {
                                return Some((i, pon));
                            }
                        }
                        None
                    })
                    .exactly_one()
                    .map_err(|_| NoPonForKakan(added))?;

                self.meld_cache[pp] =
                    Kakan::from_pon_added(pon, added).map(|kakan| Meld::Kakan(kakan));
            }
            Action::TsumoAgari(_tile) => {
                // TODO(summivox): agari
            }
            Action::AbortKyuushuukyuuhai => {
                if !is_init_abortable(s) { return Err(NotInitAbortable); }
                let n = hand.terminal_kinds();
                if n < 9 {
                    return Err(NotEnoughForKyuushuukyuuhai(n));
                }
            }
        }
        self.hand_after_action = hand;
        Ok(())
    }

    pub fn reaction(
        &mut self,
        reactor: Player,
        reaction: Reaction,
    ) -> Result<&mut Self, ReactionError> {
        self.reactions[reactor.to_usize()] = None;
        self.check_reaction(reactor, reaction)?;
        self.reactions[reactor.to_usize()] = Some(reaction);
        Ok(self)
    }

    fn check_reaction(&mut self, reactor: Player, reaction: Reaction) -> Result<(), ReactionError> {
        use ReactionError::*;

        let action = self.action.ok_or(NoAction)?;
        if action.is_terminal() { return Err(TerminalAction); }

        let s = &self.s;
        let p = s.action_player;
        let q = reactor;
        let qq = q.to_usize();
        // reactor's hand (copy to make presence test easier)
        let mut q_hand = s.closed_hands[qq];

        match reaction {
            Reaction::Chii(own0, own1) => {
                if s.riichi[qq].is_active { return Err(MeldUnderRiichi); }
                if q_hand[own0] == 0 { return Err(TileNotExist(own0)); }
                q_hand[own0] -= 1;
                if q_hand[own1] == 0 { return Err(TileNotExist(own1)); }
                q_hand[own1] -= 1;
                if player_succ(p) != q {
                    return Err(CanOnlyChiiPrevPlayer);
                }
                if let Action::Discard { tile: called, .. } = action {
                    if let Some(chii) = Chii::from_tiles(own0, own1, called) {
                        self.meld_cache[qq] = Some(Meld::Chii(chii));
                    } else {
                        return Err(InvalidChii(own0, own1, called));
                    }
                } else {
                    return Err(NotDiscard(action));
                }
            }
            Reaction::Pon(own0, own1) => {
                if s.riichi[qq].is_active { return Err(MeldUnderRiichi); }
                if q_hand[own0] == 0 {
                    return Err(TileNotExist(own0));
                }
                q_hand[own0] -= 1;
                if q_hand[own1] == 0 {
                    return Err(TileNotExist(own1));
                }
                q_hand[own1] -= 1;
                if let Action::Discard { tile: called, .. } = action {
                    if let Some(pon) = Pon::from_tiles_dir(own0, own1, called, p.wrapping_sub(q)) {
                        self.meld_cache[qq] = Some(Meld::Pon(pon));
                    } else {
                        return Err(InvalidPon(own0, own1, called));
                    }
                } else {
                    return Err(NotDiscard(action));
                }
            }
            Reaction::Daiminkan => {
                if s.riichi[qq].is_active { return Err(MeldUnderRiichi); }
                if let Action::Discard { tile: called, .. } = action {
                    let tile = called.to_normal();
                    let (num_normal, num_red) = count_for_kan(&q_hand, tile);
                    if num_normal + num_red != 3 {
                        return Err(TileNotExist(called.to_normal()));
                    }
                    let own = daiminkan_tiles(tile, num_red);
                    if let Some(daiminkan) = Daiminkan::from_tiles_dir(
                        own[0], own[1], own[2], called,
                        p.wrapping_sub(q)) {

                        self.meld_cache[qq] = Some(Meld::Daiminkan(daiminkan));
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
                if s.furiten[qq].any() { return Err(Furiten(s.furiten[qq])); }
            }
        }
        Ok(())
    }

    pub fn resolve_reaction(&self) -> Option<ActionResult> {
        let s = &self.s;
        let p = s.action_player;
        let action = self.action?;

        // Handle in-turn voluntary termination.
        match action {
            Action::TsumoAgari(_) => return Some(ActionResult::TsumoAgari),
            Action::AbortKyuushuukyuuhai => return Some(ActionResult::AbortKyuushuukyuuhai),
            _ => {}
        }

        let mut result = ActionResult::Pass;

        if let Some(reaction) = other_players_after(p).into_iter()
            .flat_map(|q| self.reactions[q.to_usize()]).max() {

            result = match reaction {
                // Meld can be preempted by:
                // - four riichi
                // - four kan
                // - wall exhausted (see <https://riichi.wiki/Haitei_raoyue_and_houtei_raoyui>)
                // Meld does not conflict with:
                // - four wind: 4th wind cannot be called
                Reaction::Chii(_, _) => ActionResult::Chii,
                Reaction::Pon(_, _) => ActionResult::Pon,
                Reaction::Daiminkan => ActionResult::Daiminkan,

                // Ron takes precedence over everything else at this point.
                Reaction::RonAgari => {

                    // Triple win => Abort
                    // TODO(summivox): also handle double?
                    let num_rons = self.reactions.into_iter()
                        .filter(|&rr| rr == Some(Reaction::RonAgari)).count();
                    return if num_rons == 3 {
                        Some(ActionResult::AbortMultiRon)
                    } else {
                        Some(ActionResult::RonAgari)
                    }
                }
            }
        }

        if is_aborted_four_wind(s, action) { return Some(ActionResult::AbortFourWind); }
        if is_aborted_four_riichi(s, action) { return Some(ActionResult::AbortFourRiichi); }
        if is_aborted_four_kan(s, result) { return Some(ActionResult::AbortFourKan); }
        if result == ActionResult::Pass && is_wall_exhausted(s) {
            if is_any_player_nagashi_mangan(s) { return Some(ActionResult::AbortNagashiMangan); }
            return Some(ActionResult::AbortWallExhausted);
        }

        Some(result)
    }

    pub fn next(&self) -> Option<(ActionResult, NextOrEnd)> {
        let action = self.action?;
        let action_result = self.resolve_reaction()?;
        if action_result.is_abort() {
            Some((action_result, self.next_abort(action_result)))
        } else if action_result.is_agari() {
            Some((action_result, self.next_agari(action_result)))
        } else {
            Some((action_result, self.next_normal(action, action_result)))
        }
    }

    fn next_normal(&self, action: Action, action_result: ActionResult) -> NextOrEnd {
        let p = self.s.action_player;
        let pp = p.to_usize();
        let caller;

        // Provide defaults for values completely dependent on the action-reaction.
        let mut next = PreActionState {
            action_player: player_succ(p),
            seq: self.s.seq + 1,
            draw: None,
            incoming_meld: None,
            ..self.s.clone()
        };

        // Handle delayed revealing of new dora indicators.
        match self.s.incoming_meld {
            Some(Meld::Kakan(_)) | Some(Meld::Daiminkan(_)) => {
                next.num_dora_indicators += 1;
            }
            _ => {}
        }

        // Find the owner of Chii/Pon/Daiminkan. If none, represent this with "caller == p".
        let caller =
            other_players_after(p).into_iter().filter(|q| match self.reactions[q] {
                Some(Reaction::Chii(_, _)) => action_result == ActionResult::Chii,
                Some(Reaction::Pon(_, _)) => action_result == ActionResult::Pon,
                Some(Reaction::Daiminkan) => action_result == ActionResult::Daiminkan,
                _ => false
            }).exactly_one().ok().unwrap_or(p);

        if caller != p {
            let meld = self.meld_cache[caller.to_usize()].unwrap();
            next.incoming_meld = Some(meld);
            next.melds[caller.to_usize()].push(meld);
        }

        match action {
            Action::Discard { tile, riichi, tsumokiri } => {
                next.discards[pp].push((tile, caller));
                if riichi {
                    next.riichi[pp] = RiichiFlags {
                        is_active: true,
                        is_ippatsu: caller == p,
                        is_double: is_init_abortable(&self.s),
                    }
                }
                if caller == p {
                    next.draw = Some(self.begin.wall[next.num_drawn_head as usize]);
                    next.num_drawn_head += 1;
                } else {
                    next.action_player = caller;
                }
            }
            Action::Ankan(_) | Action::Kakan(_) => {
                let ankan_or_kakan = self.meld_cache[pp].unwrap();
                next.incoming_meld = Some(ankan_or_kakan);
                next.melds[pp].push(ankan_or_kakan);
                next.draw = Some(self.begin.wall[
                    wall::KAN_DRAW_INDEX[next.num_drawn_tail as usize]]);
                next.num_drawn_tail += 1;

                // ankan special: reveal the next dora indicator immediately
                if let Action::Ankan(_) = action {
                    next.num_dora_indicators += 1;
                }
            }

            Action::TsumoAgari(_) | Action::AbortKyuushuukyuuhai => panic!()
        }

        // TODO(summivox): check furiten for all

        NextOrEnd::Next(next)
    }

    fn next_agari(&self, action_result: ActionResult) -> NextOrEnd {
        NextOrEnd::End(RoundEndState {
            round_result: action_result,
            pot: 0,
            points: [0; 4],
            points_delta: [0; 4],
            renchan: false,
            next_round_id: None,
            agari_summary: None,
        })
    }

    fn next_abort(&self, action_result: ActionResult) -> NextOrEnd {
        // TODO(summivox): derive from rules
        let riichi_pot = Self::RIICHI_POT;
        let no_wait = Self::NO_WAIT_PENALTY_TOTAL;

        let mut end = RoundEndState {
            round_result: action_result,
            pot: self.begin.pot + num_active_riichi(&self.s) * riichi_pot,
            points: self.begin.points,
            ..RoundEndState::default()
        };

        let round_id = self.begin.round_id;
        match action_result {
            ActionResult::AbortWallExhausted => {
                let waiting =
                    self.wait_cache.map(|w|!w.is_empty() as u8);
                let num_waiting = waiting.into_iter().sum();
                let (down, up) = match num_waiting {
                    1 => (-no_wait / 3, no_wait / 1),
                    2 => (-no_wait / 2, no_wait / 2),
                    3 => (-no_wait / 1, no_wait / 3),
                    _ => (0, 0),
                };
                end.points_delta = waiting.map(|w| if w > 0 { up } else { down });
                end.renchan = waiting[round_id.button().to_usize()] > 0;
                end.next_round_id = Some(round_id.next_honba(end.renchan));
                // TODO(summivox): handle end-of-the-entire-game
            }

            ActionResult::AbortNagashiMangan => {
                end.points_delta = resolve_nagashi_mangan(&self.s, round_id.button());
                // same renchan rules as normal wall exhausted
                end.renchan = waiting[round_id.button().to_usize()] > 0;
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

        for i in 0..4 { end.points[i] += end.delta[i]; }

        NextOrEnd::End(end)
    }

    fn cache_wait_for(&mut self, player: Player) {
        self.wait_cache[player.to_usize()] =
            self.decomposer.with_tile_set(s.closed_hands[player.to_usize()])
                .into().iter().collect_vec()
    }
    fn cache_wait_for_all(&mut self) {
        for player in all_players() { self.cache_wait_for(player); }
    }
}
