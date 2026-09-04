//! Exact replay of solver-boundary inputs captured from four seeded self-play
//! games through rustchi's Rayon encoder path.

use riichi_elements::prelude::TileSet37;
use riichi_solver::{CaptureRecord, Solver};
use std::fs::File;
use std::io::{BufWriter, Write};

#[test]
fn captured_self_play_outputs_are_exact() {
    let data = include_str!("data/solver_capture.jsonl");
    let mut solver = Solver::new();
    let mut normalized = std::env::var_os("RIICHI_SOLVER_NORMALIZE_CAPTURE")
        .map(|path| BufWriter::new(File::create(path).unwrap()));
    let mut count = 0;
    for (line_no, line) in data.lines().enumerate() {
        let record: CaptureRecord = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("capture line {}: {error}", line_no + 1));
        let actual = solver.analyze(&TileSet37(record.hand), &record.cfg);
        if let Some(output) = &mut normalized {
            serde_json::to_writer(
                &mut *output,
                &CaptureRecord { hand: record.hand, cfg: record.cfg, analysis: actual },
            )
            .unwrap();
            output.write_all(b"\n").unwrap();
        } else {
            assert_eq!(actual, record.analysis, "capture line {}", line_no + 1);
        }
        count += 1;
    }
    assert_eq!(count, 5000);
    if let Some(report) = riichi_solver::profile::report() {
        eprintln!("{report}");
    }
}
