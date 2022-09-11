//! All [`Yaku`]'s (役) known to this package.

use std::collections::{HashMap, HashSet};

/// All Yaku's (役) known to this package.
///
/// This is intended to be used as a unifying key/symbol to uniquely represent each Yaku without
/// having to use strings everywhere.
#[derive(
    Copy, Clone, Debug, Eq, PartialEq, Hash,
    num_enum::TryFromPrimitive, num_enum::IntoPrimitive,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    Honroutou,
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
    Chinroutou,
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

pub const fn get_blocked_yaku(yaku: Yaku) -> &'static [Yaku] {
    use Yaku::*;
    match yaku {
        Chinroutou | Honroutou => &[Junchantaiyaochuu, Honchantaiyaochuu],
        Rinshankaihou | Chankan => &[Haiteiraoyue, Houteiraoyui],
        _ => &[],
    }
}

// TODO(summivox): set of standard Yaku's

pub type YakuValues = HashMap<Yaku, i8>;

#[derive(Debug, Default)]
pub struct YakuBuilder {
    yaku_values: YakuValues,
    blocked_yaku: HashSet<Yaku>,
    has_yakuman: bool,
}

impl YakuBuilder {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, yaku: Yaku, value: i8) {
        if self.blocked_yaku.contains(&yaku) { return }
        if self.has_yakuman && value > 0 { return }
        self.yaku_values.insert(yaku, value);
        for blocked in get_blocked_yaku(yaku) {
            self.yaku_values.remove(blocked);
            self.blocked_yaku.insert(*blocked);
        }
        if !self.has_yakuman && value < 0 {
            self.has_yakuman = true;
            self.yaku_values.retain(|_, other_value| *other_value < 0);
        }
    }

    pub fn build(self) -> YakuValues { self.yaku_values }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaku_builder() {
        use Yaku::*;

        let mut x = YakuBuilder::new();
        x.add(Chinroutou, -1);
        x.add(Honchantaiyaochuu, 1);
        let a = x.build();
        assert_eq!(a, YakuValues::from([(Chinroutou, -1)]));

        let mut x = YakuBuilder::new();
        x.add(Honchantaiyaochuu, 1);
        x.add(Chinroutou, -1);
        let a = x.build();
        assert_eq!(a, YakuValues::from([(Chinroutou, -1)]));
    }
}
