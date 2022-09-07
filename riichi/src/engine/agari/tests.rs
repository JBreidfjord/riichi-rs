use super::*;

struct InputBuilder {

    hand: TileSet37,

    waiting_info: WaitingInfo,
}


#[test]
fn simple_test() {
    let a = AgariInput {
        begin: &Default::default(),
        winner: Default::default(),
        contributor: Default::default(),
        action: Action::AbortNineKinds,
        num_dora_indicators: 0,
        num_draws: 0,
        is_first_chance: false,
        closed_hand: &Default::default(),
        waiting_info: &Default::default(),
        riichi_flags: Default::default(),
        melds: &[],
        incoming_meld: None
    };
}
