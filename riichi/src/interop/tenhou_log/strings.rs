//! Mappings between various string representations 

use crate::{
    common::*,
    model::*,
};

/// Tenhou round result string => [`ActionResult`].
/// Due to [`ActionResult::TsumoAgari`] or [`ActionResult::RonAgari`] sharing the same string
/// [`AGARI_STR`], this map only contains reasons of round abortion.
pub static ABORT_STR_TO_ENUM: phf::Map<&'static str, ActionResult> = phf::phf_map! {
  "流局"     => ActionResult::AbortWallExhausted,
  "流し満貫" => ActionResult::AbortNagashiMangan,
  "九種九牌" => ActionResult::AbortNineKinds,
  "三家和了" => ActionResult::AbortTripleRon,
  "四風連打" => ActionResult::AbortFourWind,
  "四家立直" => ActionResult::AbortFourRiichi,
  "四槓散了" => ActionResult::AbortFourKan,
};

/// [`ActionResult`] => Tenhou round result string.
/// Here we _can_ map both [`ActionResult::TsumoAgari`] and [`ActionResult::RonAgari`].
pub const fn action_result_to_str(action_result: ActionResult) -> &'static str {
    match action_result {
        ActionResult::AbortWallExhausted => "流局",
        ActionResult::AbortNagashiMangan => "流し満貫",
        ActionResult::AbortNineKinds => "九種九牌",
        ActionResult::AbortTripleRon => "三家和了",
        ActionResult::AbortFourWind => "四風連打",
        ActionResult::AbortFourRiichi => "四家立直",
        ActionResult::AbortFourKan => "四槓散了",
        ActionResult::TsumoAgari => AGARI_STR,
        ActionResult::RonAgari => AGARI_STR,
        _ => "",
    }
}

/// Represents either [`ActionResult::TsumoAgari`] or [`ActionResult::RonAgari`].
pub const AGARI_STR: &'static str = "和了";

/// Tenhou Yaku string => [`Yaku`].
pub static YAKU_STR_TO_ENUM: phf::Map<&'static str, Yaku> = phf::phf_map! {
    "門前清自摸和" => Yaku::Menzenchintsumohou,
    "立直" => Yaku::Riichi,
    "一発" => Yaku::Ippatsu,
    "槍槓" => Yaku::Chankan,
    "嶺上開花" => Yaku::Rinshankaihou,
    "海底摸月" => Yaku::Haiteiraoyue,
    "河底撈魚" => Yaku::Houteiraoyui,
    "平和" => Yaku::Pinfu,
    "断幺九" => Yaku::Tanyaochuu,
    "一盃口" => Yaku::Iipeikou,
    "自風 東" => Yaku::JikazehaiE,
    "自風 南" => Yaku::JikazehaiS,
    "自風 西" => Yaku::JikazehaiW,
    "自風 北" => Yaku::JikazehaiN,
    "場風 東" => Yaku::BakazehaiE,
    "場風 南" => Yaku::BakazehaiS,
    "場風 西" => Yaku::BakazehaiW,
    "場風 北" => Yaku::BakazehaiN,
    "役牌 白" => Yaku::SangenpaiHaku,
    "役牌 發" => Yaku::SangenpaiHatsu,
    "役牌 中" => Yaku::SangenpaiChun,
    "両立直" => Yaku::DoubleRiichi,
    "七対子" => Yaku::Chiitoitsu,
    "混全帯幺九" => Yaku::Honchantaiyaochuu,
    "一気通貫" => Yaku::Ikkitsuukan,
    "三色同順" => Yaku::Sanshokudoujun,
    "三色同刻" => Yaku::Sanshokudoukou,
    "三槓子" => Yaku::Sankantsu,
    "対々和" => Yaku::Toitoihou,
    "三暗刻" => Yaku::Sannankou,
    "小三元" => Yaku::Shousangen,
    "混老頭" => Yaku::Honraotou,
    "二盃口" => Yaku::Ryanpeikou,
    "純全帯幺九" => Yaku::Junchantaiyaochuu,
    "混一色" => Yaku::Honniisou,
    "清一色" => Yaku::Chinniisou,
    "天和" => Yaku::Tenhou,
    "地和" => Yaku::Chiihou,
    "大三元" => Yaku::Daisangen,
    "四暗刻" => Yaku::Suuankou,
    "四暗刻単騎" => Yaku::SuuankouTanki,
    "字一色" => Yaku::Tsuuiisou,
    "緑一色" => Yaku::Ryuuiisou,
    "清老頭" => Yaku::Chinraotou,
    "九蓮宝燈" => Yaku::Chuurenpoutou,
    "純正九蓮宝燈" => Yaku::Junseichuurenpoutou,
    "国士無双" => Yaku::Kokushi,
    "国士無双１３面" => Yaku::Kokushi13,
    "大四喜" => Yaku::Daisuushi,
    "小四喜" => Yaku::Shousuushi,
    "四槓子" => Yaku::Suukantsu,
};

/// [`Yaku`] to Tenhou Yaku string.
pub const fn yaku_to_str(yaku: Yaku) -> &'static str {
    match yaku {
        Yaku::Menzenchintsumohou => "門前清自摸和",
        Yaku::Riichi => "立直",
        Yaku::Ippatsu => "一発",
        Yaku::Chankan => "槍槓",
        Yaku::Rinshankaihou => "嶺上開花",
        Yaku::Haiteiraoyue => "海底摸月",
        Yaku::Houteiraoyui => "河底撈魚",
        Yaku::Pinfu => "平和",
        Yaku::Tanyaochuu => "断幺九",
        Yaku::Iipeikou => "一盃口",
        Yaku::JikazehaiE => "自風 東",
        Yaku::JikazehaiS => "自風 南",
        Yaku::JikazehaiW => "自風 西",
        Yaku::JikazehaiN => "自風 北",
        Yaku::BakazehaiE => "場風 東",
        Yaku::BakazehaiS => "場風 南",
        Yaku::BakazehaiW => "場風 西",
        Yaku::BakazehaiN => "場風 北",
        Yaku::SangenpaiHaku => "役牌 白",
        Yaku::SangenpaiHatsu => "役牌 發",
        Yaku::SangenpaiChun => "役牌 中",
        Yaku::DoubleRiichi => "両立直",
        Yaku::Chiitoitsu => "七対子",
        Yaku::Honchantaiyaochuu => "混全帯幺九",
        Yaku::Ikkitsuukan => "一気通貫",
        Yaku::Sanshokudoujun => "三色同順",
        Yaku::Sanshokudoukou => "三色同刻",
        Yaku::Sankantsu => "三槓子",
        Yaku::Toitoihou => "対々和",
        Yaku::Sannankou => "三暗刻",
        Yaku::Shousangen => "小三元",
        Yaku::Honraotou => "混老頭",
        Yaku::Ryanpeikou => "二盃口",
        Yaku::Junchantaiyaochuu => "純全帯幺九",
        Yaku::Honniisou => "混一色",
        Yaku::Chinniisou => "清一色",
        Yaku::Tenhou => "天和",
        Yaku::Chiihou => "地和",
        Yaku::Daisangen => "大三元",
        Yaku::Suuankou => "四暗刻",
        Yaku::SuuankouTanki => "四暗刻単騎",
        Yaku::Tsuuiisou => "字一色",
        Yaku::Ryuuiisou => "緑一色",
        Yaku::Chinraotou => "清老頭",
        Yaku::Chuurenpoutou => "九蓮宝燈",
        Yaku::Junseichuurenpoutou => "純正九蓮宝燈",
        Yaku::Kokushi => "国士無双",
        Yaku::Kokushi13 => "国士無双１３面",
        Yaku::Daisuushi => "大四喜",
        Yaku::Shousuushi => "小四喜",
        Yaku::Suukantsu => "四槓子",
    }
}
