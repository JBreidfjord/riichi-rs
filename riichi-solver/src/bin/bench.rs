//! Throughput benchmark: states/sec by root-hand shanten.
//!
//! Samples random 14-tile hands from a full wall, nudges them to target
//! shanten buckets, and measures solve latency with reference-parity config
//! (18 turns, one dora indicator, uradora EV on).

use std::time::Instant;

use riichi_elements::prelude::*;
use riichi_solver::{Solver, SolverConfig};

// ponytail: xorshift instead of a rand dep — determinism is the point.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn full_deck() -> Vec<u8> {
    // 37-kind deck: 4 of each normal kind except fives (3), reds 1 each.
    let mut deck = Vec::with_capacity(136);
    for k in 0..37u8 {
        let copies = match k {
            4 | 13 | 22 => 3,
            34..=36 => 1,
            _ => 4,
        };
        for _ in 0..copies {
            deck.push(k);
        }
    }
    deck
}

fn random_hand(rng: &mut Rng) -> [u8; 37] {
    let mut deck = full_deck();
    let mut hand = [0u8; 37];
    for _ in 0..14 {
        let i = rng.below(deck.len());
        hand[deck.swap_remove(i) as usize] += 1;
    }
    hand
}

fn fold(hand: &[u8; 37]) -> TileSet34 {
    let mut a = [0u8; 34];
    a.copy_from_slice(&hand[..34]);
    a[4] += hand[34];
    a[13] += hand[35];
    a[22] += hand[36];
    TileSet34(a)
}

/// Swap random tiles until the hand reaches the target shanten.
fn hand_at_shanten(rng: &mut Rng, target: i8) -> [u8; 37] {
    loop {
        let mut hand = random_hand(rng);
        for _ in 0..200 {
            let s = riichi_decomp::shanten(&fold(&hand));
            if s == target {
                return hand;
            }
            if s < target {
                break; // overshot; resample
            }
            // Replace a random tile with one that improves shanten.
            let kinds: Vec<usize> = (0..37).filter(|&k| hand[k] > 0).collect();
            let out = kinds[rng.below(kinds.len())];
            let mut improved = false;
            for _ in 0..40 {
                let cand = rng.below(37);
                let copies = match cand {
                    4 | 13 | 22 => 3,
                    34..=36 => 1,
                    _ => 4,
                };
                if cand == out || hand[cand] as usize >= copies {
                    continue;
                }
                let mut h2 = hand;
                h2[out] -= 1;
                h2[cand] += 1;
                if riichi_decomp::shanten(&fold(&h2)) < s {
                    hand = h2;
                    improved = true;
                    break;
                }
            }
            if !improved {
                break;
            }
        }
    }
}

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(100);
    let ind: Tile = "3p".parse().unwrap();

    println!("riichi-solver bench: {n} hands per shanten bucket, t_max=18, ura on");
    println!(
        "{:>7} {:>9} {:>9} {:>9} {:>9} {:>10} {:>12} {:>11}",
        "shanten", "mean_ms", "p50_ms", "p95_ms", "max_ms", "hands/s", "vertices", "vertices/s"
    );

    for target in 0..=3i8 {
        let mut rng = Rng(0x9E3779B97F4A7C15 ^ (target as u64 + 1));
        let hands: Vec<[u8; 37]> = (0..n).map(|_| hand_at_shanten(&mut rng, target)).collect();

        let mut solver = Solver::new();
        let mut times_ms: Vec<f64> = Vec::with_capacity(n);
        let mut total_vertices = 0usize;
        let t_all = Instant::now();
        for h in &hands {
            let hand = TileSet37(*h);
            let cfg = SolverConfig::new(14, vec![ind]);
            let t0 = Instant::now();
            let (stats, searched) = solver.solve(&hand, &cfg);
            let dt = t0.elapsed().as_secs_f64() * 1e3;
            times_ms.push(dt);
            total_vertices += searched;
            std::hint::black_box(stats);
        }
        let wall = t_all.elapsed().as_secs_f64();

        times_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean = times_ms.iter().sum::<f64>() / n as f64;
        let p50 = times_ms[n / 2];
        let p95 = times_ms[(n as f64 * 0.95) as usize];
        let max = *times_ms.last().unwrap();
        println!(
            "{:>7} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>10.1} {:>12} {:>11.0}",
            target,
            mean,
            p50,
            p95,
            max,
            n as f64 / wall,
            total_vertices,
            total_vertices as f64 / wall
        );
    }
}
