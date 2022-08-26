use crate::common::Tile;
use crate::common::TileSet37;

pub fn count_for_kan(hand: &TileSet37, tile: Tile) -> (usize, usize) {
    let t = tile.to_normal();
    let num_normal = hand[t];
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
