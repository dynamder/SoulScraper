use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::data_model::extractor::ExtractedInfo;
use crate::data_model::questioner::retrieve::PrioritizedRetrieveQuery;

/// 检索功能的测试数据：包含记忆图谱、原子查询用例及查询集合
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrieveTestData {
    /// 元信息
    pub meta: TestMeta,

    /// 对 mem_graph（序列化后的 ExtractedInfo）的 BLAKE3 哈希，用于验证图谱完整性
    pub mem_graph_blake3: String,

    pub mem_graph_source: MemGraphSource,

    /// 内嵌的记忆图谱（当 source 为 inline 时使用）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mem_graph: Option<ExtractedInfo>,

    /// 原子检索测试用例
    pub test_cases: Vec<RetrieveTestCase>,

    /// 查询用例集合
    pub test_case_sets: Vec<RetrieveTestCaseSet>,
}

impl RetrieveTestData {
    /// 计算 ExtractedInfo 的 BLAKE3 哈希（hex），用于生成或验证
    pub fn compute_mem_graph_hash(graph: &ExtractedInfo) -> String {
        let bytes = serde_json::to_vec(graph).expect("ExtractedInfo serialization failed.");
        blake3::hash(&bytes).to_hex().to_string()
    }

    /// 验证内嵌图谱的哈希完整性
    pub fn verify_graph_integrity(&self) -> Result<(), String> {
        if let Some(ref graph) = self.mem_graph {
            let actual = Self::compute_mem_graph_hash(graph);
            if actual != self.mem_graph_blake3 {
                return Err(format!(
                    "hash mismatch: expected {expected}, got {actual}",
                    expected = self.mem_graph_blake3,
                    actual = actual
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestMeta {
    pub name: String,
    pub description: String,
    /// 测试数据对应的角色名称
    pub character: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemGraphSource {
    /// 图谱直接嵌入在 JSON 中
    Inline,
    /// 图谱引用外部文件路径
    File(String),
}

/// 原子检索测试用例
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrieveTestCase {
    /// 唯一标识
    pub case_id: String,
    /// 用例描述
    pub description: String,
    /// 原始自然语言问题（作为 LLM 输入）
    pub natural_query: String,
    /// 期望 LLM 生成的结构化检索查询列表
    pub retrieve_queries: Vec<PrioritizedRetrieveQuery>,
    /// 期望的检索结果
    pub expected: ExpectedResult,
}

/// 期望的检索结果，按必须/可能/禁止命中三级区分
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExpectedResult {
    /// 必须命中的 node_id 列表
    #[serde(default)]
    pub must_include: Vec<String>,

    /// 可能命中的 node_id 列表
    #[serde(default)]
    pub may_include: Vec<String>,

    /// 不应命中的 node_id 列表
    #[serde(default)]
    pub must_exclude: Vec<String>,
}

/// 查询用例集合，通过 case_id 引用原子用例，附加集合级评估指标
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrieveTestCaseSet {
    pub set_id: String,
    pub description: String,
    /// 引用的 test_case.case_id
    pub case_ids: Vec<String>,
    /// 集合级别的最低评估指标
    pub expected_metrics: ExpectedMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExpectedMetrics {
    #[serde(default)]
    pub min_recall: Option<f64>,

    #[serde(default)]
    pub min_precision: Option<f64>,

    #[serde(default)]
    pub min_precision_at_5: Option<f64>,

    #[serde(default)]
    pub min_mrr: Option<f64>,
}
