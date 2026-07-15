use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::null_to_default;

//一种抽象性情景记忆、一种具体性情景记忆
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SituationType {
    ///抽象性的情景记忆（地点、人物、情境、事件）
    AbstractSituation(AbstractSituation),
    ///具体的情景记忆（场景描述、时间跨度、上下文）
    SpecificSituation(SpecificSituation),
}

impl From<AbstractSituation> for SituationType {
    fn from(situation: AbstractSituation) -> Self {
        SituationType::AbstractSituation(situation)
    }
}
impl From<SpecificSituation> for SituationType {
    fn from(situation: SpecificSituation) -> Self {
        SituationType::SpecificSituation(situation)
    }
}

///抽象性情景记忆（地点、人物、情境、事件）
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AbstractSituation {
    Location(Location),
    Participant(Participant),
    Environment(Environment),
    Event(Event),
}

impl From<Location> for AbstractSituation {
    fn from(location: Location) -> Self {
        AbstractSituation::Location(location)
    }
}
impl From<Participant> for AbstractSituation {
    fn from(participant: Participant) -> Self {
        AbstractSituation::Participant(participant)
    }
}
impl From<Environment> for AbstractSituation {
    fn from(environment: Environment) -> Self {
        AbstractSituation::Environment(environment)
    }
}
impl From<Event> for AbstractSituation {
    fn from(event: Event) -> Self {
        AbstractSituation::Event(event)
    }
}

///具体性情景记忆
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecificSituation {
    pub narrative: String,
    #[serde(default)]
    pub time_span: Option<DateTime<Utc>>,
    pub context: Context,
}

impl SpecificSituation {
    pub fn new(narrative: String, time_span: Option<DateTime<Utc>>, context: Context) -> Self {
        SpecificSituation {
            narrative,
            time_span,
            context,
        }
    }
    pub fn get_narrative(&self) -> &String {
        &self.narrative
    }
    pub fn get_mut_narrative(&mut self) -> &mut String {
        &mut self.narrative
    }
    pub fn get_time_span(&self) -> &Option<DateTime<Utc>> {
        &self.time_span
    }
    pub fn get_mut_time_span(&mut self) -> &mut Option<DateTime<Utc>> {
        &mut self.time_span
    }
    pub fn get_context(&self) -> &Context {
        &self.context
    }
    pub fn get_mut_context(&mut self) -> &mut Context {
        &mut self.context
    }
}

///情境上下文
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Context {
    #[schemars(default)]
    #[serde(default)]
    location: Option<Location>,

    #[schemars(default)]
    #[serde(default, deserialize_with = "null_to_default")]
    participants: Vec<Participant>,

    #[schemars(default)]
    #[serde(default, deserialize_with = "null_to_default")]
    emotions: Vec<Emotion>,

    #[schemars(default)]
    #[serde(default, deserialize_with = "null_to_default")]
    sensory_data: Vec<SensoryData>,

    #[schemars(default)]
    #[serde(default)]
    environment: Option<Environment>,

    #[schemars(default)]
    #[serde(default, deserialize_with = "null_to_default")]
    event: Vec<Event>,
}

impl Context {
    pub fn new(
        location: Option<Location>,
        participants: Vec<Participant>,
        emotions: Vec<Emotion>,
        sensory_data: Vec<SensoryData>,
        environment: Option<Environment>,
        event: Vec<Event>,
    ) -> Self {
        Context {
            location,
            participants,
            emotions,
            sensory_data,
            environment,
            event,
        }
    }
    pub fn get_mut_location(&mut self) -> &mut Option<Location> {
        &mut self.location
    }
    pub fn get_location(&self) -> &Option<Location> {
        &self.location
    }
    pub fn get_mut_participants(&mut self) -> &mut Vec<Participant> {
        &mut self.participants
    }
    pub fn get_participants(&self) -> &Vec<Participant> {
        &self.participants
    }
    pub fn get_mut_emotions(&mut self) -> &mut Vec<Emotion> {
        &mut self.emotions
    }
    pub fn get_emotions(&self) -> &Vec<Emotion> {
        &self.emotions
    }
    pub fn get_mut_sensory_data(&mut self) -> &mut Vec<SensoryData> {
        &mut self.sensory_data
    }
    pub fn get_sensory_data(&self) -> &Vec<SensoryData> {
        &self.sensory_data
    }
    pub fn get_mut_environment(&mut self) -> &mut Option<Environment> {
        &mut self.environment
    }
    pub fn get_environment(&self) -> &Option<Environment> {
        &self.environment
    }
    pub fn get_mut_event(&mut self) -> &mut Vec<Event> {
        &mut self.event
    }
    pub fn get_event(&self) -> &Vec<Event> {
        &self.event
    }
}

///事件（动作，动作强度，事件发起者，动作目标）
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Event {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub action_intensity: f32,
    #[serde(default)]
    pub initiator: String,
    #[serde(default)]
    pub target: String,
}

///环境（氛围，环境色调(抽象意义上)）
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Environment {
    pub atmosphere: String,
    pub tone: String,
}

///虚拟角色情绪
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Emotion {
    pub name: String,
    pub intensity: f32,
}

///记忆事件参与者
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Participant {
    pub name: String,
    pub role: String,
}

///地点
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Location {
    pub name: String,
    pub coordinates: String,
}

///感官数据（听觉，视觉，触觉，味觉，嗅觉等）
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SensoryData {
    pub name: String,
    pub intensity: f32,
}

///情境记忆链接
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum SituationMemLink {
    AbstractToSpecific(AbstractToSpecific),
}

///抽象到具体的链接
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AbstractToSpecific {}
impl AbstractToSpecific {
    pub fn new() -> Self {
        AbstractToSpecific {}
    }
}
