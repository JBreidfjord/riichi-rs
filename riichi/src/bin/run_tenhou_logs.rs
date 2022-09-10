use std::path::Path;
use std::str::FromStr;

use riichi::{
    interop::tenhou_log::*,
    interop::tenhou_log::test_utils::run_a_round,
};

fn run_log_file(json_path: &Path) {
    println!("\n\n\n\n\n\n\n\n\n\n\n\n");
    println!("testing: {}", json_path.file_name().unwrap().to_str().unwrap());
    let json_str = std::fs::read_to_string(json_path).unwrap();
    let json_value = serde_json::Value::from_str(&json_str).unwrap();
    let deserialized = serde_json::from_value::<TenhouLog>(json_value.clone()).unwrap();
    let num_reds = deserialized.rule.num_reds();
    for round in deserialized.rounds.iter() {
        println!("{:?}", round.round_id_and_pot);
        if let Some(recovered) = recover_round(round) {
            // println!("{}", recovered);
            run_a_round(num_reds, &recovered, &round.end_info);
        }
    }
}

fn main() {
    let pattern = env!("TENHOU_LOG_GLOB");
    for path in glob::glob(pattern).unwrap() {
        run_log_file(&path.unwrap());
    }
}
