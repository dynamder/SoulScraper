use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::data_model::questioner::retrieve::RetrieveQueryVariant;

/// 检索测试用例文件（RetrQueryFileRaw）— 顶层结构
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrQueryFileRaw {
    /// 测试套件名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 指向 Graph JSON 的相对路径（相对于 query JSON 所在目录）
    pub graph_path: String,
    /// 检索执行配置
    pub config: TestConfigRaw,
    /// 权重调参配置（可选，不写则使用默认值 0.4/0.6）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub blend_sweep: Option<BlendSweepRaw>,
    /// 测试用例列表
    pub test_cases: Vec<TestCaseQueryRaw>,
}

/// 检索执行配置
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestConfigRaw {
    /// 相似度最低阈值
    pub similarity_threshold: f32,
    /// 每次搜索最大返回数
    pub max_results: usize,
    /// 评估 k 值列表
    pub test_k_values: Vec<usize>,
}

/// 权重调参配置
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlendSweepRaw {
    /// 快捷扫 tag 权重（生成 pairs: (tag, 1.0-tag)）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tag_sweep: Option<Vec<f64>>,
    /// 显式权重对列表（与 tag_sweep 互斥，pairs 优先）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub pairs: Option<Vec<BlendPairRaw>>,
}

/// 单个权重对
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlendPairRaw {
    pub tag: f64,
    pub variant: f64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sem_concept: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sem_description: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sem_concept_main: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sem_concept_aliases: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_location_name: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_location_coord: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_participant_name: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_participant_role: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_env_atmosphere: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_env_tone: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_event_initiator: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_event_target: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_event_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_event_initiator_only_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sit_event_target_only_action: Option<f64>,
}

/// 单个测试用例
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestCaseQueryRaw {
    /// 用例名称
    pub name: String,
    /// 用例描述
    pub description: String,
    /// 子查询列表
    pub sub_queries: Vec<SubQuery>,
    /// 每个子查询的期望结果（按 sub_queries 数组下标）
    pub expected_per_query: Vec<PerQueryExpectation>,
    /// 所有子查询按优先级加权合并后的期望排序结果
    pub expected_combined_ranking: Vec<String>,
    /// 动作节点期望（当前检索阶段填空数组）
    #[serde(default)]
    pub expected_actions: Vec<String>,
}

/// 单个子查询
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubQuery {
    /// 优先级（越大越重要）
    pub priority: u32,
    /// 标签数组
    pub tag: Vec<String>,
    /// 查询变体（二选一：Semantic 或 Situation）
    pub variant: RetrieveQueryVariant,
}

/// 子查询期望结果
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PerQueryExpectation {
    /// 对应 sub_queries 数组下标
    pub q: usize,
    /// 期望的节点 ID 排序（按相关度降序）
    pub ranking: Vec<String>,
}
