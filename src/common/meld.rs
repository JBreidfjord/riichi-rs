//! Meld 副露
//!
//! (TODO)
//!
//! ## Ref
//!
//! - https://riichi.wiki/Naki
//! - https://ja.wikipedia.org/wiki/%E5%89%AF%E9%9C%B2

use std::fmt::{Debug, Display, Formatter};

use derive_more::{Constructor, Display, From, Into};

use crate::common::tile::Tile;
use crate::common::typedefs::*;
use crate::common::utils::*;

mod chii;
mod pon;
mod kakan;
mod daiminkan;
mod ankan;
mod packed;

pub use chii::Chii;
pub use pon::Pon;
pub use kakan::Kakan;
pub use daiminkan::Daiminkan;
pub use ankan::Ankan;
pub(crate) use packed::PackedMeld;

#[derive(Copy, Clone, Debug, Display, Eq, PartialEq)]
pub enum Meld {
    Chii(Chii),
    Pon(Pon),
    Kakan(Kakan),
    Daiminkan(Daiminkan),
    Ankan(Ankan),
}

#[cfg(test)]
mod test {
    use super::*;
    // use assert2::check;

    #[test]
    fn pon_example() {
        let pon = Pon::from_tiles_dir(
            "5p".parse().unwrap(),
            "0p".parse().unwrap(),
            "0p".parse().unwrap(),
            Player::new(2)).unwrap();
        let meld = Meld::Pon(pon);
        assert_eq!(Meld::from_packed(0x158D), Some(meld));
        assert_eq!(meld.packed(), 0x158Du16);
        assert_eq!(pon.to_string(), "0P05p");
    }
}
