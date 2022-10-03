#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

pub mod hand_group;
pub mod meld;
pub mod player;
pub mod tile;
pub mod tile_set;
pub mod typedefs;
pub mod utils;
pub mod wall;

pub mod prelude {
    pub use crate::{
        hand_group::*,
        meld::*,
        player::*,
        tile::*,
        tile_set::*,
        typedefs::*,
        wall::{self, PartialWall, PartialWallDisplayMethod, Wall, WallDisplayMethod}
    };
}
