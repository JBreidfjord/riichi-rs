use itertools::Itertools;
use crate::analysis::{Decomposer, FullHandWaitingPattern};

use crate::common::*;
use crate::model::*;

impl RoundBeginState {
    pub fn to_initial_pre_action_state(&self) -> PreActionState {
        let wall = &self.wall;
        let button = self.round_id.button();
        PreActionState {
            action_player: button,
            seq: 0,
            num_drawn_head: 53,
            num_drawn_tail: 0,
            num_dora_indicators: 0,
            draw: Some(self.wall[52]),
            incoming_meld: None,
            closed_hands: wall::deal(wall, button),
            discards: [vec![], vec![], vec![], vec![]],
            furiten: [FuritenFlags::default(); 4],
            riichi: [RiichiFlags::default(); 4],
            melds: [vec![], vec![], vec![], vec![]],
        }
    }
}

impl TileSet37 {
    fn terminal_counter(&self, n: u8) -> u8 {
        0u8 + (self[0] == n) as u8 + (self[8] == n) as u8
            + (self[9] == n) as u8 + (self[17] == n) as u8
            + (self[18] == n) as u8 + (self[26] == n) as u8
            + (self[27] == n) as u8 + (self[28] == n) as u8
            + (self[29] == n) as u8 + (self[30] == n) as u8
            + (self[31] == n) as u8 + (self[32] == n) as u8
            + (self[33] == n) as u8
    }
    pub fn terminal_kinds(&self) -> u8 { self.terminal_counter(1) }
}

pub fn all_players() -> [Player; 4] {
    [Player::new(0), Player::new(1), Player::new(2), Player::new(3)]
}
pub fn player_succ(player: Player) -> Player { Player::new(1).wrapping_add(player) }
pub fn player_oppo(player: Player) -> Player { Player::new(2).wrapping_add(player) }
pub fn player_pred(player: Player) -> Player { Player::new(3).wrapping_add(player) }
pub fn other_players_after(player: Player) -> [Player; 3] {
    [
        Player::new(1).wrapping_add(player),
        Player::new(2).wrapping_add(player),
        Player::new(3).wrapping_add(player),
    ]
}

pub fn count_for_kan(hand: &TileSet37, tile: Tile) -> (usize, usize) {
    let t = tile.to_normal();
    let num_normal = hand[t.to_normal()];
    let num_red = if t.num() == 5 { hand[t.to_red()] } else { 0 };
    (num_normal as usize, num_red as usize)
}
pub fn ankan_tiles(tile: Tile, num_red: usize) -> [Tile; 4] {
    let mut tiles = [tile, tile, tile, tile];
    for i in 0..num_red { tiles[i] = tile.to_red(); }
    tiles
}
pub fn daiminkan_tiles(tile: Tile, num_red: usize) -> [Tile; 3] {
    let mut tiles = [tile, tile, tile];
    for i in 0..num_red { tiles[i] = tile.to_red(); }
    tiles
}

pub fn num_active_riichi(s: &PreActionState) -> usize {
    s.riichi.into_iter().filter(|flags| flags.is_active).count()
}

/// Returns if this discard immediately after calling Chii/Pon constitutes a swap call (喰い替え),
/// i.e. the discarded tile can form the same group as the meld. This is usually forbidden.
///
/// Example:
/// - Hand 456m; if 56m is used to call 7m, then 4m cannot be discarded.
/// - Hand 678m; if 68m is used to call 7m, then the other 7m in hand cannot be discarded.
///
/// <https://riichi.wiki/Kuikae>
pub fn is_forbidden_swap_call(meld: Meld, discard: Tile) -> bool {
    // TODO(summivox): rules
    let discard = discard.to_normal();
    match meld {
        Meld::Chii(chii) => {
            chii.called.to_normal() == discard ||
                (chii.dir() == 0 && Some(discard) == chii.own[1].succ()) ||
                (chii.dir() == 2 && Some(discard) == chii.min.pred())
        }
        Meld::Pon(pon) => {
            pon.called.to_normal() == discard
        }
        _ => false,
    }
}

/// <https://riichi.wiki/Kan#Kan_during_riichi>
pub fn is_ankan_ok_under_riichi(decomps: &[FullHandWaitingPattern], ankan: Tile) -> bool {
    // TODO(summivox): rule
    // TODO(summivox): okuri-kan (need to also check the discard)
    // TODO(summivox): relaxed rule (sufficient to not change the set of waiting tiles)
    let ankan = ankan.to_normal();
    decomps.iter().all(|decomp|
        decomp.groups().any(|group| group == HandGroup::Koutsu(ankan)))
}

/// Affects [`ActionResult::AbortKyuushuukyuuhai`] and [`RiichiFlags::is_double`].
pub fn is_init_abortable(s: &PreActionState) -> bool {
    s.seq <= 3 && s.melds.iter().all(|melds| melds.is_empty())
}

/// Checks if [`ActionResult::AbortWallExhausted`] applies (during end-of-turn resolution).
pub fn is_wall_exhausted(s: &PreActionState) -> bool {
    s.num_drawn_head + s.num_drawn_tail == wall::MAX_NUM_DRAWS
}

/// Checks if [`ActionResult::AbortNagashiMangan`] applies (during end-of-turn resolution) for the
/// specified player.
/// Assuming [`is_wall_exhausted`].
pub fn is_nagashi_mangan(s: &PreActionState, player: Player) -> bool {
    s.discards[player.to_usize()].iter().all(|&(tile, called_player)|
        tile.is_terminal() && called_player == player)
}

/// Checks if [`ActionResult::AbortNagashiMangan`] applies (during end-of-turn resolution) for all
/// players.
/// Assuming [`is_wall_exhausted`].
pub fn is_any_player_nagashi_mangan(s: &PreActionState) -> bool {
    all_players().into_iter().all(|player| is_nagashi_mangan(s, player))
}

/// Checks if [`ActionResult::AbortFourWind`] applies (during end-of-turn resolution).
pub fn is_aborted_four_wind(s: &PreActionState, action: Action) -> bool {
    if let Action::Discard { tile, .. } = action {
        return is_init_abortable(s) &&
            s.seq == 3 &&
            tile.is_wind() &&
            s.discards[0..3].iter().all(|river|
                river.len() == 1 && river[0].0 == tile);
    }
    false
}

/// Checks if [`ActionResult::AbortFourKan`] applies (during end-of-turn resolution).
pub fn is_aborted_four_kan(s: &PreActionState, action: Action, tentative_result: ActionResult) -> bool {
    let pp = s.action_player.to_usize();

    if matches!(action, Action::Kakan(_)) ||
        matches!(action, Action::Ankan(_)) ||
        tentative_result == ActionResult::Daiminkan {
        let kan_players =
            s.melds.iter().enumerate().flat_map(|(player, melds_p)|
                melds_p.iter().filter_map(move |meld|
                    if meld.is_kan() { Some(player) } else { None })).collect_vec();

        if kan_players.len() == 4 ||
            kan_players.len() == 3 && !kan_players.iter().all(|&player| player == pp) {
            return true;
        }
    }
    false
}

/// Checks if [`ActionResult::AbortFourRiichi`] applies (during end-of-turn resolution).
pub fn is_aborted_four_riichi(s: &PreActionState, action: Action) -> bool {
    matches!(action, Action::Discard{declare_riichi: true, ..}) && num_active_riichi(s) == 3
}

/// When the wall has been exhausted and no player has achieved
/// [`ActionResult::AbortNagashiMangan`], given whether each player is waiting (1) or not (0),
/// returns the points delta for each player.
pub fn calc_wall_exhausted_delta(waiting: [u8; 4]) -> [GamePoints; 4] {
    // TODO(summivox): rules
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
/// [`ActionResult::AbortNagashiMangan`], returns the points delta for each player.
pub fn calc_nagashi_mangan_delta(s: &PreActionState, button: Player) -> [GamePoints; 4] {
    // TODO(summivox): rules

    let mut delta = [0; 4];
    for player in all_players() {
        if is_nagashi_mangan(s, player) {
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

/// When the wall has been exhausted, returns the points delta for each player as well as if the
/// button player stays the same in the next round (renchan 連荘).
pub fn resolve_wall_exhausted(
    s: &PreActionState, waiting: [u8; 4], button: Player) -> ([GamePoints; 4], bool) {
    let renchan = waiting[button.to_usize()] > 0;
    let delta_nagashi = calc_nagashi_mangan_delta(s, button);
    if delta_nagashi == [0; 4] {
        (calc_wall_exhausted_delta(waiting), renchan)
    } else {
        (delta_nagashi, renchan)
    }
}
