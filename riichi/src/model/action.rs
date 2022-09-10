//! [`Action`] by the in-turn player.

use std::fmt::{Display, Formatter};
use crate::common::*;
use super::Discard;

/// Action by the in-turn player.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Action {
    /// Discard a tile. See [`Discard`].
    /// The `called_by` field is implied and can be safely ignored here.
    Discard(Discard),
    /// Declare an [`Ankan`] (4 in closed hand).
    Ankan(Tile),
    /// Declare a [`Kakan`] (1 in closed hand, 3 in pon).
    Kakan(Tile),
    /// Win by self-draw (ツモ和ガリ).
    /// See [`super::ActionResult::Agari`], [`super::AgariKind::Tsumo`].
    TsumoAgari(Tile),
    /// Abort by Nine Kinds of Terminals.
    /// See [`super::ActionResult::Abort`], [`super::AbortReason::NineKinds`].
    AbortNineKinds,
}

impl Action {
    pub fn from_meld(meld: &Meld) -> Option<Self> {
        match meld {
            Meld::Kakan(kakan) => Some(Action::Kakan(kakan.added)),
            Meld::Ankan(ankan) => Some(Action::Ankan(ankan.own[0].to_normal())),
            _ =>  None,
        }
    }

    pub fn tile(self) -> Option<Tile> {
        match self {
            Action::Discard(discard) => Some(discard.tile),
            Action::Ankan(tile) => Some(tile),
            Action::Kakan(tile) => Some(tile),
            Action::TsumoAgari(tile) => Some(tile),
            Action::AbortNineKinds => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Action::TsumoAgari(_) | Action::AbortNineKinds)
    }

    pub fn is_kan(self) -> bool {
        matches!(self, Action::Ankan(_)| Action::Kakan(_))
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Discard(discard) => write!(f, "{}", discard),
            Action::Ankan(tile) => write!(f, "Ankan({})", tile),
            Action::Kakan(tile) => write!(f, "Kakan({})", tile),
            Action::TsumoAgari(tile) => write!(f, "Tsumo({})", tile),
            Action::AbortNineKinds => write!(f, "NineKinds"),
        }
    }
}
