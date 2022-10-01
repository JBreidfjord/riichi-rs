use crate::{
    tile::Tile,
    tile_set::TileSet37,
};

pub fn count_for_kan(hand: &TileSet37, normal: Tile) -> (usize, usize) {
    debug_assert!(normal.is_normal());
    let num_normal = hand[normal];
    let num_red = if normal.has_red() { hand[normal.to_red()] } else { 0 };
    (num_normal as usize, num_red as usize)
}

pub fn ankan_tiles(normal: Tile, num_red: usize) -> [Tile; 4] {
    debug_assert!(normal.is_normal());
    let mut tiles = [normal, normal, normal, normal];
    for i in 0..num_red {
        tiles[i] = normal.to_red();
    }
    tiles
}

pub fn daiminkan_tiles(normal: Tile, num_red: usize) -> [Tile; 3] {
    debug_assert!(normal.is_normal());
    let mut tiles = [normal, normal, normal];
    for i in 0..num_red {
        tiles[i] = normal.to_red();
    }
    tiles
}
