pub mod proc;
pub mod sem;
pub mod sit;

use std::fmt::Display;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::data_model::soul_mem::{
    proc::{ProcMemLink, ProcMemory},
    sem::{SemMemLink, SemMemory},
    sit::{SituationMemLink, SituationType},
};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Serialize, Deserialize)]
pub struct MemoryId(Uuid);
impl MemoryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
impl From<Uuid> for MemoryId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}
impl Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Serialize, Deserialize)]
pub struct LinkId(Uuid);
impl LinkId {
    pub fn new() -> Self {
        LinkId(Uuid::new_v4())
    }
}
impl Default for LinkId {
    fn default() -> Self {
        LinkId::new()
    }
}
impl Display for LinkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryLink {
    id: LinkId,
    from: MemoryId,
    to: MemoryId,
    pub intensity: f64,
    link_type: MemoryLinkType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum MemoryLinkType {
    Proc(ProcMemLink),
    Sem(SemMemLink),
    Situation(SituationMemLink),
}

impl MemoryLink {
    pub fn new(from: MemoryId, to: MemoryId, link_type: MemoryLinkType) -> Self {
        MemoryLink {
            id: LinkId::default(),
            from,
            to,
            link_type,
            intensity: 1.0,
        }
    }
    pub fn id(&self) -> LinkId {
        self.id
    }
    pub fn from(&self) -> MemoryId {
        self.from
    }
    pub fn to(&self) -> MemoryId {
        self.to
    }
    pub fn link_type(&self) -> &MemoryLinkType {
        &self.link_type
    }
    pub fn link_type_mut(&mut self) -> &mut MemoryLinkType {
        &mut self.link_type
    }
    pub fn into_tuple(self) -> (MemoryId, MemoryId, MemoryLinkType, f64) {
        (self.from, self.to, self.link_type, self.intensity)
    }
    pub fn from_tuple(
        from: MemoryId,
        to: MemoryId,
        link_type: MemoryLinkType,
        intensity: f64,
    ) -> Self {
        MemoryLink {
            id: LinkId::default(),
            from,
            to,
            link_type,
            intensity,
        }
    }
    pub fn into_link_type(self) -> MemoryLinkType {
        self.link_type
    }
}
impl From<(MemoryId, MemoryId, MemoryLinkType, f64)> for MemoryLink {
    fn from(tuple: (MemoryId, MemoryId, MemoryLinkType, f64)) -> Self {
        MemoryLink::from_tuple(tuple.0, tuple.1, tuple.2, tuple.3)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct MemoryNote {
    id: MemoryId,                      // 记忆唯一id
    tags: Vec<String>,                 //记忆的标签，暂定会成为embedding的一部分
    retrieval_count: usize,            //记忆被提取的次数
    create_time: DateTime<Utc>,        //记忆的创建时间
    last_accessed_time: DateTime<Utc>, //记忆的最后访问时间
    mem_type: MemoryType,              //记忆的类型，存储类型特定内容
    mem_links: Vec<MemoryLink>,        //记忆的链接，用于关联其他记忆
}
impl MemoryNote {
    pub fn is_same_id(mem1: &MemoryNote, mem2: &MemoryNote) -> bool {
        mem1.id == mem2.id
    }
    pub fn id(&self) -> MemoryId {
        self.id
    }
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
    pub fn retrieval_count(&self) -> usize {
        self.retrieval_count
    }
    pub fn creation_time(&self) -> DateTime<Utc> {
        self.create_time
    }
    pub fn last_accessed_time(&self) -> DateTime<Utc> {
        self.last_accessed_time
    }
    pub fn mem_type(&self) -> &MemoryType {
        &self.mem_type
    }
    pub fn links(&self) -> &Vec<MemoryLink> {
        &self.mem_links
    }
    pub fn retrieval_increment(&mut self) {
        self.retrieval_count += 1;
        self.last_accessed_time = Utc::now();
    }
    pub fn add_link(&mut self, link: MemoryLink) {
        self.mem_links.push(link);
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum MemoryType {
    Semantic(SemMemory),
    Situation(SituationType),
    Procedure(ProcMemory),
}

//Builder pattern
pub struct MemoryNoteBuilder {
    id: Option<MemoryId>,
    tags: Option<Vec<String>>,
    retrieval_count: Option<usize>,
    create_time: Option<DateTime<Utc>>,
    last_accessed_time: Option<DateTime<Utc>>,
    mem_type: MemoryType,
    mem_links: Option<Vec<MemoryLink>>,
}
impl MemoryNoteBuilder {
    pub fn new(mem_type: MemoryType) -> Self {
        Self {
            id: None,
            tags: None,
            retrieval_count: None,
            create_time: None,
            last_accessed_time: None,
            mem_type,
            mem_links: None,
        }
    }
    pub fn id(mut self, id: impl Into<MemoryId>) -> Self {
        self.id = Some(id.into());
        self
    }
    pub fn tags(mut self, tags: impl Into<Vec<String>>) -> Self {
        self.tags = Some(tags.into());
        self
    }
    pub fn retrieval_count(mut self, retrieval_count: usize) -> Self {
        self.retrieval_count = Some(retrieval_count);
        self
    }
    pub fn create_time(mut self, create_time: DateTime<Utc>) -> Self {
        self.create_time = Some(create_time);
        self
    }
    pub fn last_accessed_time(mut self, last_accessed_time: DateTime<Utc>) -> Self {
        self.last_accessed_time = Some(last_accessed_time);
        self
    }
    pub fn mem_links(mut self, mem_links: impl Into<Vec<MemoryLink>>) -> Self {
        self.mem_links = Some(mem_links.into());
        self
    }
    pub fn build(self) -> Result<MemoryNote, MemoryNoteBuildError> {
        //允许对字段的自由控制，以便于调试和修正
        if self.last_accessed_time < self.create_time {
            return Err(MemoryNoteBuildError::TimeConflict);
        }
        let time_now = Utc::now(); //提前计算时间，由于unwrap_or是eagerly evaluated的，所以防止可能的重复计算
        Ok(MemoryNote {
            id: self.id.unwrap_or_default(),
            tags: self.tags.unwrap_or_default(),
            retrieval_count: self.retrieval_count.unwrap_or_default(),
            create_time: self.create_time.unwrap_or(time_now),
            last_accessed_time: self.last_accessed_time.unwrap_or(time_now),
            mem_type: self.mem_type,
            mem_links: self.mem_links.unwrap_or_default(),
        })
    }
}

//定义错误类型，更健壮的处理
#[derive(Debug, Error)]
pub enum MemoryNoteBuildError {
    #[error("The last_accessed_time is earlier than create_time")]
    TimeConflict, //last_accessed_time比create_time更早
}
