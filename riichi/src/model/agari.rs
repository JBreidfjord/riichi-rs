use crate::common::*;

#[derive(Clone, Debug, Default)]
pub struct AgariResult {
    pub kind: AgariKind,
    pub points_delta: [GamePoints; 4],
    pub yaku: Vec<()>,  // TODO(summivox): yaku
    pub raw_score: AgariRawScore,
    pub num_dora_hits: u8,
    pub num_ura_dora_hits: u8,
    // TODO(summivox): agari
}

#[derive(Copy, Clone, Debug, num_enum::Default, Eq, PartialEq)]
#[repr(u8)]
pub enum AgariKind {
    #[num_enum(default)]
    Ron = 0,
    Tsumo,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct AgariRawScore {
    pub han: u8,
    pub fu: u8,
}
