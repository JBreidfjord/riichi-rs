use crate::analysis::Decomposer;
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
            &closed_hand.packed(),
        );
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
            riichi_flags: Default::default(),
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

#[test]
fn mentsumo_simple_example() {
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
        assert!(candidate.yaku_values.contains_key(&Yaku::Menzenchintsumohou));
        println!("{:?}", candidate);
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
        assert!(!candidate.yaku_values.contains_key(&Yaku::Menzenchintsumohou));
        println!("{:?}", candidate);
    }
}

#[test]
fn mentsumo_negative_not_tsumo() {
    let hand = Hand::new(
        tiles_from_str("123456m123p12s44z"),
        [],
    );
    let agari_input = AgariInput {
        contributor: Player::new(1),
        winning_tile: t!("3s"),
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    assert!(!candidates.is_empty());
    for candidate in candidates {
        assert!(!candidate.yaku_values.contains_key(&Yaku::Menzenchintsumohou));
        println!("{:?}", candidate);
    }
}

#[test]
fn print_example() {
    let hand = Hand::new(
        tiles_from_str("1111234567899m"),
        [],
    );
    let agari_input = AgariInput {
        winner: Player::new(0),
        contributor: Player::new(1),
        winning_tile: t!("9m"),
        action_is_kan: true,
        ..AgariInput::from_hand(&hand)
    };
    let candidates = agari_candidates(&Default::default(), &agari_input);
    for candidate in candidates {
        println!("{:?}", candidate);
    }
}
