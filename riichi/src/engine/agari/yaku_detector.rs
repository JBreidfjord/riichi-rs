
use crate::{
    common::*,
    model::*,
    Rules
};
use super::{
    AgariInput,
    HandCommon,
    RegularWaitCommon,
};

pub fn detect_riichi_related(
    _rules: &Rules,
    riichi_flags: &RiichiFlags,
    yaku_values: &mut Vec<(Yaku, i8)>,
) {
    if riichi_flags.is_active {
        if riichi_flags.is_double {
            yaku_values.push((Yaku::DoubleRiichi, 2));
        } else {
            yaku_values.push((Yaku::Riichi, 1));
        }
        if riichi_flags.is_ippatsu {
            yaku_values.push((Yaku::Ippatsu, 1));
        }
    }
}

pub fn detect_mentsumo(
    _rules: &Rules,
    agari_kind: AgariKind,
    melds: &[Meld],
    yaku_values: &mut Vec<(Yaku, i8)>,
) {
    if melds.iter().all(|m| m.is_closed()) && agari_kind == AgariKind::Tsumo {
        yaku_values.push((Yaku::Menzenchintsumohou, 1));
    }
}

pub fn detect_rinshan(
    _rules: &Rules,
    agari_kind: AgariKind,
    incoming_meld: Option<Meld>,
    yaku_values: &mut Vec<(Yaku, i8)>,
) {
    if let Some(meld) = incoming_meld {
        if meld.is_kan() && agari_kind == AgariKind::Tsumo {
            yaku_values.push((Yaku::Rinshankaihou, 1));
        }
    }
}

pub fn detect_chankan(
    _rules: &Rules,
    agari_kind: AgariKind,
    // TODO
    yaku_values: &mut Vec<(Yaku, i8)>,
) {
    if agari_kind == AgariKind::Ron {
        unimplemented!()
    }
    unimplemented!()
}
