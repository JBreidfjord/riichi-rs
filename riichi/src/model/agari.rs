use crate::common::*;
use super::ActionResult;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgariResult {
    /// Ron or Tsumo?
    pub kind: AgariKind,

    /// Who won?
    pub winner: Player,

    /// Which player supplied the winning tile?
    /// - RonAgari: the loser.
    /// - TsumoAgari: the winner itself.
    pub contributor: Player,

    /// The net effect of this win on each player's points, before considering the collection
    /// of [`crate::model::RoundBegin::pot`] by the first winning player.
    /// Note that this does include the effects of [`crate::model::RoundId::honba`].
    pub points_delta_before_pot: [GamePoints; 4],

    /// Pot collected by this player.
    /// Note that in a multi-ron scenario, only one player will get the pot.
    /// TODO(summivox): rules (atama-hane)
    pub pot_gained: GamePoints,

    pub hand: TileSet37,
    pub melds: Vec<Meld>,
    pub yaku_han: Vec<()>,  // TODO(summivox)
    pub raw_score: AgariRawScore,
}

#[derive(Copy, Clone, Debug, num_enum::Default, Eq, PartialEq)]
#[repr(u8)]
pub enum AgariKind {
    #[num_enum(default)]
    Ron = 0,
    Tsumo,
}

impl AgariKind {
    pub fn to_action_result(self) -> ActionResult {
        match self {
            AgariKind::Ron => ActionResult::RonAgari,
            AgariKind::Tsumo => ActionResult::TsumoAgari,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct AgariRawScore {
    pub yakuman_sum: u8,
    pub yaku_sum: u8,
    pub num_dora_hits: u8,
    pub num_ura_dora_hits: u8,
    pub fu: u8,
}
