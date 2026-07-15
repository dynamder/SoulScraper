use std::collections::{HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::data_model::soul_mem::{
    MemoryLink, MemoryLinkType, MemoryNote, MemoryNoteBuilder, MemoryType,
};

use crate::util::null_to_default;

/// LLM 输出：记忆图节点数组（GraphNodeRaw[]）
pub type GraphNodeList = Vec<GraphNode>;

/// 单个记忆图节点
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphNode {
    /// 全局唯一可读 ID
    pub id: String,
    /// 标签，参与 embedding 计算
    #[serde(deserialize_with = "null_to_default")]
    pub tags: Vec<String>,
    /// 记忆类型（Semantic / Situation / Procedure）
    pub mem_type: MemoryType,
    /// 关联边，默认为空
    #[serde(default)]
    pub mem_links: Vec<GraphLink>,
}

/// 记忆图边
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphLink {
    /// 源节点 ID
    pub from: String,
    /// 目标节点 ID
    pub to: String,
    /// 关联强度 0~1
    pub intensity: f64,
    /// 边类型（Sem / Proc / Situation）
    pub link_type: MemoryLinkType,
}

/// LLM Phase 2 输出：仅边描述
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EdgeList {
    pub edges: Vec<GraphLink>,
}

/// 将 Phase 2 生成的边注入到节点列表中
pub fn apply_edges(nodes: &mut [GraphNode], edges: &EdgeList) {
    // 清空所有已有边
    for node in nodes.iter_mut() {
        node.mem_links.clear();
    }
    // 按 from 分组注入
    let mut by_from: HashMap<&str, Vec<&GraphLink>> = HashMap::new();
    for edge in &edges.edges {
        by_from.entry(edge.from.as_str()).or_default().push(edge);
    }
    for node in nodes.iter_mut() {
        if let Some(links) = by_from.remove(node.id.as_str()) {
            node.mem_links = links.into_iter().cloned().collect();
        }
    }
}

/// 按 id 合并两组节点：fix 列表中的节点覆盖 valid 中同 id 的节点
pub fn merge_nodes(valid: &[GraphNode], fix: &[GraphNode]) -> Vec<GraphNode> {
    let fix_ids: HashSet<&str> = fix.iter().map(|n| n.id.as_str()).collect();
    let mut merged: Vec<GraphNode> = valid
        .iter()
        .filter(|n| !fix_ids.contains(n.id.as_str()))
        .cloned()
        .collect();
    merged.extend(fix.iter().cloned());
    merged
}

pub fn graph_node_list_to_memory_notes(nodes: GraphNodeList) -> Vec<MemoryNote> {
    let mut id_map: HashMap<String, crate::data_model::soul_mem::MemoryId> = HashMap::new();
    let mut nodes_map: HashMap<crate::data_model::soul_mem::MemoryId, MemoryNote> = HashMap::new();

    // 第一遍：创建所有 MemoryNote，建立 ID 映射
    for node in &nodes {
        let mem_note = MemoryNoteBuilder::new(node.mem_type.clone())
            .tags(node.tags.clone())
            .build()
            .unwrap();
        id_map.insert(node.id.clone(), mem_note.id());
        nodes_map.insert(mem_note.id(), mem_note);
    }

    // 第二遍：添加边
    for node in &nodes {
        let source_id = id_map.get(&node.id).expect("node id must exist in id_map");
        for link in &node.mem_links {
            let target_id = id_map
                .get(&link.to)
                .expect("link target id must exist in id_map");
            let mut mem_link = MemoryLink::new(*source_id, *target_id, link.link_type.clone());
            mem_link.intensity = link.intensity;
            nodes_map
                .get_mut(source_id)
                .map(|note| note.add_link(mem_link));
        }
    }

    nodes_map.into_values().collect()
}
