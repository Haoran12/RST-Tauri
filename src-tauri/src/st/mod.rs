//! ST (SillyTavern) mode modules
//!
//! 包含角色卡解析、导出、世界书转换、关键词匹配、Regex 扩展、预设系统、运行时组装等功能。

pub mod character;
pub mod keyword_matcher;
pub mod macros;
pub mod preset;
pub mod regex_engine;
pub mod runtime_assembly;
pub mod world_info_injection;
pub mod worldbook;

pub use character::{
    export_character_to_json, export_character_to_png, parse_character_from_json,
    parse_character_from_png,
};
pub use keyword_matcher::{GlobalScanData, KeywordMatcher, MatchContext, MatchResult};
pub use macros::{substitute_params, MacroContext};
pub use preset::{
    AutoSelectConfig, ContextTemplate, InstructTemplate, PresetFile, PresetType,
    PromptPreset, ReasoningTemplate, SamplerPreset, SystemPrompt,
};
pub use regex_engine::{
    RegexEngine, RegexExtensionSettings, RegexPlacement, RegexPreset, RegexRunOptions,
    RegexScriptData, SubstituteRegex,
};
pub use runtime_assembly::{
    AssembledMessage, AssembledReasoningParams, AssembledRequest, AssembledSamplingParams,
    CharLoreBinding, GlobalAppState, ProviderRequestMapper, RequestAssembler, RuntimeContext,
    STChatMessage, STChatMetadata, STSessionData, STWorldInfoSettings, WorldInfoInjectionResult,
};
pub use world_info_injection::{WorldInfoInjector, WorldInfoManager, WorldInfoSource};
pub use worldbook::convert_character_book;
