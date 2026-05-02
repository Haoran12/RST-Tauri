//! ST Regex extension system
//!
//! Regex 扩展兼容系统，支持 global/preset/scoped 三类脚本。
//! 参考: SillyTavern/public/scripts/extensions/regex/engine.js

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Regex 脚本作用点
pub struct RegexPlacement;

impl RegexPlacement {
    pub const USER_INPUT: i32 = 1;
    pub const AI_OUTPUT: i32 = 2;
    pub const SLASH_COMMAND: i32 = 3;
    pub const WORLD_INFO: i32 = 5;
    pub const REASONING: i32 = 6;
}

/// 宏替换策略
pub struct SubstituteRegex;

impl SubstituteRegex {
    pub const NONE: i32 = 0;
    pub const RAW: i32 = 1;
    pub const ESCAPED: i32 = 2;
}

/// Regex 脚本数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexScriptData {
    pub id: String,
    pub script_name: String,
    pub find_regex: String,
    pub replace_string: String,
    #[serde(default)]
    pub trim_strings: Vec<String>,
    #[serde(default)]
    pub placement: Vec<i32>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub markdown_only: bool,
    #[serde(default)]
    pub prompt_only: bool,
    #[serde(default)]
    pub run_on_edit: bool,
    #[serde(default)]
    pub substitute_regex: i32,
    #[serde(default)]
    pub min_depth: Option<i32>,
    #[serde(default)]
    pub max_depth: Option<i32>,
}

impl RegexScriptData {
    /// 创建新的脚本（带随机 ID）
    pub fn new(script_name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            script_name: script_name.to_string(),
            find_regex: String::new(),
            replace_string: String::new(),
            trim_strings: Vec::new(),
            placement: Vec::new(),
            disabled: false,
            markdown_only: false,
            prompt_only: false,
            run_on_edit: true,
            substitute_regex: SubstituteRegex::NONE,
            min_depth: None,
            max_depth: None,
        }
    }
}

/// Regex 扩展全局设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegexExtensionSettings {
    #[serde(default)]
    pub regex: Vec<RegexScriptData>,
    #[serde(default)]
    pub regex_presets: Vec<RegexPreset>,
    #[serde(default)]
    pub character_allowed_regex: Vec<String>,
    #[serde(default)]
    pub preset_allowed_regex: HashMap<String, Vec<String>>,
}

/// Regex Preset（启用脚本 ID 列表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_selected: bool,
    #[serde(default)]
    pub global: Vec<RegexPresetItem>,
    #[serde(default)]
    pub scoped: Vec<RegexPresetItem>,
    #[serde(default)]
    pub preset: Vec<RegexPresetItem>,
}

impl RegexPreset {
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            is_selected: false,
            global: Vec::new(),
            scoped: Vec::new(),
            preset: Vec::new(),
        }
    }
}

/// Regex Preset 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexPresetItem {
    pub id: String,
}

/// 运行时选项
#[derive(Debug, Clone, Default)]
pub struct RegexRunOptions {
    pub is_markdown: bool,
    pub is_prompt: bool,
    pub is_edit: bool,
    pub depth: Option<i32>,
    pub character_override: Option<String>,
    /// 当前预设 key（用于 preset 脚本授权）
    pub preset_key: Option<String>,
    /// 当前角色名（用于 scoped 脚本授权）
    pub character_name: Option<String>,
}

/// 脚本来源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptSource {
    Global,
    Preset,
    Scoped,
}

/// 带来源标记的脚本
#[derive(Debug, Clone)]
pub struct SourcedScript {
    pub script: RegexScriptData,
    pub source: ScriptSource,
}

/// Regex 执行引擎
pub struct RegexEngine {
    /// 正则缓存
    regex_cache: lru::LruCache<String, Regex>,
}

impl RegexEngine {
    /// 创建新的引擎
    pub fn new() -> Self {
        Self {
            regex_cache: lru::LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()),
        }
    }

    /// 对文本执行 Regex 替换
    ///
    /// `get_regexed_string` 的等价实现。
    /// 按照 global -> preset -> scoped 的顺序执行脚本。
    pub fn get_regexed_string(
        &mut self,
        raw: &str,
        placement: i32,
        settings: &RegexExtensionSettings,
        options: &RegexRunOptions,
    ) -> String {
        // 非字符串输入返回空字符串
        if raw.is_empty() {
            return String::new();
        }

        // 空字符串直接返回
        let mut text = raw.to_string();

        // 合并允许运行的脚本（按 global -> preset -> scoped 顺序）
        let scripts = self.get_allowed_scripts(settings, options);

        // 依次执行每个脚本
        for sourced_script in scripts {
            if self.should_run_script(&sourced_script.script, placement, options) {
                text = self.run_script(&sourced_script.script, &text, options);
            }
        }

        text
    }

    /// 获取允许运行的脚本列表（按顺序）
    ///
    /// 合并顺序：global -> preset -> scoped
    fn get_allowed_scripts(
        &self,
        settings: &RegexExtensionSettings,
        options: &RegexRunOptions,
    ) -> Vec<SourcedScript> {
        let mut result = Vec::new();

        // 1. Global 脚本：全部可见
        for script in &settings.regex {
            result.push(SourcedScript {
                script: script.clone(),
                source: ScriptSource::Global,
            });
        }

        // 2. Preset 脚本：需要通过 preset_allowed_regex
        // TODO: 从当前预设加载脚本
        // 当前预设名需要在 options.preset_key 中传递
        // if let Some(preset_key) = &options.preset_key {
        //     if let Some(allowed_ids) = settings.preset_allowed_regex.get(preset_key) {
        //         // 只添加在 allow list 中的脚本
        //     }
        // }

        // 3. Scoped 脚本：需要通过 character_allowed_regex
        // TODO: 从当前角色卡加载脚本
        // 当前角色名需要在 options.character_name 中传递
        // if let Some(character_name) = &options.character_name {
        //     if settings.character_allowed_regex.contains(character_name) {
        //         // 添加角色卡内嵌脚本
        //     }
        // }

        result
    }

    /// 判断脚本是否应该运行
    fn should_run_script(
        &self,
        script: &RegexScriptData,
        placement: i32,
        options: &RegexRunOptions,
    ) -> bool {
        // 禁用的脚本跳过
        if script.disabled {
            return false;
        }

        // find_regex 为空或无效跳过
        if script.find_regex.is_empty() {
            return false;
        }

        // placement 不包含当前作用点跳过
        if !script.placement.contains(&placement) {
            return false;
        }

        // 编辑模式下检查 run_on_edit
        if options.is_edit && !script.run_on_edit {
            return false;
        }

        // 深度过滤
        if let Some(depth) = options.depth {
            if let Some(min_depth) = script.min_depth {
                if depth < min_depth {
                    return false;
                }
            }
            if let Some(max_depth) = script.max_depth {
                if depth > max_depth {
                    return false;
                }
            }
        }

        // markdownOnly 只在 is_markdown 时运行
        if script.markdown_only && !options.is_markdown {
            return false;
        }

        // promptOnly 只在 is_prompt 时运行
        if script.prompt_only && !options.is_prompt {
            return false;
        }

        // 两者都为 false 时，只在非 markdown、非 prompt 的源文本阶段运行
        if !script.markdown_only && !script.prompt_only {
            if options.is_markdown || options.is_prompt {
                return false;
            }
        }

        true
    }

    /// 执行单个脚本
    fn run_script(&mut self, script: &RegexScriptData, text: &str, _options: &RegexRunOptions) -> String {
        // 编译正则
        let regex = match self.compile_regex(&script.find_regex, script.substitute_regex) {
            Some(re) => re,
            None => return text.to_string(),
        };

        // 执行替换
        let result = regex.replace_all(text, &script.replace_string);

        // TODO: 处理 trim_strings
        // TODO: 处理宏替换

        result.to_string()
    }

    /// 编译正则表达式
    fn compile_regex(&mut self, pattern: &str, _substitute: i32) -> Option<Regex> {
        // 尝试从缓存获取
        if let Some(re) = self.regex_cache.get(pattern) {
            return Some(re.clone());
        }

        // 编译正则
        match Regex::new(pattern) {
            Ok(re) => {
                self.regex_cache.put(pattern.to_string(), re.clone());
                Some(re)
            }
            Err(_) => None,
        }
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.regex_cache.clear();
    }
}

impl Default for RegexEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_replace() {
        let mut engine = RegexEngine::new();
        let script = RegexScriptData {
            id: "test".to_string(),
            script_name: "test".to_string(),
            find_regex: r"hello".to_string(),
            replace_string: "hi".to_string(),
            placement: vec![RegexPlacement::USER_INPUT],
            disabled: false,
            markdown_only: false,
            prompt_only: false,
            run_on_edit: true,
            substitute_regex: SubstituteRegex::NONE,
            min_depth: None,
            max_depth: None,
            trim_strings: Vec::new(),
        };

        let settings = RegexExtensionSettings {
            regex: vec![script],
            ..Default::default()
        };

        let options = RegexRunOptions::default();

        let result = engine.get_regexed_string(
            "hello world",
            RegexPlacement::USER_INPUT,
            &settings,
            &options,
        );

        assert_eq!(result, "hi world");
    }

    #[test]
    fn test_placement_filter() {
        let mut engine = RegexEngine::new();
        let script = RegexScriptData {
            id: "test".to_string(),
            script_name: "test".to_string(),
            find_regex: r"hello".to_string(),
            replace_string: "hi".to_string(),
            placement: vec![RegexPlacement::USER_INPUT],
            disabled: false,
            markdown_only: false,
            prompt_only: false,
            run_on_edit: true,
            substitute_regex: SubstituteRegex::NONE,
            min_depth: None,
            max_depth: None,
            trim_strings: Vec::new(),
        };

        let settings = RegexExtensionSettings {
            regex: vec![script],
            ..Default::default()
        };

        let options = RegexRunOptions::default();

        // AI_OUTPUT 不匹配 USER_INPUT placement
        let result = engine.get_regexed_string(
            "hello world",
            RegexPlacement::AI_OUTPUT,
            &settings,
            &options,
        );

        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_disabled_script() {
        let mut engine = RegexEngine::new();
        let script = RegexScriptData {
            id: "test".to_string(),
            script_name: "test".to_string(),
            find_regex: r"hello".to_string(),
            replace_string: "hi".to_string(),
            placement: vec![RegexPlacement::USER_INPUT],
            disabled: true,
            markdown_only: false,
            prompt_only: false,
            run_on_edit: true,
            substitute_regex: SubstituteRegex::NONE,
            min_depth: None,
            max_depth: None,
            trim_strings: Vec::new(),
        };

        let settings = RegexExtensionSettings {
            regex: vec![script],
            ..Default::default()
        };

        let options = RegexRunOptions::default();

        let result = engine.get_regexed_string(
            "hello world",
            RegexPlacement::USER_INPUT,
            &settings,
            &options,
        );

        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_depth_filter() {
        let mut engine = RegexEngine::new();
        let script = RegexScriptData {
            id: "test".to_string(),
            script_name: "test".to_string(),
            find_regex: r"hello".to_string(),
            replace_string: "hi".to_string(),
            placement: vec![RegexPlacement::USER_INPUT],
            disabled: false,
            markdown_only: false,
            prompt_only: false,
            run_on_edit: true,
            substitute_regex: SubstituteRegex::NONE,
            min_depth: Some(2),
            max_depth: Some(5),
            trim_strings: Vec::new(),
        };

        let settings = RegexExtensionSettings {
            regex: vec![script],
            ..Default::default()
        };

        // depth = 3，在 [2, 5] 范围内
        let options = RegexRunOptions {
            depth: Some(3),
            ..Default::default()
        };

        let result = engine.get_regexed_string(
            "hello world",
            RegexPlacement::USER_INPUT,
            &settings,
            &options,
        );

        assert_eq!(result, "hi world");

        // depth = 1，不在 [2, 5] 范围内
        let options = RegexRunOptions {
            depth: Some(1),
            ..Default::default()
        };

        let result = engine.get_regexed_string(
            "hello world",
            RegexPlacement::USER_INPUT,
            &settings,
            &options,
        );

        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_prompt_only() {
        let mut engine = RegexEngine::new();
        let script = RegexScriptData {
            id: "test".to_string(),
            script_name: "test".to_string(),
            find_regex: r"hello".to_string(),
            replace_string: "hi".to_string(),
            placement: vec![RegexPlacement::USER_INPUT],
            disabled: false,
            markdown_only: false,
            prompt_only: true,
            run_on_edit: true,
            substitute_regex: SubstituteRegex::NONE,
            min_depth: None,
            max_depth: None,
            trim_strings: Vec::new(),
        };

        let settings = RegexExtensionSettings {
            regex: vec![script],
            ..Default::default()
        };

        // is_prompt = true 时运行
        let options = RegexRunOptions {
            is_prompt: true,
            ..Default::default()
        };

        let result = engine.get_regexed_string(
            "hello world",
            RegexPlacement::USER_INPUT,
            &settings,
            &options,
        );

        assert_eq!(result, "hi world");

        // is_prompt = false 时不运行
        let options = RegexRunOptions::default();

        let result = engine.get_regexed_string(
            "hello world",
            RegexPlacement::USER_INPUT,
            &settings,
            &options,
        );

        assert_eq!(result, "hello world");
    }
}
