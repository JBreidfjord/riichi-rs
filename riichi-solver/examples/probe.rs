use riichi_solver::{hand_from_mpsz, Solver, SolverConfig};
fn main() {
    let hand_str = std::env::args().nth(1).unwrap();
    let hand = hand_from_mpsz(&hand_str);
    let n: u32 = riichi_elements::prelude::TileSet34::from(&hand).0.iter().map(|&c| c as u32).sum();
    let cfg = SolverConfig::new(n, vec!["3p".parse().unwrap()]);
    let mut s = Solver::new();
    let (stats, searched) = s.solve(&hand, &cfg);
    println!("searched {searched}");
    for st in stats {
        println!("tile {:?} shanten {}", st.tile, st.shanten);
        for t in 1..=18 {
            println!("  t{:02} tenpai {:.12} win {:.12} ev {:.3}", t, st.tenpai_prob[t], st.win_prob[t], st.exp_score[t]);
        }
    }
}
