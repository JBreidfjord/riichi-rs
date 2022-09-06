
use crate::{
    common::*,
    model::*,
};
use super::{AgariKind, AgariInput};

/// A bundle of intermediate results during the Agari computation.
#[derive(Clone, Debug, Default)]
pub struct HandCommon {
    pub agari_kind: AgariKind,
    pub all_tiles: TileSet37,
}

pub fn calc_hand_common(input: &AgariInput) -> HandCommon {
    HandCommon {
        agari_kind: if input.contributor == input.winner { AgariKind::Tsumo } else { AgariKind::Ron },
        all_tiles: get_all_tiles(input.closed_hand, input.winning_tile, input.melds),

    }
}

fn get_all_tiles(closed_hand: &TileSet37, winning_tile: Tile, melds: &[Meld]) -> TileSet37 {
    let mut all_tiles = closed_hand.clone();
    all_tiles[winning_tile] += 1;
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
    all_tiles
}
