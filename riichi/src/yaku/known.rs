/// An non-exhaustive list of Yaku's (役) known to this crate.
/// <https://riichi.wiki/Yaku>
#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Hash,
    strum::FromRepr,
    strum::AsRefStr,
    strum::IntoStaticStr,
    strum::Display,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
#[repr(u16)]
pub enum Yaku {
    /// 門前清自摸和
    /// <https://riichi.wiki/Menzenchin_tsumohou>
    #[strum(to_string = "門前清自摸和")]
    Menzenchintsumohou,

    /// 立直 / リーチ
    /// <https://riichi.wiki/Riichi>
    #[strum(to_string = "立直")] // tenhou
    #[strum(serialize = "リーチ")] // katakana
    Riichi,

    /// 一発
    /// <https://riichi.wiki/Ippatsu>
    #[strum(to_string = "一発")]
    Ippatsu,

    /// 槍槓
    /// <https://riichi.wiki/Chankan>
    #[strum(to_string = "槍槓")]
    Chankan,

    /// 嶺上開花
    /// <https://riichi.wiki/Rinshan_kaihou>
    #[strum(to_string = "嶺上開花")]
    Rinshankaihou,

    /// 海底摸月 / 海底撈月
    /// - <https://riichi.wiki/Haitei_raoyue_and_houtei_raoyui>
    /// - <https://ja.wikipedia.org/wiki/海底_(麻雀)#海底摸月>
    #[strum(to_string = "海底摸月")] // tenhou
    #[strum(serialize = "海底撈月")] // common alternative
    Haiteimouyue,

    /// 河底撈魚
    /// - <https://riichi.wiki/Haitei_raoyue_and_houtei_raoyui>
    /// - <https://ja.wikipedia.org/wiki/海底_(麻雀)#河底撈魚>
    #[strum(to_string = "河底撈魚")]
    Houteiraoyui,

    /// 平和
    /// <https://riichi.wiki/Pinfu>
    #[strum(to_string = "平和")]
    Pinfu,

    /// 断幺九 / 断么九 / 断ヤオ九 / ...
    /// <https://riichi.wiki/Tanyao>
    #[strum(to_string = "断幺九")] // tenhou
    #[strum(serialize = "断么九")] // common alternative
    #[strum(serialize = "断ヤオ九")] // common alternative
    Tanyaochuu,

    /// 一盃口
    /// <https://riichi.wiki/Iipeikou>
    #[strum(to_string = "一盃口")]
    Iipeikou,

    /// 場風 _(unspecified self wind)_
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "役牌:自風牌")] // majsoul
    JikazehaiAny,

    /// 自風 東 (Self East)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "自風 東")]
    JikazehaiE,

    /// 自風 南 (Self South)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "自風 南")]
    JikazehaiS,

    /// 自風 西 (Self West)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "自風 西")]
    JikazehaiW,

    /// 自風 北 (Self North)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "自風 北")]
    JikazehaiN,

    /// 場風 _(unspecified prevalent wind)_
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "役牌:場風牌")] // majsoul
    BakazehaiAny,

    /// 場風 東 (Prevalent East)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "場風 東")]
    BakazehaiE,

    /// 場風 南 (Prevalent West)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "場風 南")]
    BakazehaiS,

    /// 場風 西 (Prevalent West)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "場風 西")]
    BakazehaiW,

    /// 場風 北 (Prevalent North)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "場風 北")]
    BakazehaiN,

    /// 役牌 白 (White Dragon)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "役牌 白")]
    SangenpaiHaku,

    /// 役牌 發 (Green Dragon)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "役牌 發")]
    SangenpaiHatsu,

    /// 役牌 中 (Red Dragon)
    /// <https://riichi.wiki/Yakuhai>
    #[strum(to_string = "役牌 中")]
    SangenpaiChun,

    /// 両立直
    /// <https://riichi.wiki/Daburu_riichi>
    #[strum(to_string = "両立直")] // tenhou
    #[strum(serialize = "ダブル立直")] // majsoul (half katakana)
    #[strum(serialize = "ダブルリーチ")] // majsoul (full katakana)
    #[strum(serialize = "W立直")] // common alternative (W = double)
    #[strum(serialize = "Wリーチ")] // common alternative (W = double)
    DoubleRiichi,

    /// 七対子 (Seven Pairs)
    /// <https://riichi.wiki/Chiitoitsu>
    #[strum(to_string = "七対子")]
    Chiitoitsu,

    /// 混全帯幺九
    /// <https://riichi.wiki/Chanta>
    #[strum(to_string = "混全帯幺九")]
    Honchantaiyaochuu,

    /// 一気通貫
    /// <https://riichi.wiki/Ikkitsuukan>
    #[strum(to_string = "一気通貫")]
    Ikkitsuukan,

    /// 三色同順
    /// <https://riichi.wiki/Sanshoku_doujun>
    #[strum(to_string = "三色同順")]
    Sanshokudoujun,

    /// 三色同刻
    /// <https://riichi.wiki/Sanshoku_doukou>
    #[strum(to_string = "三色同刻")]
    Sanshokudoukou,

    /// 三槓子
    /// <https://riichi.wiki/Sankantsu>
    #[strum(to_string = "三槓子")]
    Sankantsu,

    /// 対々和
    /// <https://riichi.wiki/Toitoihou>
    #[strum(to_string = "対々和")]
    Toitoihou,

    /// 三暗刻
    /// <https://riichi.wiki/Sanankou>
    #[strum(to_string = "三暗刻")]
    Sannankou,

    /// 小三元
    /// <https://riichi.wiki/Shousangen>
    #[strum(to_string = "小三元")]
    Shousangen,

    /// 混老頭
    /// <https://riichi.wiki/Honroutou>
    #[strum(to_string = "混老頭")]
    Honroutou,

    /// 二盃口
    /// <https://riichi.wiki/Ryanpeikou>
    #[strum(to_string = "二盃口")]
    Ryanpeikou,

    /// 純全帯幺九
    /// <https://riichi.wiki/Junchantaiyaochuu>
    #[strum(to_string = "純全帯幺九")]
    Junchantaiyaochuu,

    /// 混一色
    /// <https://riichi.wiki/Honiisou>
    #[strum(to_string = "混一色")]
    Honniisou,

    /// 清一色
    /// <https://riichi.wiki/Chiniisou>
    #[strum(to_string = "清一色")]
    Chinniisou,

    /// 天和
    /// <https://riichi.wiki/Tenhou_and_chiihou>
    #[strum(to_string = "天和")]
    Tenhou,

    /// 地和
    /// <https://riichi.wiki/Tenhou_and_chiihou>
    #[strum(to_string = "地和")]
    Chiihou,

    /// 人和 _(non-standard)_
    /// <https://riichi.wiki/Renhou>
    #[strum(to_string = "人和")]
    Renhou,

    /// 大三元
    /// <https://riichi.wiki/Daisangen>
    #[strum(to_string = "大三元")]
    Daisangen,

    /// 四暗刻
    /// <https://riichi.wiki/Suuankou>
    #[strum(to_string = "四暗刻")]
    Suuankou,

    /// 四暗刻単騎
    /// <https://riichi.wiki/Suuankou>
    #[strum(to_string = "四暗刻単騎")]
    SuuankouTanki,

    /// 字一色
    /// <https://riichi.wiki/Tsuuiisou>
    #[strum(to_string = "字一色")]
    Tsuuiisou,

    /// 緑一色
    /// <https://riichi.wiki/Ryuuiisou>
    #[strum(to_string = "緑一色")]
    Ryuuiisou,

    /// 清老頭
    /// <https://riichi.wiki/Chinroutou>
    #[strum(to_string = "清老頭")]
    Chinroutou,

    /// 九蓮宝燈
    /// <https://riichi.wiki/Chuuren_poutou>
    #[strum(to_string = "九蓮宝燈")]
    Chuurenpoutou,

    /// 純正九蓮宝燈
    /// <https://riichi.wiki/Chuuren_poutou>
    #[strum(to_string = "純正九蓮宝燈")]
    Junseichuurenpoutou,

    /// 国士無双
    /// <https://riichi.wiki/Kokushi_musou>
    #[strum(to_string = "国士無双")]
    Kokushi,

    /// 国士無双１３面
    /// <https://riichi.wiki/Kokushi_musou>
    #[strum(to_string = "国士無双１３面")] // tenhou
    #[strum(serialize = "国士無双十三面待ち")] // majsoul
    Kokushi13,

    /// 大四喜
    /// <https://riichi.wiki/Suushiihou>
    #[strum(to_string = "大四喜")]
    Daisuushi,

    /// 小四喜
    /// <https://riichi.wiki/Suushiihou>
    #[strum(to_string = "小四喜")]
    Shousuushi,

    /// 四槓子 (Four Kans)
    /// <https://riichi.wiki/Suukantsu>
    #[strum(to_string = "四槓子")]
    Suukantsu,
}
