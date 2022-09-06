

/// All Yaku's (役) known to this package.
/// This is intended to be used as a unifying key/symbol to uniquely represent each Yaku without
/// having to use strings everywhere.
#[derive(
    Copy, Clone, Debug, Eq, PartialEq, Hash,
    num_enum::TryFromPrimitive, num_enum::IntoPrimitive,
)]
#[non_exhaustive]
#[repr(u16)]
pub enum Yaku {
    /// 門前清自摸和
    Menzenchintsumohou,
    /// 立直
    Riichi,
    /// 一発
    Ippatsu,
    /// 槍槓
    Chankan,
    /// 嶺上開花
    Rinshankaihou,
    /// 海底摸月
    Haiteiraoyue,
    /// 河底撈魚
    Houteiraoyui,
    /// 平和
    Pinfu,
    /// 断幺九
    Tanyaochuu,
    /// 一盃口
    Iipeikou,
    /// 自風 東
    JikazehaiE,
    /// 自風 南
    JikazehaiS,
    /// 自風 西
    JikazehaiW,
    /// 自風 北
    JikazehaiN,
    /// 場風 東
    BakazehaiE,
    /// 場風 南
    BakazehaiS,
    /// 場風 西
    BakazehaiW,
    /// 場風 北
    BakazehaiN,
    /// 役牌 白
    SangenpaiHaku,
    /// 役牌 發
    SangenpaiHatsu,
    /// 役牌 中
    SangenpaiChun,
    /// 両立直
    DoubleRiichi,
    /// 七対子
    Chiitoitsu,
    /// 混全帯幺九
    Honchantaiyaochuu,
    /// 一気通貫
    Ikkitsuukan,
    /// 三色同順
    Sanshokudoujun,
    /// 三色同刻
    Sanshokudoukou,
    /// 三槓子
    Sankantsu,
    /// 対々和
    Toitoihou,
    /// 三暗刻
    Sannankou,
    /// 小三元
    Shousangen,
    /// 混老頭
    Honraotou,
    /// 二盃口
    Ryanpeikou,
    /// 純全帯幺九
    Junchantaiyaochuu,
    /// 混一色
    Honniisou,
    /// 清一色
    Chinniisou,
    /// 天和
    Tenhou,
    /// 地和
    Chiihou,
    /// 人和
    Renhou,
    /// 大三元
    Daisangen,
    /// 四暗刻
    Suuankou,
    /// 四暗刻単騎
    SuuankouTanki,
    /// 字一色
    Tsuuiisou,
    /// 緑一色
    Ryuuiisou,
    /// 清老頭
    Chinraotou,
    /// 九蓮宝燈
    Chuurenpoutou,
    /// 純正九蓮宝燈
    Junseichuurenpoutou,
    /// 国士無双
    Kokushi,
    /// 国士無双１３面
    Kokushi13,
    /// 大四喜
    Daisuushi,
    /// 小四喜
    Shousuushi,
    /// 四槓子
    Suukantsu,
}
