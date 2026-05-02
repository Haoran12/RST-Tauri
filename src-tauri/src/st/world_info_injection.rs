//! ST World Info Injection
//!
//! 世界书从来源合并到 Prompt 落槽的完整运行时流程。
//! 参考: docs/73_st_worldbook_injection.md
//! 实现依据: SillyTavern public/scripts/world-info.js

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::st::keyword_matcher::{KeywordMatcher, MatchContext, GlobalScanData};
use crate::st::runtime_assembly::{STWorldInfoSettings, WorldInfoInjectionResult, STChatMessage};
use crate::storage::st_resources::{WorldInfoFile, WorldInfoEntry, WorldInfoPosition};

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
    ) -> WorldInfoInjectionResult {
        // 1. 合并并排序所有来源的词条
        let sorted_entries = self.get_sorted_entries(sources, settings).await;

        // 2. 计算 token 预算
        let budget = self.calculate_budget(max_context, settings);

        // 3. 构造扫描文本
        let scan_text = self.build_scan_text(chat_for_wi, settings);

        // 4. 执行扫描
        let scan_result = self.scan_entries(
            &sorted_entries,
            &scan_text,
            budget,
            settings,
            global_scan_data,
        ).await;

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
    ) -> Vec<(WorldInfoEntry, String)> {
        let mut all_entries: Vec<(WorldInfoEntry, String, i32)> = Vec::new();
        let mut seen_worlds: HashSet<String> = HashSet::new();

        // 按 priority 排序来源
        let mut sorted_sources = sources;
        sorted_sources.sort_by_key(|s| s.priority());

        for source in sorted_sources {
            let world_name = source.file().name.clone();

            // 去重：如果同一世界书名称已在更高优先级来源中启用，跳过
            if seen_worlds.contains(&world_name) {
                continue;
            }
            seen_worlds.insert(world_name.clone());

            // 提取词条并添加来源标记
            for (_, entry) in &source.file().entries {
                all_entries.push((entry.clone(), source.source_name().to_string(), source.priority()));
            }
        }

        // 根据 world_info_character_strategy 排序
        // 0 = evenly: 全部合并后按 order 降序
        // 1 = character_first: Character 内部按 order 降序，然后 Global
        // 2 = global_first: Global 内部按 order 降序，然后 Character
        match settings.world_info_character_strategy {
            0 => {
                // evenly: 全部合并后按 order 降序
                all_entries.sort_by(|a, b| b.0.order.cmp(&a.0.order));
            }
            1 => {
                // character_first: Character 优先，然后 Global
                all_entries.sort_by(|a, b| {
                    match a.2.cmp(&b.2) {
                        std::cmp::Ordering::Equal => b.0.order.cmp(&a.0.order),
                        other => other,
                    }
                });
            }
            2 => {
                // global_first: Global 优先，然后 Character
                all_entries.sort_by(|a, b| {
                    match a.2.cmp(&b.2).reverse() {
                        std::cmp::Ordering::Equal => b.0.order.cmp(&a.0.order),
                        other => other,
                    }
                });
            }
            _ => {}
        }

        // 返回 (entry, source_name)
        all_entries.into_iter().map(|(e, s, _)| (e, s)).collect()
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
    fn build_scan_text(&self, chat: &[STChatMessage], settings: &STWorldInfoSettings) -> Vec<String> {
        // 反转聊天顺序（最近消息在前）
        let mut scan_text: Vec<String> = Vec::new();

        for msg in chat.iter().rev() {
            let text = if settings.world_info_include_names {
                let name = msg.name.clone().unwrap_or_default();
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
        entries: &[(WorldInfoEntry, String)],
        scan_text: &[String],
        budget: i32,
        settings: &STWorldInfoSettings,
        global_scan_data: &GlobalScanData,
    ) -> ScanResult {
        let mut result = ScanResult::default();
        let mut used_budget = 0i32;
        let mut recursion_buffer: Vec<WorldInfoEntry> = Vec::new();
        let recursion_depth = 0;

        // 构造匹配上下文
        let combined_scan_text = scan_text.join("\n");
        let context = MatchContext {
            scan_text: &combined_scan_text,
            global_scan_data: global_scan_data,
            global_case_sensitive: settings.world_info_case_sensitive,
            global_match_whole_words: settings.world_info_match_whole_words,
            global_scan_depth: settings.world_info_depth,
        };

        for (entry, _source) in entries {
            // 跳过禁用的词条
            if entry.disable {
                continue;
            }

            // 检查 constant 或 sticky
            if entry.constant {
                result.activated_entries.push(entry.clone());
                continue;
            }

            // 概率检查
            if entry.use_probability && entry.probability < 100 {
                // 简化的概率检查（实际应使用随机数）
                // 这里暂时跳过概率逻辑
            }

            // 检查延迟/冷却/sticky
            // 简化实现，跳过时间控制逻辑

            // 检查递归门控
            if entry.exclude_recursion && recursion_depth > 0 {
                continue;
            }

            // 执行关键词匹配
            let match_result = self.matcher.match_entry(entry, &context);

            if match_result.is_some() {
                // 预算检查
                let entry_tokens = self.estimate_tokens(&entry.content);
                if !entry.ignore_budget && used_budget + entry_tokens > budget {
                    continue;
                }

                result.activated_entries.push(entry.clone());
                used_budget += entry_tokens;

                // 递归处理
                if settings.world_info_recursive && !entry.prevent_recursion {
                    recursion_buffer.push(entry.clone());
                }
            }
        }

        // 处理递归扫描
        if !recursion_buffer.is_empty() && recursion_depth < settings.world_info_max_recursion_steps {
            // 简化的递归实现
            // 实际实现需要用递归缓冲区的内容继续扫描
        }

        result.used_budget = used_budget;
        result
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
                    result.world_info_depth
                        .entry(depth)
                        .or_insert_with(HashMap::new)
                        .entry(role)
                        .or_insert_with(String::new)
                        .push_str(&content);
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
                        result.outlets
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
}

impl WorldInfoManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
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
        cache.insert(id, worldbook);
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
