use std::fmt::Formatter;
use serde::{
    ser::{Serialize, Serializer},
    de::{Deserialize, Deserializer, Error, Visitor},
};
use crate::{
    common::*,
    model::*,
};
use super::{
    meld::*,
    tile::*,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TenhouIncoming {
    Draw(Tile),
    ChiiPonDaiminkan(Meld),
}

impl Serialize for TenhouIncoming {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            TenhouIncoming::Draw(tile) => serializer.serialize_u8(to_tenhou_tile(*tile)),
            TenhouIncoming::ChiiPonDaiminkan(meld) => serializer.serialize_str(&to_tenhou_meld(meld)),
        }
    }
}

impl<'de> Deserialize<'de> for TenhouIncoming {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_any(TenhouIncomingVisitor)
    }
}

pub struct TenhouIncomingVisitor;
impl<'de> Visitor<'de> for TenhouIncomingVisitor {
    type Value = TenhouIncoming;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an integer (drawn tile) or string (meld)")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: Error {
        parse_tenhou_tile(v as u8)
            .map(TenhouIncoming::Draw)
            .ok_or_else(|| E::custom("not tenhou draw tile"))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> where E: Error {
        self.visit_i64(v as i64)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
        parse_tenhou_meld(v)
            .map(TenhouIncoming::ChiiPonDaiminkan)
            .ok_or_else(|| E::custom("not tenhou chii/pon/daiminkan"))
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TenhouOutgoing {
    DaiminkanDummy,
    Discard(Discard),
    KakanAnkan(Meld),
}

impl Serialize for TenhouOutgoing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            TenhouOutgoing::DaiminkanDummy =>
                serializer.serialize_u8(0),
            TenhouOutgoing::Discard(discard) => {
                let n = if discard.is_tsumogiri { 60 } else { to_tenhou_tile(discard.tile) };
                if discard.declares_riichi {
                    serializer.serialize_str(&format!("r{}", n))
                } else {
                    serializer.serialize_u8(n)
                }
            }
            TenhouOutgoing::KakanAnkan(meld) =>
                serializer.serialize_str(&to_tenhou_meld(meld)),
        }
    }
}

impl<'de> Deserialize<'de> for TenhouOutgoing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_any(TenhouOutgoingVisitor)
    }
}

pub struct TenhouOutgoingVisitor;
impl<'de> Visitor<'de> for TenhouOutgoingVisitor {
    type Value = TenhouOutgoing;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an integer (discarded tile) or string (riichi or meld)")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: Error {
        match v {
            0 => Ok(TenhouOutgoing::DaiminkanDummy),
            60 => Ok(TenhouOutgoing::Discard(Discard {
                tile: Default::default(),
                called_by: Default::default(),
                declares_riichi: false,
                is_tsumogiri: true,
            })),
            _ => {
                parse_tenhou_tile(v as u8)
                    .map(|tile|
                        TenhouOutgoing::Discard(Discard {
                            tile,
                            called_by: Default::default(),
                            declares_riichi: false,
                            is_tsumogiri: false,
                        }))
                    .ok_or_else(|| E::custom("not tenhou tile"))
            }
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
        if let Some('r') = v.chars().next() {
            v[1..].parse::<u8>().ok()
                .map(|n| if n == 60 {
                    TenhouOutgoing::Discard(Discard {
                        tile: Default::default(),
                        called_by: Default::default(),
                        declares_riichi: true,
                        is_tsumogiri: true,
                    })
                } else {
                    TenhouOutgoing::Discard(Discard {
                        tile: parse_tenhou_tile(n).unwrap(),
                        called_by: Default::default(),
                        declares_riichi: true,
                        is_tsumogiri: false,
                    })
                })
                .ok_or_else(|| E::custom("not tenhou riichi tile"))
        } else {
            parse_tenhou_meld(v)
                .map(TenhouOutgoing::KakanAnkan)
                .ok_or_else(|| E::custom("not tenhou kakan/ankan"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use serde_test::{Token, assert_tokens};
    use super::*;


    fn t(s: &str) -> Tile { Tile::from_str(s).unwrap() }

    #[test]
    fn tenhou_incoming_serde() {
        assert_tokens(
            &TenhouIncoming::Draw(t("0m")),
            &[Token::U8(51)]
        );
        assert_tokens(
            &TenhouIncoming::ChiiPonDaiminkan(Meld::Chii(Chii::from_tiles(
                t("1m"), t("3m"), t("2m")
            ).unwrap())),
            &[Token::Str("c121113")]
        );
    }

    #[test]
    fn tenhou_outgoing_serde() {
        assert_tokens(
            &TenhouOutgoing::Discard(Discard {
                tile: t("7z"),
                called_by: Default::default(),
                declares_riichi: false,
                is_tsumogiri: false,
            }),
            &[Token::U8(47)]
        );
        assert_tokens(
            &TenhouOutgoing::Discard(Discard {
                tile: Default::default(),
                called_by: Default::default(),
                declares_riichi: false,
                is_tsumogiri: true,
            }),
            &[Token::U8(60)]
        );
        assert_tokens(
            &TenhouOutgoing::Discard(Discard{
                tile: t("2s"),
                called_by: Default::default(),
                declares_riichi: true,
                is_tsumogiri: false,
            }),
            &[Token::Str("r32")]
        );
        assert_tokens(
            &TenhouOutgoing::Discard(Discard{
                tile: Default::default(),
                called_by: Default::default(),
                declares_riichi: true,
                is_tsumogiri: true,
            }),
            &[Token::Str("r60")]
        );
        assert_tokens(
            &TenhouOutgoing::KakanAnkan(Meld::Ankan(Ankan::from_tiles(
                [t("9m"), t("9m"), t("9m"), t("9m")]
            ).unwrap())),
            &[Token::Str("191919a19")]
        );
    }
}
