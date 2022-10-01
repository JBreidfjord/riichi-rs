//! Unordered (multi-)sets of tiles, represented as histograms.
//!
//! - When red 5's are counted separately, use [`TileSet37`].
//! - If red 5's are treated the same as normal 5's, use [`TileSet34`].
//! - If all 4 tiles of the same kind are treated the same (i.e. you only need to deal with distinct
//!   tiles), use [`TileMask34`].
//!
//! As the names suggest, the specific encoding of [`Tile`] is assumed.
//!
//! Both [`TileSet37`] and [`TileSet34`] can be indexed with [`Tile`] (red or normal, 37 or 34),
//! without first converting to encoding. A [`TileSet37`] can be converted to a [`TileSet34`] with
//! red 5's folded into normal 5's.
//!

mod tile_set_37;
mod tile_set_34;
mod tile_mask_34;

pub use self::{
    tile_set_37::*,
    tile_set_34::*,
    tile_mask_34::*,
};
