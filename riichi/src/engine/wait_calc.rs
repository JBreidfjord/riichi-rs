use itertools::Itertools;
use crate::analysis::{Decomposer, RegularWait, IrregularWait, detect_irregular_wait};
use crate::common::*;

#[derive(Clone, Debug, Default)]
pub struct WaitingInfo {
    pub waiting_set: TileMask34,
    pub regular: Vec<RegularWait>,
    pub irregular: Option<IrregularWait>,
}

impl WaitingInfo {
    pub fn from_keys(decomposer: &mut Decomposer, keys: &[u32; 4]) -> Self {
        let mut waiting_set = TileMask34::default();
        let regular = decomposer.with_keys(*keys).iter().collect_vec();
        for wait in regular.iter() {
            waiting_set.0 |= 1 << wait.waiting_tile.encoding() as u64;
        }
        let irregular = detect_irregular_wait(*keys);
        if let Some(irregular) = irregular {
            waiting_set |= irregular.to_waiting_set();
        }
        Self { waiting_set, regular, irregular }
    }
}

