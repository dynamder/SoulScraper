use std::fs;
use std::path::Path;

fn main() {
    let base = r"D:\Soul-Plan\SoulFlasher\soul_scraper\test_batch_output-pipeline-refactor";
    let dirs = [
        (
            "fu_hua",
            "zh_moegirl_org_cn_E7_AC_A6_E5_8D_8E_E5_B4_A9_E5_9D_8F3",
        ),
        (
            "lei_dian",
            "zh_moegirl_org_cn_E9_9B_B7_E7_94_B5_E8_8A_BD_E8_A1_A3_E5_B4_A9_E5_9D_8F3",
        ),
        (
            "dong_feng",
            "zh_moegirl_org_cn_E4_B8_9C_E9_A3_8E_E8_B0_B7_E6_97_A9_E8_8B_97",
        ),
        (
            "fu_lan",
            "zh_moegirl_org_cn_E8_8A_99_E5_85_B0_E6_9C_B5_E9_9C_B2_C2_B7_E6_96_AF_E5_8D_A1_E8_95_BE_E7_89_B9",
        ),
    ];

    for (name, dir) in &dirs {
        for suffix in &["raw_failed_nodes.json", "raw_failed_nodes_fix.json"] {
            let path = Path::new(base).join(dir).join(suffix);
            if !path.exists() {
                println!("{} {}: FILE NOT FOUND", name, suffix);
                continue;
            }
            let content = fs::read_to_string(&path).unwrap();
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(_v) => {
                    println!("{} {}: Value parse OK", name, suffix);
                }
                Err(e) => {
                    println!("{} {}: Value parse FAILED: {}", name, suffix, e);
                }
            }
        }
    }
}
