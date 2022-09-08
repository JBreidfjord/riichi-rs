
use crate::{
    common::*,
    engine::utils::get_all_tiles,
    Rules,
};
use super::{
    AgariKind,
    AgariInput,
};

/// A bundle of intermediate results during the Agari computation.
#[derive(Clone, Debug, Default)]
pub struct HandCommon {
    pub agari_kind: AgariKind,
    pub all_tiles: TileSet37,
    pub all_tiles_packed: [u32; 4],
    pub is_closed: bool,
}

pub fn calc_hand_common(_rules: &Rules, input: &AgariInput) -> HandCommon {
    let agari_kind =
        if input.contributor == input.winner { AgariKind::Tsumo } else { AgariKind::Ron };
    let all_tiles = get_all_tiles(
        input.closed_hand,
        input.winning_tile,
        input.melds);
    let all_tiles_packed = TileSet34::from(&all_tiles).packed();
    let is_closed = input.melds.iter().all(|m| m.is_closed());
    HandCommon {
        agari_kind,
        all_tiles,
        all_tiles_packed,
        is_closed,
    }
}
