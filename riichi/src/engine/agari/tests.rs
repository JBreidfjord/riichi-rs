use crate::{
    analysis::Decomposer,
};
use super::*;

struct Hand {
    closed_hand: TileSet37,
    melds: Vec<Meld>,
    waiting_info: WaitingInfo,
}

impl Hand {
    pub fn new(
        closed_hand: impl IntoIterator<Item=Tile>,
        melds: impl IntoIterator<Item=Meld>,
    ) -> Hand {
        let closed_hand = TileSet37::from_iter(closed_hand);
        let melds = Vec::from_iter(melds);
        let waiting_info = WaitingInfo::from_keys(
            &mut Decomposer::new(),
            &closed_hand.packed_34(),
        );
        for w in waiting_info.regular.iter() {
            println!("{}", w);
        }
        Hand {
            closed_hand,
            melds,
            waiting_info,
        }
    }
}

impl<'a> AgariInput<'a> {
    fn from_hand(hand: &'a Hand) -> AgariInput<'a> {
        AgariInput {
            round_id: Default::default(),

            winner: Default::default(),
            closed_hand: &hand.closed_hand,
            riichi: Default::default(),
            melds: &hand.melds,
            waiting_info: &hand.waiting_info,

            contributor: Default::default(),
            winning_tile: Default::default(),
            incoming_is_kan: false,
            action_is_kan: false,

            is_first_chance: false,
            is_last_draw: false
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn mentsumo_example() {
    let hand = Hand::new(
        tiles_from_str("123456678m99p77z"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("9p"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Menzenchintsumohou));
    }
}

#[test]
fn mentsumo_negative_not_menzen() {
    let hand = Hand::new(
        tiles_from_str("456m123p12s44z"),
        [
            Meld::Chii(Chii::from_tiles(t!("1m"), t!("2m"), t!("3m")).unwrap()),
        ],
    );
    let agari_input = AgariInput {
        winning_tile: t!("3s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Menzenchintsumohou));
    }
}

#[test]
fn mentsumo_negative_not_tsumo() {
    let hand = Hand::new(
        tiles_from_str("123456m123p12s44z"),
        [],
    );
    let agari_input = AgariInput {
        contributor: P1,
        winning_tile: t!("3s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Menzenchintsumohou));
    }
}

#[test]
fn riichi_simple_example() {
    let hand = Hand::new(
        tiles_from_str("123456678m99p77z"),
        [],
    );
    let agari_input = AgariInput {
        contributor: P1,
        winning_tile: t!("9p"),
        riichi: Some(Riichi { is_double: false, is_ippatsu: true }),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Riichi));
        assert!(!candidate.yaku_values.contains_key(&Yaku::DoubleRiichi));
        assert!(candidate.yaku_values.contains_key(&Yaku::Ippatsu));
    }
}

#[test]
fn pinfu_simple_example() {
    let hand = Hand::new(
        tiles_from_str("123456m123p23s44z"),
        [],
    );
    let agari_input = AgariInput {
        contributor: P1,
        winning_tile: t!("4s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Pinfu));
        assert_eq!(candidate.scoring.fu, 30);  // closed ron
    }
}

#[test]
fn pinfu_negative_penchan() {
    let hand = Hand::new(
        tiles_from_str("123456m123p12s44z"),
        [],
    );
    let agari_input = AgariInput {
        contributor: P1,
        winning_tile: t!("3s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Pinfu));
    }
}

#[test]
fn pinfu_negative_prevalent_wind_pair() {
    let hand = Hand::new(
        tiles_from_str("123456m123p23s11z"),
        [],
    );
    let agari_input = AgariInput {
        contributor: P1,
        winning_tile: t!("1s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Pinfu));
    }
}

#[test]
fn pinfu_negative_self_wind_pair() {
    let hand = Hand::new(
        tiles_from_str("123456m123p23s44z"),
        [],
    );
    let agari_input = AgariInput {
        winner: P3,
        contributor: P1,
        winning_tile: t!("1s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Pinfu));
    }
}

#[test]
fn pinfu_negative_two_side_tanki() {
    let hand = Hand::new(
        tiles_from_str("456m123s3456678p"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("3p"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Pinfu));
    }
}

#[test]
fn pinfu_positive_two_side_tanki_alt_decomp() {
    let hand = Hand::new(
        tiles_from_str("456m123s3456678p"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("6p"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    assert!(candidates.iter().any(|candidate| {
        println!("{:?}", candidate);
        candidate.yaku_values.contains_key(&Yaku::Pinfu)
    }));
}

#[test]
fn iipeikou_example() {
    let hand = Hand::new(
        tiles_from_str("33445m99s999p777z"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("5m"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|candidate| {
        println!("{:?}", candidate);
        candidate.yaku_values.contains_key(&Yaku::Iipeikou)
    }));
}

#[test]
fn iipeikou_negative_open() {
    let hand = Hand::new(
        tiles_from_str("34m99s999p777z"),
        [
            Meld::Chii(Chii::from_tiles(t!("3m"), t!("4m"), t!("5m")).unwrap()),
        ],
    );
    let agari_input = AgariInput {
        winning_tile: t!("5m"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|candidate| {
        println!("{:?}", candidate);
        !candidate.yaku_values.contains_key(&Yaku::Iipeikou)
    }));
}

#[test]
fn iipeikou_downgrade() {
    let hand = Hand::new(
        tiles_from_str("33445m99s999p777z"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("2m"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|candidate| {
        println!("{:?}", candidate);
        !candidate.yaku_values.contains_key(&Yaku::Iipeikou)
    }));
}

#[test]
fn haitei_example() {
    let hand = Hand::new(
        tiles_from_str("123456678m99p77z"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("9p"),
        is_last_draw: true,
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Haiteimouyue));
        assert!(!candidate.yaku_values.contains_key(&Yaku::Houteiraoyui));
    }
}

#[test]
fn haitei_negative_rinshan() {
    let hand = Hand::new(
        tiles_from_str("456678m99p77z"),
        [
            Meld::Daiminkan(Daiminkan::from_tiles_dir(
                [t!("1m"), t!("1m"), t!("1m")], t!("1m"), P1,
            ).unwrap()),
        ],
    );
    let agari_input = AgariInput {
        winning_tile: t!("9p"),
        is_last_draw: true,
        incoming_is_kan: true,
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Haiteimouyue));
        assert!(candidate.yaku_values.contains_key(&Yaku::Rinshankaihou));
    }
}

#[test]
fn houtei_example() {
    let hand = Hand::new(
        tiles_from_str("123456678m99p77z"),
        [],
    );
    let agari_input = AgariInput {
        contributor: P1,
        winning_tile: t!("9p"),
        is_last_draw: true,
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Haiteimouyue));
        assert!(candidate.yaku_values.contains_key(&Yaku::Houteiraoyui));
    }
}

#[test]
fn tanyao_example() {
    let hand = Hand::new(
        tiles_from_str("234456678m55p66s"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("0p"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Tanyaochuu));
    }
}

#[test]
fn tanyao_open_example() {
    let hand = Hand::new(
        tiles_from_str("456678m55p66s"),
        [
            Meld::Chii(Chii::from_tiles(t!("2m"), t!("3m"), t!("4m")).unwrap()),
        ],
    );
    let agari_input = AgariInput {
        winning_tile: t!("0p"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Tanyaochuu));
    }
}

#[test]
fn tanyao_negative() {
    let hand = Hand::new(
        tiles_from_str("234456678m55p66z"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("0p"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Tanyaochuu));
    }
}

// TODO(summivox): test the following yakus
// winds, dragons (with kans)
// chanta
// 3shoku
// 1-tsu
// toitoi
// 3ankou
// 3shoku doukou
// 3kantsu

#[test]
fn chiitoi_example() {
    let hand = Hand::new(
        tiles_from_str("114477m225588p3s"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("3s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Chiitoitsu));
    }
}

#[test]
fn chiitoi_with_honroutou() {
    let hand = Hand::new(
        tiles_from_str("1199m1199p1199s7z"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("7z"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Chiitoitsu));
        assert!(candidate.yaku_values.contains_key(&Yaku::Honroutou));
    }
}

#[test]
fn chiitoi_upgrade_ryanpeikou() {
    let hand = Hand::new(
        tiles_from_str("112233m4455667s"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("7s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    assert!(candidates.iter().any(|candidate|
        candidate.yaku_values.contains_key(&Yaku::Chiitoitsu)));
    assert!(candidates.iter().any(|candidate|
        candidate.yaku_values.contains_key(&Yaku::Ryanpeikou)));
}

#[test]
fn chiitoi_negative_open() {
    let hand = Hand::new(
        tiles_from_str("4455667s"),
        [
            Meld::Chii(Chii::from_tiles(t!("3m"), t!("4m"), t!("5m")).unwrap()),
            Meld::Chii(Chii::from_tiles(t!("3m"), t!("4m"), t!("5m")).unwrap()),
        ],
    );
    let agari_input = AgariInput {
        winning_tile: t!("7s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|candidate|
        !candidate.yaku_values.contains_key(&Yaku::Chiitoitsu)));
}

// TODO(summivox): test the following yakus
// honroutou
// shou3gen
// honnitsu
// jun-chanta
// ryanpeikou
// chinnitsu
// kokushi
// 4ankou
// dai3gen
// shou4shii
// dai4shii
// tsuuiisou
// chinroutou
// ryuuiisou
// 4kantsu

#[test]
fn chuuren_example() {
    let hand = Hand::new(
        tiles_from_str("1111234567899m"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("9m"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Chuurenpoutou));
    }
}

#[test]
fn jun_chuuren_example() {
    let hand = Hand::new(
        tiles_from_str("1112345678999m"),
        [],
    );
    let agari_input = AgariInput {
        winner: P0,
        contributor: P1,
        winning_tile: t!("9m"),
        action_is_kan: true,
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(candidate.yaku_values.contains_key(&Yaku::Junseichuurenpoutou));
    }
}

#[test]
fn chuuren_downgrade() {
    let hand = Hand::new(
        tiles_from_str("1111234567899m"),
        [],
    );
    let agari_input = AgariInput {
        winning_tile: t!("6m"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);

    assert!(!candidates.is_empty());
    for candidate in candidates {
        println!("{:?}", candidate);
        assert!(!candidate.yaku_values.contains_key(&Yaku::Chuurenpoutou));
        assert!(!candidate.yaku_values.contains_key(&Yaku::Junseichuurenpoutou));
    }
}
