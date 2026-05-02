//! Log context

use serde::{Deserialize, Serialize};

/// Log mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogMode {
    St,
    Agent,
}

/// LLM node type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LlmNode {
    STChat,
    SceneInitializer,
    SceneStateExtractor,
    CharacterCognitivePass,
    OutcomePlanner,
    SurfaceRealizer,
}

/// Log context for LLM calls
#[derive(Debug, Clone)]
pub struct LogContext {
    pub mode: LogMode,
    pub world_id: Option<String>,
    pub scene_turn_id: Option<String>,
    pub character_id: Option<String>,
    pub trace_id: Option<String>,
    pub llm_node: LlmNode,
    pub api_config_id: String,
    pub request_id: String,
}
