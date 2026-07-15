use serde::Deserialize;

/// 反序列化辅助：将 `null` 或缺失字段处理为 `Default::default()`（如 `null` → `vec![]`）
pub(crate) fn null_to_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

/// 反序列化辅助：接受单个对象、数组、或 null，统一转换为 `Option<Vec<T>>`
pub(crate) fn one_or_many<'de, D, T>(d: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Array(arr) => {
            let items: Vec<T> = arr
                .into_iter()
                .map(|item| serde_json::from_value(item).map_err(serde::de::Error::custom))
                .collect::<Result<_, _>>()?;
            Ok(Some(items))
        }
        serde_json::Value::Object(_) => {
            let item: T = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
            Ok(Some(vec![item]))
        }
        _ => Err(serde::de::Error::custom(
            "expected an array, object, or null",
        )),
    }
}

/// 从 serde_json 错误中提取 JSON 上下文（错误位置前后各 100 字符）
pub fn format_json_error(json_str: &str, err: &serde_json::Error) -> String {
    let snippet = match (err.line(), err.column()) {
        (0, _) | (_, 0) => {
            let start = json_str.floor_char_boundary(json_str.len().saturating_sub(200));
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
            let safe_end = json_str.floor_char_boundary(200.min(json_str.len()));
            &json_str[..safe_end]
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

            // Fix concept_type: "AbstractConcept" → "Abstract", "Action" → "Entity"
            if let Some(ct) = map.get("concept_type") {
                if let Some(s) = ct.as_str() {
                    let fixed = match s {
                        "AbstractConcept" => "Abstract",
                        "Action" => "Entity",
                        _ => s,
                    };
                    if fixed != s {
                        map.insert(
                            "concept_type".to_string(),
                            serde_json::Value::String(fixed.to_string()),
                        );
                    }
                }
            }

            // Fix null String fields → empty string (for known nullable fields)
            for (key, val) in map.iter_mut() {
                if val.is_null() {
                    let nullable = matches!(
                        key.as_str(),
                        "description"
                            | "name"
                            | "role"
                            | "atmosphere"
                            | "tone"
                            | "coordinates"
                            | "initiator"
                            | "target"
                    );
                    if nullable {
                        *val = serde_json::Value::String(String::new());
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
