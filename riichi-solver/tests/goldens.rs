//! Compare solver output against mahjong-cpp golden values
//! (tests/data/goldens*.jsonl, produced by running the reference binary —
//! program output, not GPL-derived data).
//!
//! Probabilities must match tightly (same DP, same wall model). Expected
//! score is compared loosely: the riichi crate's fu computation is
//! approximate where mahjong-cpp's is exact, and that divergence is an
//! accepted decision (solver-scope ticket 03) — we report it rather than
//! chase parity.

use riichi_elements::prelude::*;
use riichi_solver::{hand_from_mpsz, Solver, SolverConfig};
use serde_json::Value;

const PROB_TOL: f64 = 1e-9;

fn parse_tile(s: &str) -> Tile {
    s.parse().unwrap()
}

/// Meld token from the fixture ("pon:5z", "chi:1m", "ankan:1z",
/// "daiminkan:9s"; "0m/0p/0s" for reds), mirroring the oracle harness.
fn parse_meld(tok: &str) -> Meld {
    let (kind, t) = tok.split_once(':').unwrap();
    let tile: Tile = t.parse().unwrap();
    let normal = tile.to_normal();
    let next = |t: Tile| Tile::from_encoding(t.encoding() + 1).unwrap();
    match kind {
        "pon" => Meld::Pon(Pon::from_tiles_dir(normal, normal, tile, P3).unwrap()),
        "chi" => Meld::Chii(Chii::from_tiles(next(normal), next(next(normal)), tile).unwrap()),
        "ankan" => Meld::Ankan(Ankan::from_tiles([tile; 4]).unwrap()),
        "daiminkan" => {
            Meld::Daiminkan(Daiminkan::from_tiles_dir([normal; 3], tile, P3).unwrap())
        }
        _ => panic!("bad meld token {tok}"),
    }
}

fn run_goldens(data: &str) {
    let mut ev_report: Vec<(String, String, f64)> = Vec::new();
    let mut max_prob_diff = 0.0f64;

    for line in data.lines().filter(|l| !l.trim().is_empty()) {
        let g: Value = serde_json::from_str(line).unwrap();
        let hand_str = g["hand"].as_str().unwrap();
        let ind = parse_tile(g["indicator"].as_str().unwrap());
        let hand = hand_from_mpsz(hand_str);
        let n_tiles: u32 = TileSet34::from(&hand).0.iter().map(|&c| c as u32).sum();

        let melds: Vec<Meld> = g["melds"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|t| parse_meld(t.as_str().unwrap()))
                    .collect()
            })
            .unwrap_or_default();

        let cfg = SolverConfig::new(n_tiles, vec![ind]).with_melds(melds);
        assert_eq!(cfg.sum, g["sum"].as_u64().unwrap() as u32, "{hand_str}: sum");

        let mut solver = Solver::new();
        let (stats, searched) = solver.solve(&hand, &cfg);

        let gstats = g["stats"].as_array().unwrap();
        assert_eq!(
            stats.len(),
            gstats.len(),
            "{hand_str}: candidate count (ours {} vs golden {}) ours: {:?} golden: {:?}",
            stats.len(),
            gstats.len(),
            stats.iter().map(|s| s.tile).collect::<Vec<_>>(),
            gstats.iter().map(|s| s["tile"].as_i64().unwrap()).collect::<Vec<_>>(),
        );
        eprintln!(
            "{hand_str}: searched ours={} golden={}",
            searched,
            g["searched"].as_i64().unwrap()
        );

        for gs in gstats {
            let gtile = gs["tile"].as_i64().unwrap();
            let ours = stats
                .iter()
                .find(|s| match s.tile {
                    None => gtile == -1,
                    Some(t) => t as i64 == gtile,
                })
                .unwrap_or_else(|| panic!("{hand_str}: no candidate for tile {gtile}"));

            assert_eq!(
                ours.shanten,
                gs["shanten"].as_i64().unwrap() as i8,
                "{hand_str}/{gtile}: shanten"
            );

            let gnec: Vec<(u8, u8)> = gs["necessary"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    (
                        p[0].as_u64().unwrap() as u8,
                        p[1].as_u64().unwrap() as u8,
                    )
                })
                .collect();
            assert_eq!(ours.necessary, gnec, "{hand_str}/{gtile}: necessary tiles");

            for (name, gv, ov) in [
                ("tenpai", &gs["tenpai_prob"], &ours.tenpai_prob),
                ("win", &gs["win_prob"], &ours.win_prob),
            ] {
                let gv = gv.as_array().unwrap();
                for t in 1..=18usize {
                    let g = gv[t].as_f64().unwrap();
                    let o = ov[t];
                    let d = (g - o).abs();
                    max_prob_diff = max_prob_diff.max(d);
                    assert!(
                        d < PROB_TOL,
                        "{hand_str}/{gtile}: {name}[{t}] golden={g} ours={o}"
                    );
                }
            }

            // EV: report max relative diff over turns with meaningful mass.
            let gv = gs["exp_score"].as_array().unwrap();
            let mut worst = 0.0f64;
            for t in 1..=18usize {
                let g = gv[t].as_f64().unwrap();
                let o = ours.exp_score[t];
                if g > 1.0 {
                    worst = worst.max((g - o).abs() / g);
                }
            }
            ev_report.push((hand_str.to_string(), gtile.to_string(), worst));
        }
    }

    eprintln!("max prob abs diff: {max_prob_diff:.3e}");
    eprintln!("EV max relative diff per candidate:");
    let mut worst_ev = 0.0f64;
    for (h, t, d) in &ev_report {
        eprintln!("  {h:24} discard {t:>3}: {:.4}", d);
        worst_ev = worst_ev.max(*d);
    }
    // Measured parity on these sets is exact (riichi's fu approximation
    // happens to agree with mahjong-cpp's full fu on all golden hands). Keep
    // a small margin for future hands where the approximation could differ.
    assert!(worst_ev < 0.01, "EV diverges beyond fu-approximation range");
}

#[test]
fn goldens() {
    run_goldens(include_str!("data/goldens.jsonl"));
}

#[test]
fn goldens_open() {
    run_goldens(include_str!("data/goldens-open.jsonl"));
}
