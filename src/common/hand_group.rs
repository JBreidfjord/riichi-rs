//! Hand Group, a.k.a. Mentsu 面子
//!
//! ## Ref
//! - <https://riichi.wiki/Mentsu>
//! - <https://ja.wikipedia.org/wiki/%E9%9D%A2%E5%AD%90_(%E9%BA%BB%E9%9B%80)>

use crate::Tile;

/// 面子 A group of 3 tiles within a player's closed hand.
pub enum HandGroup {
    Shuntsu(Tile),
    Koutsu(Tile),
}
