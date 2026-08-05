use riichi_solver::{hand_from_mpsz, Solver, SolverConfig};
fn main() {
    let hand_str = std::env::args().nth(1).unwrap();
    let hand = hand_from_mpsz(&hand_str);
    let n: u32 = riichi_elements::prelude::TileSet34::from(&hand).0.iter().map(|&c| c as u32).sum();
    let cfg = SolverConfig::new(n, vec!["3p".parse().unwrap()]);
    let mut s = Solver::new();
    println!("{}", s.graph_json(&hand, &cfg));
}
