use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::data_model::soul_mem::{
    MemoryLink, MemoryLinkType, MemoryNote, MemoryNoteBuilder, MemoryType,
};

/// LLM 输出：记忆图节点数组（GraphNodeRaw[]）
pub type GraphNodeList = Vec<GraphNode>;

/// 单个记忆图节点
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphNode {
    /// 全局唯一可读 ID
    pub id: String,
    /// 标签，参与 embedding 计算
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
