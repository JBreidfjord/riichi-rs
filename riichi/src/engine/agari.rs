
use crate::analysis::FullHandWaitingPattern;
use crate::common::*;
use crate::model::*;

#[derive(Copy, Clone, Default)]
pub struct AgariFacts {
    pub kind: AgariKind,
}

#[derive(Copy, Clone, Debug, num_enum::Default, Eq, PartialEq)]
#[repr(u8)]
pub enum AgariKind {
    #[num_enum(default)]
    Ron = 0,
    Tsumo,
}

pub struct AgariResult {
    // TODO
}

pub fn calc_agari(s: &PreActionState, winner: Player, waits: &[FullHandWaitingPattern], wait_mask: TileMask34)
    -> Option<AgariResult> {

    let mut facts = AgariFacts::default();

    let win_from = s.action_player;
    facts.kind = if win_from == winner { AgariKind::Tsumo } else { AgariKind::Ron };

    unimplemented!()
}
