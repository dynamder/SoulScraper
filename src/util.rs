/// 从 serde_json 错误中提取 JSON 上下文（错误位置前后各 100 字符）
pub fn format_json_error(json_str: &str, err: &serde_json::Error) -> String {
    let snippet = match (err.line(), err.column()) {
        (0, _) | (_, 0) => {
            let start = json_str.len().saturating_sub(200);
            &json_str[start..]
        }
        (line, col) => {
            let cursor = line_col_to_byte_offset(json_str, line, col);
            let start = cursor.saturating_sub(100);
            let end = (cursor + 100).min(json_str.len());
            if let Some(s) = json_str.get(start..end) {
                let prefix = if start > 0 { "..." } else { "" };
                let suffix = if end < json_str.len() { "..." } else { "" };
                // 在错误位置插入 ‹── 标记
                let local_cursor = cursor - start;
                let before = &s[..local_cursor];
                let after = &s[local_cursor..];
                return format!("{prefix}{before}‹── HERE ──›{after}{suffix}");
            }
            &json_str[0..200.min(json_str.len())]
        }
    };

    format!("{snippet}")
}

/// 将所有 expected_per_query 的 ranking 合并去重，生成 expected_combined_ranking
pub fn combine_rankings(
    per_query: &[crate::data_model::retrieve_question::PerQueryExpectation],
) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for pq in per_query {
        for id in &pq.ranking {
            if seen.insert(id.clone()) {
                result.push(id.clone());
            }
        }
    }
    result
}

/// 去除 LLM 输出中常见的 markdown 代码块包裹（```json / ```）
pub fn strip_markdown_wrapping(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("```") {
        // 去掉开头的 ```json 或 ``` 以及结尾的 ```
        let without_start = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        let cleaned = if let Some(end) = without_start.rfind("```") {
            &without_start[..end]
        } else {
            without_start
        };
        cleaned.trim().to_string()
    } else {
        raw.to_string()
    }
}

/// 将行号+列号转换为字节偏移量
fn line_col_to_byte_offset(text: &str, line: usize, column: usize) -> usize {
    let mut current_line = 1;
    let mut offset = 0;
    for ch in text.chars() {
        if current_line == line {
            return offset + column.saturating_sub(1);
        }
        if ch == '\n' {
            current_line += 1;
        }
        offset += ch.len_utf8();
    }
    text.len()
}
