use std::time::Instant;
use riichi_elements::prelude::*;

fn main() {
    // xorshift
    let mut x = 0x9E3779B97F4A7C15u64;
    let mut rng = move || { x ^= x<<13; x ^= x>>7; x ^= x<<17; x };
    let mut deck = Vec::new();
    for k in 0..34u8 { for _ in 0..4 { deck.push(k); } }
    let mut hands = Vec::new();
    for _ in 0..1000 {
        let mut d = deck.clone();
        let mut h = [0u8; 34];
        for _ in 0..13 {
            let i = (rng() % d.len() as u64) as usize;
            h[d.swap_remove(i) as usize] += 1;
        }
        hands.push(TileSet34(h));
    }
    let t0 = Instant::now();
    let mut acc = 0i64;
    const REPS: usize = 300;
    for _ in 0..REPS {
        for h in &hands {
            acc += riichi_decomp::shanten(h) as i64;
        }
    }
    let dt = t0.elapsed().as_secs_f64();
    println!("shanten: {:.0} ns/call (acc {acc})", dt / (REPS * hands.len()) as f64 * 1e9);
}
