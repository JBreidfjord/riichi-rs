//! Sanma (3-player) engine tests.
//!
//! Deliberately scoped to **what a replay oracle cannot reach at volume**: the absent-seat
//! invariant (which real logs cannot exercise, because they never mention a fourth seat), the
//! settlement arithmetic at values a log only shows indirectly, and the legality gates that fire
//! on cases too rare to rely on finding.
//!
//! Everything here is model-free: no wall shuffling, no log fixtures.

use riichi::engine::utils::*;
use riichi::engine::{check_action, check_reaction, distribute_points, Engine, EngineCache};
use riichi::prelude::*;
use riichi::rules::Ruleset;

fn sanma_ruleset() -> Ruleset {
    Ruleset::for_variant(Variant::Sanma)
}

fn sanma_begin(kyoku: u8) -> RoundBegin {
    RoundBegin {
        ruleset: sanma_ruleset(),
        round_id: RoundId { kyoku, honba: 0 },
        wall: wall::make_sorted_wall_in(Variant::Sanma, [0, 0, 0]),
        pot: 0,
        points: [35000, 35000, 35000, 0],
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// The absent seat
//////////////////////////////////////////////////////////////////////////////////////////////////

/// The core invariant ADR 0006 buys byte-stability with, and which the type system cannot
/// enforce: seat 3 is present in every array and plays no part in the game.
#[test]
fn absent_seat_is_dealt_nothing_and_never_acts() {
    for kyoku in [0u8, 1, 2, 4, 5, 6] {
        let begin = sanma_begin(kyoku);
        let mut engine = Engine::new();
        engine.begin_round(begin);

        let absent = Variant::Sanma.absent_seat().unwrap();
        let state = engine.state();

        // Arrays are still 4-wide ...
        assert_eq!(state.closed_hands.len(), 4);
        assert_eq!(state.melds.len(), 4);
        assert_eq!(state.discards.len(), 4);
        // ... and the absent seat's slots are empty, not short.
        assert_eq!(state.closed_hands[absent.to_usize()].0.iter().map(|&n| n as u32).sum::<u32>(),
                   0, "kyoku {}: absent seat was dealt tiles", kyoku);
        assert!(state.melds[absent.to_usize()].is_empty());
        assert!(state.discards[absent.to_usize()].is_empty());

        // Each real seat got a full hand.
        for &p in Variant::Sanma.active_seats() {
            assert_eq!(state.closed_hands[p.to_usize()].0.iter().map(|&n| n as u32).sum::<u32>(),
                       13, "kyoku {}: seat {} short-dealt", kyoku, p);
        }

        // The button is never the absent seat.
        assert_ne!(state.core.actor, absent, "kyoku {} put the absent seat on the button", kyoku);
    }
}

/// Turn order must wrap `P2 -> P0`. The mod-4 successor of P2 is the absent seat, so this is a
/// place a missed filter produces a game that hangs rather than a wrong number.
#[test]
fn turn_passes_from_p2_back_to_p0() {
    let mut engine = Engine::new();
    engine.begin_round(sanma_begin(0));

    let mut seen = vec![];
    for _ in 0..6 {
        let actor = engine.state().core.actor;
        seen.push(actor.to_u8());
        let tile = engine.state().core.draw.unwrap();
        engine.register_action(Action::Discard(Discard {
            tile, called_by: actor, is_tsumogiri: true, declares_riichi: false,
        })).unwrap();
        engine.step();
    }
    assert_eq!(seen, vec![0, 1, 2, 0, 1, 2]);
}

/// Regression for the vacuous-truth bug: the absent seat's discard list is empty, and
/// `[].iter().all(..)` is `true`, so an unfiltered scan hands it a Nagashi Mangan in every single
/// sanma round.
#[test]
fn absent_seat_never_wins_nagashi_mangan() {
    let mut state = State::default();
    // Give every real seat one plainly non-terminal discard, so none of them qualifies.
    for &p in Variant::Sanma.active_seats() {
        state.discards[p.to_usize()].push(Discard {
            tile: t!("5p"), called_by: p, is_tsumogiri: false, declares_riichi: false,
        });
    }
    // Seat 3 has no discards at all.
    assert!(state.discards[3].is_empty());

    assert!(!is_any_player_nagashi_mangan(Variant::Sanma, &state),
            "the absent seat was awarded a Nagashi Mangan");
    assert!(!is_nagashi_mangan(Variant::Sanma, &state, P3));

    // The same state in yonma *does* trip, which is what makes this a guard and not a no-op:
    // seat 3 there is a real player who genuinely discarded nothing.
    assert!(is_nagashi_mangan(Variant::Yonma, &state, P3));
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Kita legality
//////////////////////////////////////////////////////////////////////////////////////////////////

/// Drives the engine to the given seat's turn, replacing its hand, so the Kita gates can be
/// probed without a scripted 50-turn game.
fn kita_probe(hand: &str, draw: Option<Tile>, riichi: bool, num_draws_used: u8)
    -> Result<(), String>
{
    let begin = sanma_begin(0);
    let mut state = State::new(&begin);
    state.closed_hands[0] = TileSet37::from_iter(tiles_from_str(hand));
    state.core.actor = P0;
    state.core.draw = draw;
    state.core.num_drawn_head = num_draws_used;
    if riichi {
        state.core.riichi[0] = Some(Riichi { is_double: false, is_ippatsu: false });
    }
    let mut cache = EngineCache::new();
    cache.init_wait_cache(&state.closed_hands);
    check_action(&begin, &state, Action::Kita(t!("4z")), &mut cache)
        .map_err(|e| e.to_string())
}

#[test]
fn kita_takes_any_north_in_hand_not_only_the_drawn_tile() {
    // North held in hand, drew something else entirely: legal.
    // ~12,600 of 23,104 houou 3p extractions were of a North already held.
    assert!(kita_probe("119m1234567899p4z", Some(t!("1p")), false, 41).is_ok());

    // No North anywhere: rejected.
    let err = kita_probe("119m1234567899p1z", Some(t!("1p")), false, 41).unwrap_err();
    assert!(err.contains("no North"), "unexpected error: {}", err);
}

/// Not stated anywhere in Tenhou's rule text, but decisive in the logs: of 23,104 extractions,
/// zero occur after the 55th (last) draw. At draw 55 the behaviour flips -- 34 players drew a
/// North and discarded it, 0 extracted.
#[test]
fn kita_is_barred_on_the_haitei_draw() {
    let max = Variant::Sanma.max_num_draws();
    // One draw before the end: fine.
    assert!(kita_probe("119m1234567899p4z", Some(t!("1p")), false, max - 1).is_ok());
    // The last draw: barred, exactly like a Kan.
    let err = kita_probe("119m1234567899p4z", Some(t!("1p")), false, max).unwrap_err();
    assert!(err.contains("last draw"), "unexpected error: {}", err);
}

/// UNSETTLED (see the note in `check_action`): no source states this and #115 did not test it.
/// The conservative reading is that a committed riichi hand may only extract a *just-drawn*
/// North, mirroring the Okuri-Kan prohibition. Since Norths are fungible it costs nothing.
#[test]
fn kita_under_riichi_requires_the_drawn_north() {
    // Drew the North: legal, and routine (1,029 observed extractions by a riichi seat).
    assert!(kita_probe("119m1234567899p4z", Some(t!("4z")), true, 41).is_ok());
    // Drew something else, North sitting in the standing hand: rejected.
    let err = kita_probe("119m1234567899p4z", Some(t!("1p")), true, 41).unwrap_err();
    assert!(err.contains("riichi"), "unexpected error: {}", err);
}

#[test]
fn kita_is_rejected_in_yonma() {
    let mut begin = sanma_begin(0);
    begin.ruleset = Ruleset::default();
    begin.wall = wall::make_sorted_wall([0, 0, 0]);
    let mut state = State::new(&begin);
    state.closed_hands[0] = TileSet37::from_iter(tiles_from_str("119m1234567899p4z"));
    state.core.actor = P0;
    state.core.draw = Some(t!("1p"));
    let mut cache = EngineCache::new();
    cache.init_wait_cache(&state.closed_hands);
    let err = check_action(&begin, &state, Action::Kita(t!("4z")), &mut cache)
        .unwrap_err().to_string();
    assert!(err.contains("Yonma"), "unexpected error: {}", err);
}

/// 「チーはできない」. Also: the Kita reaction window is ron-only, and that needs no code --
/// Chii/Pon/Daiminkan all require a Discard to react to.
#[test]
fn chii_is_rejected_in_sanma_and_no_call_reacts_to_a_kita() {
    let begin = sanma_begin(0);
    let mut state = State::new(&begin);
    state.core.actor = P0;
    let mut cache = EngineCache::new();
    cache.init_wait_cache(&state.closed_hands);

    let discard = Action::Discard(Discard {
        tile: t!("2p"), called_by: P0, is_tsumogiri: false, declares_riichi: false,
    });
    let err = check_reaction(&begin, &state, discard, P1, Reaction::Chii(t!("3p"), t!("4p")),
                             &mut cache).unwrap_err().to_string();
    assert!(err.contains("Chii is not allowed"), "unexpected error: {}", err);

    // Nothing but a Ron may react to a Kita.
    let kita = Action::Kita(t!("4z"));
    for reaction in [Reaction::Pon(t!("4z"), t!("4z")), Reaction::Daiminkan] {
        let err = check_reaction(&begin, &state, kita, P1, reaction, &mut cache)
            .unwrap_err().to_string();
        assert!(err.contains("only call a discarded tile") || err.contains("not allowed"),
                "kita accepted a non-ron reaction {:?}: {}", reaction, err);
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Settlement arithmetic
//////////////////////////////////////////////////////////////////////////////////////////////////

/// Total 2000, not the yonma 3000 with a seat removed -- the pot itself shrinks.
/// Verified against 1,127 houou 3p exhaustive draws.
#[test]
fn noten_bappu_total_is_2000_in_sanma() {
    let v = Variant::Sanma;
    assert_eq!(calc_wall_exhausted_delta(v, [1, 0, 0, 0]), [2000, -1000, -1000, 0]);
    assert_eq!(calc_wall_exhausted_delta(v, [0, 1, 1, 0]), [-2000, 1000, 1000, 0]);
    assert_eq!(calc_wall_exhausted_delta(v, [1, 1, 1, 0]), [0, 0, 0, 0]);
    assert_eq!(calc_wall_exhausted_delta(v, [0, 0, 0, 0]), [0, 0, 0, 0]);
    // Every schedule is zero-sum and never touches the absent seat.
    for w in [[1, 0, 0, 0], [0, 1, 1, 0], [1, 1, 0, 0]] {
        let d = calc_wall_exhausted_delta(v, w);
        assert_eq!(d.iter().sum::<GamePoints>(), 0);
        assert_eq!(d[3], 0);
    }
}

#[test]
fn noten_bappu_is_unchanged_in_yonma() {
    let v = Variant::Yonma;
    assert_eq!(calc_wall_exhausted_delta(v, [1, 0, 0, 0]), [3000, -1000, -1000, -1000]);
    assert_eq!(calc_wall_exhausted_delta(v, [1, 1, 0, 0]), [1500, 1500, -1500, -1500]);
    assert_eq!(calc_wall_exhausted_delta(v, [1, 1, 1, 0]), [1000, 1000, 1000, -3000]);
    assert_eq!(calc_wall_exhausted_delta(v, [1, 1, 1, 1]), [0, 0, 0, 0]);
    assert_eq!(calc_wall_exhausted_delta(v, [0, 0, 0, 0]), [0, 0, 0, 0]);
}

/// Pure tsumo loss: every per-payer amount is the yonma amount, and the absent seat's
/// non-dealer share is simply never paid. A non-dealer mangan tsumo collects 6000, not 8000.
#[test]
fn sanma_tsumo_loss() {
    let ruleset = sanma_ruleset();
    let e1 = RoundId { kyoku: 0, honba: 0 };  // button = P0
    let mangan = 2000;

    // Non-dealer (P1) tsumo: 4000 from the dealer + 2000 from the other non-dealer = 6000.
    // The yonma equivalent is 8000.
    assert_eq!(distribute_points(&ruleset, e1, true, P1, P1, mangan),
               [-4000, 6000, -2000, 0]);
    // Dealer (P0) tsumo: 4000 x 2 = 8000. Yonma pays 12000.
    assert_eq!(distribute_points(&ruleset, e1, true, P0, P0, mangan),
               [8000, -4000, -4000, 0]);
    // Ron totals are unchanged by tsumo loss.
    assert_eq!(distribute_points(&ruleset, e1, true, P1, P2, mangan),
               [0, 8000, -8000, 0]);
    assert_eq!(distribute_points(&ruleset, e1, true, P0, P1, mangan),
               [12000, -12000, 0, 0]);
}

/// 200 per honba on ron -- `(players - 1) x 100` -- and 100 per payer on tsumo, which needed no
/// change. Confirmed on 3,265 houou 3p wins.
#[test]
fn sanma_honba_is_200_on_ron_and_100_per_payer_on_tsumo() {
    let ruleset = sanma_ruleset();
    let r = RoundId { kyoku: 0, honba: 2 };
    let mangan = 2000;

    // Ron: 8000 + 2 honba x 200 = 8400.
    assert_eq!(distribute_points(&ruleset, r, true, P1, P2, mangan),
               [0, 8400, -8400, 0]);
    // Tsumo: (4000 + 200) + (2000 + 200) = 6400.
    assert_eq!(distribute_points(&ruleset, r, true, P1, P1, mangan),
               [-4200, 6400, -2200, 0]);

    // Yonma is untouched: 8000 + 2 x 300 = 8600.
    let y = Ruleset::default();
    assert_eq!(distribute_points(&y, r, true, P1, P2, mangan),
               [0, 8600, -8600, 0]);
}

/// Nagashi Mangan settles as a Mangan **tsumo**, so tsumo loss applies to it too: 19/19 observed
/// non-dealer nagashi pay 4000 + 2000 = 6000, not the 8000 a yonma non-dealer mangan tsumo pays.
#[test]
fn nagashi_mangan_takes_tsumo_loss() {
    let mut state = State::default();
    // Seat 1 (a non-dealer, button is P0) discarded only terminals, uncalled.
    for tile in tiles_from_str("119m9p1z") {
        state.discards[1].push(Discard {
            tile, called_by: P1, is_tsumogiri: false, declares_riichi: false,
        });
    }
    // Seats 0 and 2 discarded something ordinary.
    for p in [P0, P2] {
        state.discards[p.to_usize()].push(Discard {
            tile: t!("5p"), called_by: p, is_tsumogiri: false, declares_riichi: false,
        });
    }

    assert_eq!(calc_nagashi_mangan_delta(Variant::Sanma, &state, P0),
               [-4000, 6000, -2000, 0]);

    // Dealer nagashi: 4000 from each of the two payers. UNOBSERVED in the sample -- inferred.
    let mut dealer = State::default();
    for tile in tiles_from_str("119m9p1z") {
        dealer.discards[0].push(Discard {
            tile, called_by: P0, is_tsumogiri: false, declares_riichi: false,
        });
    }
    for p in [P1, P2] {
        dealer.discards[p.to_usize()].push(Discard {
            tile: t!("5p"), called_by: p, is_tsumogiri: false, declares_riichi: false,
        });
    }
    assert_eq!(calc_nagashi_mangan_delta(Variant::Sanma, &dealer, P0),
               [8000, -4000, -4000, 0]);
}

/// The rewrite from a whole-table lump into a per-payer loop must be byte-identical in yonma.
#[test]
fn nagashi_mangan_is_unchanged_in_yonma() {
    let mut state = State::default();
    for tile in tiles_from_str("119m9p1z") {
        state.discards[1].push(Discard {
            tile, called_by: P1, is_tsumogiri: false, declares_riichi: false,
        });
    }
    for p in [P0, P2, P3] {
        state.discards[p.to_usize()].push(Discard {
            tile: t!("5p"), called_by: p, is_tsumogiri: false, declares_riichi: false,
        });
    }
    // Non-dealer: +8000, dealer -4000, others -2000.
    assert_eq!(calc_nagashi_mangan_delta(Variant::Yonma, &state, P0),
               [-4000, 8000, -2000, -2000]);

    let mut dealer = State::default();
    for tile in tiles_from_str("119m9p1z") {
        dealer.discards[0].push(Discard {
            tile, called_by: P0, is_tsumogiri: false, declares_riichi: false,
        });
    }
    for p in [P1, P2, P3] {
        dealer.discards[p.to_usize()].push(Discard {
            tile: t!("5p"), called_by: p, is_tsumogiri: false, declares_riichi: false,
        });
    }
    assert_eq!(calc_nagashi_mangan_delta(Variant::Yonma, &dealer, P0),
               [12000, -4000, -4000, -4000]);
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Aborts that must never fire in sanma
//////////////////////////////////////////////////////////////////////////////////////////////////

/// 「三人打ちの三人立直は流局にならない」 -- all three players under riichi is explicitly NOT an
/// abort. The gate is explicit so that nobody generalizes four-riichi to three.
#[test]
fn three_riichi_is_not_an_abort_in_sanma() {
    let mut state = State::default();
    for p in [P0, P1] {
        state.core.riichi[p.to_usize()] = Some(Riichi { is_double: false, is_ippatsu: false });
    }
    state.core.actor = P2;
    let action = Action::Discard(Discard {
        tile: t!("5p"), called_by: P2, is_tsumogiri: true, declares_riichi: true,
    });
    assert!(!is_aborted_four_riichi(Variant::Sanma, &state, action));

    // In yonma the same shape (3 active + a 4th declaring) *is* the abort.
    let mut y = state.clone();
    y.core.riichi[2] = Some(Riichi { is_double: false, is_ippatsu: false });
    y.core.actor = P3;
    let y_action = Action::Discard(Discard {
        tile: t!("5p"), called_by: P3, is_tsumogiri: true, declares_riichi: true,
    });
    assert!(is_aborted_four_riichi(Variant::Yonma, &y, y_action));
}

/// There are only three first discards, and there is no three-wind variant.
#[test]
fn four_wind_abort_never_fires_in_sanma() {
    let mut state = State::default();
    state.core.seq = Variant::Sanma.first_chance_max_seq();
    state.core.actor = P2;
    for p in [P0, P1] {
        state.discards[p.to_usize()].push(Discard {
            tile: t!("1z"), called_by: p, is_tsumogiri: true, declares_riichi: false,
        });
    }
    let action = Action::Discard(Discard {
        tile: t!("1z"), called_by: P2, is_tsumogiri: true, declares_riichi: false,
    });
    assert!(!is_aborted_four_wind(Variant::Sanma, &state, action));
}

/// The first uninterrupted go-around is 3 turns, not 4. Using 4 would let the dealer's *second*
/// discard declare Double Riichi.
#[test]
fn first_chance_is_three_turns_in_sanma() {
    assert_eq!(Variant::Sanma.first_chance_max_seq(), 2);
    assert_eq!(Variant::Yonma.first_chance_max_seq(), 3);

    let mut state = State::default();
    state.core.seq = 3;
    assert!(!is_first_chance(Variant::Sanma, &state));
    assert!(is_first_chance(Variant::Yonma, &state));

    state.core.seq = 2;
    assert!(is_first_chance(Variant::Sanma, &state));

    // A Kita counts as an interruption, like any call:
    // 「抜きは鳴きと同じ扱い(一発/地和/九種/両立直は消える)」.
    state.melds[0].push(Meld::Kita(Kita::new()));
    assert!(!is_first_chance(Variant::Sanma, &state));
}

//////////////////////////////////////////////////////////////////////////////////////////////////
// Wall horizon
//////////////////////////////////////////////////////////////////////////////////////////////////

/// The live wall is exactly 55 draws, and a Kita shifts haitei back exactly like a Kan --
/// every one of 1,127 houou 3p exhaustive-draw kyoku contains exactly 55 draw events,
/// across every observed kan/kita combination.
#[test]
fn sanma_live_wall_is_55_draws_and_kita_shifts_haitei_like_a_kan() {
    let v = Variant::Sanma;
    let mut state = State::default();

    // No kan, no kita: haitei is the 55th head draw.
    state.core.num_drawn_head = v.num_dealt() + 55;
    state.core.num_drawn_tail = 0;
    assert!(is_last_draw(v, &state));
    state.core.num_drawn_head -= 1;
    assert!(!is_last_draw(v, &state));

    // Each tail draw -- Kan or Kita, indistinguishably -- costs one head draw.
    for tail in 1..=8u8 {
        let mut s = State::default();
        s.core.num_drawn_tail = tail;
        s.core.num_drawn_head = v.num_dealt() + 55 - tail;
        assert!(is_last_draw(v, &s), "tail={} should be haitei", tail);
        assert_eq!(num_draws(&s), v.max_num_draws());
    }

    // Sanma's 55-draw live wall gives ~18 tsumos per seat, one more than `MAX_TSUMOS_LEFT`'s
    // cap of 17 -- so sanma loses at most one turn of lookahead on the first draw of a hand.
    // The cap deliberately stays 17 for both variants: raising it would change 4p solver output
    // and break encoder byte-parity. (Asserted in riichi-solver, which owns the constant.)
    assert_eq!(v.num_live_wall_draws(), 55);
}
