use itertools::Itertools;
use log::log_enabled;

use crate::{
    analysis::RegularWait,
    common::*,
    model::*,
    rules::Ruleset
};
use crate::analysis::{Decomposer, WaitingInfo};

// TODO(summivox): Consider porting these directly to `impl TileSet37`.

pub fn terminal_kinds(h: &TileSet37) -> u8 {
    pure_terminal_kinds(h) + honor_kinds(h)
}

pub fn terminal_count(h: &TileSet37) -> u8 {
    pure_terminal_count(h) + honor_count(h)
}

pub fn pure_terminal_kinds(h: &TileSet37) -> u8 {
    0u8 + (h[0] > 0) as u8 + (h[8] > 0) as u8
        + (h[9] > 0) as u8 + (h[17] > 0) as u8
        + (h[18] > 0) as u8 + (h[26] > 0) as u8
}

pub fn pure_terminal_count(h: &TileSet37) -> u8 {
    h[0] + h[8] + h[9] + h[17] + h[18] + h[26]
}

pub fn honor_kinds(h: &TileSet37) -> u8 {
    0u8 + (h[27] > 0) as u8 + (h[28] > 0) as u8
        + (h[29] > 0) as u8 + (h[30] > 0) as u8
        + (h[31] > 0) as u8 + (h[32] > 0) as u8
        + (h[33] > 0) as u8
}

pub fn honor_count(h: &TileSet37) -> u8 {
    h[27] + h[28] + h[29] + h[30] + h[31] + h[32] + h[33]
}

pub fn green_count(h: &TileSet37) -> u8 {
    h[19] + h[20] + h[21] + h[23] + h[25] + h[32]
}

pub fn m_count(h: &TileSet37) -> u8 {
    (&h.0[0..9]).iter().sum::<u8>() + h[34]
}
pub fn p_count(h: &TileSet37) -> u8 {
    (&h.0[9..18]).iter().sum::<u8>() + h[35]
}
pub fn s_count(h: &TileSet37) -> u8 {
    (&h.0[18..27]).iter().sum::<u8>() + h[36]
}
/// Alias of `honor_count`.
pub fn z_count(h: &TileSet37) -> u8 { honor_count(h) }


// TODO(summivox): We don't actually need the pack --- convert this to use normal bins

/// Determine whether a packed suit (3N+2) satisfies the [`Yaku::Chuurenpoutou`] form, i.e.
/// `311111113` + any. If it does, then returns the _position_ of the winning tile (0..=8).
pub fn chuuren_agari(x: u32) -> Option<u8> {
    // check x is at least 0o311111113 (each bin must individually apply, without overflow)
    if (x + 0o133333331) & 0o444444444 != 0o444444444 { return None; }
    // subtract our target, and now only 1 shall remain (full closed hand, n == 14, target n = 13)
    let r = x - 0o311111113;
    // sanity check (what if we started with more than 14?)
    if !r.is_power_of_two() { return None; }
    Some(r.trailing_zeros() as u8 / 3)
}

/// Determines whether a _non-packed_ suit (3N+1) is 1 tile away from the [`Yaku::Chuurenpoutou`]
/// form, i.e. `311111113` - some + other. If it does, then returns the _position_ of:
///
/// - the lacking tile
/// - the over tile
///
/// Special case: `311111113` (pure chuuren) => `Some(0, 0)`
pub fn chuuren_wait(h: &[u8]) -> Option<(u8, u8)> {
    const TARGET: [i8; 9] = [3, 1, 1, 1, 1, 1, 1, 1, 3];
    let mut lack = 100;
    let mut over = 100;
    for (i, (a, b)) in itertools::zip_eq(h, TARGET).enumerate() {
        let x = *a as i8 - b;
        match x {
            -1 => {
                if lack < 9 { return None; }
                lack = i;
            }
            0 => {}
            1 => {
                if over < 9 { return None; }
                over = i;
            }
            _ => return None,
        }
    }
    if lack > 9 && over > 9 {
        Some((0, 0))
    } else if lack < 9 && over < 9 {
        Some((lack as u8, over as u8))
    } else {
        None
    }
}

/// Returns if this discard immediately after calling Chii/Pon constitutes a swap call (喰い替え),
/// i.e. the discarded tile can form a similar group as the meld. This is usually forbidden.
///
/// Example:
/// - Hand 678m; if 78m is used to call 9m, then 6m cannot be discarded.
/// - Hand 456m; if 46m is used to call (red) 0m, then the (normal) 5m in hand cannot be discarded.
///
/// <https://riichi.wiki/Kuikae>
pub fn is_forbidden_swap_call(ruleset: &Ruleset, meld: Meld, discard: Tile) -> bool {
    let discard = discard.to_normal();
    let (allow_same, allow_other) = (ruleset.swap_call_allow_same, ruleset.swap_call_allow_other);
    match meld {
        Meld::Chii(chii) => {
            (!allow_same && chii.called.to_normal() == discard) ||
                (!allow_other && chii.dir() == 0 && Some(discard) == chii.own[1].succ()) ||
                (!allow_other && chii.dir() == 2 && Some(discard) == chii.min.pred())
        }
        Meld::Pon(pon) => {
            !allow_same && pon.called.to_normal() == discard
        }
        _ => false,
    }
}

/// <https://riichi.wiki/Kan#Kan_during_riichi>
pub fn is_ankan_ok_under_riichi(
    ruleset: &Ruleset,
    decomposer: &mut Decomposer,
    hand: &TileSet37,
    waiting_info: &WaitingInfo,
    draw: Tile,
    ankan: Tile,
) -> bool {
    let draw = draw.to_normal();
    let ankan = ankan.to_normal();
    if ruleset.riichi_ankan_strict_mode {
        is_ankan_ok_under_riichi_strict(hand, &waiting_info.regular, draw, ankan)
    } else {
        is_ankan_ok_under_riichi_relaxed(hand, decomposer, waiting_info, ankan)
    }
}

pub fn is_ankan_ok_under_riichi_strict(
    hand: &TileSet37,
    regulars: &[RegularWait],
    draw: Tile,
    ankan: Tile,
) -> bool {
    // Okuri-Kan (送り槓) is not allowed under strict mode.
    if draw != ankan { return false; }

    // Every way of normal decomposition must include `ankan` as a Koutsu
    if !regulars.iter().all(|regular|
        regular.groups().any(|group| group == HandGroup::Koutsu(ankan))) {
        return false;
    }

    // Must not destroy Chuuren form
    if ankan.suit() == 3 { return true }
    let i = (ankan.suit() * 9) as usize;
    let mut hand = TileSet34::from(hand);
    hand[ankan] -= 1;
    !chuuren_wait(&hand.0[i..(i + 9)]).is_some()
}

pub fn is_ankan_ok_under_riichi_relaxed(
    hand: &TileSet37,
    decomposer: &mut Decomposer,
    waiting_info: &WaitingInfo,
    ankan: Tile,
) -> bool {
    let mut hand = hand.clone();
    hand[ankan] -= 1;
    let new_waiting_info = WaitingInfo::from_keys(decomposer, &hand.packed_34());
    waiting_info.waiting_set == new_waiting_info.waiting_set
}

/********/

pub fn num_active_riichi(state: &State) -> usize {
    state.core.riichi.into_iter().flatten().count()
}

pub fn num_draws(state: &State) -> u8 {
    state.core.num_drawn_head + state.core.num_drawn_tail
}

/// The prerequisite of Haitei and Houtei: no more draws available.
pub fn is_last_draw(state: &State) -> bool {
    debug_assert!(num_draws(state) <= wall::MAX_NUM_DRAWS);
    num_draws(state) == wall::MAX_NUM_DRAWS
}

/// First 4 turns of the game without being interrupted by any meld.
/// Affects:
/// - [`AbortReason::NineKinds`] (active), [`AbortReason::FourWind`] (passive)
/// - [`RiichiFlags::is_double`]
/// - [`Yaku::Tenhou`], [`Yaku::Chiihou`], [`Yaku::Renhou`]
pub fn is_first_chance(state: &State) -> bool {
    state.core.seq <= 3 && state.melds.iter().all(|melds| melds.is_empty())
}

/// Checks if [`AbortReason::NagashiMangan`] applies (during end-of-turn resolution) for the
/// specified player.
/// Assuming [`is_last_draw`].
pub fn is_nagashi_mangan(state: &State, player: Player) -> bool {
    state.discards[player.to_usize()].iter().all(|discard|
        discard.tile.is_terminal() && discard.called_by == player)
}

/// Checks if [`AbortReason::NagashiMangan`] applies (during end-of-turn resolution) for all
/// players.
/// Assuming [`is_last_draw`].
pub fn is_any_player_nagashi_mangan(state: &State) -> bool {
    ALL_PLAYERS.into_iter().any(|player| is_nagashi_mangan(state, player))
}

/// Checks if [`AbortReason::FourWind`] applies (during end-of-turn resolution).
pub fn is_aborted_four_wind(state: &State, action: Action) -> bool {
    if let Action::Discard(discard) = action {
        return is_first_chance(state) &&
            state.core.seq == 3 &&
            discard.tile.is_wind() &&
            other_players_after(state.core.actor).iter()
                .map(|actor| &state.discards[actor.to_usize()])
                .all(|discards|
                    discards.len() == 1 && discards[0].tile == discard.tile)
    }
    false
}

/// Checks if [`AbortReason::FourKan`] applies (during end-of-turn resolution).
pub fn is_aborted_four_kan(state: &State, action: Action, reaction: Option<Reaction>) -> bool {
    let actor_i = state.core.actor.to_usize();

    if matches!(action, Action::Kakan(_)) ||
        matches!(action, Action::Ankan(_)) ||
        matches!(reaction, Some(Reaction::Daiminkan)) {
        // Gather the owner of each kan on the table into one list.
        let kan_players =
            state.melds.iter().enumerate().flat_map(|(player, melds_p)|
                melds_p.iter().filter_map(move |meld|
                    if meld.is_kan() { Some(player) } else { None })).collect_vec();
        // - 3 existing kans + this one => ok if all 4 are from the same player. 
        // - 4 existing kans + this one => not ok (max number of kans on the table is 4).
        if kan_players.len() == 4 ||
            kan_players.len() == 3 && !kan_players.iter().all(|&player| player == actor_i) {
            return true;
        }
    }
    false
}

/// Checks if [`AbortReason::FourRiichi`] applies (during end-of-turn resolution).
pub fn is_aborted_four_riichi(state: &State, action: Action) -> bool {
    matches!(action, Action::Discard(Discard{declares_riichi: true, ..})) &&
        num_active_riichi(state) == 3  // not a typo --- the last player only declared => not active yet
}

/// When the wall has been exhausted and no player has achieved
/// [`AbortReason::NagashiMangan`], given whether each player is waiting (1) or not (0),
/// returns the points delta for each player.
pub fn calc_wall_exhausted_delta(waiting: [u8; 4]) -> [GamePoints; 4] {
    // TODO(summivox): rules (ten-no-ten points)
    const NO_WAIT_PENALTY_TOTAL: GamePoints = 3000;
    let no_wait = NO_WAIT_PENALTY_TOTAL;

    let num_waiting = waiting.into_iter().sum();
    let (down, up) = match num_waiting {
        1 => (-no_wait / 3, no_wait / 1),
        2 => (-no_wait / 2, no_wait / 2),
        3 => (-no_wait / 1, no_wait / 3),
        _ => (0, 0),
    };
    waiting.map(|w| if w > 0 { up } else { down })
}

/// When the wall has been exhausted and some player has achieved
/// [`AbortReason::NagashiMangan`], returns the points delta for each player.
pub fn calc_nagashi_mangan_delta(state: &State, button: Player) -> [GamePoints; 4] {
    // TODO(summivox): rules (nagashi-mangan-points)

    let mut delta = [0; 4];
    for player in ALL_PLAYERS {
        if is_nagashi_mangan(state, player) {
            if player == button {
                delta[player.to_usize()] += 12000 + 4000;
                for qq in 0..4 { delta[qq] -= 4000; }
            } else {
                delta[player.to_usize()] += 8000 + 2000;
                delta[button.to_usize()] -= 2000;
                for qq in 0..4 { delta[qq] -= 2000; }
            }
        }
    }
    delta
}

/// Each player with active riichi must pay into the pot.
pub fn calc_pot_delta(riichi: &[Option<Riichi>; 4]) -> [GamePoints; 4] {
    riichi.map(|r| if r.is_some() { -super::RIICHI_POT } else { 0 })
}

/// All tiles at win condition = closed hand + the winning tile + all tiles in melds .
/// A fully closed hand win will be 14 tiles.
/// Chii/Pon will not change this number, while each Kan introduces 1 more tile.
/// At the extreme, 4 Kan's will result in 18 tiles (4x4 for each Kan + 2 for the pair).
pub fn get_all_tiles(
    closed_hand: &TileSet37,
    winning_tile: Tile,
    melds: &[Meld],
) -> TileSet37 {
    let mut all_tiles = closed_hand.clone();
    log::debug!("closed_hand: {}", all_tiles);
    all_tiles[winning_tile] += 1;
    log::debug!("+winning   : {}", all_tiles);
    for meld in melds {
        match meld {
            Meld::Chii(chii) => {
                for own in chii.own { all_tiles[own] += 1 }
                all_tiles[chii.called] += 1;
            }
            Meld::Pon(pon) => {
                for own in pon.own { all_tiles[own] += 1 }
                all_tiles[pon.called] += 1;
            }
            Meld::Kakan(kakan) => {
                for own in kakan.pon.own { all_tiles[own] += 1 }
                all_tiles[kakan.pon.called] += 1;
                all_tiles[kakan.added] += 1;
            }
            Meld::Daiminkan(daiminkan) => {
                for own in daiminkan.own { all_tiles[own] += 1 }
                all_tiles[daiminkan.called] += 1;
            }
            Meld::Ankan(ankan) => {
                for own in ankan.own { all_tiles[own] += 1; }
            }
        }
    }
    log::debug!("+meld      : {}", all_tiles);
    all_tiles
}

pub fn count_doras(
    ruleset: &Ruleset,
    all_tiles: &TileSet37,
    num_dora_indicators: u8,
    wall: &Wall,
    is_riichi: bool,
) -> DoraHits {
    let all_tiles_normal = TileSet34::from(all_tiles);

    let n = if ruleset.dora_allow_kan { num_dora_indicators as usize } else { 1 };
    let n_ura = if ruleset.dora_allow_kan_ura { n } else { 1 };

    if log_enabled!(log::Level::Debug) {
        log::debug!("count doras: n={} n_ura={} di={} udi={}, all_tiles={}",
            n,
            n_ura,
            wall::dora_indicators(wall).iter().map(|t| t.as_str()).join(","),
            wall::ura_dora_indicators(wall).iter().map(|t| t.as_str()).join(","),
            all_tiles,
        );
    }

    DoraHits {
        dora:
        (&wall::dora_indicators(wall)[0..n])
            .iter()
            .map(|t| all_tiles_normal[t.indicated_dora()])
            .sum(),

        ura_dora:
        if is_riichi && ruleset.dora_allow_ura {
            (&wall::ura_dora_indicators(wall)[0..n_ura])
                .iter()
                .map(|t| all_tiles_normal[t.indicated_dora()])
                .sum()
        } else { 0 },

        aka_dora: all_tiles[34] + all_tiles[35] + all_tiles[36],
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet as HashSet;
    use super::*;

    #[test]
    fn chuuren_wait_exhaustive() {
        let mut s: HashSet<[u8; 9]> = HashSet::default();

        let target = [3, 1, 1, 1, 1, 1, 1, 1, 3];
        s.insert(target);
        assert_eq!(chuuren_wait(&target[..]), Some((0, 0)));

        for lack in 0..9 {
            for over in 0..9 {
                if lack == over { continue; }
                let mut x = target;
                x[lack] -= 1;
                x[over] += 1;
                s.insert(x);
                assert_eq!(chuuren_wait(&x[..]), Some((lack as u8, over as u8)));
            }
        }

        for x in itertools::repeat_n(0..4, 9).multi_cartesian_product() {
            let x: [u8; 9] = x.try_into().unwrap();
            if !s.contains(&x) {
                assert_eq!(chuuren_wait(&x), None);
            }
        }
    }
}
