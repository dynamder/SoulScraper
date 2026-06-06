use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Questioner 输出：LLM 生成的结构化检索查询及查询集合
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrieveAssessInfo {
    /// 原子检索查询
    pub queries: Vec<PrioritizedRetrieveQuery>,
    /// 查询集合：同一检索意图的多种表达变体，用于测试检索泛化性能
    pub query_sets: Vec<QuerySet>,
}

/// 同一检索意图的多种口语化表达，测试检索系统对不同措辞的泛化能力
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuerySet {
    /// 集合唯一标识
    pub set_id: String,
    /// 集合的统一描述，说明检索意图（如"查询希儿最重要的伙伴"）
    pub description: String,
    /// 同一意图的多种表达变体（2-8 个）
    #[serde(default)]
    pub queries: Vec<PrioritizedRetrieveQuery>,
}

/// 带优先级的检索查询，优先级决定多个查询合并时该查询分数的权重
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PrioritizedRetrieveQuery {
    pub priority: u32,

    #[serde(flatten)]
    pub query: RetrieveQuery,
}

/// 单个检索查询：统一标签 + 查询变体（语义/情境）
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrieveQuery {
    /// 标签，决定 embedding 中 tag 部分的向量
    pub tags: Vec<String>,
    pub variant: RetrieveQueryVariant,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "variant_kind")]
pub enum RetrieveQueryVariant {
    Semantic { units: Vec<SemanticQueryUnit> },
    Situation { units: Vec<SituationQueryUnit> },
}

// ── 语义查询子结构 ──

/// 一个语义查询单元代表一个概念或实体
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SemanticQueryUnit {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub concept_identifier: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
}

// ── 情境查询子结构 ──

/// 一个情境查询单元内的所有字段是「与」关系
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SituationQueryUnit {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub narrative: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub location: Option<Vec<LocationQueryUnit>>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub participants: Option<Vec<ParticipantQueryUnit>>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_span: Option<Vec<TimeSpanQueryUnit>>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub environment: Option<EnvironmentQueryUnit>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event: Option<Vec<EventQueryUnit>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocationQueryUnit {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub coordinates: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ParticipantQueryUnit {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimeSpanQueryUnit {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start: Option<chrono::DateTime<chrono::Utc>>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnvironmentQueryUnit {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub atmosphere: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EventQueryUnit {
    pub action: String,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub initiator: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub target: Option<String>,
}