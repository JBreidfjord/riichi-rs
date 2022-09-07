
use crate::{common::*, model::*, Rules};
use super::{AgariKind, AgariInput};

/// A bundle of intermediate results during the Agari computation.
#[derive(Clone, Debug, Default)]
pub struct HandCommon {
    pub agari_kind: AgariKind,
    pub all_tiles: TileSet37,
    pub all_tiles_packed: [u32; 4],
    pub winning_tile: Tile,
    pub is_closed: bool,
    pub dora_hits: DoraHits,
}

pub fn calc_hand_common(rules: &Rules, input: &AgariInput) -> HandCommon {
    let agari_kind =
        if input.contributor == input.winner { AgariKind::Tsumo } else { AgariKind::Ron };
    let winning_tile = input.action.tile().unwrap();  // guaranteed to exist
    let all_tiles = get_all_tiles(
        input.closed_hand,
        winning_tile,
        input.melds);
    let all_tiles_packed = TileSet34::from(&all_tiles).packed();
    let is_closed = input.melds.iter().all(|m| m.is_closed());
    let dora_hits = count_doras(rules,
                                &all_tiles,
                                input.num_dora_indicators,
                                &input.begin.wall,
                                input.riichi_flags.is_active);
    HandCommon {
        agari_kind,
        all_tiles,
        all_tiles_packed,
        winning_tile,
        is_closed,
        dora_hits,
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

pub fn count_doras(
    _rules: &Rules,
    all_hand: &TileSet37,
    num_dora_indicators: u8,
    wall: &Wall,
    is_riichi: bool,
) -> DoraHits {
    let n = num_dora_indicators as usize;
    DoraHits {
        dora:
        (&wall::dora_indicators(wall)[0..n])
            .iter()
            .map(|t| all_hand[t.indicated_dora()])
            .sum(),

        ura_dora:
        if is_riichi {
            (&wall::ura_dora_indicators(wall)[0..n])
                .iter()
                .map(|t| all_hand[t.indicated_dora()])
                .sum()
        } else { 0 },

        aka_dora: all_hand[34] + all_hand[35] + all_hand[36],
    }
}
