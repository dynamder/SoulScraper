use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::null_to_default;

// 概念类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum ConceptType {
    Entity,
    Abstract,
}

/// 语义记忆节点
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SemMemory {
    /// 概念名称
    pub content: String,

    /// 别名
    #[serde(default, deserialize_with = "null_to_default")]
    #[schemars(default)]
    pub aliases: Vec<String>,

    /// 概念类型
    pub concept_type: ConceptType,
    /// 描述
    pub description: String,
}

impl SemMemory {
    pub fn new(content: String, concept_type: ConceptType, description: String) -> Self {
        Self {
            content,
            aliases: Vec::new(),
            concept_type,
            description,
        }
    }
}

/// 语义记忆Link
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SemMemLink {
    pub verb: String,
    pub confidence: f32,
}

impl SemMemLink {
    pub fn new(verb: String, confidence: f32) -> Self {
        Self { verb, confidence }
    }
}
