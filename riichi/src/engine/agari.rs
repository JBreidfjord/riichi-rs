use crate::{
    analysis::RegularWait,
    common::*,
    engine::WaitingInfo,
    model::*
};

/// A bundle of intermediate results during the Agari computation.
#[derive(Copy, Clone, Default)]
pub struct AgariFacts {
    pub kind: AgariKind,
}

pub fn calc_agari(
    begin: &RoundBegin,
    state: &State,
    winner: Player,
    hand: &TileSet37,
    waits: &WaitingInfo,
    wait_mask: TileMask34,
    winning_tile: Tile,
) -> Option<AgariResult> {

    let mut facts = AgariFacts::default();

    let win_from = state.action_player;
    facts.kind = if win_from == winner { AgariKind::Tsumo } else { AgariKind::Ron };

    unimplemented!()
}
