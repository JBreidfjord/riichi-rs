
use serde_tuple::{Serialize_tuple, Deserialize_tuple};
use crate::{
    common::*,
    model::*,
};
use super::{
    end_info::*,
    entry::*,
};

/// Serde model for the compact representation of a round.
#[derive(Clone, Debug, Default, Serialize_tuple, Deserialize_tuple)]
#[cfg_attr(test, derive(Eq, PartialEq))]  // No need to compare other than in tests.
pub struct TenhouRoundRaw {
    pub round_id_and_pot: RoundIdAndPot,
    pub points: [GamePoints; 4],
    pub dora_indicators: Vec<TenhouIncoming>,
    pub ura_dora_indicators: Vec<TenhouIncoming>,

    pub deal0: Vec<TenhouIncoming>,
    pub incoming0: Vec<TenhouIncoming>,
    pub outgoing0: Vec<TenhouOutgoing>,

    pub deal1: Vec<TenhouIncoming>,
    pub incoming1: Vec<TenhouIncoming>,
    pub outgoing1: Vec<TenhouOutgoing>,

    pub deal2: Vec<TenhouIncoming>,
    pub incoming2: Vec<TenhouIncoming>,
    pub outgoing2: Vec<TenhouOutgoing>,

    pub deal3: Vec<TenhouIncoming>,
    pub incoming3: Vec<TenhouIncoming>,
    pub outgoing3: Vec<TenhouOutgoing>,

    pub end_info: TenhouEndInfo,
}

/// [`RoundId`] plus the _number_ of riichi sticks.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Serialize_tuple, Deserialize_tuple)]
pub struct RoundIdAndPot {
    /// Same as in [`RoundId`].
    pub kyoku: u8,
    /// Same as in [`RoundId`].
    pub honba: u8,

    /// Represents 1000 x value in [`GamePoints`] (i.e. the _number_ of riichi sticks).
    /// This is different from the definition in [`RoundBegin`].
    pub pot_count: u8,
}

impl RoundIdAndPot {
    pub fn from_parts(round_id: RoundId, pot_count: u8) -> Self {
        Self {
            kyoku: round_id.kyoku,
            honba: round_id.honba,
            pot_count,
        }
    }
    pub fn round_id(self) -> RoundId {
        RoundId { kyoku: self.kyoku, honba: self.honba }
    }
    pub fn to_parts(self) -> (RoundId, u8) {
        (self.round_id(), self.pot_count)
    }
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;
    use itertools::Itertools;
    use crate::model::Discard;
    use crate::interop::tenhou_log::*;
    use super::*;

    fn t(s: &str) -> Tile { t!(s) }
    fn it(s: &str) -> TenhouIncoming { TenhouIncoming::Draw(t(s)) }
    fn ts(s: &str) -> Vec<Tile> { tiles_from_str(s) }
    fn its(s: &str) -> Vec<TenhouIncoming> {
        ts(s).into_iter().map(TenhouIncoming::Draw).collect_vec()
    }
    fn od(s: &str) -> TenhouOutgoing {
        TenhouOutgoing::Discard(Discard{
            tile: t(s),
            called_by: Default::default(),
            declares_riichi: false,
            is_tsumokiri: false,
        })
    }
    fn odd() -> TenhouOutgoing {
        TenhouOutgoing::Discard(Discard{
            tile: Default::default(),
            called_by: Default::default(),
            declares_riichi: false,
            is_tsumokiri: true,
        })
    }
    fn or(s: &str) -> TenhouOutgoing {
        TenhouOutgoing::Discard(Discard{
            tile: t(s),
            called_by: Default::default(),
            declares_riichi: true,
            is_tsumokiri: false,
        })
    }

    #[allow(unused)]
    fn orr() -> TenhouOutgoing {
        TenhouOutgoing::Discard(Discard{
            tile: Default::default(),
            called_by: Default::default(),
            declares_riichi: true,
            is_tsumokiri: true,
        })
    }

    #[test]
    fn round_json_example() {
        let round_struct = TenhouRoundRaw {
            round_id_and_pot: RoundIdAndPot { kyoku: 3, honba: 2, pot_count: 2 },
            points: [45200, 19500, 7300, 26000],
            dora_indicators: vec![it("8s"), it("8m")],
            ura_dora_indicators: vec![],

            deal0: its("1147m67p1458s456z"),
            incoming0: vec![
                it("1p"), it("3p"), it("3z"), it("6p"), it("1z"), it("2m"),
                it("1p"), it("9s"), it("6z"),
            ],
            outgoing0: vec![
                odd(), od("1s"), od("4z"), od("3z"), od("3p"), od("7m"),
                od("4m"), od("6z"), odd(),
            ],

            deal1: its("1247m55p122567s1z"),
            incoming1: vec![
                it("8p"), it("4p"), it("9s"), it("3m"),
                TenhouIncoming::ChiiPonDaiminkan(Meld::Chii(Chii::from_tiles(
                    t("4p"), t("5p"), t("3p")
                ).unwrap())),
                it("3m"), it("8s"), it("6s"), it("7s"),
            ],
            outgoing1: vec![
                od("1m"), od("1z"), od("8p"), od("9s"), od("1s"), odd(),
                od("7m"), od("5p")
            ],

            deal2: its("999m1334077p2s36z"),
            incoming2: its("2z6p4s2p7m9m4m3m8p"),
            outgoing2: vec![
                od("3z"), od("2z"), od("6z"), or("3p"), odd(),
                TenhouOutgoing::KakanAnkan(Meld::Ankan(Ankan::from_tiles([
                    t("9m"), t("9m"), t("9m"), t("9m"),
                ]).unwrap())),
                odd(), odd(), odd(),
            ],

            deal3: its("556m149p1348s237z"),
            incoming3: its("8p8m4z2s3z8m6z2p3s"),
            outgoing3: vec![
                od("1p"), od("2z"), odd(), od("3z"), odd(), od("1s"),
                odd(), od("6m"), od("8p"),
            ],

            end_info: TenhouEndInfo {
                result: ActionResult::Agari(AgariKind::Tsumo),
                overall_delta: [-500, 4700, -500, 700],
                agari: vec![
                    TenhouAgariResult {
                        winner: P1,
                        contributor: P1,
                        liable_player: P1,
                        points_delta_after_pot: [-500, 4700, -500, -700],
                        scoring: TenhouScoring {
                            kind: TenhouScoringKind::HanFu { han: 1, fu: 30 },
                            payout: TenhouPayout::TsumoByNonButton { non_button: 300, button: 500 },
                        },
                        details: vec![
                            YakuOrDora::Yaku(Yaku::Tanyaochuu, 1),
                        ]
                    },
                ]
            },
        };
        // from: 2022013100gm-00a9-0000-af91b2de.json
        let round_json = serde_json::json!([
          [3, 2, 2],  // 0
          [45200, 19500, 7300, 26000],
          [38, 18],
          [],

          [11, 11, 14, 17, 26, 27, 31, 34, 35, 38, 44, 45, 46],  // 4
          [21, 23, 43, 26, 41, 12, 21, 39, 46],
          [60, 31, 44, 43, 23, 17, 14, 46, 60],

          [11, 12, 14, 17, 25, 25, 31, 32, 32, 35, 36, 37, 41],  // 7
          [28, 24, 39, 13, "c232425", 13, 38, 36, 37],
          [11, 41, 28, 39, 31, 60, 17, 25],

          [19, 19, 19, 21, 23, 23, 24, 52, 27, 27, 32, 43, 46],  // 10
          [42, 26, 34, 22, 17, 19, 14, 13, 28],
          [43, 42, 46, "r23", 60, "191919a19", 60, 60, 60],

          [15, 15, 16, 21, 24, 29, 31, 33, 34, 38, 42, 43, 47],  // 13
          [28, 18, 44, 32, 43, 18, 46, 22, 33],
          [21, 42, 60, 43, 60, 31, 60, 16, 28],

          ["和了", [-500, 4700, -500, -700], [1, 1, 1, "30符1飜300-500点", "断幺九(1飜)"]]
        ]);
        let deserialized = serde_json::from_value::<TenhouRoundRaw>(round_json.clone()).unwrap();
        let serialized = serde_json::to_value(&round_struct).unwrap();
        assert_json_eq!(round_json, serialized);
        assert_json_eq!(round_struct, deserialized);
    }
}
