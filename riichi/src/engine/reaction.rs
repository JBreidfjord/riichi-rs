use thiserror::Error;

use crate::common::*;
use crate::model::*;
use super::EngineCache;
use super::utils::*;

#[derive(Error, Debug)]
pub enum ReactionError {
    #[error("No action to react to.")]
    NoAction,

    #[error("The action is terminal; no reactions possible.")]
    TerminalAction,

    #[error("Cannot declare an open meld under Riichi.")]
    MeldUnderRiichi,

    #[error("Cannot declare an open meld on the last draw")]
    MeldOnLastDraw,

    #[error("Tile {0} does not exist in the closed hand.")]
    TileNotExist(Tile),

    #[error("You can only call a discarded tile (is actually {0:?})")]
    NotDiscard(Action),

    #[error("Can only Chii on the previous player's discard.")]
    CanOnlyChiiPrevPlayer,

    #[error("Cannot Chii {2} with own {0}{1} (own may not exist).")]
    InvalidChii(Tile, Tile, Tile),

    #[error("Cannot Pon {2} with own {0}{1} (own may not exist).")]
    InvalidPon(Tile, Tile, Tile),

    #[error("Cannot Daiminkan.")]
    InvalidDaiminkan,

    #[error("No Ron when you are furiten: {0:?}")]
    Furiten(FuritenFlags),
}

pub(crate) fn check_reaction(
    state: &State,
    action: Action,
    reactor: Player,
    reaction: Reaction,
    cache: &mut EngineCache,
) -> Result<(), ReactionError> {
    use ReactionError::*;

    if action.is_terminal() { return Err(TerminalAction); }

    let actor = state.action_player;
    let reactor_i = reactor.to_usize();
    let hand = &state.closed_hands[reactor_i];

    // TODO(summivox): cannot chi/pon/daiminkan over houtei
    match reaction {
        Reaction::Chii(own0, own1) => {
            if state.riichi[reactor_i].is_active { return Err(MeldUnderRiichi); }
            if is_last_draw(state) { return Err(MeldOnLastDraw); }
            if player_succ(actor) != reactor {
                return Err(CanOnlyChiiPrevPlayer);
            }
            if let Action::Discard (discard) = action {
                let called = discard.tile;
                if let Some(chii) = Chii::from_tiles(own0, own1, called) {
                    if chii.is_in_hand(hand) {
                        cache.meld[reactor_i] = Some(Meld::Chii(chii));
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
            if is_last_draw(state) { return Err(MeldOnLastDraw); }
            if let Action::Discard(discard) = action {
                let called = discard.tile;
                let dir = actor.wrapping_sub(reactor);
                if let Some(pon) = Pon::from_tiles_dir(own0, own1, called, dir) {
                    if pon.is_in_hand(hand) {
                        cache.meld[reactor_i] = Some(Meld::Pon(pon));
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
            if is_last_draw(state) { return Err(MeldOnLastDraw); }
            if let Action::Discard(discard) = action {
                let called = discard.tile;
                let dir = actor.wrapping_sub(reactor);
                if let Some(daiminkan) = Daiminkan::from_hand_dir(hand, called, dir) {
                    cache.meld[reactor_i] = Some(Meld::Daiminkan(daiminkan));
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

pub(crate) fn resolve_reaction(
    state: &State,
    action: Action,
    reactions: &[Option<Reaction>; 4],
) -> ActionResult {
    let actor = state.action_player;

    // Handle in-turn voluntary termination.
    match action {
        Action::TsumoAgari(_) => return ActionResult::TsumoAgari,
        Action::AbortNineKinds => return ActionResult::AbortNineKinds,
        _ => {}
    }

    let highest_priority_reaction = other_players_after(actor).into_iter()
        .flat_map(|reactor| reactions[reactor.to_usize()]).max();
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
            let num_rons = reactions.iter()
                .filter(|&&reaction| reaction == Some(Reaction::RonAgari))
                .count();
            return if num_rons == 3 {
                ActionResult::AbortTripleRon
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
    if result == ActionResult::Pass && is_last_draw(state) {
        return if is_any_player_nagashi_mangan(state) {
            ActionResult::AbortNagashiMangan
        } else {
            ActionResult::AbortWallExhausted
        }
    }

    result
}
