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
    match serde_json::from_str::<soul_scraper::data_model::extractor::GraphNodeList>(&raw) {
        Ok(nodes) => println!("OK: {} nodes", nodes.len()),
        Err(e) => {
            println!("FAIL: {e}");
            let line = e.line();
            let col = e.column();
            println!("  line {line} col {col}");
            // Safe UTF-8 snippet: skip chars, not bytes
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
    let base =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_batch_output-pipeline-refactor");

    // 东风谷早苗
    test_file(
        "早苗_raw",
        base.join(
            "zh_moegirl_org_cn_E4_B8_9C_E9_A3_8E_E8_B0_B7_E6_97_A9_E8_8B_97/raw_failed_nodes.json",
        )
        .to_str()
        .unwrap(),
    );
    test_file(
        "早苗_fix",
        base.join("zh_moegirl_org_cn_E4_B8_9C_E9_A3_8E_E8_B0_B7_E6_97_A9_E8_8B_97/raw_failed_nodes_fix.json")
            .to_str().unwrap(),
    );

    // 芙兰朵露
    test_file(
        "芙兰朵露_raw",
        base.join("zh_moegirl_org_cn_E8_8A_99_E5_85_B0_E6_9C_B5_E9_9C_B2_C2_B7_E6_96_AF_E5_8D_A1_E8_95_BE_E7_89_B9/raw_failed_nodes.json")
            .to_str().unwrap(),
    );
    test_file(
        "芙兰朵露_fix",
        base.join("zh_moegirl_org_cn_E8_8A_99_E5_85_B0_E6_9C_B5_E9_9C_B2_C2_B7_E6_96_AF_E5_8D_A1_E8_95_BE_E7_89_B9/raw_failed_nodes_fix.json")
            .to_str().unwrap(),
    );

    // 雷电芽衣
    test_file(
        "芽衣_raw",
        base.join("zh_moegirl_org_cn_E9_9B_B7_E7_94_B5_E8_8A_BD_E8_A1_A3_E5_B4_A9_E5_9D_8F3/raw_failed_nodes.json")
            .to_str().unwrap(),
    );
    test_file(
        "芽衣_fix",
        base.join("zh_moegirl_org_cn_E9_9B_B7_E7_94_B5_E8_8A_BD_E8_A1_A3_E5_B4_A9_E5_9D_8F3/raw_failed_nodes_fix.json")
            .to_str().unwrap(),
    );

    // 符华
    test_file(
        "符华_raw",
        base.join("zh_moegirl_org_cn_E7_AC_A6_E5_8D_8E_E5_B4_A9_E5_9D_8F3/raw_failed_nodes.json")
            .to_str()
            .unwrap(),
    );
}
