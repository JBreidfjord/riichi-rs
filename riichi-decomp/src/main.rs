use std::{
    io::stdin,
    time::Instant,
};
use itertools::Itertools;

use riichi_decomp::*;
use riichi_elements::prelude::*;

pub fn print_decomp(decomposer: &mut Decomposer, s: &str) {
    let tiles = tiles_from_str(&s);
    let tileset = TileSet34::from_iter(tiles);
    let t0 = Instant::now();
    let result = WaitSet::from_keys(decomposer, &tileset.packed_34());
    let t1 = Instant::now();
    println!("[{}us] {} => {}", (t1 - t0).as_micros(), tileset, result);
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
