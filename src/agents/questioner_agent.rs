use funera::{Agent, AgentRuntime};
use funera::OpenAIProvider;
use schemars::schema_for;

use crate::data_model::questioner::retrieve::RetrieveAssessInfo;

pub struct QuestionerAgent;

impl QuestionerAgent {
    pub async fn quest(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        query: Option<&str>,
        tendency: Option<&str>,
    ) -> anyhow::Result<RetrieveAssessInfo> {
        let (system_prompt, user_msg) = Self::prepare_msgs(query, tendency);

        let runtime = AgentRuntime::<OpenAIProvider>::builder()
            .api_key(api_key.to_string())
            .base_url(api_base.map(|s| s.to_string()))
            .model(model.to_string())
            .build()?;

        let agent = Agent::builder()
            .system_prompt(system_prompt)
            .build();

        let resp = agent.fire(&user_msg, &runtime).await?;

        let retrieve_assess_info =
            serde_json::from_str::<RetrieveAssessInfo>(&resp.content);

        match retrieve_assess_info {
            Ok(info) => Ok(info),
            Err(e) => {
                tracing::warn!("json deserialization failed, trying fix...");
                let fix_response = Self::try_fix_json(
                    api_key,
                    api_base,
                    model,
                    &resp.content,
                    query,
                    tendency,
                    e,
                )
                .await?;
                serde_json::from_str::<RetrieveAssessInfo>(&fix_response).map_err(
                    |fatal_err| {
                        tracing::error!(
                            "fatal error in deserialization after fix. received:\n{fix_response}"
                        );
                        fatal_err.into()
                    },
                )
            }
        }
    }

    fn prepare_msgs(query: Option<&str>, tendency: Option<&str>) -> (String, String) {
        let system_prompt_head =
            include_str!("../prompt_template/questioner/retrieve_system");
        let info_schema = schema_for!(RetrieveAssessInfo);
        let system_prompt = format!(
            "{system_prompt_head}\n\n{}",
            serde_json::to_string_pretty(&info_schema).unwrap()
        );

        let user_msg = match (query, tendency) {
            (Some(q), Some(t)) => {
                format!("根据以下角色记忆图谱，自动生成检索查询集合。倾向：{t}\n\n{q}")
            }
            (Some(q), None) => {
                format!("根据以下角色记忆图谱，自动生成检索查询集合：\n\n{q}")
            }
            (None, Some(t)) => format!("自动生成检索查询集合。倾向：{t}"),
            (None, None) => "自动生成检索查询集合。".to_string(),
        };

        (system_prompt, user_msg)
    }

    async fn try_fix_json(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        json_str: &str,
        query: Option<&str>,
        tendency: Option<&str>,
        de_err: serde_json::Error,
    ) -> anyhow::Result<String> {
        let fixer_head = "你是一个 JSON 修复助手。以下 JSON 解析失败，请根据错误信息和原始上下文修复它，使其符合要求的 Schema。仅输出修复后的 JSON，不包含任何解释。";
        let info_schema = schema_for!(RetrieveAssessInfo);
        let fixer_system = format!(
            "{fixer_head}\n{}",
            serde_json::to_string_pretty(&info_schema).unwrap()
        );

        let runtime = AgentRuntime::<OpenAIProvider>::builder()
            .api_key(api_key.to_string())
            .base_url(api_base.map(|s| s.to_string()))
            .model(model.to_string())
            .build()?;

        let agent = Agent::builder()
            .system_prompt(fixer_system)
            .build();

        let user_msg = match (query, tendency) {
            (Some(q), Some(t)) => {
                format!(
                    "角色记忆图谱:\n{q}\n\n生成倾向:\n{t}\n\n# 损坏的 JSON\n{json_str}\n\n# 错误原因\n{de_err}"
                )
            }
            (Some(q), None) => {
                format!(
                    "角色记忆图谱:\n{q}\n\n# 损坏的 JSON\n{json_str}\n\n# 错误原因\n{de_err}"
                )
            }
            (None, Some(t)) => {
                format!(
                    "生成倾向:\n{t}\n\n# 损坏的 JSON\n{json_str}\n\n# 错误原因\n{de_err}"
                )
            }
            (None, None) => format!("# 损坏的 JSON\n{json_str}\n\n# 错误原因\n{de_err}"),
        };

        let resp = agent.fire(&user_msg, &runtime).await?;

        Ok(resp.content)
    }
}
