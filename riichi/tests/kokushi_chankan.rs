//! [`Ruleset::kokushi_chankan_allow_ankan`] must gate the reaction, not only the Furiten.
//!
//! The option answers one question -- may a Thirteen Orphans wait rob a closed Kan? -- and it
//! was only ever consulted in `next_normal`'s Furiten loop, which runs *after* the reaction has
//! already been accepted. `check_reaction` waved every kokushi wait through, so an Ankan was
//! robbable under every ruleset including the default that forbids it.
//!
//! It stayed invisible because no wrapper in the wild opened a reaction window on an Ankan;
//! reaching it needed a caller that offers every seat a window on every non-terminal action.
//!
//! Both directions are asserted here: off must refuse, on must accept. A test that only pinned
//! the default would pass against a hardcoded refusal, which is the other way to get this wrong.

use riichi::engine::{check_reaction, EngineCache};
use riichi::prelude::*;
use riichi::rules::Ruleset;

/// Thirteen Orphans, one tile short of the set, waiting on East (1z). The pair is 3z, so this is
/// the ordinary 13-orphans tenpai and not the 13-way wait.
const KOKUSHI_WAITING_ON_EAST: &str = "19m19p19s2334567z";

/// A hand with no irregular wait at all -- the control that keeps the assertions below from
/// passing because *nothing* can rob an Ankan.
const ORDINARY_TENPAI: &str = "123m456m789m123p5s";

fn yonma_begin(allow: bool) -> RoundBegin {
    let mut ruleset = Ruleset::for_variant(Variant::Yonma);
    ruleset.kokushi_chankan_allow_ankan = allow;
    RoundBegin {
        ruleset,
        round_id: RoundId { kyoku: 0, honba: 0 },
        wall: wall::make_sorted_wall([0, 0, 0]),
        pot: 0,
        points: [25000; 4],
    }
}

/// P0 declares an Ankan of `tile`; P1 holds `hand` and tries to Ron it.
fn ron_over_ankan(allow: bool, hand: &str, tile: Tile) -> Result<(), String> {
    let begin = yonma_begin(allow);
    let mut state = State::new(&begin);
    state.core.actor = P0;
    state.closed_hands[1] = TileSet37::from_iter(tiles_from_str(hand));
    let mut cache = EngineCache::new();
    cache.init_wait_cache(&state.closed_hands);
    check_reaction(&begin, &state, Action::Ankan(tile), P1, Reaction::RonAgari, &mut cache)
        .map_err(|e| e.to_string())
}

#[test]
fn kokushi_cannot_rob_an_ankan_when_the_ruleset_forbids_it() {
    assert!(!Ruleset::default().kokushi_chankan_allow_ankan,
            "the default is 'no', and this test is about honouring it");

    let err = ron_over_ankan(false, KOKUSHI_WAITING_ON_EAST, t!("1z")).unwrap_err();
    assert!(err.contains("Cannot Ron over Ankan"), "unexpected error: {}", err);
}

#[test]
fn kokushi_can_rob_an_ankan_when_the_ruleset_allows_it() {
    assert!(ron_over_ankan(true, KOKUSHI_WAITING_ON_EAST, t!("1z")).is_ok(),
            "with the option on, a Thirteen Orphans wait may rob a closed Kan");
}

/// Neither setting lets an ordinary wait rob an Ankan: the option widens the kokushi carve-out,
/// it does not open Ankan to chankan generally.
#[test]
fn an_ordinary_wait_never_robs_an_ankan() {
    for allow in [false, true] {
        let err = ron_over_ankan(allow, ORDINARY_TENPAI, t!("5s")).unwrap_err();
        assert!(err.contains("Cannot Ron over Ankan"),
                "allow={}: unexpected error: {}", allow, err);
    }
}
