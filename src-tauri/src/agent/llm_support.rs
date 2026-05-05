use chrono::Utc;

use crate::storage::st_resources::ApiConfig;

pub(crate) fn default_agent_api_config() -> ApiConfig {
    let now = Utc::now().to_rfc3339();
    ApiConfig {
        id: "agent-default".to_string(),
        name: "agent-default".to_string(),
        provider: "openai_chat".to_string(),
        model: "gpt-4o-mini".to_string(),
        base_url: None,
        api_key: None,
        enabled: true,
        settings: serde_json::Map::new(),
        created_at: now.clone(),
        updated_at: now,
    }
}
