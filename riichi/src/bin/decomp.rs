use std::io::stdin;
use itertools::Itertools;

use riichi::{
    analysis::{Decomposer, WaitingInfo},
    prelude::*
};

pub fn print_decomp(decomposer: &mut Decomposer, s: &str) {
    let tiles = tiles_from_str(&s);
    let ts = TileSet34::from_iter(tiles);
    let t0 = std::time::Instant::now();
    let wi = WaitingInfo::from_keys(decomposer, &ts.packed_34());
    let t1 = std::time::Instant::now();
    println!("[{}us] {} => {}", (t1 - t0).as_micros(), ts, wi);
}

pub fn main() {
    let mut decomposer = Decomposer::new();
    let args = std::env::args().collect_vec();

    if args.len() >= 2 {
        for s in &args[1..] {
            print_decomp(&mut decomposer, s);
        }
    } else {
        println!("Input hands, 1 per line. Example: 1112345678999m");
        for x in stdin().lines() {
            if let Ok(s) = x {
                print_decomp(&mut decomposer, &s);
            } else { break; }
        }
    }
}
