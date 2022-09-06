use std::fmt::{Display, Formatter};
use std::str::FromStr;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use crate::common::*;
use crate::model::*;
use super::strings::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TenhouScoring {
    pub kind: TenhouScoringKind,
    pub payout: TenhouPayout,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TenhouScoringKind {
    HanFu { han: u8, fu: u8 },
    Mangan,
    Haneman,
    Baiman,
    Sanbaiman,
    Yakuman,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TenhouPayout {
    Ron(GamePoints),
    TsumoByButton(GamePoints),
    TsumoByNonButton { non_button: GamePoints, button: GamePoints },
}

impl FromStr for TenhouScoring {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_tenhou_scoring(s).ok_or(())
    }
}

pub fn parse_tenhou_scoring(s: &str) -> Option<TenhouScoring> {
    static RE_SCORING: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?x)^
        (?:(\d+)符(\d+)飜|  # 1 => fu, 2 => han,
           (満貫|跳満|倍満|三倍満|役満))  # 3 => mangan-plus
        (?:(\d+)点|  # 4 => ron,
           (\d+)点∀|  # 5 => tsumo by button
           (\d+)-(\d+)点)  # 6/7 => tsumo by non button
    $").unwrap());
    static MANGANS: phf::Map<&'static str, TenhouScoringKind> = phf::phf_map! {
            "満貫" => TenhouScoringKind::Mangan,
            "跳満" => TenhouScoringKind::Haneman,
            "倍満" => TenhouScoringKind::Baiman,
            "三倍満" => TenhouScoringKind::Sanbaiman,
            "役満" => TenhouScoringKind::Yakuman,
        };
    let groups = RE_SCORING.captures(s)?;
    let kind =
        if let (Some(fu_match), Some(han_match)) = (groups.get(1), groups.get(2)) {
            let fu = fu_match.as_str().parse::<u8>().ok()?;
            let han = han_match.as_str().parse::<u8>().ok()?;
            TenhouScoringKind::HanFu { han, fu }
        } else if let Some(mangan_plus_match) = groups.get(3) {
            MANGANS.get(mangan_plus_match.as_str()).copied()?
        } else { panic!() };
    let payout =
        if let Some(ron_match) = groups.get(4) {
            TenhouPayout::Ron(ron_match.as_str().parse::<GamePoints>().ok()?)
        } else if let Some(button_match) = groups.get(5) {
            TenhouPayout::TsumoByButton(button_match.as_str().parse::<GamePoints>().ok()?)
        } else if let (Some(non_button_match), Some(button_match)) =
                      (groups.get(6), groups.get(7)) {
            TenhouPayout::TsumoByNonButton {
                non_button: non_button_match.as_str().parse::<GamePoints>().ok()?,
                button: button_match.as_str().parse::<GamePoints>().ok()?,
            }
        } else { panic!() };
    Some(TenhouScoring { kind, payout })
}

impl Display for TenhouScoring {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            TenhouScoringKind::HanFu { han, fu } => write!(f, "{}符{}飜", fu, han),
            TenhouScoringKind::Mangan => write!(f, "満貫"),
            TenhouScoringKind::Haneman => write!(f, "跳満"),
            TenhouScoringKind::Baiman => write!(f, "倍満"),
            TenhouScoringKind::Sanbaiman => write!(f, "三倍満"),
            TenhouScoringKind::Yakuman => write!(f, "役満"),
        }?;
        match self.payout {
            TenhouPayout::Ron(points) => write!(f, "{}点", points),
            TenhouPayout::TsumoByButton(points) => write!(f, "{}点∀", points),
            TenhouPayout::TsumoByNonButton{non_button, button} =>
                write!(f, "{}-{}点", non_button, button),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum YakuOrDora {
    Yaku(Yaku, i8),
    Yakuman(Yaku),
    Dora(i8),
    AkaDora(i8),
    UraDora(i8),
}

impl FromStr for YakuOrDora {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_yaku_or_dora(s).ok_or(())
    }
}

pub fn parse_yaku_or_dora(s: &str) -> Option<YakuOrDora> {
    static RE_YAKU: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"^([^()]+)\((?:(\d+)飜|役満)\)").unwrap());
    let groups = RE_YAKU.captures(s)?;
    let yaku_str = groups.get(1)?.as_str();
    let han = groups.get(2).and_then(|g| g.as_str().parse::<i8>().ok());
    match yaku_str {
        DORA_STR => Some(YakuOrDora::Dora(han?)),
        AKA_DORA_STR => Some(YakuOrDora::AkaDora(han?)),
        URA_DORA_STR => Some(YakuOrDora::UraDora(han?)),
        _ => if let Some(han) = han {
            yaku_from_str(yaku_str).map(|yaku| YakuOrDora::Yaku(yaku, han))
        } else {
            yaku_from_str(yaku_str).map(|yaku| YakuOrDora::Yakuman(yaku))
        }
    }
}

impl Display for YakuOrDora {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            YakuOrDora::Yaku(yaku, han) => write!(f, "{}({}飜)", yaku_to_str(*yaku), han),
            YakuOrDora::Yakuman(yaku) => write!(f, "{}(役満)", yaku_to_str(*yaku)),
            YakuOrDora::Dora(han) => write!(f, "{}({}飜)", DORA_STR, han),
            YakuOrDora::AkaDora(han) => write!(f, "{}({}飜)", AKA_DORA_STR, han),
            YakuOrDora::UraDora(han) => write!(f, "{}({}飜)", URA_DORA_STR, han),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn scoring_examples() {
        let examples = [
            ("30符3飜3900点", TenhouScoring {
                kind: TenhouScoringKind::HanFu { han: 3, fu: 30 },
                payout: TenhouPayout::Ron(3900),
            }),
            ("50符3飜3200点∀", TenhouScoring {
                kind: TenhouScoringKind::HanFu { han: 3, fu: 50 },
                payout: TenhouPayout::TsumoByButton(3200),
            }),
            ("満貫2000-4000点", TenhouScoring {
                kind: TenhouScoringKind::Mangan,
                payout: TenhouPayout::TsumoByNonButton { non_button: 2000, button: 4000 },
            }),
        ];
        for (scoring_str, scoring) in examples {
            assert_eq!(scoring.to_string(), scoring_str);
            assert_eq!(scoring_str.parse::<TenhouScoring>().unwrap(), scoring);
        }
    }

    #[test]
    fn yaku_examples() {
        let examples = [
            ("対々和(2飜)", YakuOrDora::Yaku(Yaku::Toitoihou, 2)),
            ("四槓子(役満)", YakuOrDora::Yakuman(Yaku::Suukantsu)),
            ("ドラ(2飜)", YakuOrDora::Dora(2)),
            ("裏ドラ(1飜)", YakuOrDora::UraDora(1)),
            ("赤ドラ(3飜)", YakuOrDora::AkaDora(3)),
        ];
        for (yaku_or_dora_str, yaku_or_dora) in examples {
            assert_eq!(yaku_or_dora.to_string(), yaku_or_dora_str);
            assert_eq!(yaku_or_dora_str.parse::<YakuOrDora>().unwrap(), yaku_or_dora);
        }
    }
}
