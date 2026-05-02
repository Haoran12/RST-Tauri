//! ST Runtime Assembly
//!
//! 运行时组装流程：从全局状态、预设、会话内容到最终 AI 请求的完整流程。
//! 参考: docs/75_st_runtime_assembly.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::st::preset::{
    SamplerPreset, InstructTemplate, ContextTemplate, SystemPrompt,
    ReasoningTemplate, PromptPreset,
};
use crate::st::regex_engine::RegexExtensionSettings;
use crate::st::keyword_matcher::GlobalScanData;
use crate::storage::st_resources::{ApiConfig, TavernCardV3};

// ============================================================================
// 全局应用状态
// ============================================================================

/// 全局应用状态
///
/// API 配置与预设、世界书选择完全独立，用户可随时切换，不与会话绑定。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAppState {
    /// 当前激活的 API 配置 ID
    pub active_api_config_id: Option<String>,

    /// 当前激活的各类预设名称
    pub active_sampler_preset: String,
    pub active_instruct_preset: String,
    pub active_context_preset: String,
    pub active_sysprompt_preset: String,
    pub active_reasoning_preset: String,
    pub active_prompt_preset: String,

    /// 是否启用自动预设选择
    pub auto_select_preset: bool,

    /// ST 世界书全局设置
    pub world_info_settings: STWorldInfoSettings,

    /// ST Regex 扩展全局设置
    pub regex_settings: RegexExtensionSettings,
}

impl Default for GlobalAppState {
    fn default() -> Self {
        Self {
            active_api_config_id: None,
            active_sampler_preset: String::new(),
            active_instruct_preset: String::new(),
            active_context_preset: String::new(),
            active_sysprompt_preset: String::new(),
            active_reasoning_preset: String::new(),
            active_prompt_preset: String::new(),
            auto_select_preset: false,
            world_info_settings: STWorldInfoSettings::default(),
            regex_settings: RegexExtensionSettings::default(),
        }
    }
}

/// ST 世界书全局设置
///
/// 对应 SillyTavern settings.world_info_settings / world_info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STWorldInfoSettings {
    /// 全局选中的世界书列表（RST 内部使用 lore_id）
    pub global_select: Vec<String>,

    /// 扫描深度
    pub world_info_depth: i32,

    /// 最小激活次数
    pub world_info_min_activations: i32,

    /// 最小激活次数最大深度
    pub world_info_min_activations_depth_max: i32,

    /// 世界书预算（max context 百分比）
    pub world_info_budget: i32,

    /// 世界书预算上限
    pub world_info_budget_cap: i32,

    /// 是否在注入内容中包含名字
    pub world_info_include_names: bool,

    /// 是否启用递归扫描
    pub world_info_recursive: bool,

    /// 是否启用预算溢出警告
    pub world_info_overflow_alert: bool,

    /// 是否区分大小写
    pub world_info_case_sensitive: bool,

    /// 是否匹配完整单词
    pub world_info_match_whole_words: bool,

    /// 是否使用分组评分
    pub world_info_use_group_scoring: bool,

    /// 角色世界书策略：0=evenly, 1=character_first, 2=global_first
    pub world_info_character_strategy: i32,

    /// 最大递归步数
    pub world_info_max_recursion_steps: i32,

    /// 角色额外世界书（按角色文件名绑定）
    pub char_lore: Vec<CharLoreBinding>,
}

impl Default for STWorldInfoSettings {
    fn default() -> Self {
        Self {
            global_select: Vec::new(),
            world_info_depth: 4,
            world_info_min_activations: 0,
            world_info_min_activations_depth_max: 0,
            world_info_budget: 25,
            world_info_budget_cap: 0,
            world_info_include_names: false,
            world_info_recursive: true,
            world_info_overflow_alert: true,
            world_info_case_sensitive: false,
            world_info_match_whole_words: false,
            world_info_use_group_scoring: false,
            world_info_character_strategy: 1, // character_first
            world_info_max_recursion_steps: 5,
            char_lore: Vec::new(),
        }
    }
}

/// 角色额外世界书绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharLoreBinding {
    /// 角色名称（对应 ST 的角色文件名）
    pub name: String,
    /// 额外世界书列表
    pub extra_books: Vec<String>,
}

// ============================================================================
// 会话数据
// ============================================================================

/// ST 会话数据
///
/// 存储聊天记录、角色卡引用和 ST 兼容的聊天元数据。
/// 不存储 API 配置或预设引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STSessionData {
    pub session_id: String,
    pub character_id: Option<String>,
    pub group_id: Option<String>,
    pub chat_metadata: STChatMetadata,
    pub messages: Vec<STChatMessage>,
}

/// ST 聊天元数据
///
/// 与 SillyTavern chat_metadata 兼容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STChatMetadata {
    /// Chat lore：当前聊天绑定的单本世界书（RST 内部使用 lore_id）
    pub world_info: Option<String>,

    /// 其他扩展字段（Author's Note、变量、脚本注入、书签等）
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for STChatMetadata {
    fn default() -> Self {
        Self {
            world_info: None,
            extra: serde_json::Map::new(),
        }
    }
}

/// ST 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STChatMessage {
    pub id: String,
    pub role: String, // user | assistant | system
    pub content: String,
    pub created_at: String,
    pub name: Option<String>,
}

// ============================================================================
// 运行时组装上下文
// ============================================================================

/// 运行时组装上下文
///
/// 包含一次生成请求所需的所有数据。
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    /// API 配置
    pub api_config: ApiConfig,

    /// 预设
    pub sampler_preset: Option<SamplerPreset>,
    pub instruct_template: Option<InstructTemplate>,
    pub context_template: Option<ContextTemplate>,
    pub system_prompt: Option<SystemPrompt>,
    pub reasoning_template: Option<ReasoningTemplate>,
    pub prompt_preset: Option<PromptPreset>,

    /// 角色卡
    pub character: Option<TavernCardV3>,

    /// 会话数据
    pub session: STSessionData,

    /// 全局扫描数据
    pub global_scan_data: GlobalScanData,

    /// 世界书注入结果
    pub world_info_result: Option<WorldInfoInjectionResult>,
}

/// 世界书注入结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldInfoInjectionResult {
    /// BEFORE_CHAR 位置的内容
    pub world_info_before: String,

    /// AFTER_CHAR 位置的内容
    pub world_info_after: String,

    /// AT_DEPTH 位置的内容（按 depth + role 分组）
    pub world_info_depth: HashMap<i32, HashMap<i32, String>>, // depth -> role -> content

    /// EM_TOP / EM_BOTTOM 示例消息前后插入
    pub em_top: String,
    pub em_bottom: String,

    /// AN_TOP / AN_BOTTOM 作者注释上下拼接
    pub an_top: String,
    pub an_bottom: String,

    /// OUTLET 命名出口
    pub outlets: HashMap<String, String>,

    /// 激活的词条 UID 列表
    pub activated_entries: Vec<i32>,

    /// 使用的 token 数量
    pub tokens_used: i32,
}

// ============================================================================
// 请求组装器
// ============================================================================

/// 请求组装器
///
/// 负责把当前 preset + API config + ST prompt 组装成中立 ChatRequest。
pub struct RequestAssembler;

impl RequestAssembler {
    /// 组装运行时上下文为 ChatRequest
    pub fn assemble(context: &RuntimeContext) -> AssembledRequest {
        let mut request = AssembledRequest::default();

        // 1. 构建系统提示词
        request.system_prompt = Self::build_system_prompt(context);

        // 2. 构建对话历史
        request.messages = Self::build_messages(context);

        // 3. 应用采样参数
        if let Some(sampler) = &context.sampler_preset {
            request.sampling.temperature = Some(sampler.temperature);
            request.sampling.top_p = Some(sampler.top_p);
            request.sampling.top_k = if sampler.top_k > 0 { Some(sampler.top_k) } else { None };
            request.sampling.frequency_penalty = Some(sampler.frequency_penalty);
            request.sampling.presence_penalty = Some(sampler.presence_penalty);
            request.sampling.repetition_penalty = Some(sampler.repetition_penalty);
        }

        // 4. 应用停止序列
        if let Some(instruct) = &context.instruct_template {
            if !instruct.stop_sequence.is_empty() {
                request.stop_sequences.push(instruct.stop_sequence.clone());
            }
            if instruct.sequences_as_stop_strings {
                if !instruct.input_sequence.is_empty() {
                    request.stop_sequences.push(instruct.input_sequence.clone());
                }
                if !instruct.output_sequence.is_empty() {
                    request.stop_sequences.push(instruct.output_sequence.clone());
                }
            }
        }

        // 5. 设置 max_tokens
        request.max_tokens = Some(4096);

        request
    }

    /// 构建系统提示词
    fn build_system_prompt(context: &RuntimeContext) -> String {
        let mut parts: Vec<String> = Vec::new();

        // 系统提示词预设
        if let Some(sp) = &context.system_prompt {
            if !sp.content.is_empty() {
                parts.push(sp.content.clone());
            }
        }

        // 角色卡系统提示词
        if let Some(char) = &context.character {
            if !char.data.system_prompt.is_empty() {
                parts.push(char.data.system_prompt.clone());
            }
        }

        // 世界书注入结果 - BEFORE_CHAR
        if let Some(wi) = &context.world_info_result {
            if !wi.world_info_before.is_empty() {
                parts.push(wi.world_info_before.clone());
            }
        }

        // 角色描述
        if let Some(char) = &context.character {
            if !char.data.description.is_empty() {
                parts.push(format!("Description: {}", char.data.description));
            }
            if !char.data.personality.is_empty() {
                parts.push(format!("Personality: {}", char.data.personality));
            }
            if !char.data.scenario.is_empty() {
                parts.push(format!("Scenario: {}", char.data.scenario));
            }
        }

        // 世界书注入结果 - AFTER_CHAR
        if let Some(wi) = &context.world_info_result {
            if !wi.world_info_after.is_empty() {
                parts.push(wi.world_info_after.clone());
            }
        }

        parts.join("\n\n")
    }

    /// 构建消息列表
    fn build_messages(context: &RuntimeContext) -> Vec<AssembledMessage> {
        let mut messages: Vec<AssembledMessage> = Vec::new();

        // 转换聊天历史
        for msg in &context.session.messages {
            let role = match msg.role.as_str() {
                "user" => "user",
                "assistant" => "assistant",
                "system" => "system",
                _ => "user",
            };

            // 简化处理：不包含名字前缀
            // 如果需要包含名字，可以在调用时通过参数控制
            messages.push(AssembledMessage {
                role: role.to_string(),
                content: msg.content.clone(),
            });
        }

        messages
    }
}

// ============================================================================
// 组装后的请求
// ============================================================================

/// 组装后的中立请求
///
/// 可被 ProviderRequestMapper 映射到具体 Provider 参数。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssembledRequest {
    /// 系统提示词
    pub system_prompt: String,

    /// 消息列表
    pub messages: Vec<AssembledMessage>,

    /// 采样参数
    pub sampling: AssembledSamplingParams,

    /// 停止序列
    pub stop_sequences: Vec<String>,

    /// 最大 token 数
    pub max_tokens: Option<i32>,

    /// 推理设置
    pub reasoning: Option<AssembledReasoningParams>,
}

/// 组装后的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledMessage {
    pub role: String,
    pub content: String,
}

/// 组装后的采样参数
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssembledSamplingParams {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<i32>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub repetition_penalty: Option<f64>,
}

/// 组装后的推理参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledReasoningParams {
    pub enabled: bool,
    pub effort: Option<String>, // low, medium, high
    pub budget_tokens: Option<i32>,
}

// ============================================================================
// Provider 请求映射器
// ============================================================================

/// Provider 请求映射器
///
/// 负责把中立请求映射到具体 Provider 参数，并处理不支持字段。
pub struct ProviderRequestMapper;

impl ProviderRequestMapper {
    /// 映射到 OpenAI Responses API 格式
    pub fn map_to_openai_responses(request: &AssembledRequest, model: &str) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        if !request.system_prompt.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": request.system_prompt
            }));
        }

        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        serde_json::json!({
            "model": model,
            "input": messages,
            "temperature": request.sampling.temperature,
            "top_p": request.sampling.top_p,
            "stream": true
        })
    }

    /// 映射到 OpenAI Chat Completions API 格式
    pub fn map_to_openai_chat(request: &AssembledRequest, model: &str) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        if !request.system_prompt.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": request.system_prompt
            }));
        }

        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": request.sampling.temperature,
            "top_p": request.sampling.top_p,
            "frequency_penalty": request.sampling.frequency_penalty,
            "presence_penalty": request.sampling.presence_penalty,
            "stop": request.stop_sequences,
            "stream": true,
            "stream_options": { "include_usage": true }
        })
    }

    /// 映射到 DeepSeek API 格式
    pub fn map_to_deepseek(request: &AssembledRequest, model: &str) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        if !request.system_prompt.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": request.system_prompt
            }));
        }

        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": request.sampling.temperature,
            "top_p": request.sampling.top_p,
            "frequency_penalty": request.sampling.frequency_penalty,
            "presence_penalty": request.sampling.presence_penalty,
            "stop": request.stop_sequences,
            "stream": true
        });

        // 推理参数
        if let Some(reasoning) = &request.reasoning {
            if reasoning.enabled {
                body["thinking"] = serde_json::json!({
                    "type": "enabled",
                    "reasoning_effort": reasoning.effort
                });
            }
        }

        body
    }

    /// 映射到 Anthropic Messages API 格式
    pub fn map_to_anthropic(request: &AssembledRequest, model: &str) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // Anthropic 系统提示词单独传递
        let system = request.system_prompt.clone();

        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "system": system,
            "messages": messages,
            "temperature": request.sampling.temperature,
            "top_p": request.sampling.top_p,
            "stream": true
        });

        // top_k 仅 Anthropic 支持
        if let Some(top_k) = request.sampling.top_k {
            body["top_k"] = serde_json::json!(top_k);
        }

        // 推理参数
        if let Some(reasoning) = &request.reasoning {
            if reasoning.enabled {
                body["thinking"] = serde_json::json!({
                    "type": "enabled",
                    "budget_tokens": reasoning.budget_tokens.unwrap_or(2048)
                });
            }
        }

        body
    }

    /// 映射到 Gemini GenerateContent API 格式
    pub fn map_to_gemini(request: &AssembledRequest, model: &str) -> serde_json::Value {
        let mut contents: Vec<serde_json::Value> = Vec::new();

        for msg in &request.messages {
            let role = match msg.role.as_str() {
                "user" => "user",
                "assistant" => "model",
                "system" => "user", // Gemini 没有 system role，用 user + system prompt 处理
                _ => "user",
            };
            contents.push(serde_json::json!({
                "role": role,
                "parts": [{ "text": msg.content }]
            }));
        }

        serde_json::json!({
            "model": model,
            "contents": contents,
            "systemInstruction": { "parts": [{ "text": request.system_prompt }] },
            "generationConfig": {
                "temperature": request.sampling.temperature,
                "topP": request.sampling.top_p,
                "topK": request.sampling.top_k,
                "maxOutputTokens": request.max_tokens,
                "stopSequences": request.stop_sequences
            }
        })
    }

    /// 映射到 Claude Code Interface 格式
    pub fn map_to_claude_code(request: &AssembledRequest) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        serde_json::json!({
            "system": request.system_prompt,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true
        })
    }
}