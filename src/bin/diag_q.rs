use std::path::PathBuf;

fn test_file(name: &str, path_str: &str) {
    let raw = match std::fs::read_to_string(path_str) {
        Ok(s) => s,
        Err(_) => {
            println!("SKIP {name}: file not found");
            return;
        }
    };
    println!("\n=== {name} ({} bytes) ===", raw.len());
    match serde_json::from_str::<soul_scraper::data_model::questioner::retrieve::RetrieveAssessInfo>(
        &raw,
    ) {
        Ok(info) => println!(
            "OK: {} queries, {} sets",
            info.queries.len(),
            info.query_sets.len()
        ),
        Err(e) => {
            println!("FAIL: {e}");
            let line = e.line();
            let col = e.column();
            println!("  line {line} col {col}");
            let start: usize = (col as usize).saturating_sub(1);
            let chars: Vec<char> = raw.chars().collect();
            let from = start.min(chars.len().saturating_sub(1));
            let to = (start + 200).min(chars.len());
            let snippet: String = chars[from..to].iter().collect();
            println!("  >> {snippet}");
        }
    }
}

fn main() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_batch_output-serde-fix/zh_moegirl_org_cn_E4_B8_9C_E9_A3_8E_E8_B0_B7_E6_97_A9_E8_8B_97");

    test_file(
        "早苗_question_raw",
        base.join("raw_failed_question.json").to_str().unwrap(),
    );
    test_file(
        "早苗_question_fix",
        base.join("raw_failed_question_fix.json").to_str().unwrap(),
    );
}
