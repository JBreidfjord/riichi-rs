
use crate::analysis::FullHandWaitingPattern;
use crate::common::*;
use crate::model::*;

/// A bundle of intermediate results during the Agari computation.
#[derive(Copy, Clone, Default)]
pub struct AgariFacts {
    pub kind: AgariKind,
}

pub fn calc_agari(s: &State, winner: Player, waits: &[FullHandWaitingPattern], wait_mask: TileMask34)
    -> Option<AgariResult> {

    let mut facts = AgariFacts::default();

    let win_from = s.action_player;
    facts.kind = if win_from == winner { AgariKind::Tsumo } else { AgariKind::Ron };

    unimplemented!()
}
