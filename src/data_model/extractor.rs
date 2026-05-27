use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::data_model::soul_mem::{
    MemoryLink, MemoryLinkType, MemoryNote, MemoryNoteBuilder, MemoryType,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedNode {
    pub node_id: String,
    pub tags: Vec<String>,
    pub mem_type: MemoryType,
}
impl From<ExtractedNode> for MemoryNote {
    fn from(value: ExtractedNode) -> Self {
        MemoryNoteBuilder::new(value.mem_type)
            .tags(value.tags)
            .build()
            .unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedLink {
    pub link_id: String,
    pub source_id: String,
    pub target_id: String,

    ///记忆链接强度，0~1
    pub intensity: f64,
    pub link_type: MemoryLinkType,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedGraph {
    pub nodes: Vec<ExtractedNode>,
    pub links: Vec<ExtractedLink>,
}

impl From<ExtractedGraph> for Vec<MemoryNote> {
    fn from(value: ExtractedGraph) -> Self {
        let mut id_map = HashMap::new();
        let ExtractedGraph { nodes, links } = value;

        let mut nodes_map = HashMap::new();
        for node in nodes {
            let string_id = node.node_id.clone();
            let mem_note = MemoryNote::from(node);
            id_map.insert(string_id, mem_note.id());
            nodes_map.insert(mem_note.id(), mem_note);
        }

        for link in links {
            let source_id = id_map[&link.source_id];
            let target_id = id_map[&link.target_id];

            let mut mem_link = MemoryLink::new(source_id, target_id, link.link_type);
            mem_link.intensity = link.intensity;

            nodes_map
                .get_mut(&source_id)
                .map(|note| note.add_link(mem_link));
        }

        nodes_map.into_iter().map(|(_, note)| note).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedInfo {
    pub graph: ExtractedGraph,

    ///简要描述角色的特征，重要经历，重要人物关系
    pub summary: String,
}
