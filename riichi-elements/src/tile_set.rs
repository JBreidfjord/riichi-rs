//! Unordered multi- and single-sets of tiles (histograms).
//!
//! These can be used to represent any unordered set of tiles, such as a closed hand, a full winning
//! hand, all waiting tiles of a waiting hand, and tiles discarded by a player.
//!
//! - [`TileSet37`]: multi-set; treats "red 5" tiles separately from "normal 5" (34 + 3 = 37 kinds)
//! - [`TileSet34`]: multi-set; treats "red 5" tiles the same as "normal 5" (34 kinds)
//! - [`TileMask34`]: single-set counting unique tiles;
//!   treats "red 5" tiles the same as "normal 5" (34 kinds)
//!
//! As the names suggest, the specific encoding of [Tile] is assumed.
//!
//! Both [`TileSet37`] and [`TileSet34`] can be indexed with [`Tile`][Tile] (red or normal, 37 or
//! 34), without first converting to encoding. A [`TileSet37`] can be converted to a [`TileSet34`]
//! with red 5's folded into normal 5's. Any multi-set can be converted to a [`TileMask34`].
//!
//! [Tile]: crate::tile::Tile
//!

mod tile_set_37;
mod tile_set_34;
mod tile_mask_34;

pub use self::{
    tile_set_37::*,
    tile_set_34::*,
    tile_mask_34::*,
};
