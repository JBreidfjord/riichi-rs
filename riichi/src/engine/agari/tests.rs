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
