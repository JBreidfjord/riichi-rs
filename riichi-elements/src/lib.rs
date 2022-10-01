#![cfg_attr(not(feature = "std"), no_std)]

pub mod hand_group;
pub mod meld;
pub mod player;
pub mod tile;
pub mod tile_set;
pub mod typedefs;
pub mod utils;
pub mod wall;

pub mod prelude {
    pub use crate::hand_group::*;
    pub use crate::meld::*;
    pub use crate::player::*;
    pub use crate::tile::*;
    pub use crate::tile_set::*;
    pub use crate::typedefs::*;
    pub use crate::utils::*;
    pub use crate::wall::{self, PartialWall, Wall, PartialWallDisplayMethod, WallDisplayMethod};
}
