//! ST Preset system
//!
//! 预设系统，支持 Sampler/Instruct/Context/SystemPrompt/Reasoning/Prompt 六类预设。
//! 预设与 API 配置解耦，可跨 Provider 共享。
//! 参考: docs/74_st_presets.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Sampler Preset - 采样参数预设
// ============================================================================

/// Sampler 预设
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplerPreset {
    pub name: String,
    #[serde(default)]
    pub source_api_id: Option<String>,

    // 基础采样参数
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_top_p")]
    pub top_p: f64,
    #[serde(default)]
    pub top_k: i32,
    #[serde(default)]
    pub top_a: f64,
    #[serde(default)]
    pub min_p: f64,
    #[serde(default)]
    pub typical_p: f64,
    #[serde(default)]
    pub tfs: f64,
    #[serde(default)]
    pub epsilon_cutoff: f64,
    #[serde(default)]
    pub eta_cutoff: f64,

    // 重复惩罚
    #[serde(default)]
    pub repetition_penalty: f64,
    #[serde(default)]
    pub rep_pen_range: i32,
    #[serde(default)]
    pub rep_pen_decay: f64,
    #[serde(default)]
    pub rep_pen_slope: f64,
    #[serde(default)]
    pub frequency_penalty: f64,
    #[serde(default)]
    pub presence_penalty: f64,
    #[serde(default)]
    pub encoder_rep_pen: f64,

    // DRY
    #[serde(default)]
    pub dry_allowed_length: i32,
    #[serde(default)]
    pub dry_multiplier: f64,
    #[serde(default)]
    pub dry_base: f64,
    #[serde(default)]
    pub dry_sequence_breakers: String,

    // Mirostat
    #[serde(default)]
    pub mirostat_mode: i32,
    #[serde(default)]
    pub mirostat_tau: f64,
    #[serde(default)]
    pub mirostat_eta: f64,

    // 其他
    #[serde(default)]
    pub no_repeat_ngram_size: i32,
    #[serde(default)]
    pub guidance_scale: f64,
    #[serde(default)]
    pub negative_prompt: String,

    // 采样顺序
    #[serde(default)]
    pub sampler_priority: Vec<String>,
    #[serde(default)]
    pub temperature_last: bool,

    // 扩展
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub provider_overrides: HashMap<String, HashMap<String, serde_json::Value>>,
}

fn default_temperature() -> f64 { 1.0 }
fn default_top_p() -> f64 { 1.0 }

impl SamplerPreset {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            source_api_id: None,
            temperature: 1.0,
            top_p: 1.0,
            top_k: 0,
            top_a: 0.0,
            min_p: 0.0,
            typical_p: 1.0,
            tfs: 1.0,
            epsilon_cutoff: 0.0,
            eta_cutoff: 0.0,
            repetition_penalty: 1.0,
            rep_pen_range: 0,
            rep_pen_decay: 0.0,
            rep_pen_slope: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            encoder_rep_pen: 1.0,
            dry_allowed_length: 0,
            dry_multiplier: 0.0,
            dry_base: 0.0,
            dry_sequence_breakers: String::new(),
            mirostat_mode: 0,
            mirostat_tau: 5.0,
            mirostat_eta: 0.1,
            no_repeat_ngram_size: 0,
            guidance_scale: 1.0,
            negative_prompt: String::new(),
            sampler_priority: Vec::new(),
            temperature_last: false,
            extensions: HashMap::new(),
            provider_overrides: HashMap::new(),
        }
    }
}

// ============================================================================
// Instruct Template - 对话格式模板
// ============================================================================

/// Instruct 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructTemplate {
    pub name: String,

    // 序列定义
    #[serde(default)]
    pub input_sequence: String,
    #[serde(default)]
    pub output_sequence: String,
    #[serde(default)]
    pub system_sequence: String,
    #[serde(default)]
    pub stop_sequence: String,

    // 后缀
    #[serde(default)]
    pub input_suffix: String,
    #[serde(default)]
    pub output_suffix: String,
    #[serde(default)]
    pub system_suffix: String,

    // 特殊序列
    #[serde(default)]
    pub first_input_sequence: String,
    #[serde(default)]
    pub last_input_sequence: String,
    #[serde(default)]
    pub first_output_sequence: String,
    #[serde(default)]
    pub last_output_sequence: String,

    // 故事字符串
    #[serde(default)]
    pub story_string_prefix: String,
    #[serde(default)]
    pub story_string_suffix: String,

    // 行为设置
    #[serde(default)]
    pub wrap: bool,
    #[serde(default, rename = "macro")]
    pub use_macro: bool,
    #[serde(default = "default_names_behavior")]
    pub names_behavior: String,
    #[serde(default)]
    pub system_same_as_user: bool,
    #[serde(default)]
    pub skip_examples: bool,
    #[serde(default)]
    pub sequences_as_stop_strings: bool,

    // 激活正则
    #[serde(default)]
    pub activation_regex: String,

    // 扩展
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

fn default_names_behavior() -> String { "none".to_string() }

impl InstructTemplate {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            input_sequence: String::new(),
            output_sequence: String::new(),
            system_sequence: String::new(),
            stop_sequence: String::new(),
            input_suffix: String::new(),
            output_suffix: String::new(),
            system_suffix: String::new(),
            first_input_sequence: String::new(),
            last_input_sequence: String::new(),
            first_output_sequence: String::new(),
            last_output_sequence: String::new(),
            story_string_prefix: String::new(),
            story_string_suffix: String::new(),
            wrap: false,
            use_macro: true,
            names_behavior: "none".to_string(),
            system_same_as_user: false,
            skip_examples: false,
            sequences_as_stop_strings: false,
            activation_regex: String::new(),
            extensions: HashMap::new(),
        }
    }
}

// ============================================================================
// Context Template - 上下文组装模板
// ============================================================================

/// Context 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTemplate {
    pub name: String,

    // 模板内容
    #[serde(default)]
    pub story_string: String,
    #[serde(default)]
    pub example_separator: String,
    #[serde(default)]
    pub chat_start: String,

    // 停止字符串
    #[serde(default)]
    pub use_stop_strings: bool,
    #[serde(default)]
    pub names_as_stop_strings: bool,

    // 故事字符串位置
    #[serde(default)]
    pub story_string_position: i32,
    #[serde(default)]
    pub story_string_depth: i32,
    #[serde(default)]
    pub story_string_role: i32,

    // 其他设置
    #[serde(default)]
    pub always_force_name2: bool,
    #[serde(default)]
    pub trim_sentences: bool,
    #[serde(default)]
    pub single_line: bool,

    // 扩展
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl ContextTemplate {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            story_string: String::new(),
            example_separator: String::new(),
            chat_start: String::new(),
            use_stop_strings: true,
            names_as_stop_strings: false,
            story_string_position: 0,
            story_string_depth: 4,
            story_string_role: 0,
            always_force_name2: false,
            trim_sentences: false,
            single_line: false,
            extensions: HashMap::new(),
        }
    }
}

// ============================================================================
// System Prompt - 系统提示词
// ============================================================================

/// System Prompt 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPrompt {
    pub name: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl SystemPrompt {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            content: String::new(),
            extensions: HashMap::new(),
        }
    }
}

// ============================================================================
// Reasoning Template - 思维链格式模板
// ============================================================================

/// Reasoning 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTemplate {
    pub name: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub separator: String,
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl ReasoningTemplate {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            prefix: String::new(),
            suffix: String::new(),
            separator: String::new(),
            extensions: HashMap::new(),
        }
    }
}

// ============================================================================
// Prompt Preset - 完整提示词组装配置
// ============================================================================

/// Prompt 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptItem {
    pub identifier: String,
    pub name: String,
    pub role: String, // system | user | assistant
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub system_prompt: bool,
    #[serde(default)]
    pub marker: bool,
}

/// Prompt 顺序条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOrderItem {
    pub identifier: String,
    #[serde(default)]
    pub enabled: bool,
}

/// Prompt 顺序
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOrder {
    #[serde(default)]
    pub character_id: i32, // 100000=默认，100001=群聊
    #[serde(default)]
    pub order: Vec<PromptOrderItem>,
}

/// Prompt 预设
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPreset {
    pub name: String,

    // 提示词列表
    #[serde(default)]
    pub prompts: Vec<PromptItem>,
    #[serde(default)]
    pub prompt_order: Vec<PromptOrder>,

    // 格式化模板
    #[serde(default)]
    pub wi_format: String,
    #[serde(default)]
    pub scenario_format: String,
    #[serde(default)]
    pub personality_format: String,

    // 特殊提示词
    #[serde(default)]
    pub new_chat_prompt: String,
    #[serde(default)]
    pub new_group_chat_prompt: String,
    #[serde(default)]
    pub continue_nudge_prompt: String,
    #[serde(default)]
    pub group_nudge_prompt: String,
    #[serde(default)]
    pub impersonation_prompt: String,

    // 扩展
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl PromptPreset {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            prompts: Vec::new(),
            prompt_order: Vec::new(),
            wi_format: String::new(),
            scenario_format: String::new(),
            personality_format: String::new(),
            new_chat_prompt: String::new(),
            new_group_chat_prompt: String::new(),
            continue_nudge_prompt: String::new(),
            group_nudge_prompt: String::new(),
            impersonation_prompt: String::new(),
            extensions: HashMap::new(),
        }
    }
}

// ============================================================================
// Preset Type Enum
// ============================================================================

/// 预设类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetType {
    Sampler,
    Instruct,
    Context,
    Sysprompt,
    Reasoning,
    Prompt,
}

impl PresetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PresetType::Sampler => "samplers",
            PresetType::Instruct => "instruct",
            PresetType::Context => "context",
            PresetType::Sysprompt => "sysprompt",
            PresetType::Reasoning => "reasoning",
            PresetType::Prompt => "prompts",
        }
    }
}

// ============================================================================
// Auto Select Config
// ============================================================================

/// 自动选择绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSelectBinding {
    #[serde(default)]
    pub character_name: Option<String>,
    #[serde(default)]
    pub group_name: Option<String>,
    pub preset_type: PresetType,
    pub preset_name: String,
}

/// 自动选择配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoSelectConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bindings: Vec<AutoSelectBinding>,
}
