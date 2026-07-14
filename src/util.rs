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

/// 修复 LLM 输出中已知的 JSON 格式错误，使 serde 能正常解析
pub fn sanitize_json(raw: &str) -> String {
    let mut v: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return raw.to_string(), // syntax error, let serde report it
    };
    sanitize_value(&mut v);
    serde_json::to_string(&v).unwrap_or_else(|_| raw.to_string())
}

/// 递归遍历 JSON 值，修复已知的 LLM 格式错误
fn sanitize_value(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(map) => {
            // Fix action_type: {"Speak": {}} -> "Speak", {"Think": {}} -> "Think"
            if let Some(at) = map.get("action_type") {
                if let Some(inner) = at.as_object() {
                    if inner.len() == 1 {
                        if let Some(val) = inner.get("Speak").or_else(|| inner.get("Think")) {
                            if val == &serde_json::Value::Object(serde_json::Map::new()) {
                                if let Some(k) = inner.keys().next() {
                                    map.insert(
                                        "action_type".to_string(),
                                        serde_json::Value::String(k.clone()),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Fix link_type: move misplaced "confidence" inside Sem/Proc/Sit
            if let Some(lt) = map.get("link_type") {
                if let Some(lt_obj) = lt.as_object() {
                    // Check if "confidence" is at the link_type level
                    if lt_obj.contains_key("Sem") && lt_obj.contains_key("confidence") {
                        let conf = lt_obj["confidence"].clone();
                        if let Some(sem) = lt_obj.get("Sem") {
                            if let Some(sem_obj) = sem.as_object() {
                                let mut new_sem = sem_obj.clone();
                                new_sem.insert("confidence".to_string(), conf);
                                let mut new_lt = lt_obj.clone();
                                new_lt
                                    .insert("Sem".to_string(), serde_json::Value::Object(new_sem));
                                new_lt.remove("confidence");
                                map.insert(
                                    "link_type".to_string(),
                                    serde_json::Value::Object(new_lt),
                                );
                            }
                        }
                    }
                }
            }

            // Recursively process all child values
            for val in map.values_mut() {
                sanitize_value(val);
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                sanitize_value(val);
            }
        }
        _ => {}
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
