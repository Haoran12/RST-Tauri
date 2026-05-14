//! ST World Info Injection
//!
//! 世界书从来源合并到 Prompt 落槽的完整运行时流程。
//! 参考: docs/73_st_worldbook_injection.md
//! 实现依据: SillyTavern public/scripts/world-info.js

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::st::keyword_matcher::{GlobalScanData, KeywordMatcher, MatchContext};
use crate::st::macros::{substitute_params, MacroContext};
use crate::st::runtime_assembly::{STChatMessage, STWorldInfoSettings, WorldInfoInjectionResult};
use crate::storage::st_resources::{WorldInfoEntry, WorldInfoFile, WorldInfoPosition};

// ============================================================================
// 世界书来源类型
// ============================================================================

/// 世界书来源
#[derive(Debug, Clone)]
pub enum WorldInfoSource {
    /// Chat lore - 当前聊天绑定的世界书
    ChatLore(WorldInfoFile),
    /// Persona lore - 用户 Persona 的世界书
    PersonaLore(WorldInfoFile),
    /// Global lore - 全局选择的世界书
    GlobalLore(WorldInfoFile),
    /// Character lore - 角色卡绑定的世界书
    CharacterLore(WorldInfoFile),
}

impl WorldInfoSource {
    /// 获取来源优先级（用于排序）
    pub fn priority(&self) -> i32 {
        match self {
            WorldInfoSource::ChatLore(_) => 0,
            WorldInfoSource::PersonaLore(_) => 1,
            WorldInfoSource::CharacterLore(_) => 2,
            WorldInfoSource::GlobalLore(_) => 3,
        }
    }

    /// 获取世界书文件
    pub fn file(&self) -> &WorldInfoFile {
        match self {
            WorldInfoSource::ChatLore(f) => f,
            WorldInfoSource::PersonaLore(f) => f,
            WorldInfoSource::GlobalLore(f) => f,
            WorldInfoSource::CharacterLore(f) => f,
        }
    }

    /// 获取来源名称
    pub fn source_name(&self) -> &'static str {
        match self {
            WorldInfoSource::ChatLore(_) => "chat",
            WorldInfoSource::PersonaLore(_) => "persona",
            WorldInfoSource::GlobalLore(_) => "global",
            WorldInfoSource::CharacterLore(_) => "character",
        }
    }
}

// ============================================================================
// 世界书注入器
// ============================================================================

/// 世界书注入器
///
/// 负责世界书来源合并、排序、扫描、递归、预算裁剪和 Prompt 落槽。
pub struct WorldInfoInjector {
    /// 关键词匹配器
    matcher: KeywordMatcher,
    /// 世界书缓存
    worldbook_cache: Arc<RwLock<HashMap<String, WorldInfoFile>>>,
}

#[derive(Debug, Clone)]
struct SortedEntry {
    entry: WorldInfoEntry,
    source_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ActivationKey {
    source_key: String,
    uid: i32,
}

#[derive(Debug, Clone)]
struct ScanCandidate {
    entry: WorldInfoEntry,
    activation_key: ActivationKey,
}

impl WorldInfoInjector {
    pub fn new() -> Self {
        Self {
            matcher: KeywordMatcher::new(),
            worldbook_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 执行世界书注入
    ///
    /// # 参数
    /// - `chat_for_wi`: 用于世界书扫描的聊天文本（已反转顺序）
    /// - `max_context`: 最大上下文 token 数
    /// - `settings`: 世界书设置
    /// - `sources`: 世界书来源列表
    /// - `global_scan_data`: 全局扫描数据
    pub async fn check_world_info(
        &mut self,
        chat_for_wi: &[STChatMessage],
        max_context: i32,
        settings: &STWorldInfoSettings,
        sources: Vec<WorldInfoSource>,
        global_scan_data: &GlobalScanData,
        macro_context: &MacroContext,
    ) -> WorldInfoInjectionResult {
        // 1. 合并并排序所有来源的词条
        let sorted_entries = self.get_sorted_entries(sources, settings, macro_context).await;

        // 2. 计算 token 预算
        let budget = self.calculate_budget(max_context, settings);

        // 3. 构造扫描文本
        let scan_text = self.build_scan_text(chat_for_wi, settings, macro_context);

        // 4. 执行扫描
        let scan_result = self
            .scan_entries(
                &sorted_entries,
                &scan_text,
                budget,
                settings,
                global_scan_data,
            )
            .await;

        // 5. 按 position 分流落槽
        let mut result = self.distribute_to_positions(&scan_result.activated_entries);
        result.tokens_used = scan_result.used_budget;
        result
    }

    /// 合并并排序所有来源的词条
    async fn get_sorted_entries(
        &self,
        sources: Vec<WorldInfoSource>,
        settings: &STWorldInfoSettings,
        macro_context: &MacroContext,
    ) -> Vec<SortedEntry> {
        let mut all_entries: Vec<(SortedEntry, i32)> = Vec::new();
        let mut seen_worlds: HashSet<String> = HashSet::new();

        // 按 priority 排序来源
        let mut sorted_sources = sources;
        sorted_sources.sort_by_key(|s| s.priority());

        for source in sorted_sources {
            let world_name = source.file().name.clone();
            let source_key = source
                .file()
                .rst_lore_id
                .clone()
                .unwrap_or_else(|| format!("{}:{}", source.source_name(), world_name));

            // 去重：如果同一世界书名称已在更高优先级来源中启用，跳过
            if seen_worlds.contains(&world_name) {
                continue;
            }
            seen_worlds.insert(world_name.clone());

            // 提取词条并添加来源标记
            for (_, entry) in &source.file().entries {
                let mut entry = entry.clone();
                entry.content = substitute_params(entry.content.as_str(), macro_context);
                entry.key = entry
                    .key
                    .iter()
                    .map(|key| substitute_params(key, macro_context))
                    .collect();
                entry.keysecondary = entry
                    .keysecondary
                    .iter()
                    .map(|key| substitute_params(key, macro_context))
                    .collect();
                all_entries.push((
                    SortedEntry {
                        entry,
                        source_key: source_key.clone(),
                    },
                    source.priority(),
                ));
            }
        }

        // 根据 world_info_character_strategy 排序
        // 0 = evenly: 全部合并后按 order 降序
        // 1 = character_first: Character 内部按 order 降序，然后 Global
        // 2 = global_first: Global 内部按 order 降序，然后 Character
        match settings.world_info_character_strategy {
            0 => {
                // evenly: 全部合并后按 order 降序
                all_entries.sort_by(|a, b| b.0.entry.order.cmp(&a.0.entry.order));
            }
            1 => {
                // character_first: Character 优先，然后 Global
                all_entries.sort_by(|a, b| match a.1.cmp(&b.1) {
                    std::cmp::Ordering::Equal => b.0.entry.order.cmp(&a.0.entry.order),
                    other => other,
                });
            }
            2 => {
                // global_first: Global 优先，然后 Character
                all_entries.sort_by(|a, b| match a.1.cmp(&b.1).reverse() {
                    std::cmp::Ordering::Equal => b.0.entry.order.cmp(&a.0.entry.order),
                    other => other,
                });
            }
            _ => {}
        }

        all_entries.into_iter().map(|(entry, _)| entry).collect()
    }

    /// 计算 token 预算
    fn calculate_budget(&self, max_context: i32, settings: &STWorldInfoSettings) -> i32 {
        let budget = (settings.world_info_budget as f64 * max_context as f64 / 100.0) as i32;
        if settings.world_info_budget_cap > 0 {
            budget.min(settings.world_info_budget_cap)
        } else {
            budget
        }
    }

    /// 构造扫描文本
    fn build_scan_text(
        &self,
        chat: &[STChatMessage],
        settings: &STWorldInfoSettings,
        macro_context: &MacroContext,
    ) -> Vec<String> {
        // 反转聊天顺序（最近消息在前）
        let mut scan_text: Vec<String> = Vec::new();

        for msg in chat.iter().rev() {
            let text = if settings.world_info_include_names {
                let name = macro_context.message_speaker_name(msg);
                if !name.is_empty() {
                    format!("{}: {}", name, msg.content)
                } else {
                    msg.content.clone()
                }
            } else {
                msg.content.clone()
            };
            scan_text.push(text);
        }

        scan_text
    }

    /// 扫描词条
    async fn scan_entries(
        &mut self,
        entries: &[SortedEntry],
        scan_text: &[String],
        budget: i32,
        settings: &STWorldInfoSettings,
        global_scan_data: &GlobalScanData,
    ) -> ScanResult {
        let mut result = ScanResult::default();
        let mut used_budget = 0i32;
        let mut activated_keys: HashSet<ActivationKey> = HashSet::new();
        let mut current_scan_text = scan_text.to_vec();
        let max_steps = if settings.world_info_recursive {
            settings.world_info_max_recursion_steps.max(0)
        } else {
            0
        };

        for recursion_depth in 0..=max_steps {
            let candidates = self.scan_pass(
                entries,
                &current_scan_text,
                recursion_depth,
                settings,
                global_scan_data,
                &activated_keys,
            );
            let candidates = self.apply_group_pruning(candidates);

            let mut recursion_additions = Vec::new();
            let mut added_this_pass = 0usize;
            for candidate in candidates {
                if activated_keys.contains(&candidate.activation_key) {
                    continue;
                }
                if !self.passes_probability(&candidate.entry, &current_scan_text, recursion_depth) {
                    continue;
                }

                let entry_tokens = self.estimate_tokens(&candidate.entry.content);
                if !candidate.entry.ignore_budget && used_budget + entry_tokens > budget {
                    continue;
                }

                used_budget += entry_tokens;
                activated_keys.insert(candidate.activation_key);
                if settings.world_info_recursive
                    && !candidate.entry.prevent_recursion
                    && !candidate.entry.content.is_empty()
                {
                    recursion_additions.push(candidate.entry.content.clone());
                }
                result.activated_entries.push(candidate.entry);
                added_this_pass += 1;
            }

            if !settings.world_info_recursive
                || recursion_depth >= max_steps
                || recursion_additions.is_empty()
            {
                if result.activated_entries.len() as i32 >= settings.world_info_min_activations
                    || added_this_pass == 0
                {
                    break;
                }
            }

            // ST uses newly activated content to extend the next recursive scan.
            for content in recursion_additions.into_iter().rev() {
                current_scan_text.insert(0, content);
            }
        }

        result.used_budget = used_budget;
        result
    }

    fn scan_pass(
        &mut self,
        entries: &[SortedEntry],
        scan_text: &[String],
        recursion_depth: i32,
        settings: &STWorldInfoSettings,
        global_scan_data: &GlobalScanData,
        activated_keys: &HashSet<ActivationKey>,
    ) -> Vec<ScanCandidate> {
        let mut candidates = Vec::new();

        for sorted_entry in entries {
            let entry = &sorted_entry.entry;
            let activation_key = ActivationKey {
                source_key: sorted_entry.source_key.clone(),
                uid: entry.uid,
            };

            if entry.disable || activated_keys.contains(&activation_key) {
                continue;
            }
            if !Self::passes_recursion_gate(entry, recursion_depth) {
                continue;
            }
            if recursion_depth == 0 && !Self::passes_initial_delay(entry, scan_text.len()) {
                continue;
            }

            if entry.constant {
                candidates.push(ScanCandidate {
                    entry: entry.clone(),
                    activation_key,
                });
                continue;
            }

            let entry_scan_depth =
                entry.scan_depth.unwrap_or(settings.world_info_depth).max(0) as usize;
            let entry_scan_text = scan_text
                .iter()
                .take(entry_scan_depth)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            let context = MatchContext {
                scan_text: &entry_scan_text,
                global_scan_data,
                global_case_sensitive: settings.world_info_case_sensitive,
                global_match_whole_words: settings.world_info_match_whole_words,
                global_scan_depth: entry_scan_depth as i32,
            };

            if self.matcher.match_entry(entry, &context).is_some() {
                candidates.push(ScanCandidate {
                    entry: entry.clone(),
                    activation_key,
                });
            }
        }

        candidates
    }

    fn passes_recursion_gate(entry: &WorldInfoEntry, recursion_depth: i32) -> bool {
        if recursion_depth > 0 && entry.exclude_recursion {
            return false;
        }

        match &entry.delay_until_recursion {
            serde_json::Value::Bool(true) => recursion_depth > 0,
            serde_json::Value::Number(n) => {
                let required_depth = n.as_i64().unwrap_or(0).max(0) as i32;
                recursion_depth >= required_depth
            }
            _ => true,
        }
    }

    fn passes_initial_delay(entry: &WorldInfoEntry, available_messages: usize) -> bool {
        match entry.delay {
            Some(delay) if delay > 0 => available_messages > delay as usize,
            _ => true,
        }
    }

    fn apply_group_pruning(&self, entries: Vec<ScanCandidate>) -> Vec<ScanCandidate> {
        let mut passthrough = Vec::new();
        let mut grouped: HashMap<String, ScanCandidate> = HashMap::new();

        for candidate in entries {
            if candidate.entry.group.is_empty() || candidate.entry.group_override {
                passthrough.push(candidate);
                continue;
            }

            grouped
                .entry(candidate.entry.group.clone())
                .and_modify(|existing| {
                    if Self::group_rank(&candidate.entry) > Self::group_rank(&existing.entry) {
                        *existing = candidate.clone();
                    }
                })
                .or_insert(candidate);
        }

        passthrough.extend(grouped.into_values());
        passthrough.sort_by(|a, b| {
            b.entry
                .order
                .cmp(&a.entry.order)
                .then_with(|| a.entry.uid.cmp(&b.entry.uid))
                .then_with(|| a.activation_key.source_key.cmp(&b.activation_key.source_key))
        });
        passthrough
    }

    fn group_rank(entry: &WorldInfoEntry) -> (i32, i32, i32) {
        (entry.group_weight, entry.order, -entry.uid)
    }

    fn passes_probability(
        &self,
        entry: &WorldInfoEntry,
        scan_text: &[String],
        recursion_depth: i32,
    ) -> bool {
        if !entry.use_probability {
            return true;
        }
        let probability = entry.probability.clamp(0, 100);
        if probability >= 100 {
            return true;
        }
        if probability <= 0 {
            return false;
        }

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        entry.uid.hash(&mut hasher);
        entry.content.hash(&mut hasher);
        recursion_depth.hash(&mut hasher);
        for text in scan_text {
            text.hash(&mut hasher);
        }
        (hasher.finish() % 100) < probability as u64
    }

    /// 按 position 分流落槽
    fn distribute_to_positions(&self, entries: &[WorldInfoEntry]) -> WorldInfoInjectionResult {
        let mut result = WorldInfoInjectionResult::default();

        for entry in entries {
            let content = entry.content.clone();

            match entry.position {
                p if p == WorldInfoPosition::BEFORE_CHAR => {
                    if !result.world_info_before.is_empty() {
                        result.world_info_before.push('\n');
                    }
                    result.world_info_before.push_str(&content);
                }
                p if p == WorldInfoPosition::AFTER_CHAR => {
                    if !result.world_info_after.is_empty() {
                        result.world_info_after.push('\n');
                    }
                    result.world_info_after.push_str(&content);
                }
                p if p == WorldInfoPosition::AT_DEPTH => {
                    let depth = entry.depth;
                    let role = entry.role;
                    let slot = result
                        .world_info_depth
                        .entry(depth)
                        .or_insert_with(HashMap::new)
                        .entry(role)
                        .or_insert_with(String::new);
                    if !slot.is_empty() {
                        slot.push('\n');
                    }
                    slot.push_str(&content);
                }
                p if p == WorldInfoPosition::EM_TOP => {
                    result.em_top.push_str(&content);
                    result.em_top.push('\n');
                }
                p if p == WorldInfoPosition::EM_BOTTOM => {
                    result.em_bottom.push_str(&content);
                    result.em_bottom.push('\n');
                }
                p if p == WorldInfoPosition::AN_TOP => {
                    result.an_top.push_str(&content);
                    result.an_top.push('\n');
                }
                p if p == WorldInfoPosition::AN_BOTTOM => {
                    result.an_bottom.push_str(&content);
                    result.an_bottom.push('\n');
                }
                p if p == WorldInfoPosition::OUTLET => {
                    if !entry.outlet_name.is_empty() {
                        result
                            .outlets
                            .entry(entry.outlet_name.clone())
                            .or_insert_with(String::new)
                            .push_str(&content);
                    }
                }
                _ => {}
            }

            result.activated_entries.push(entry.uid);
        }

        result
    }

    /// 估算 token 数（简化实现）
    fn estimate_tokens(&self, text: &str) -> i32 {
        // 简化的 token 估算：约 4 字符 = 1 token
        (text.len() / 4) as i32
    }
}

impl Default for WorldInfoInjector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 扫描结果
// ============================================================================

#[derive(Debug, Default)]
struct ScanResult {
    activated_entries: Vec<WorldInfoEntry>,
    used_budget: i32,
}

// ============================================================================
// 世界书管理器
// ============================================================================

/// 世界书管理器
///
/// 负责加载和管理世界书文件。
pub struct WorldInfoManager {
    cache: Arc<RwLock<HashMap<String, WorldInfoFile>>>,
    max_cached_worldbooks: usize,
}

impl WorldInfoManager {
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    pub fn with_capacity(max_cached_worldbooks: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_cached_worldbooks: max_cached_worldbooks.max(1),
        }
    }

    /// 加载世界书
    pub async fn load_worldbook(&self, id: &str) -> Option<WorldInfoFile> {
        let cache = self.cache.read().await;
        cache.get(id).cloned()
    }

    /// 缓存世界书
    pub async fn cache_worldbook(&self, id: String, worldbook: WorldInfoFile) {
        let mut cache = self.cache.write().await;
        if cache.len() >= self.max_cached_worldbooks && !cache.contains_key(&id) {
            if let Some(evict_id) = cache.keys().next().cloned() {
                cache.remove(&evict_id);
            }
        }
        cache.insert(id, worldbook);
    }

    pub async fn invalidate_worldbook(&self, id: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(id);
    }

    pub async fn cache_len(&self) -> usize {
        self.cache.read().await.len()
    }

    /// 清除缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

impl Default for WorldInfoManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::st::runtime_assembly::{STChatMetadata, STUserPersona};
    use crate::storage::st_resources::{CharacterData, TavernCardV3};

    fn empty_worldbook(name: &str) -> WorldInfoFile {
        WorldInfoFile {
            entries: HashMap::new(),
            original_data: None,
            rst_lore_id: Some(name.to_string()),
            name: name.to_string(),
            description: String::new(),
            extensions: serde_json::Map::new(),
            extra: serde_json::Map::new(),
        }
    }

    #[tokio::test]
    async fn world_info_manager_bounds_worldbook_pool_cache() {
        let manager = WorldInfoManager::with_capacity(1);

        manager
            .cache_worldbook("lore-a".to_string(), empty_worldbook("lore-a"))
            .await;
        manager
            .cache_worldbook("lore-b".to_string(), empty_worldbook("lore-b"))
            .await;

        assert_eq!(manager.cache_len().await, 1);
        assert!(manager.load_worldbook("lore-b").await.is_some());
    }

    #[tokio::test]
    async fn world_info_manager_invalidates_changed_worldbook() {
        let manager = WorldInfoManager::new();
        manager
            .cache_worldbook("lore-a".to_string(), empty_worldbook("lore-a"))
            .await;

        manager.invalidate_worldbook("lore-a").await;

        assert!(manager.load_worldbook("lore-a").await.is_none());
    }

    #[tokio::test]
    async fn check_world_info_substitutes_macros_and_uses_role_name_fallback() {
        let mut entries = HashMap::new();
        let mut entry = crate::storage::st_resources::WorldInfoEntry::new(1);
        entry.key = vec!["{{char}}".to_string()];
        entry.content = "Known as ally of {{user}}".to_string();
        entries.insert("1".to_string(), entry);

        let source = WorldInfoSource::CharacterLore(WorldInfoFile {
            entries,
            original_data: None,
            rst_lore_id: Some("lore-1".to_string()),
            name: "Lore".to_string(),
            description: String::new(),
            extensions: serde_json::Map::new(),
            extra: serde_json::Map::new(),
        });

        let messages = vec![STChatMessage {
            id: "m1".to_string(),
            role: "assistant".to_string(),
            content: "Bob arrives".to_string(),
            created_at: String::new(),
            name: None,
            attachments: Vec::new(),
        }];
        let chat_metadata = STChatMetadata {
            world_info: None,
            enabled_world_info: Vec::new(),
            disabled_world_info: Vec::new(),
            user_persona: Some(STUserPersona {
                name: "Alice".to_string(),
                description: "Ranger".to_string(),
            }),
            extra: serde_json::Map::new(),
        };
        let character = TavernCardV3 {
            spec: "chara_card_v3".to_string(),
            spec_version: "3.0".to_string(),
            data: CharacterData {
                name: "Bob".to_string(),
                description: String::new(),
                personality: String::new(),
                scenario: String::new(),
                first_mes: String::new(),
                mes_example: String::new(),
                creator_notes: String::new(),
                system_prompt: String::new(),
                post_history_instructions: String::new(),
                alternate_greetings: Vec::new(),
                tags: Vec::new(),
                creator: String::new(),
                character_version: String::new(),
                extensions: serde_json::Map::new(),
                character_book: None,
                extra: serde_json::Map::new(),
            },
            extra: serde_json::Map::new(),
        };
        let macro_context = MacroContext::from_chat_metadata(&chat_metadata, Some(&character), "");

        let mut injector = WorldInfoInjector::new();
        let result = injector
            .check_world_info(
                &messages,
                4096,
                &STWorldInfoSettings::default(),
                vec![source],
                &GlobalScanData::default(),
                &macro_context,
            )
            .await;

        assert_eq!(result.activated_entries, vec![1]);
        assert!(result.world_info_before.contains("Known as ally of Alice"));
    }

    #[tokio::test]
    async fn check_world_info_keeps_entries_from_different_worldbooks_with_same_uid() {
        let mut first_entries = HashMap::new();
        let mut first = crate::storage::st_resources::WorldInfoEntry::new(1);
        first.constant = true;
        first.content = "First lore".to_string();
        first_entries.insert("1".to_string(), first);

        let mut second_entries = HashMap::new();
        let mut second = crate::storage::st_resources::WorldInfoEntry::new(1);
        second.constant = true;
        second.content = "Second lore".to_string();
        second_entries.insert("1".to_string(), second);

        let sources = vec![
            WorldInfoSource::ChatLore(WorldInfoFile {
                entries: first_entries,
                original_data: None,
                rst_lore_id: Some("lore-a".to_string()),
                name: "Lore A".to_string(),
                description: String::new(),
                extensions: serde_json::Map::new(),
                extra: serde_json::Map::new(),
            }),
            WorldInfoSource::GlobalLore(WorldInfoFile {
                entries: second_entries,
                original_data: None,
                rst_lore_id: Some("lore-b".to_string()),
                name: "Lore B".to_string(),
                description: String::new(),
                extensions: serde_json::Map::new(),
                extra: serde_json::Map::new(),
            }),
        ];

        let mut injector = WorldInfoInjector::new();
        let result = injector
            .check_world_info(
                &[],
                4096,
                &STWorldInfoSettings::default(),
                sources,
                &GlobalScanData::default(),
                &MacroContext::default(),
            )
            .await;

        assert!(result.world_info_before.contains("First lore"));
        assert!(result.world_info_before.contains("Second lore"));
        assert_eq!(result.activated_entries, vec![1, 1]);
    }
}
