
use crate::common::*;
use crate::model::*;

impl RoundBeginState {
    pub fn to_initial_pre_action_state(&self) -> PreActionState {
        let wall = &self.wall;
        let button = self.round_id.button();
        PreActionState {
            action_player: button,
            seq: 0,
            num_drawn_head: 52,
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

impl PreActionState {
    pub fn is_init_abortable(&self) -> bool {
        self.seq <= 3 && self.melds.iter().all(|melds| melds.is_empty())
    }
}
