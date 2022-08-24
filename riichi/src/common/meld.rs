//! Meld 副露
//!
//! (TODO)
//!
//! ## Ref
//!
//! - <https://riichi.wiki/Naki>
//! - <https://ja.wikipedia.org/wiki/%E5%89%AF%E9%9C%B2>

use derive_more::{Display};

use crate::common::typedefs::*;

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

#[derive(Copy, Clone, Debug, Display, Eq, PartialEq)]
pub enum Meld {
    Chii(Chii),
    Pon(Pon),
    Kakan(Kakan),
    Daiminkan(Daiminkan),
    Ankan(Ankan),
}

impl Meld {
    pub fn is_kan(&self) -> bool {
        match self {
            Self::Kakan(_) | Self::Daiminkan(_) | Self::Ankan(_) => true,
            _ => false
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // use assert2::check;

    #[test]
    fn chii_example() {
        let chii = Chii::from_tiles(
            "4s".parse().unwrap(),
            "6s".parse().unwrap(),
            "0s".parse().unwrap()).unwrap();
        let meld = Meld::Chii(chii);
        assert_eq!(Meld::from_packed(0x0155), Some(meld));
        assert_eq!(meld.packed(), 0x0155);
        assert_eq!(chii.to_string(), "C046s");
    }

    #[test]
    fn pon_example() {
        let pon = Pon::from_tiles_dir(
            "5p".parse().unwrap(),
            "0p".parse().unwrap(),
            "0p".parse().unwrap(),
            Player::new(2)).unwrap();
        let meld = Meld::Pon(pon);
        assert_eq!(Meld::from_packed(0x158D), Some(meld));
        assert_eq!(meld.packed(), 0x158D);
        assert_eq!(pon.to_string(), "0P05p");
    }
}
