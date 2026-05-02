# RST-Tauri 任务清单

> 基于 `docs/implementation_plan.md` 的持久化任务跟踪文档。
> 状态：✅ 已完成 | 🔄 进行中 | ⏳ 待开始 | ❌ 已阻塞

---

## 阶段一：基础框架 (MVP)

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 1.1 | 初始化 Tauri + Vue 3 + TypeScript + Naive UI | ✅ | 2026-05-02 | |
| 1.2 | 配置 Vue Router + Pinia | ✅ | 2026-05-02 | |
| 1.3 | 实现 App Shell、资源工作台默认首页、基础主题和响应式布局 | ✅ | 2026-05-02 | |
| 1.4 | 实现 JSON 存储层 | ✅ | 2026-05-02 | |
| 1.5 | 实现 AI Provider 抽象层 | ✅ | 2026-05-02 | |
| 1.6 | 集成 OpenAI Responses API | ✅ | 2026-05-02 | |
| 1.7 | 集成 OpenAI Chat Completions API | ✅ | 2026-05-02 | |
| 1.8 | 集成 DeepSeek API | ✅ | 2026-05-02 | |
| 1.9 | 集成 Anthropic Messages API | ✅ | 2026-05-02 | |
| 1.10 | 集成 Gemini API | ✅ | 2026-05-02 | |
| 1.11 | 集成 Claude Code Interface | ✅ | 2026-05-02 | |
| 1.12 | 全局运行日志系统 (LlmCallLogger + EventLogger + Retention) | ✅ | 2026-05-02 | SQLite 存储 |
| 1.13 | 基础聊天功能 (STChatView + ChatStore) | ✅ | 2026-05-02 | |

---

## 阶段二：SillyTavern 模式

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 2.0 | 对照 `E:\AIPlay\ST_latest` 修订 ST 角色卡 / 世界书兼容设计 | ✅ | 2026-05-02 | 明确 PNG `ccv3/chara`、`extensions.world` 字符串绑定、CharacterBook 导入转换；上一轮原型代码已回退 |
| 2.1 | 角色卡 V3 管理（创建 / 编辑 / 导入 / 导出） | ✅ | 2026-05-02 | PNG/JSON 导入导出、未知字段保留、CharacterBook 转换 |
| 2.2 | 角色卡头像上传与显示 | ✅ | 2026-05-02 | 已在 2.1 中实现 |
| 2.3 | 世界书编辑器 - 基础 CRUD | ✅ | 2026-05-02 | WorldInfoFile 对象格式、词条 CRUD、导入导出 |
| 2.4 | 世界书编辑器 - 分组管理 | ✅ | 2026-05-02 | 已在 2.3 中实现（group/group_override/group_weight） |
| 2.5 | 世界书编辑器 - 概率控制 | ✅ | 2026-05-02 | 已在 2.3 中实现（probability/use_probability） |
| 2.6 | 世界书编辑器 - 递归扫描 | ✅ | 2026-05-02 | 已在 2.3 中实现（exclude_recursion/prevent_recursion/delay_until_recursion） |
| 2.7 | 世界书编辑器 - 时间控制 | ✅ | 2026-05-02 | 已在 2.3 中实现（sticky/cooldown/delay） |
| 2.8 | 关键词触发系统 - 基础匹配 | ✅ | 2026-05-02 | Rust + TypeScript 双端实现 |
| 2.9 | 关键词触发系统 - 正则匹配 | ✅ | 2026-05-02 | 支持 /pattern/flags 格式、LRU 缓存 |
| 2.10 | 关键词触发系统 - 匹配目标扩展 | ✅ | 2026-05-02 | match_persona_description 等 6 个扩展目标 |
| 2.11 | Regex 扩展兼容 - global 脚本 | ✅ | 2026-05-02 | Rust + TypeScript 双端实现 |
| 2.12 | Regex 扩展兼容 - preset 脚本 | ✅ | 2026-05-02 | preset_allowed_regex 授权框架 |
| 2.13 | Regex 扩展兼容 - scoped 脚本 | ✅ | 2026-05-02 | character_allowed_regex 授权框架 |
| 2.14 | Regex 扩展兼容 - prompt-only / display-only | ✅ | 2026-05-02 | markdownOnly/promptOnly 过滤逻辑 |
| 2.15 | Regex 扩展兼容 - 内嵌脚本授权 | ✅ | 2026-05-02 | 授权机制框架已实现 |
| 2.16 | 预设系统 - 基础结构 | ✅ | 2026-05-02 | Rust + TypeScript 双端数据模型 |
| 2.17 | 预设系统 - Sampler 预设 | ✅ | 2026-05-02 | 采样参数完整定义 |
| 2.18 | 预设系统 - Instruct 预设 | ✅ | 2026-05-02 | 对话格式模板 |
| 2.19 | 预设系统 - Context 预设 | ✅ | 2026-05-02 | 上下文组装模板 |
| 2.20 | 预设系统 - System Prompt 预设 | ✅ | 2026-05-02 | 系统提示词模板 |
| 2.21 | 预设系统 - Reasoning 预设 | ✅ | 2026-05-02 | 思维链格式模板 |
| 2.22 | 多 API 支持 - Anthropic Messages | ✅ | 2026-05-02 | 已在阶段一完成 |
| 2.23 | 多 API 支持 - Gemini | ✅ | 2026-05-02 | 已在阶段一完成 |
| 2.24 | 多 API 支持 - Claude Code Interface | ✅ | 2026-05-02 | 已在阶段一完成 |
| 2.25 | 运行时组装 (RuntimeAssembly) | ✅ | 2026-05-02 | Rust + TypeScript 双端实现 |
| 2.26 | 世界书注入流程 | ✅ | 2026-05-02 | 来源合并、排序、扫描、落槽 |

---

## 阶段三：Agent 模式 — 数据模型层

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 3.1 | SQLite 表结构设计 | ⏳ | | |
| 3.2 | 三层语义隔离 (Layer 1 / Layer 2 / Layer 3) | ⏳ | | |
| 3.3 | WorldMainlineCursor 实现 | ⏳ | | |
| 3.4 | AgentSession / SessionTurn 实现 | ⏳ | | |
| 3.5 | TimeAnchor / story_time_anchor 实现 | ⏳ | | |
| 3.6 | WorldStateAt 基础类型 | ⏳ | | |
| 3.7 | SceneModel 完整定义 (Layer 1) | ⏳ | | |
| 3.8 | ManaField 定义 | ⏳ | | |
| 3.9 | PhysicalConditions 定义 | ⏳ | | |
| 3.10 | LocationGraph - LocationNode | ⏳ | | |
| 3.11 | LocationGraph - LocationSpatialRelation | ⏳ | | |
| 3.12 | LocationGraph - LocationEdge | ⏳ | | |
| 3.13 | LocationGraph - alias / polity template | ⏳ | | |
| 3.14 | 地区事实继承字段 | ⏳ | | |
| 3.15 | KnowledgeEntry 体系 - kind / subject | ⏳ | | |
| 3.16 | KnowledgeEntry 体系 - access_policy | ⏳ | | |
| 3.17 | KnowledgeEntry 体系 - subject_awareness | ⏳ | | |
| 3.18 | KnowledgeEntry 体系 - apparent_content | ⏳ | | |
| 3.19 | 访问派生索引表 | ⏳ | | |
| 3.20 | L1 客观关系 / 授权表 | ⏳ | | |
| 3.21 | HistoricalEvent 定义 | ⏳ | | |
| 3.22 | TruthGuidance 定义 | ⏳ | | |
| 3.23 | provisional_session_truth 定义 | ⏳ | | |
| 3.24 | ConflictReport 定义 | ⏳ | | |
| 3.25 | KnowledgeEntry content sub-schemas | ⏳ | | |
| 3.26 | CharacterRecord - base_attributes | ⏳ | | |
| 3.27 | CharacterRecord - baseline_body_profile | ⏳ | | |
| 3.28 | CharacterRecord - mind_model_card | ⏳ | | |
| 3.29 | CharacterRecord - temporary_state | ⏳ | | |
| 3.30 | TemporalStateRecord 实现 | ⏳ | | |
| 3.31 | CharacterSubjectiveState (Layer 3) | ⏳ | | |
| 3.32 | SubjectiveStateReducer 实现 | ⏳ | | |
| 3.33 | EmbodimentState (Layer 2) | ⏳ | | |
| 3.34 | FilteredSceneView (Layer 2) | ⏳ | | |
| 3.35 | AccessibleKnowledge (Layer 2) | ⏳ | | |
| 3.36 | CognitivePass I/O 类型 | ⏳ | | |
| 3.37 | ConfidenceShift / BodyReactionDelta | ⏳ | | |
| 3.38 | SceneInitializationDraft 定义 | ⏳ | | |
| 3.39 | SceneStateExtractorOutput 定义 | ⏳ | | |
| 3.40 | UserInputDelta 定义 | ⏳ | | |
| 3.41 | StyleConstraints 定义 | ⏳ | | |
| 3.42 | OutcomePlannerOutput 定义 | ⏳ | | |
| 3.43 | SurfaceRealizerInput 定义 | ⏳ | | |
| 3.44 | Agent Trace 表结构 | ⏳ | | |
| 3.45 | LLM Logs 表结构 | ✅ | 2026-05-02 | 已在阶段一完成 |
| 3.46 | app_event_logs 表结构 | ✅ | 2026-05-02 | 已在阶段一完成 |
| 3.47 | AgentLlmProfile 定义 | ⏳ | | |
| 3.48 | World Agent settings | ⏳ | | |
| 3.49 | Agent World Editor - 基础 CRUD | ⏳ | | |
| 3.50 | Agent World Editor - paused-only 提交 | ⏳ | | |
| 3.51 | Agent World Editor - 影响分析 | ⏳ | | |
| 3.52 | Agent World Editor - editor commit journal | ⏳ | | |

---

## 阶段四：Agent 模式 — 程序化核心

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 4.1 | KnowledgeStore - Layer 1 CRUD | ⏳ | | |
| 4.2 | KnowledgeAccessResolver - 三谓词合并 | ⏳ | | |
| 4.3 | KnowledgeAccessProtocol - SQLite 派生索引预筛 | ⏳ | | |
| 4.4 | KnowledgeAccessProtocol - AccessibleKnowledge 构建 | ⏳ | | |
| 4.5 | KnowledgeAccessProtocol - TimeAnchor 查询 | ⏳ | | |
| 4.6 | LocationResolver - 地点消歧 | ⏳ | | |
| 4.7 | LocationFactResolver - 父级事实继承 | ⏳ | | |
| 4.8 | LocationFactResolver - 自然地理影响 | ⏳ | | |
| 4.9 | RoutePlanner - 路线与路程估算 | ⏳ | | |
| 4.10 | AttributeResolver - 基础属性有效值 | ⏳ | | |
| 4.11 | EmbodimentResolver - 灵觉 + 环境档位翻译 | ⏳ | | |
| 4.12 | SceneFilter - observable_facets 计算 | ⏳ | | |
| 4.13 | SceneFilter - WeatherPerception | ⏳ | | |
| 4.14 | SceneFilter - ManaSignal | ⏳ | | |
| 4.15 | InputAssembly - 拒绝 Layer 1 原始对象 | ⏳ | | |
| 4.16 | PhysicsResolver - 物理数值骨架 | ⏳ | | |
| 4.17 | CombatMathResolver - Mana Combat Resolution | ⏳ | | |
| 4.18 | EffectValidator - 技能契约校验 | ⏳ | | |
| 4.19 | EffectValidator - 硬约束校验 | ⏳ | | |
| 4.20 | KnowledgeRevealEvent 处理 | ⏳ | | |
| 4.21 | HistoricalTruthResolver 实现 | ⏳ | | |
| 4.22 | TemporalConsistencyValidator 实现 | ⏳ | | |

---

## 阶段五：Agent 模式 — 认知与叙事层

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 5.1 | PromptBuilder - AgentPromptBundle | ⏳ | | |
| 5.2 | PromptBuilder - 五类节点静态提示词 | ⏳ | | |
| 5.3 | PromptBuilder - 动态 JSON 输入 | ⏳ | | |
| 5.4 | PromptBuilder - JSON schema 注入 | ⏳ | | |
| 5.5 | PromptBuilder - prompt version/hash | ⏳ | | |
| 5.6 | SceneInitializer 实现 | ⏳ | | |
| 5.7 | SceneStateExtractor 实现 | ⏳ | | |
| 5.8 | CharacterCognitivePass 实现 | ⏳ | | |
| 5.9 | JSON 输出容错修复器 | ⏳ | | |
| 5.10 | OutcomePlanner 实现 | ⏳ | | |
| 5.11 | SurfaceRealizer 实现 | ⏳ | | |
| 5.12 | AI Provider chat_structured - OpenAI Responses | ⏳ | | |
| 5.13 | AI Provider chat_structured - OpenAI Chat Completions | ⏳ | | |
| 5.14 | AI Provider chat_structured - Anthropic | ⏳ | | |
| 5.15 | AI Provider chat_structured - Gemini | ⏳ | | |
| 5.16 | AI Provider chat_structured - DeepSeek | ⏳ | | |
| 5.17 | AI Provider chat_structured - Claude Code Interface | ⏳ | | |
| 5.18 | LLM 调用日志 - stream chunks | ⏳ | | |
| 5.19 | LLM 调用日志 - readable_text | ⏳ | | |
| 5.20 | Agent LLM 节点配置选择 UI | ⏳ | | |

---

## 阶段六：Agent 模式 — 验证与运行时

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 6.1 | 验证规则 - SelfAwareness | ⏳ | | |
| 6.2 | 验证规则 - GodOnly | ⏳ | | |
| 6.3 | 验证规则 - ApparentVsTrue | ⏳ | | |
| 6.4 | 验证规则 - ReactionWindow | ⏳ | | |
| 6.5 | 验证规则 - NarrativeFactCheck | ⏳ | | |
| 6.6 | 验证规则 - SchemaConformance | ⏳ | | |
| 6.7 | 验证规则 - TemporalCanon | ⏳ | | |
| 6.8 | AgentRuntime 主循环 | ⏳ | | |
| 6.9 | 固定快照机制 | ⏳ | | |
| 6.10 | 并行 CognitivePass | ⏳ | | |
| 6.11 | 统一验证 | ⏳ | | |
| 6.12 | 单写提交 | ⏳ | | |
| 6.13 | Dirty Flags 实现 | ⏳ | | |
| 6.14 | Active Set 实现 | ⏳ | | |
| 6.15 | 角色 Tier 分级 | ⏳ | | |
| 6.16 | 调用预算监控 | ⏳ | | |
| 6.17 | ReactionWindow 资格判定 | ⏳ | | |
| 6.18 | ReactionIntent 收集 | ⏳ | | |
| 6.19 | 递归深度限制 | ⏳ | | |
| 6.20 | 过去线冲突 UX | ⏳ | | |
| 6.21 | Agent Trace 写入点 | ⏳ | | |

---

## 阶段七：用户角色扮演

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 7.1 | 用户角色选择 | ⏳ | | |
| 7.2 | 用户输入心理活动 / 言行 | ⏳ | | |
| 7.3 | DirectorHint - 结果规划导演权 | ⏳ | | |
| 7.4 | DirectorHint - 文风导演权 | ⏳ | | |
| 7.5 | 同一 World 多时期会话创建 | ⏳ | | |
| 7.6 | 过去线补完 HistoricalEvent 开放细节 | ⏳ | | |
| 7.7 | 正史资格提升机制 | ⏳ | | |

---

## 阶段八：优化与扩展

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 8.1 | 性能优化 - 缓存 | ⏳ | | |
| 8.2 | 性能优化 - 事件批处理 | ⏳ | | |
| 8.3 | UI / UX 改进 | ⏳ | | |
| 8.4 | 高级 Trace 可视化 | ⏳ | | |
| 8.5 | 测试覆盖 | ⏳ | | |
| 8.6 | 插件系统 | ⏳ | | |
| 8.7 | 日志管理 UI - 大小统计 | ⏳ | | |
| 8.8 | 日志管理 UI - 30 天未更新 World 提示 | ⏳ | | |
| 8.9 | 日志管理 UI - 手动清理 / 导出 | ⏳ | | |

---

## 统计

| 阶段 | 总任务 | 已完成 | 进行中 | 待开始 |
|------|--------|--------|--------|--------|
| 阶段一 | 13 | 13 | 0 | 0 |
| 段二 | 27 | 27 | 0 | 0 |
| 阶段三 | 52 | 2 | 0 | 50 |
| 阶段四 | 22 | 0 | 0 | 22 |
| 阶段五 | 20 | 0 | 0 | 20 |
| 阶段六 | 21 | 0 | 0 | 21 |
| 阶段七 | 7 | 0 | 0 | 7 |
| 阶段八 | 9 | 0 | 0 | 9 |
| **总计** | **171** | **41** | **0** | **130** |

---

## 更新日志

| 日期 | 更新内容 |
|------|----------|
| 2026-05-02 | 初始化任务清单；阶段一全部完成 |
| 2026-05-02 | 完成任务 2.1 角色卡 V3 管理、2.2 头像上传与显示 |
| 2026-05-02 | 完成任务 2.3-2.7 世界书编辑器（基础 CRUD、分组管理、概率控制、递归扫描、时间控制） |
| 2026-05-02 | 完成任务 2.8-2.10 关键词触发系统（基础匹配、正则匹配、匹配目标扩展） |
| 2026-05-02 | 完成任务 2.11-2.15 Regex 扩展兼容（global/preset/scoped 脚本、prompt-only/display-only、内嵌脚本授权） |
| 2026-05-02 | 完成任务 2.16-2.21 预设系统（Sampler/Instruct/Context/SystemPrompt/Reasoning 预设） |
| 2026-05-02 | 完成任务 2.25 运行时组装（GlobalAppState、RuntimeContext、RequestAssembler、ProviderRequestMapper） |
| 2026-05-02 | 完成任务 2.26 世界书注入流程（WorldInfoInjector、来源合并、排序、扫描、落槽） |
