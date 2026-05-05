//! PromptBuilder and prompt contract control plane for Agent LLM nodes.
//!
//! This module centralizes:
//! - static node contracts
//! - dynamic structured input packaging
//! - deterministic message layout
//! - prompt hash generation
//! - heuristic token budgeting and deterministic pruning

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api::provider::{
    ChatMessage, ChatRequest, ChatRole, ContentPart, ReasoningParams, ResponseFormat,
    SamplingParams,
};
use crate::logging::context::LlmNode;

/// Agent LLM node identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentLlmNode {
    SceneInitializer,
    SceneStateExtractor,
    CharacterCognitivePass,
    OutcomePlanner,
    SurfaceRealizer,
}

impl AgentLlmNode {
    pub fn prompt_template_id(self) -> &'static str {
        match self {
            Self::SceneInitializer => "agent.scene_initializer",
            Self::SceneStateExtractor => "agent.scene_state_extractor",
            Self::CharacterCognitivePass => "agent.character_cognitive_pass",
            Self::OutcomePlanner => "agent.outcome_planner",
            Self::SurfaceRealizer => "agent.surface_realizer",
        }
    }

    pub fn default_prompt_version(self) -> &'static str {
        "1.0.0"
    }

    pub fn default_output_schema_id(self) -> &'static str {
        match self {
            Self::SceneInitializer => "SceneInitializationDraft",
            Self::SceneStateExtractor => "SceneStateExtractorOutput",
            Self::CharacterCognitivePass => "CharacterCognitivePassOutput",
            Self::OutcomePlanner => "OutcomePlannerOutput",
            Self::SurfaceRealizer => "SurfaceRealizerOutput",
        }
    }

    pub fn system_contract(self) -> String {
        let base = match self {
            Self::SceneInitializer => SCENE_INITIALIZER_CONTRACT,
            Self::SceneStateExtractor => SCENE_STATE_EXTRACTOR_CONTRACT,
            Self::CharacterCognitivePass => CHARACTER_COGNITIVE_PASS_CONTRACT,
            Self::OutcomePlanner => OUTCOME_PLANNER_CONTRACT,
            Self::SurfaceRealizer => SURFACE_REALIZER_CONTRACT,
        };

        format!("{base}{COMMON_RULES_CONTRACT}")
    }
}

impl From<AgentLlmNode> for LlmNode {
    fn from(value: AgentLlmNode) -> Self {
        match value {
            AgentLlmNode::SceneInitializer => LlmNode::SceneInitializer,
            AgentLlmNode::SceneStateExtractor => LlmNode::SceneStateExtractor,
            AgentLlmNode::CharacterCognitivePass => LlmNode::CharacterCognitivePass,
            AgentLlmNode::OutcomePlanner => LlmNode::OutcomePlanner,
            AgentLlmNode::SurfaceRealizer => LlmNode::SurfaceRealizer,
        }
    }
}

/// Generic prompt bundle that can be logged, replayed, and sent to providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPromptBundle<TInput = Value> {
    pub prompt_template_id: String,
    pub prompt_version: String,
    pub llm_node: AgentLlmNode,
    pub system_contract: String,
    pub task_instructions: Vec<String>,
    pub input: TInput,
    pub output_schema_id: Option<String>,
    pub output_schema_json: Option<Value>,
    pub prompt_hash: String,
    pub budget_report: PromptBudgetReport,
}

/// Budget report recorded in trace/logs, not reused as business input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBudgetReport {
    pub estimated_input_tokens: u32,
    pub effective_max_context_tokens: u32,
    pub budget_stage: String,
    pub section_breakdown: Value,
    pub compressed_sections: Vec<String>,
    pub pruned_refs: Vec<String>,
    pub fit_iterations: u8,
}

/// Token budget policy for Agent prompts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBudgetSettings {
    pub critical_attention_tokens: u32,
    pub soft_input_tokens: u32,
    pub max_context_tokens: u32,
    pub reserved_output_tokens: u32,
}

impl PromptBudgetSettings {
    pub fn effective_max_context_tokens(&self) -> u32 {
        self.max_context_tokens
            .saturating_sub(self.reserved_output_tokens)
    }
}

impl Default for PromptBudgetSettings {
    fn default() -> Self {
        Self {
            critical_attention_tokens: 8192,
            soft_input_tokens: 16384,
            max_context_tokens: 32768,
            reserved_output_tokens: 4096,
        }
    }
}

/// Priority buckets used by deterministic pruning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptPriority {
    P0Required,
    P1DecisionCritical,
    P2Contextual,
    P3OptionalFlavor,
}

/// Declarative input section metadata for deterministic pruning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInputSection {
    pub pointer: String,
    pub label: String,
    pub priority: PromptPriority,
}

/// Build-time options for a single Agent prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBuildOptions {
    pub prompt_version: Option<String>,
    pub task_instructions: Vec<String>,
    pub output_schema_id: Option<String>,
    pub output_schema_json: Option<Value>,
    pub input_sections: Vec<PromptInputSection>,
}

impl Default for PromptBuildOptions {
    fn default() -> Self {
        Self {
            prompt_version: None,
            task_instructions: Vec::new(),
            output_schema_id: None,
            output_schema_json: None,
            input_sections: Vec::new(),
        }
    }
}

/// Options for converting a prompt bundle into a provider request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRequestOptions {
    pub request_id: String,
    pub api_config_id: String,
    pub sampling: SamplingParams,
    pub stop_sequences: Vec<String>,
    pub max_tokens: Option<u32>,
    pub reasoning: Option<ReasoningParams>,
    pub provider_overrides: Value,
}

impl PromptRequestOptions {
    pub fn new(request_id: impl Into<String>, api_config_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            api_config_id: api_config_id.into(),
            sampling: SamplingParams::default(),
            stop_sequences: Vec::new(),
            max_tokens: None,
            reasoning: None,
            provider_overrides: Value::Object(Default::default()),
        }
    }
}

/// Unified PromptBuilder for Agent nodes.
#[derive(Debug, Clone)]
pub struct PromptBuilder {
    budget: PromptBudgetSettings,
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new(PromptBudgetSettings::default())
    }
}

impl PromptBuilder {
    pub fn new(budget: PromptBudgetSettings) -> Self {
        Self { budget }
    }

    pub fn budget(&self) -> &PromptBudgetSettings {
        &self.budget
    }

    pub fn build_bundle<T: Serialize>(
        &self,
        llm_node: AgentLlmNode,
        input: &T,
        options: PromptBuildOptions,
    ) -> Result<AgentPromptBundle<Value>, String> {
        let mut input_value = serde_json::to_value(input)
            .map_err(|e| format!("Failed to serialize prompt input: {e}"))?;

        let system_contract = llm_node.system_contract().trim().to_string();
        let prompt_version = options
            .prompt_version
            .unwrap_or_else(|| llm_node.default_prompt_version().to_string());
        let mut task_instructions = dedupe_non_empty_lines(options.task_instructions);
        let mut compressed_sections = Vec::new();
        let mut pruned_refs = Vec::new();
        let mut fit_iterations = 0u8;
        let effective_max = self.budget.effective_max_context_tokens();

        if dedupe_and_compact_task_instructions(&mut task_instructions) {
            compressed_sections.push("task_instructions".to_string());
        }

        let mut section_breakdown = self.measure_sections(
            &system_contract,
            &task_instructions,
            &input_value,
            options.output_schema_json.as_ref(),
        );
        let mut estimated_input_tokens = section_breakdown
            .get("estimated_total_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32;

        if estimated_input_tokens > self.budget.soft_input_tokens {
            fit_iterations = fit_iterations.saturating_add(1);
            for priority in [
                PromptPriority::P3OptionalFlavor,
                PromptPriority::P2Contextual,
            ] {
                let refs: Vec<String> = options
                    .input_sections
                    .iter()
                    .filter(|section| section.priority == priority)
                    .map(|section| section.pointer.clone())
                    .collect();

                for pointer in refs {
                    if estimated_input_tokens <= effective_max {
                        break;
                    }
                    if remove_json_pointer(&mut input_value, &pointer) {
                        pruned_refs.push(pointer);
                        fit_iterations = fit_iterations.saturating_add(1);
                        section_breakdown = self.measure_sections(
                            &system_contract,
                            &task_instructions,
                            &input_value,
                            options.output_schema_json.as_ref(),
                        );
                        estimated_input_tokens = section_breakdown
                            .get("estimated_total_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u32;
                    }
                }
            }
        }

        let budget_stage = if estimated_input_tokens <= self.budget.critical_attention_tokens {
            "within_8k"
        } else if estimated_input_tokens <= self.budget.soft_input_tokens {
            if pruned_refs.is_empty() && compressed_sections.is_empty() {
                "within_16k"
            } else if !pruned_refs.is_empty() {
                "pruned"
            } else {
                "compressed"
            }
        } else if estimated_input_tokens <= effective_max {
            "max_context_fit"
        } else {
            "blocked_unfit"
        }
        .to_string();

        let output_schema_id = options
            .output_schema_id
            .or_else(|| Some(llm_node.default_output_schema_id().to_string()));

        let prompt_hash = compute_prompt_hash(
            &llm_node,
            &prompt_version,
            &system_contract,
            &task_instructions,
            &input_value,
            output_schema_id.as_deref(),
            options.output_schema_json.as_ref(),
        );

        Ok(AgentPromptBundle {
            prompt_template_id: llm_node.prompt_template_id().to_string(),
            prompt_version,
            llm_node,
            system_contract,
            task_instructions,
            input: input_value,
            output_schema_id,
            output_schema_json: options.output_schema_json,
            prompt_hash,
            budget_report: PromptBudgetReport {
                estimated_input_tokens,
                effective_max_context_tokens: effective_max,
                budget_stage,
                section_breakdown,
                compressed_sections,
                pruned_refs,
                fit_iterations,
            },
        })
    }

    pub fn build_messages(
        &self,
        bundle: &AgentPromptBundle<Value>,
    ) -> Result<Vec<ChatMessage>, String> {
        let user_payload = serde_json::to_string(&json!({ "input": bundle.input }))
            .map_err(|e| format!("Failed to serialize structured prompt input payload: {e}"))?;

        let mut messages = vec![ChatMessage::system(bundle.system_contract.clone())];
        if !bundle.task_instructions.is_empty() {
            messages.push(ChatMessage {
                role: ChatRole::Developer,
                content: vec![ContentPart::Text {
                    text: bundle.task_instructions.join("\n"),
                }],
                name: None,
            });
        }
        messages.push(ChatMessage::user(user_payload));
        Ok(messages)
    }

    pub fn build_chat_request(
        &self,
        bundle: &AgentPromptBundle<Value>,
        options: PromptRequestOptions,
    ) -> Result<ChatRequest, String> {
        let messages = self.build_messages(bundle)?;
        let response_format =
            bundle
                .output_schema_json
                .as_ref()
                .map(|schema| ResponseFormat::JsonSchema {
                    schema: schema.clone(),
                    strict: true,
                });

        Ok(ChatRequest {
            request_id: options.request_id,
            api_config_id: options.api_config_id,
            messages,
            sampling: options.sampling,
            stop_sequences: options.stop_sequences,
            max_tokens: options.max_tokens,
            stream: false,
            reasoning: options.reasoning,
            response_format,
            provider_overrides: options.provider_overrides,
        })
    }

    fn measure_sections(
        &self,
        system_contract: &str,
        task_instructions: &[String],
        input: &Value,
        output_schema_json: Option<&Value>,
    ) -> Value {
        let system_tokens = estimate_tokens(system_contract);
        let task_text = task_instructions.join("\n");
        let task_tokens = estimate_tokens(&task_text);
        let input_json = compact_json_string(input);
        let input_tokens = estimate_tokens(&input_json);
        let schema_json = output_schema_json
            .map(compact_json_string)
            .unwrap_or_default();
        let schema_tokens = estimate_tokens(&schema_json);

        let sections = BTreeMap::from([
            ("system_contract", json!(system_tokens)),
            ("task_instructions", json!(task_tokens)),
            ("input", json!(input_tokens)),
            ("output_schema", json!(schema_tokens)),
            (
                "estimated_total_tokens",
                json!(system_tokens + task_tokens + input_tokens + schema_tokens),
            ),
        ]);

        serde_json::to_value(sections).unwrap_or_else(|_| Value::Object(Default::default()))
    }
}

fn dedupe_non_empty_lines(lines: Vec<String>) -> Vec<String> {
    let mut seen = Vec::<String>::new();
    let mut result = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !seen.iter().any(|existing| existing == trimmed) {
            seen.push(trimmed.to_string());
            result.push(trimmed.to_string());
        }
    }
    result
}

fn dedupe_and_compact_task_instructions(lines: &mut Vec<String>) -> bool {
    let before = lines.clone();
    *lines = lines
        .iter()
        .map(|line| compact_whitespace(line))
        .filter(|line| !line.is_empty())
        .collect();
    *lines = dedupe_non_empty_lines(std::mem::take(lines));
    before != *lines
}

fn compact_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn compact_json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn estimate_tokens(input: &str) -> u32 {
    if input.is_empty() {
        return 0;
    }

    let mut ascii_chars = 0u32;
    let mut non_ascii_chars = 0u32;
    for ch in input.chars() {
        if ch.is_ascii() {
            ascii_chars = ascii_chars.saturating_add(1);
        } else {
            non_ascii_chars = non_ascii_chars.saturating_add(1);
        }
    }

    let ascii_tokens = (ascii_chars + 3) / 4;
    ascii_tokens.saturating_add(non_ascii_chars)
}

fn compute_prompt_hash(
    llm_node: &AgentLlmNode,
    prompt_version: &str,
    system_contract: &str,
    task_instructions: &[String],
    input: &Value,
    output_schema_id: Option<&str>,
    output_schema_json: Option<&Value>,
) -> String {
    let canonical = json!({
        "llm_node": llm_node,
        "prompt_version": prompt_version,
        "system_contract": system_contract,
        "task_instructions": task_instructions,
        "input": input,
        "output_schema_id": output_schema_id,
        "output_schema_json": output_schema_json,
    });

    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn remove_json_pointer(value: &mut Value, pointer: &str) -> bool {
    if pointer.is_empty() || pointer == "/" {
        return false;
    }

    let mut tokens = Vec::new();
    for raw in pointer.trim_start_matches('/').split('/') {
        tokens.push(raw.replace("~1", "/").replace("~0", "~"));
    }

    remove_pointer_tokens(value, &tokens)
}

fn remove_pointer_tokens(value: &mut Value, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return false;
    }

    if tokens.len() == 1 {
        let key = &tokens[0];
        return match value {
            Value::Object(map) => map.remove(key).is_some(),
            Value::Array(items) => key
                .parse::<usize>()
                .ok()
                .filter(|index| *index < items.len())
                .map(|index| {
                    items.remove(index);
                    true
                })
                .unwrap_or(false),
            _ => false,
        };
    }

    let head = &tokens[0];
    let tail = &tokens[1..];
    match value {
        Value::Object(map) => map
            .get_mut(head)
            .map(|child| remove_pointer_tokens(child, tail))
            .unwrap_or(false),
        Value::Array(items) => head
            .parse::<usize>()
            .ok()
            .and_then(|index| items.get_mut(index))
            .map(|child| remove_pointer_tokens(child, tail))
            .unwrap_or(false),
        _ => false,
    }
}

const SCENE_INITIALIZER_CONTRACT: &str = "\
You are SceneInitializer, the scene bootstrap node for Agent mode.\n\
Generate a candidate SceneInitializationDraft from structured seed, public context, location context, participant context, continuity context, private scene constraints, truth guidance, and generation policy.\n\
Only fill detail domains allowed by generation_policy.\n\
You may use private_scene_constraints only to preserve continuity and hidden consistency; do not expose them as public facts.\n\
Do not create named persistent entities, routes, secrets, or canonical truths that are not grounded in the input.\n\
Every inferred addition must be represented as an assumption with source, confidence, risk, and rationale.\n\
Output is a candidate draft only and never commits state.\n\
Follow these common rules:\n\
";

const SCENE_STATE_EXTRACTOR_CONTRACT: &str = "\
You are SceneStateExtractor, the structured parser for recent free text against the current scene.\n\
Classify the input into scene candidate changes, player roleplay intent, subjective player state, director bias, or session control.\n\
Preserve raw user text only in the designated raw_text field.\n\
You may read current_scene and private_scene_constraints for scene-domain consistency, but you may not access unrelated hidden history or global secrets.\n\
Emit only candidate deltas and conflict warnings; do not commit anything.\n\
Do not upgrade user authority or bypass validation, temporal canon, or access control.\n\
Follow these common rules:\n\
";

const CHARACTER_COGNITIVE_PASS_CONTRACT: &str = "\
You are CharacterCognitivePass, a single-character subjective reasoning node.\n\
Reason only from filtered_scene_view, embodiment_state, accessible_knowledge, recent_event_delta, and prior_subjective_state.\n\
Stay in the character's limited perspective. Hidden truth is unavailable unless it already appears in the provided accessible inputs.\n\
Belief, emotion, and intent changes must use the discrete schema fields instead of arbitrary numeric guesses.\n\
BodyReactionDelta is only a candidate outward reaction and does not directly change world truth.\n\
If the prior subjective state conflicts with new observations, express the tension instead of silently flattening it.\n\
Follow these common rules:\n\
";

const OUTCOME_PLANNER_CONTRACT: &str = "\
You are OutcomePlanner, the God-read orchestration node for candidate outcomes.\n\
You may synthesize from Layer 1 truth, character outputs, user roleplay intents, reaction windows, skills, relevant knowledge, and truth guidance.\n\
Your output is a candidate OutcomePlannerOutput only; final legality and commits are enforced by deterministic program validators.\n\
Separate hard state changes, soft effects, and blocked effects.\n\
Do not directly write lasting inner belief acceptance or other subjective state unless the schema explicitly references a validated subjective update source.\n\
Reaction windows must be resolved inside the current turn and must not expand into unbounded recursive chains.\n\
Follow these common rules:\n\
";

const SURFACE_REALIZER_CONTRACT: &str = "\
You are SurfaceRealizer, the final narrative rendering node.\n\
Render only from narration_scope, scene_view, character_views, outcome_plan, and style.\n\
Do not introduce new facts. Every concrete narrated fact must be grounded in outcome_plan.narratable_facts and reported through used_fact_ids.\n\
Respect narration_scope. Objective camera cannot enter private thoughts. Character-focused narration cannot claim facts outside that focus character's allowed perspective.\n\
Blocked effects may be narrated only as failed or prevented attempts, never as successful hard outcomes.\n\
Output must be SurfaceRealizerOutput JSON.\n\
Follow these common rules:\n\
";

const COMMON_RULES_CONTRACT: &str = "\
1. Use only facts explicitly present in the provided structured input.\n\
2. Respect the node permission boundary; hidden truth does not grant commit authority.\n\
3. Do not invent, rename, or guess ids, entities, skills, locations, or knowledge records.\n\
4. Output only schema-conformant JSON with no markdown, prose wrapper, or extra fields.\n\
5. Use llm_readable text only for explanation or narration, never as a hard rule or numeric basis.\n\
6. If information is insufficient, prefer conservative uncertainty over filling in new facts.\n\
7. Do not derive hard outcomes from raw physical or mana numbers unless the structured input already translated them into allowed abstractions.\n\
8. SceneInitializer may infer only within the allowed generation domains, and every inference must be recorded as an assumption.\n\
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_message_layout_with_structured_user_payload() {
        let builder = PromptBuilder::default();
        let bundle = builder
            .build_bundle(
                AgentLlmNode::SurfaceRealizer,
                &json!({ "foo": "bar" }),
                PromptBuildOptions {
                    task_instructions: vec!["Return terse prose.".to_string()],
                    output_schema_json: Some(json!({
                        "type": "object",
                        "properties": {
                            "narrative_text": { "type": "string" },
                            "used_fact_ids": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["narrative_text", "used_fact_ids"]
                    })),
                    ..Default::default()
                },
            )
            .expect("bundle should build");

        let messages = builder
            .build_messages(&bundle)
            .expect("messages should build");
        assert_eq!(messages.len(), 3);
        assert!(matches!(messages[0].role, ChatRole::System));
        assert!(matches!(messages[1].role, ChatRole::Developer));
        assert!(matches!(messages[2].role, ChatRole::User));

        let ContentPart::Text { text } = &messages[2].content[0] else {
            panic!("expected text content");
        };
        assert_eq!(text, r#"{"input":{"foo":"bar"}}"#);
    }

    #[test]
    fn prunes_optional_sections_when_budget_is_exceeded() {
        let builder = PromptBuilder::new(PromptBudgetSettings {
            critical_attention_tokens: 50,
            soft_input_tokens: 80,
            max_context_tokens: 128,
            reserved_output_tokens: 16,
        });

        let bundle = builder
            .build_bundle(
                AgentLlmNode::CharacterCognitivePass,
                &json!({
                    "required": "short",
                    "optional": "x".repeat(400),
                    "contextual": "y".repeat(200)
                }),
                PromptBuildOptions {
                    input_sections: vec![
                        PromptInputSection {
                            pointer: "/optional".to_string(),
                            label: "optional".to_string(),
                            priority: PromptPriority::P3OptionalFlavor,
                        },
                        PromptInputSection {
                            pointer: "/contextual".to_string(),
                            label: "contextual".to_string(),
                            priority: PromptPriority::P2Contextual,
                        },
                    ],
                    ..Default::default()
                },
            )
            .expect("bundle should build");

        assert!(bundle
            .budget_report
            .pruned_refs
            .iter()
            .any(|p| p == "/optional"));
        assert_eq!(
            bundle.input.get("required").and_then(Value::as_str),
            Some("short")
        );
        assert!(bundle.input.get("optional").is_none());
    }
}
