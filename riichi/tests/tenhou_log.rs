#[cfg(feature = "tenhou-log")]
mod tenhou_log_tests {
    use std::{
        path::{Path, PathBuf},
        str::FromStr,
    };

    use assert_json_diff::assert_json_include;
    use glob::glob;

    use riichi::interop::tenhou_log::*;

    fn sample_json_paths() -> impl Iterator<Item=PathBuf> {
        let samples_dir: PathBuf = [
            &std::env::var("CARGO_MANIFEST_DIR").unwrap(),
            "data", "t6-samples", "*.json"
        ].iter().collect();
        glob(samples_dir.as_os_str().to_str().unwrap()).unwrap().flatten()
    }

    #[test]
    fn sample_passes_serde_roundtrip() {
        for json_path in sample_json_paths() {
            println!("testing: {}", json_path.file_name().unwrap().to_str().unwrap());
            let json_str = std::fs::read_to_string(json_path).unwrap();
            let json_value = serde_json::Value::from_str(&json_str).unwrap();
            let deserialized = serde_json::from_value::<TenhouLog>(json_value.clone()).unwrap();
            let reserialized = serde_json::to_value(deserialized).unwrap();
            /*
            println!("=== original ===");
            println!("{}", json_value);
            println!("=== reserialized ===");
            println!("{}", reserialized);
            */
            assert_json_include!(actual: json_value, expected: reserialized);
        }
    }

    #[test]
    fn sample_rounds_can_be_recovered() {
        for json_path in sample_json_paths() {
            println!("testing: {}", json_path.file_name().unwrap().to_str().unwrap());
            let json_str = std::fs::read_to_string(json_path).unwrap();
            let json_value = serde_json::Value::from_str(&json_str).unwrap();
            let deserialized = serde_json::from_value::<TenhouLog>(json_value.clone()).unwrap();
            for round in deserialized.rounds.iter() {
                let recovered = recover_round(round).unwrap();
                println!("{}", recovered);
            }
        }
    }
}
