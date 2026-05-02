//! WorldBook keyword matching system
//!
//! 关键词匹配系统，支持基础匹配、正则匹配和匹配目标扩展。
//! 参考: SillyTavern/public/scripts/world-info.js

use regex::Regex;
use crate::storage::st_resources::{WorldInfoEntry, WorldInfoLogic};

/// 关键词匹配结果
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub entry_uid: i32,
    pub matched_keys: Vec<String>,
    pub matched_secondary: Vec<String>,
}

/// 全局扫描数据
///
/// 包含角色卡各部分文本，用于 match_* 扩展匹配目标。
#[derive(Debug, Clone, Default)]
pub struct GlobalScanData {
    pub persona_description: String,
    pub character_description: String,
    pub character_personality: String,
    pub character_depth_prompt: String,
    pub scenario: String,
    pub creator_notes: String,
    pub trigger: Option<String>,
}

/// 匹配上下文
#[derive(Debug, Clone)]
pub struct MatchContext<'a> {
    /// 扫描文本（聊天历史）
    pub scan_text: &'a str,
    /// 全局扫描数据
    pub global_scan_data: &'a GlobalScanData,
    /// 全局大小写敏感设置
    pub global_case_sensitive: bool,
    /// 全局全词匹配设置
    pub global_match_whole_words: bool,
    /// 全局扫描深度
    pub global_scan_depth: i32,
}

/// 关键词匹配器
pub struct KeywordMatcher {
    /// 正则缓存
    regex_cache: lru::LruCache<String, Regex>,
}

impl KeywordMatcher {
    /// 创建新的匹配器
    pub fn new() -> Self {
        Self {
            regex_cache: lru::LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()),
        }
    }

    /// 检查单个词条是否匹配
    ///
    /// 返回匹配结果，如果不匹配则返回 None。
    pub fn match_entry(
        &mut self,
        entry: &WorldInfoEntry,
        context: &MatchContext,
    ) -> Option<MatchResult> {
        // 跳过禁用的词条
        if entry.disable {
            return None;
        }

        // 检查触发器
        if !entry.triggers.is_empty() {
            if let Some(ref trigger) = context.global_scan_data.trigger {
                if !entry.triggers.iter().any(|t| t == trigger) {
                    return None;
                }
            } else {
                return None;
            }
        }

        // 获取匹配设置
        let case_sensitive = entry.case_sensitive.unwrap_or(context.global_case_sensitive);
        let match_whole_words = entry.match_whole_words.unwrap_or(context.global_match_whole_words);

        // 匹配主关键词
        // 来源可以是：扫描文本、或启用的扩展目标
        let matched_keys = self.match_keys_with_extensions(
            &entry.key,
            context.scan_text,
            case_sensitive,
            match_whole_words,
            entry,
            context.global_scan_data,
        )?;

        // 如果启用 selective，检查次关键词逻辑
        let matched_secondary = if entry.selective && !entry.keysecondary.is_empty() {
            let secondary_matches = self.match_keys(
                &entry.keysecondary,
                context.scan_text,
                case_sensitive,
                match_whole_words,
            );

            // 应用 selective_logic
            let logic = entry.selective_logic;
            let has_secondary = secondary_matches.is_some();

            match logic {
                x if x == WorldInfoLogic::AND_ANY => {
                    // 需要至少一个次关键词匹配
                    if !has_secondary {
                        return None;
                    }
                    secondary_matches.unwrap_or_default()
                }
                x if x == WorldInfoLogic::NOT_ALL => {
                    // 需要不是所有次关键词都匹配
                    // 即：如果所有次关键词都匹配，则失败
                    if has_secondary && secondary_matches.as_ref().map(|m| m.len()).unwrap_or(0) == entry.keysecondary.len() {
                        return None;
                    }
                    secondary_matches.unwrap_or_default()
                }
                x if x == WorldInfoLogic::NOT_ANY => {
                    // 需要没有任何次关键词匹配
                    if has_secondary {
                        return None;
                    }
                    Vec::new()
                }
                x if x == WorldInfoLogic::AND_ALL => {
                    // 需要所有次关键词都匹配
                    if !has_secondary || secondary_matches.as_ref().map(|m| m.len()).unwrap_or(0) != entry.keysecondary.len() {
                        return None;
                    }
                    secondary_matches.unwrap_or_default()
                }
                _ => {
                    // 默认 AND_ANY
                    if !has_secondary {
                        return None;
                    }
                    secondary_matches.unwrap_or_default()
                }
            }
        } else {
            Vec::new()
        };

        Some(MatchResult {
            entry_uid: entry.uid,
            matched_keys,
            matched_secondary,
        })
    }

    /// 匹配关键词（含扩展目标）
    ///
    /// 关键词可以在扫描文本或启用的扩展目标中匹配。
    fn match_keys_with_extensions(
        &mut self,
        keys: &[String],
        scan_text: &str,
        case_sensitive: bool,
        match_whole_words: bool,
        entry: &WorldInfoEntry,
        global_scan_data: &GlobalScanData,
    ) -> Option<Vec<String>> {
        if keys.is_empty() {
            return None;
        }

        // 首先在扫描文本中匹配
        if let Some(matched) = self.match_keys(keys, scan_text, case_sensitive, match_whole_words) {
            return Some(matched);
        }

        // 如果扫描文本没有匹配，检查扩展目标
        let mut matched = Vec::new();

        for key in keys {
            if key.is_empty() {
                continue;
            }

            let mut found = false;

            // 检查各扩展目标
            if entry.match_persona_description && !global_scan_data.persona_description.is_empty() {
                if self.match_single_key(key, &global_scan_data.persona_description, case_sensitive) {
                    found = true;
                }
            }

            if !found && entry.match_character_description && !global_scan_data.character_description.is_empty() {
                if self.match_single_key(key, &global_scan_data.character_description, case_sensitive) {
                    found = true;
                }
            }

            if !found && entry.match_character_personality && !global_scan_data.character_personality.is_empty() {
                if self.match_single_key(key, &global_scan_data.character_personality, case_sensitive) {
                    found = true;
                }
            }

            if !found && entry.match_character_depth_prompt && !global_scan_data.character_depth_prompt.is_empty() {
                if self.match_single_key(key, &global_scan_data.character_depth_prompt, case_sensitive) {
                    found = true;
                }
            }

            if !found && entry.match_scenario && !global_scan_data.scenario.is_empty() {
                if self.match_single_key(key, &global_scan_data.scenario, case_sensitive) {
                    found = true;
                }
            }

            if !found && entry.match_creator_notes && !global_scan_data.creator_notes.is_empty() {
                if self.match_single_key(key, &global_scan_data.creator_notes, case_sensitive) {
                    found = true;
                }
            }

            if found {
                matched.push(key.clone());
            }
        }

        if matched.is_empty() {
            None
        } else {
            Some(matched)
        }
    }

    /// 匹配单个关键词
    fn match_single_key(&mut self, key: &str, text: &str, case_sensitive: bool) -> bool {
        if key.is_empty() || text.is_empty() {
            return false;
        }

        // 检查是否是正则表达式
        if key.starts_with('/') {
            self.match_regex(key, text, case_sensitive).is_some()
        } else {
            let search_text = if case_sensitive {
                text
            } else {
                &text.to_lowercase()
            };

            let search_key = if case_sensitive {
                key
            } else {
                &key.to_lowercase()
            };

            search_text.contains(search_key)
        }
    }

    /// 匹配关键词列表
    ///
    /// 返回匹配到的关键词列表，如果没有匹配则返回 None。
    fn match_keys(
        &mut self,
        keys: &[String],
        text: &str,
        case_sensitive: bool,
        match_whole_words: bool,
    ) -> Option<Vec<String>> {
        if keys.is_empty() {
            return None;
        }

        let text_lower = if case_sensitive {
            None
        } else {
            Some(text.to_lowercase())
        };

        let mut matched = Vec::new();

        for key in keys {
            if key.is_empty() {
                continue;
            }

            // 检查是否是正则表达式（以 / 开头）
            if key.starts_with('/') {
                if let Some(regex_matched) = self.match_regex(key, text, case_sensitive) {
                    matched.push(regex_matched);
                }
            } else {
                // 普通关键词匹配
                let search_text = if case_sensitive {
                    text
                } else {
                    text_lower.as_ref().unwrap()
                };

                let search_key = if case_sensitive {
                    key.as_str()
                } else {
                    &key.to_lowercase()
                };

                let found = if match_whole_words {
                    self.match_whole_word(search_text, search_key)
                } else {
                    search_text.contains(search_key)
                };

                if found {
                    matched.push(key.clone());
                }
            }
        }

        if matched.is_empty() {
            None
        } else {
            Some(matched)
        }
    }

    /// 匹配正则表达式
    fn match_regex(
        &mut self,
        pattern: &str,
        text: &str,
        case_sensitive: bool,
    ) -> Option<String> {
        // 解析 /pattern/flags 格式
        let (regex_pattern, flags) = self.parse_regex_pattern(pattern, case_sensitive);

        // 构建缓存 key
        let cache_key = format!("{}|{}", regex_pattern, flags);

        // 尝试从缓存获取或编译正则
        let regex = if let Some(re) = self.regex_cache.get(&cache_key) {
            re.clone()
        } else {
            match Regex::new(&regex_pattern) {
                Ok(re) => {
                    self.regex_cache.put(cache_key.clone(), re.clone());
                    re
                }
                Err(_) => return None,
            }
        };

        // 执行匹配
        if regex.is_match(text) {
            Some(pattern.to_string())
        } else {
            None
        }
    }

    /// 解析正则表达式模式
    ///
    /// 支持 /pattern/flags 格式，也支持普通 pattern。
    fn parse_regex_pattern<'a>(&self, pattern: &'a str, case_sensitive: bool) -> (String, String) {
        // 检查是否是 /pattern/flags 格式
        if pattern.starts_with('/') {
            // 找到结束的 /
            if let Some(end) = pattern.rfind('/') {
                if end > 1 {
                    let inner = &pattern[1..end];
                    let flags = &pattern[end + 1..];

                    // 处理 flags
                    let mut final_flags = String::new();
                    let mut has_i = false;

                    for c in flags.chars() {
                        match c {
                            'i' => has_i = true,
                            'm' => final_flags.push('m'),
                            's' => final_flags.push('s'),
                            _ => {}
                        }
                    }

                    // 如果全局大小写敏感且没有 i flag，则不添加 (?i)
                    let final_pattern = if !case_sensitive && !has_i {
                        format!("(?i){}", inner)
                    } else if has_i {
                        format!("(?i){}", inner)
                    } else {
                        inner.to_string()
                    };

                    return (final_pattern, final_flags);
                }
            }
        }

        // 普通 pattern
        let final_pattern = if !case_sensitive {
            format!("(?i){}", pattern)
        } else {
            pattern.to_string()
        };

        (final_pattern, String::new())
    }

    /// 全词匹配
    fn match_whole_word(&self, text: &str, word: &str) -> bool {
        // 使用单词边界匹配
        let pattern = format!(r"\b{}\b", regex::escape(word));
        match Regex::new(&pattern) {
            Ok(re) => re.is_match(text),
            Err(_) => text.contains(word),
        }
    }
}

impl Default for KeywordMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_keyword_match() {
        let mut matcher = KeywordMatcher::new();
        let entry = WorldInfoEntry::new(1);
        let mut entry = entry;
        entry.key = vec!["hello".to_string(), "world".to_string()];

        let global_scan_data = GlobalScanData::default();
        let context = MatchContext {
            scan_text: "hello there, world!",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!(result.matched_keys.len(), 2);
    }

    #[test]
    fn test_case_sensitive_match() {
        let mut matcher = KeywordMatcher::new();
        let entry = WorldInfoEntry::new(1);
        let mut entry = entry;
        entry.key = vec!["Hello".to_string()];
        entry.case_sensitive = Some(true);

        let global_scan_data = GlobalScanData::default();
        let context = MatchContext {
            scan_text: "hello there",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_none());

        let context = MatchContext {
            scan_text: "Hello there",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_some());
    }

    #[test]
    fn test_whole_word_match() {
        let mut matcher = KeywordMatcher::new();
        let entry = WorldInfoEntry::new(1);
        let mut entry = entry;
        entry.key = vec!["cat".to_string()];
        entry.match_whole_words = Some(true);

        let global_scan_data = GlobalScanData::default();
        let context = MatchContext {
            scan_text: "the cat sat on the mat",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_some());

        let context = MatchContext {
            scan_text: "the category is wrong",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_none());
    }

    #[test]
    fn test_regex_match() {
        let mut matcher = KeywordMatcher::new();
        let entry = WorldInfoEntry::new(1);
        let mut entry = entry;
        entry.key = vec!["/\\d{3}-\\d{4}/".to_string()];

        let global_scan_data = GlobalScanData::default();
        let context = MatchContext {
            scan_text: "call 123-4567 for help",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_some());
    }

    #[test]
    fn test_selective_logic_and_any() {
        let mut matcher = KeywordMatcher::new();
        let entry = WorldInfoEntry::new(1);
        let mut entry = entry;
        entry.key = vec!["main".to_string()];
        entry.keysecondary = vec!["secondary".to_string(), "other".to_string()];
        entry.selective = true;
        entry.selective_logic = WorldInfoLogic::AND_ANY;

        let global_scan_data = GlobalScanData::default();
        let context = MatchContext {
            scan_text: "main keyword with secondary",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_some());

        let context = MatchContext {
            scan_text: "main keyword alone",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_none());
    }

    #[test]
    fn test_extended_target_match() {
        let mut matcher = KeywordMatcher::new();
        let entry = WorldInfoEntry::new(1);
        let mut entry = entry;
        entry.key = vec!["magic".to_string()];
        entry.match_scenario = true;

        let global_scan_data = GlobalScanData {
            scenario: "A world of magic and wonder".to_string(),
            ..Default::default()
        };

        let context = MatchContext {
            scan_text: "hello there",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        // 即使扫描文本不匹配，scenario 中有 magic 也可以
        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_some());
    }

    #[test]
    fn test_trigger_match() {
        let mut matcher = KeywordMatcher::new();
        let entry = WorldInfoEntry::new(1);
        let mut entry = entry;
        entry.key = vec!["hello".to_string()];
        entry.triggers = vec!["greeting".to_string(), "welcome".to_string()];

        let global_scan_data = GlobalScanData {
            trigger: Some("greeting".to_string()),
            ..Default::default()
        };

        let context = MatchContext {
            scan_text: "hello there",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_some());

        // 不匹配的 trigger
        let global_scan_data = GlobalScanData {
            trigger: Some("farewell".to_string()),
            ..Default::default()
        };

        let context = MatchContext {
            scan_text: "hello there",
            global_scan_data: &global_scan_data,
            global_case_sensitive: false,
            global_match_whole_words: false,
            global_scan_depth: 4,
        };

        let result = matcher.match_entry(&entry, &context);
        assert!(result.is_none());
    }
}
