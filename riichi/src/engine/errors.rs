use thiserror::Error;

use crate::common::*;
use crate::model::*;

#[derive(Error, Debug)]
pub enum ActionError {
    #[error("Tsumokiri discard tile {0} != drawn tile {1:?}")]
    TsumokiriMismatch(Tile, Option<Tile>),

    #[error("Discarding from the closed hand while under riichi.")]
    DiscardClosedHandUnderRiichi,

    #[error("Discarding {0} is swap-calling (kuikae) due to {1}")]
    NoSwapCalling(Tile, Meld),

    #[error("Tile {0} does not exist in the closed hand.")]
    TileNotExist(Tile),

    #[error("Attempting to declare riichi when already under riichi.")]
    DeclareRiichiAgain,

    #[error("Attempting to declare riichi without enough points.")]
    DeclareRiichiWithoutPoints,

    #[error("Attempting to declare riichi with an open hand.")]
    DeclareRiichiWithOpenMeld,

    #[error("Attempting to declare riichi with a hand not ready after discarding.")]
    DeclareRiichiWhileNotReady,

    #[error("Attempting invalid ankan on {0} under riichi.")]
    InvalidAnkanUnderRiichi(Tile),

    #[error("Cannot ankan on {0}; not enough in hand")]
    NotEnoughForAnkan(Tile),

    #[error("Attempting kakan on {0} without corresponding pon.")]
    NoPonForKakan(Tile),

    #[error("Cannot declare Kyuushuukyuuhai with only {0} kinds of terminals in hand.")]
    NotEnoughKindsForNineKinds(u8),

    #[error("Cannot abort after the first go-around.")]
    NotInitAbortable,

    #[error("Can only declare tsumo-agari (win by self-draw) on the drawn tile.")]
    MustDeclareTsumoAgariOnDraw,
}

#[derive(Error, Debug)]
pub enum ReactionError {
    #[error("No action to react to.")]
    NoAction,

    #[error("The action is terminal; no reactions possible.")]
    TerminalAction,

    #[error("Cannot declare an open meld under riichi.")]
    MeldUnderRiichi,

    #[error("Tile {0} does not exist in the closed hand.")]
    TileNotExist(Tile),

    #[error("You can only call a discarded tile (is actually {0:?})")]
    NotDiscard(Action),

    #[error("Can only Chii on the previous player's discard.")]
    CanOnlyChiiPrevPlayer,

    #[error("Cannot Chii {2} with own {0}{1} (own may not exist).")]
    InvalidChii(Tile, Tile, Tile),

    #[error("Cannot Pon {2} with own {0}{1} (own may not exist).")]
    InvalidPon(Tile, Tile, Tile),

    #[error("Cannot Daiminkan.")]
    InvalidDaiminkan,

    #[error("No ron when you are furiten: {0:?}")]
    Furiten(FuritenFlags),
}
