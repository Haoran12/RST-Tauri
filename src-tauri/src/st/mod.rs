//! ST (SillyTavern) mode modules
//!
//! 包含角色卡解析、导出、世界书转换、关键词匹配、Regex 扩展、预设系统、运行时组装等功能。

pub mod character;
pub mod keyword_matcher;
pub mod preset;
pub mod regex_engine;
pub mod runtime_assembly;
pub mod world_info_injection;
pub mod worldbook;

pub use character::{parse_character_from_png, parse_character_from_json, export_character_to_png, export_character_to_json};
pub use keyword_matcher::{KeywordMatcher, MatchContext, MatchResult, GlobalScanData};
pub use preset::{
    SamplerPreset, InstructTemplate, ContextTemplate, SystemPrompt,
    ReasoningTemplate, PromptPreset, PresetType, AutoSelectConfig,
};
pub use regex_engine::{RegexEngine, RegexExtensionSettings, RegexScriptData, RegexPreset, RegexRunOptions, RegexPlacement, SubstituteRegex};
pub use runtime_assembly::{
    GlobalAppState, STWorldInfoSettings, STSessionData, STChatMetadata, STChatMessage,
    RuntimeContext, WorldInfoInjectionResult,
    RequestAssembler, AssembledRequest, AssembledMessage, AssembledSamplingParams,
    AssembledReasoningParams, ProviderRequestMapper, CharLoreBinding,
};
pub use world_info_injection::{WorldInfoInjector, WorldInfoSource, WorldInfoManager};
pub use worldbook::convert_character_book;
