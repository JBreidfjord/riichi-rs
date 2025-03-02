use polars::prelude::*;
use riichi::prelude::*;

/// A loose upper bound of the number of legal actions per Round/Kyoku.
/// - 70 x discards after self-draw (including ones from Kans).
/// - 16 x discards after ([`Chii`] + [`Pon`]).
/// - 4 x ([`Ankan`] + [`Kakan`]).
/// - 1 x [`Action::TsumoAgari`].
const MAX_NUM_ACTIONS: usize = 70 + 16 + 4 + 1;

pub struct FlatRounds {
    //////////////////////////////////////
    // RoundBegin + RoundEnd

    /// `round_begin.round_id.kyoku`
    kyoku: Vec<u8>,
    /// `round_begin.round_id.honba`
    honba: Vec<u8>,
    /// `round_begin.wall`
    wall: Vec<[u8; 136]>,
    /// `round_begin.pot`
    pot_start: Vec<i32>,
    /// `round_end.pot`
    pot_end: Vec<i32>,
    /// `round_begin.points`
    points_start: Vec<[i32; 4]>,
    /// `round_end.points`
    points_end: Vec<[i32; 4]>,

    /// `round_end.round_result`
    /// Must be either [`ActionResult::Agari`] or [`ActionResult::Abort`].
    /// Directly serialized as enum str.
    round_result: Vec<&'static str>,

    num_steps: Vec<u8>,

    //////////////////////////////////////
    // GameStep with StateCore separated

    actor: Vec<[u8; MAX_NUM_ACTIONS]>,
    num_drawn_head: Vec<[u8; MAX_NUM_ACTIONS]>,
    num_drawn_tail: Vec<[u8; MAX_NUM_ACTIONS]>,
    num_dora_indicators: Vec<[u8; MAX_NUM_ACTIONS]>,
    draw: Vec<[Option<u8>; MAX_NUM_ACTIONS]>,
    /// Meld is serialized into its Display str.
    incoming_meld: Vec<[String; MAX_NUM_ACTIONS]>,
    is_furiten: Vec<[[bool; 4]; MAX_NUM_ACTIONS]>,
    is_riichi: Vec<[[bool; 4]; MAX_NUM_ACTIONS]>,

    /// Directly serialized as enum str.
    action_type: Vec<[&'static str; MAX_NUM_ACTIONS]>,
    action_tile: Vec<[u8; MAX_NUM_ACTIONS]>,
    is_tsumogiri: Vec<[bool; MAX_NUM_ACTIONS]>,
    declares_riichi: Vec<[bool; MAX_NUM_ACTIONS]>,

    reactor: Vec<[Option<u8>; MAX_NUM_ACTIONS]>,
    reaction_type: Vec<[&'static str; MAX_NUM_ACTIONS]>,
    chii_pon_tiles: Vec<[Option<(u8, u8)>; MAX_NUM_ACTIONS]>,

    /// Same as `round_result` --- flattened enum.
    /// `CalledBy` player is implied by `reactor`.
    action_result: Vec<[&'static str; MAX_NUM_ACTIONS]>,

    //////////////////////////////////////
    // Auxiliary / Analysis

}

#[cfg(test)]
mod tests {
    use super::*;
}
