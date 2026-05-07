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
| 1.14 | 结构化文本编辑器（CodeMirror 6 Plain / JSON / YAML） | ✅ | 2026-05-04 | 已落地共享 `StructuredTextEditor`，接入 ST 世界书与 Agent 结构化字段原型编辑 |

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
| 3.1 | SQLite 表结构设计 | ✅ | 2026-05-02 | 完整实现 docs/14_agent_persistence.md 所有表结构 |
| 3.2 | 三层语义隔离 (Layer 1 / Layer 2 / Layer 3) | ✅ | 2026-05-02 | KnowledgeAccessResolver 实现，GodOnly hard deny |
| 3.3 | WorldMainlineCursor 实现 | ✅ | 2026-05-02 | 存储层 CRUD 完成 |
| 3.4 | AgentSession / SessionTurn 实现 | ✅ | 2026-05-02 | 数据模型和存储层完成 |
| 3.5 | TimeAnchor / story_time_anchor 实现 | ✅ | 2026-05-02 | common.rs 中定义完成 |
| 3.6 | WorldStateAt 基础类型 | ✅ | 2026-05-02 | session.rs 完成 |
| 3.7 | SceneModel 完整定义 (Layer 1) | ✅ | 2026-05-02 | scene.rs 完成 |
| 3.8 | ManaField 定义 | ✅ | 2026-05-02 | scene.rs 完成 |
| 3.9 | PhysicalConditions 定义 | ✅ | 2026-05-02 | scene.rs 完成 |
| 3.10 | LocationGraph - LocationNode | ✅ | 2026-05-02 | location.rs 完成 |
| 3.11 | LocationGraph - LocationSpatialRelation | ✅ | 2026-05-02 | location.rs 完成 |
| 3.12 | LocationGraph - LocationEdge | ✅ | 2026-05-02 | location.rs 完成 |
| 3.13 | LocationGraph - alias / polity template | ✅ | 2026-05-02 | location.rs 完成 |
| 3.14 | 地区事实继承字段 | ✅ | 2026-05-02 | knowledge.rs RegionFactContent 完成 |
| 3.15 | KnowledgeEntry 体系 - kind / subject | ✅ | 2026-05-02 | knowledge.rs 完成 |
| 3.16 | KnowledgeEntry 体系 - access_policy | ✅ | 2026-05-02 | knowledge.rs 完成 |
| 3.17 | KnowledgeEntry 体系 - subject_awareness | ✅ | 2026-05-02 | knowledge.rs 完成 |
| 3.18 | KnowledgeEntry 体系 - apparent_content | ✅ | 2026-05-02 | knowledge.rs 完成 |
| 3.19 | 访问派生索引表 | ✅ | 2026-05-02 | knowledge_access_known_by / knowledge_access_scopes |
| 3.20 | L1 客观关系 / 授权表 | ✅ | 2026-05-02 | session.rs ObjectiveRelationship 完成 |
| 3.21 | HistoricalEvent 定义 | ✅ | 2026-05-02 | knowledge.rs HistoricalEventContent 完成 |
| 3.22 | TruthGuidance 定义 | ✅ | 2026-05-02 | knowledge.rs 完成 |
| 3.23 | provisional_session_truth 定义 | ✅ | 2026-05-02 | session.rs 完成 |
| 3.24 | ConflictReport 定义 | ✅ | 2026-05-02 | session.rs 完成 |
| 3.25 | KnowledgeEntry content sub-schemas | ✅ | 2026-05-02 | knowledge.rs 完成 |
| 3.26 | CharacterRecord - base_attributes | ✅ | 2026-05-02 | character.rs 完成 |
| 3.27 | CharacterRecord - baseline_body_profile | ✅ | 2026-05-02 | character.rs 完成 |
| 3.28 | CharacterRecord - mind_model_card | ✅ | 2026-05-02 | knowledge.rs MindModelCardContent 完成 |
| 3.29 | CharacterRecord - temporary_state | ✅ | 2026-05-02 | character.rs 完成 |
| 3.30 | TemporalStateRecord 实现 | ✅ | 2026-05-02 | session.rs 完成 |
| 3.31 | CharacterSubjectiveState (Layer 3) | ✅ | 2026-05-02 | subjective.rs 完成 |
| 3.32 | SubjectiveStateReducer 实现 | ✅ | 2026-05-02 | subjective_reducer.rs 完成 |
| 3.33 | EmbodimentState (Layer 2) | ✅ | 2026-05-02 | scene.rs 完成 |
| 3.34 | FilteredSceneView (Layer 2) | ✅ | 2026-05-02 | scene.rs 完成 |
| 3.35 | AccessibleKnowledge (Layer 2) | ✅ | 2026-05-02 | knowledge.rs 完成 |
| 3.36 | CognitivePass I/O 类型 | ✅ | 2026-05-02 | subjective.rs 完成 |
| 3.37 | ConfidenceShift / BodyReactionDelta | ✅ | 2026-05-02 | subjective.rs 完成 |
| 3.38 | SceneInitializationDraft 定义 | ✅ | 2026-05-02 | scene.rs 完成 |
| 3.39 | SceneStateExtractorOutput 定义 | ✅ | 2026-05-02 | subjective.rs 完成 |
| 3.40 | UserInputDelta 定义 | ✅ | 2026-05-02 | subjective.rs 完成 |
| 3.41 | StyleConstraints 定义 | ✅ | 2026-05-02 | subjective.rs 完成 |
| 3.42 | OutcomePlannerOutput 定义 | ✅ | 2026-05-02 | subjective.rs 完成 |
| 3.43 | SurfaceRealizerInput 定义 | ✅ | 2026-05-02 | scene.rs 完成 |
| 3.44 | Agent Trace 表结构 | ✅ | 2026-05-02 | turn_traces / agent_step_traces |
| 3.45 | LLM Logs 表结构 | ✅ | 2026-05-02 | 已在阶段一完成 |
| 3.46 | app_event_logs 表结构 | ✅ | 2026-05-02 | 已在阶段一完成 |
| 3.47 | AgentLlmProfile 定义 | ✅ | 2026-05-02 | schema.sql 完成 |
| 3.48 | World Agent settings | ✅ | 2026-05-02 | schema.sql 完成 |
| 3.49 | Agent World Editor - 基础 CRUD | ✅ | 2026-05-02 | world_editor/editor.rs 完成 |
| 3.50 | Agent World Editor - paused-only 提交 | ✅ | 2026-05-02 | world_editor/commit.rs 完成 |
| 3.51 | Agent World Editor - 影响分析 | ✅ | 2026-05-02 | world_editor/validator.rs 完成 |
| 3.52 | Agent World Editor - editor commit journal | ✅ | 2026-05-02 | world_editor/commit.rs 完成 |

---

## 阶段四：Agent 模式 — 程序化核心

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 4.1 | KnowledgeStore - Layer 1 CRUD | ✅ | 2026-05-03 | store.rs 完成 |
| 4.2 | KnowledgeAccessResolver - 三谓词合并 | ✅ | 2026-05-03 | access_resolver.rs 完成 |
| 4.3 | KnowledgeAccessProtocol - SQLite 派生索引预筛 | ✅ | 2026-05-03 | access_protocol.rs 完成 |
| 4.4 | KnowledgeAccessProtocol - AccessibleKnowledge 构建 | ✅ | 2026-05-03 | access_protocol.rs 完成 |
| 4.5 | KnowledgeAccessProtocol - TimeAnchor 查询 | ✅ | 2026-05-03 | access_protocol.rs 完成 |
| 4.6 | LocationResolver - 地点消歧 | ✅ | 2026-05-03 | resolver.rs 完成 |
| 4.7 | LocationFactResolver - 父级事实继承 | ✅ | 2026-05-03 | fact_resolver.rs 完成 |
| 4.8 | LocationFactResolver - 自然地理影响 | ✅ | 2026-05-03 | fact_resolver.rs 完成 |
| 4.9 | RoutePlanner - 路线与路程估算 | ✅ | 2026-05-03 | route_planner.rs 完成 |
| 4.10 | AttributeResolver - 基础属性有效值 | ✅ | 2026-05-03 | attribute_resolver.rs 完成 |
| 4.11 | EmbodimentResolver - 灵觉 + 环境档位翻译 | ✅ | 2026-05-03 | embodiment_resolver.rs 完成 |
| 4.12 | SceneFilter - observable_facets 计算 | ✅ | 2026-05-05 | `scene_filter.rs` 已补基础 perceived attributes / mana signal 派生，避免向 LLM 暴露 Layer 1 原始数值 |
| 4.13 | SceneFilter - WeatherPerception | ✅ | 2026-05-03 | scene_filter.rs 完成 |
| 4.14 | SceneFilter - ManaSignal | ✅ | 2026-05-03 | scene_filter.rs 完成 |
| 4.15 | InputAssembly - 拒绝 Layer 1 原始对象 | ✅ | 2026-05-03 | input_assembly.rs 完成 |
| 4.16 | PhysicsResolver - 物理数值骨架 | ✅ | 2026-05-03 | physics_resolver.rs 完成 |
| 4.17 | CombatMathResolver - Mana Combat Resolution | ✅ | 2026-05-03 | combat_math_resolver.rs 完成 |
| 4.18 | EffectValidator - 技能契约校验 | 🔄 | | 已接 CharacterFacet `KnownAbility/HiddenAbility -> Skill` 解析桥接，并按 `actor_id + skill_id` 归属命中真实技能；冷却、材料、激活条件、reaction/passive/interrupt 契约继续走统一校验，仍待补更细粒度契约与更多技能 schema 覆盖 |
| 4.19 | EffectValidator - 硬约束校验 | 🔄 | | 已将缺失目标、超出射程/视线、冷却中、无法确认材料资源、interrupt 未就绪、被动场未激活转为硬失败，并把同一预检链路前置到 ReactionWindow skill option 生成；完整硬约束覆盖和测试矩阵仍未闭环 |
| 4.20 | KnowledgeRevealEvent 处理 | ✅ | 2026-05-03 | reveal.rs 完成 |
| 4.21 | HistoricalTruthResolver 实现 | ✅ | 2026-05-03 | historical_truth_resolver.rs 完成 |
| 4.22 | TemporalConsistencyValidator 实现 | ✅ | 2026-05-03 | temporal_consistency_validator.rs 完成 |

---

## 阶段五：Agent 模式 — 认知与叙事层

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 5.1 | PromptBuilder - AgentPromptBundle | ✅ | 2026-05-03 | `agent/prompting` 模块完成 bundle、统一消息布局与 provider request 构建 |
| 5.2 | PromptBuilder - 五类节点静态提示词 | ✅ | 2026-05-03 | 五类 Agent LLM 节点静态契约已内置 |
| 5.3 | PromptBuilder - 动态 JSON 输入 | ✅ | 2026-05-03 | 统一 `{ "input": <TInput> }` 结构化输入布局 |
| 5.4 | PromptBuilder - JSON schema 注入 | ✅ | 2026-05-03 | `ResponseFormat::JsonSchema` 与 bundle schema 元数据接线完成 |
| 5.5 | PromptBuilder - prompt version/hash | ✅ | 2026-05-03 | prompt template id/version/hash 与预算报告已实现 |
| 5.6 | SceneInitializer 实现 | ✅ | 2026-05-04 | `simulation::scene_initializer` 完成，含 PromptBuilder 接线、structured output schema 与基础边界校验 |
| 5.7 | SceneStateExtractor 实现 | ✅ | 2026-05-04 | `simulation::scene_extractor` 完成，含 raw_text 保真、authority 分类与 scene delta 基础校验 |
| 5.8 | CharacterCognitivePass 实现 | ✅ | 2026-05-04 | `cognitive::cognitive_pass` 完成，含 PromptBuilder 接线、structured output schema 与 Validator 硬错误拦截 |
| 5.9 | JSON 输出容错修复器 | ✅ | 2026-05-04 | `simulation::json_repair` 完成，支持缺逗号、尾随逗号、未转义引号、缺失字段修复 |
| 5.10 | OutcomePlanner 实现 | ✅ | 2026-05-04 | `simulation::outcome_planner` 完成，含 narratable_facts / actor_id 基础约束 |
| 5.11 | SurfaceRealizer 实现 | ✅ | 2026-05-04 | `presentation::surface_realizer` 完成，含 used_fact_ids 白名单校验 |
| 5.12 | AI Provider chat_structured - OpenAI Responses | ✅ | 2026-05-04 | `api/openai_responses.rs` 已支持 structured JSON 输出 |
| 5.13 | AI Provider chat_structured - OpenAI Chat Completions | ✅ | 2026-05-04 | `api/openai_chat.rs` 已支持 structured JSON 输出 |
| 5.14 | AI Provider chat_structured - Anthropic | ✅ | 2026-05-04 | `api/anthropic.rs` 已支持 structured JSON 输出 |
| 5.15 | AI Provider chat_structured - Gemini | ✅ | 2026-05-04 | `api/gemini.rs` 已支持 structured JSON 输出 |
| 5.16 | AI Provider chat_structured - DeepSeek | ✅ | 2026-05-04 | `api/deepseek.rs` 复用 OpenAI Chat structured output 路径 |
| 5.17 | AI Provider chat_structured - Claude Code Interface | ✅ | 2026-05-04 | `api/claude_code.rs` 已支持传入 schema 并解析 JSON 内容 |
| 5.18 | LLM 调用日志 - stream chunks | ✅ | 2026-05-04 | `logging::llm_logger` 新增 `log_stream_chunk` 与 `get_stream_chunks` 方法 |
| 5.19 | LLM 调用日志 - readable_text | ✅ | 2026-05-02 | `logging::llm_logger` 已生成 request / response 的 `readable_text` 展示字段 |
| 5.20 | Agent LLM 节点配置选择 UI | ✅ | 2026-05-04 | `AgentLlmConfigEditor.vue` 完成，支持五类节点独立配置与默认配置继承 |

---

## 阶段六：Agent 模式 — 验证与运行时

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 6.1 | 验证规则 - SelfAwareness | ✅ | 2026-05-04 | `validation::validator` 已校验 character_id / input 视角一致性 |
| 6.2 | 验证规则 - GodOnly | ✅ | 2026-05-04 | `validation::validator` 已拦截不可访问 belief_ref |
| 6.3 | 验证规则 - ApparentVsTrue | ✅ | 2026-05-04 | `validation::validator` 已检查 apparent / true 信息混用 |
| 6.4 | 验证规则 - ReactionWindow | 🔄 | | 已有窗口管理、explicit target 透传、客观关系 ally guard、真实 CharacterFacet 技能接线，以及基于 `EffectValidator::preview_skill_use` 的 passive/interrupt 预检；L2 感知细化与更复杂 interrupt 深度规则仍为 TODO |
| 6.5 | 验证规则 - NarrativeFactCheck | ✅ | 2026-05-04 | runtime 已实现 `used_fact_ids` 白名单校验 |
| 6.6 | 验证规则 - SchemaConformance | ✅ | 2026-05-04 | 各 Agent LLM 节点已接 `serde_json` 反序列化与字段边界校验 |
| 6.7 | 验证规则 - TemporalCanon | ✅ | 2026-05-04 | `validation::temporal_validator` 已实现 RequiredOutcome/ForbiddenOutcome/KnownAfterEffect 三类约束检查 |
| 6.8 | AgentRuntime 主循环 | ✅ | 2026-05-05 | `process_turn` 已形成可连续回合闭环：恢复最近 SceneSnapshot、加载角色临时状态、生成 event_delta、执行 ActiveSet/CognitivePass fallback、Outcome/Surface、单写提交、Trace 落库 |
| 6.9 | 固定快照机制 | ✅ | 2026-05-04 | `runtime/config_snapshot.rs` 实现 RuntimeConfigSnapshot/WorldRulesSnapshot/SnapshotManager，回合开始时捕获配置快照 |
| 6.10 | 并行 CognitivePass | ✅ | 2026-05-05 | ParallelCognitiveExecutor 已接入 DB-backed KnowledgeAccessProtocol 与 SceneFilter 构建真实 L2 输入；Embodiment 仍按最小派生继续增强 |
| 6.11 | 统一验证 | ✅ | 2026-05-04 | `TurnValidationResult` 已实现统一汇总，各节点验证结果聚合完成 |
| 6.12 | 单写提交 | ✅ | 2026-05-05 | `AgentRuntime::commit_state` 已接通 `StateCommitter` 热路径，统一写入 world_turns/state_commit_records |
| 6.13 | Dirty Flags 实现 | ✅ | 2026-05-04 | `DirtyFlags` 结构与触发条件计算已完成，含 hard/soft conditions |
| 6.14 | Active Set 实现 | ✅ | 2026-05-04 | `calculate_active_set` 已完成优先级排序与预算约束筛选 |
| 6.15 | 角色 Tier 分级 | ✅ | 2026-05-04 | `CharacterTier` (TierA/TierB/TierC) 已实现，用于 ActiveSet 优先级排序 |
| 6.16 | 调用预算监控 | ✅ | 2026-05-04 | `runtime/budget_monitor.rs` 已实现 BudgetMonitor/BudgetConfig/BudgetTraceEntry，集成到 AgentRuntime 主循环 |
| 6.17 | ReactionWindow 资格判定 | 🔄 | | 已接 explicit primary target、`objective_relationships` 盟友援护资格、CharacterFacet 技能归属与资源/冷却/视线预检；非目标型 interrupt 反应不再发放通用 dodge/counter，复杂 L2 感知与更细资源规则仍需继续补强 |
| 6.18 | ReactionIntent 收集 | ✅ | 2026-05-05 | AgentRuntime 已用真实角色/有效属性打开 ReactionWindow，并为 eligible reactor 收集 deterministic ReactionIntent；复杂 reaction pass 仍归 6.17 资格与技能规则补强 |
| 6.19 | 递归深度限制 | ✅ | 2026-05-04 | `ReactionWindow.max_reaction_depth = 1` 与 `no_reaction_to_reaction` 已实现 |
| 6.20 | 过去线冲突 UX | ✅ | 2026-05-04 | `runtime/conflict.rs` 实现 ConflictManager/ConflictReport/ConflictPolicyDecision，支持 NonCanonAfterConflict/WholeSessionNonCanon |
| 6.21 | Agent Trace 写入点 | ✅ | 2026-05-05 | AgentRuntime finalize 后写入 config_snapshots/turn_traces/agent_step_traces，并回填 commit trace_ids |

---

## 阶段七：用户角色扮演

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 7.1 | 用户角色选择 | ✅ | 2026-05-05 | 实现 PlayerMode 枚举，更新 AgentSession 结构，创建 SessionCreateDialog 组件 |
| 7.2 | 用户输入心理活动 / 言行 | ✅ | 2026-05-05 | 实现 InputPreparser，支持 *...*/[[...]]/引号//command 分段解析 |
| 7.3 | DirectorHint - 结果规划导演权 | ✅ | 2026-05-05 | 实现 OutcomeBias 结构，支持方向/角色偏向/事件建议/张力/节奏 |
| 7.4 | DirectorHint - 文风导演权 | ✅ | 2026-05-05 | 实现 StyleOverride 结构，支持基调/视角/细节级别/焦点/格式提示 |
| 7.5 | 同一 World 多时期会话创建 | ✅ | 2026-05-05 | 创建 agent store，增强 AgentWorldView 展示主线/过去线/未来预演会话列表，Session Launcher 支持时间锚点选择；Agent 聊天入口仍未开放 |
| 7.6 | 过去线补完 HistoricalEvent 开放细节 | ✅ | 2026-05-05 | 实现 ProvisionalTruthManager，增强 HistoricalTruthResolver 提取 open_detail_slots，添加 Tauri 命令和前端 API |
| 7.7 | 正史资格提升机制 | ✅ | 2026-05-05 | 实现 CanonStatusManager，管理会话正史资格判定和候选事实提升，集成 TemporalConsistencyValidator |

---

## 阶段八：优化与扩展

| # | 任务 | 状态 | 完成日期 | 备注 |
|---|------|------|----------|------|
| 8.1 | 性能优化 - 缓存 | ✅ | 2026-05-05 | TurnScopedCache/KnowledgeAccessCache/DerivedAttributeCache/SceneDerivedCache；补充 WorldInfoManager 世界书池容量与失效接口 |
| 8.2 | 性能优化 - 事件批处理 | ✅ | 2026-05-05 | BatchStateWriter 已提供单 SQLite 写事务批量落库入口与事务测试 |
| 8.3 | UI / UX 改进 | 🔄 | | 资源工作台与 API 配置页已接入；世界书 / 预设改为左侧文件选择器 + 条目/分区列表，左侧上下文栏滚动收敛到列表区域；主导航补齐 Regex，左侧上下文栏已接 ST 会话 / Agent 会话 / 角色卡 / 预设真实列表与点击动作；预设页已补齐 Sampler/Instruct/Context/System Prompt/Reasoning/Prompt 六个分区的实际编辑表单与 prompt 条目启停/编辑。2026-05-06 起新增前端模式分离规划：引入全局模式选择器、拆分 STShell / AgentShell / SharedShell，并把混合 `/library` 与混合导航迁移为独立 ST / Agent 首页与路由分区；同日主导航入口改用 `RouterLink` 真实 hash 链接，修复“角色卡 / 世界书 / API 配置”等左侧入口点击不稳定的问题。 |
| 8.4 | 高级 Trace 可视化 | ✅ | 2026-05-06 | `/logs` 已支持 Agent Trace 列表、Trace 详情 step 展示、step linked_request_id 跳转与 request 反查 |
| 8.5 | 测试覆盖 | 🔄 | | 已补 GodOnly 揭示、StateCommitter 非正史边界、附件脱敏、Provider 能力 fail-fast、附件 magic bytes 与缓存/批量事务测试；测试矩阵仍需继续扩大 |
| 8.6 | 插件系统 | ⏳ | | |
| 8.7 | 日志管理 UI - 大小统计 | ✅ | 2026-05-06 | `/logs` 顶部容量摘要已读取全局与 World 日志库大小、LLM/Event/Trace/chunk 计数 |
| 8.8 | 日志管理 UI - 30 天未更新 World 提示 | ✅ | 2026-05-06 | 后端 `get_log_storage_summary` 已按 World 最后日志时间与体积生成 `stale_prompt_required` |
| 8.9 | 日志管理 UI - 手动清理 / 导出 | ✅ | 2026-05-06 | 已支持当前筛选结果 JSON/CSV 导出；全局清理改为 preview plan + 用户确认后执行 retention |
| 8.10 | 应用数据目录 C 盘写入边界收紧 | ✅ | 2026-05-06 | 默认业务数据与 WebView 数据均写入应用 `data/`；应用不在 C 盘时拒绝 `RST_DATA_DIR` 指向 C 盘 |
| 8.11 | 日志页面规划 | ✅ | 2026-05-06 | 已补 `/logs` 页面信息架构、筛选/详情/Trace 跳转、容量清理、后端命令边界与 MVP 切片 |
| 8.12 | 内置预设提示词条目 | ✅ | 2026-05-06 | 改为直接采用 SillyTavern 风格默认 prompt 预设：`main/nsfw/dialogueExamples/jailbreak/worldInfoBefore/worldInfoAfter/charDescription/charPersonality/scenario/personaDescription/chatHistory`，默认预设与新建预设都带完整内容，运行时按 `prompt_order` 实际组装 |
| 8.13 | Tauri 生产构建样式稳定性 | ✅ | 2026-05-06 | Vite 生产构建改为单入口 CSS，首屏 Naive UI provider 不再异步拆分，CSP 明确允许运行时 style 注入，避免 build 产物退化为近似浏览器原生控件 |
| 8.14 | ST 会话级角色卡、世界书与 User Persona 设置 | ✅ | 2026-05-06 | 会话保存 `character_id`、`enabled_world_info` 多选与 `user_persona`；ST 会话侧栏三点菜单可编辑名称、角色卡、世界书和 Persona Description，运行时组装读取这些字段 |
| 8.15 | ST 会话页预设 / 主 API 选择与契约参数适配 | ✅ | 2026-05-07 | 会话页页头新增紧凑预设和主 API 配置选择器；`stream_openai` 实际控制流式路径；运行时按 `llm_api_contract` 裁剪/映射采样参数并修复 provider stream 请求体 |

---

## 统计

| 阶段 | 总任务 | 已完成 | 进行中 | 待开始 |
|------|--------|--------|--------|--------|
| 阶段一 | 14 | 14 | 0 | 0 |
| 阶段二 | 27 | 27 | 0 | 0 |
| 阶段三 | 52 | 52 | 0 | 0 |
| 阶段四 | 22 | 20 | 2 | 0 |
| 阶段五 | 20 | 20 | 0 | 0 |
| 阶段六 | 21 | 19 | 2 | 0 |
| 阶段七 | 7 | 7 | 0 | 0 |
| 阶段八 | 14 | 11 | 2 | 1 |
| **总计** | **177** | **170** | **6** | **1** |

---

## 更新日志

| 日期 | 更新内容 |
|------|----------|
| 2026-05-06 | 梳理 `E:\AIPlay\SillyTavern` Extension 启用和主流程交互：新增 `docs/78_st_extensions.md`，明确 manifest / hooks / eventSource / setExtensionPrompt / generate_interceptor，以及与世界书扫描、Regex、Prompt Manager 和 coreChat 提取过滤的关系 |
| 2026-05-06 | 加固左侧主导航跳转：将主导航入口从 `button + router.push` 改为 `RouterLink` 真实 hash 链接，点击图标或 hover 名称都会走同一链接目标，避免 WebView 下部分入口点击后不触发页面切换 |
| 2026-05-06 | 修复左侧主导航名称点击无响应：图标右侧 hover tooltip 改为可接收指针事件并冒泡到导航按钮，避免点击“角色卡 / 世界书 / API 配置”等名称时事件穿透到下层内容 |
| 2026-05-06 | 规划前端全局模式选择与双壳层分离：在 `41_frontend_interaction.md` 中新增 `Mode Select`、`STShell` / `AgentShell` / `SharedShell`、新路由分区 `/st/**` `/agent/**`、模式切换确认与最近模式恢复规则；同步更新实现计划与阶段 8.3 任务备注 |
| 2026-05-06 | 完善预设编辑 UI：预设页右侧六个分区从占位说明改为可实际编辑的表单，支持采样参数、Instruct、Context、System Prompt、Reasoning、Prompt 格式字段及 prompt 条目启停/编辑；保存后回读当前预设，避免前端草稿与落盘规范化内容脱节 |
| 2026-05-06 | 修复 ST 预设默认内容与提示词组装脱节：统一 Rust/前端为扁平 `PresetFile`，默认预设与新建预设改用 SillyTavern `Default.json` 风格 prompt 列表与格式字段，运行时按 `prompt_order` 实际注入 `main/worldInfoBefore/charDescription/charPersonality/scenario/personaDescription/dialogueExamples/jailbreak`，恢复系统内置预设与会话预设装配 |
| 2026-05-06 | 增加 ST 会话级角色卡绑定、世界书多选与 User Persona：会话文件保存 `character_id` / `enabled_world_info` / `user_persona`，侧栏三点菜单可编辑，会话发送与世界书匹配读取 Persona Description |
| 2026-05-06 | 修复 Tauri 生产构建 Naive UI 样式退化：关闭 CSS 分包、取消首屏 provider 异步拆分、收敛 vendor 手动分包并明确 CSP style 注入策略；入口 CSS 不再全局清零所有元素 margin/padding |
| 2026-05-06 | 继续修复生产构建页面布局异常：App Shell 新增 `route-host` 统一承载路由页面，补齐 `NLayout` / `NLayoutContent` 宽度与 flex 链路，并增加低优先级 Naive UI 按钮 / 卡片 / 输入控件结构兜底，避免内容区和按钮在 build 产物中按内容收缩 |
| 2026-05-06 | 修复 Tauri 生产构建导航与聊天输入布局：Vite build 明确 `base: './'`，左侧主导航改为固定尺寸图标按钮 + absolute hover tooltip，ST / Agent 聊天输入改为稳定 flex composer，并补齐根节点与滚动链路的 `min-width` / `min-height` / `overflow` 约束 |
| 2026-05-06 | 修复打包安装后资源工作台布局 / 图标尺寸异常：主导航默认进入折叠图标栏状态，并为 Naive UI `NIcon` / `BaseIcon` 增加静态 SVG 尺寸兜底，避免生产 WebView 启动阶段图标按 512 viewBox 撑开布局 |
| 2026-05-06 | 改造 ST / Agent 通用聊天消息展示：新增共享气泡组件，恢复 User/Assistant/System 全角色逐条渲染；支持安全 Markdown 子集、楼层/日期/时间/估算 token 元数据、单条删除/修改/复制；新增气泡颜色透明度与 Markdown 段落/标题/斜体/粗体/双引号内容字体样式配置 |
| 2026-05-06 | 优化 Tauri 冷启动白屏：新增零 JS 启动壳，根组件拆为轻量入口 + 异步 Naive UI provider，侧栏/检查面板延后加载，并改用 hash history 提高打包环境路由稳定性 |
| 2026-05-06 | 修复 ST 发送时报 `LLM API contracts snapshot not initialized`：contracts 改为运行时文件 + 编译期内嵌兜底，发送前可惰性补齐快照；同时发送失败只移除助手占位消息，保留并持久化用户本地消息 |
| 2026-05-06 | 修复 ST 会话实际发送失败：恢复内置预设条目后端构建、修复预设侧栏 TypeScript 阻断，OpenAI-compatible 文本请求改回字符串 `content` 以兼容 DeepSeek，并补齐会话/角色世界书稳定 ID 传递与发送错误提示 |
| 2026-05-06 | 实现内置预设提示词条目：参考 SillyTavern 设计，定义 6 个内置条目（`builtin:system_prompt`/`character_description`/`character_personality`/`scenario`/`world_info`/`chat_history`），后端加载时自动合并，前端显示内置标签和描述，内置条目不可删除，支持禁用开关和排序位置 |
| 2026-05-06 | 修复世界书 / 预设右侧编辑区滚动条消失：右侧 `NCard` 内容区选择器对齐 Naive UI `.n-card-content`，补 `min-height: 0` 后由编辑区内部滚动 |
| 2026-05-06 | 继续收敛世界书页面高度链：Shell 主 `NLayout` / `NLayoutContent` 改为 native 容器并禁用整行滚动，避免左侧标题区随主布局滚动 |
| 2026-05-06 | 修复世界书管理页左右滚动联动：Shell 左侧 `NLayoutSider` 不再自身滚动，世界书条目滚动条收敛到左侧条目列表区域，顶部标题 / 文件选择器 / 新建删除按钮保持固定 |
| 2026-05-06 | 修复左侧边栏多处点击无响应：上下文栏不再使用空占位列表，接入 ST 会话、Agent 会话、角色卡、预设文件/分区的真实数据与选择动作；侧栏新建/导入按钮转发到对应页面弹窗，并补齐 Regex 主导航入口 |
| 2026-05-06 | 修复预设默认值与运行时选择：存储层自动补齐 `./data/presets/Default.json`，全局状态收敛为单个 `active_preset`，聊天请求只传 `preset_name`；预设页按世界书页面布局调整为左侧文件/分区选择、右侧当前 section 编辑，并保留 API 连接与预设解耦 |
| 2026-05-06 | 完成日志页面开发：新增日志查询 / 详情 / stream chunk / Trace / 容量摘要 / 导出 / 清理预览确认命令，`/logs` 替换占位规划页并移除左侧“规划中”上下文项 |
| 2026-05-06 | 优化 Structured Text Editor 初始模式体验：未显式传入 mode 时按内容低开销推断 JSON / YAML / Plain，并将推断结果同步给父组件 |
| 2026-05-06 | 移除 JSON Format 字段重排行为：后端 `serde_json` 启用 `preserve_order`，格式化时保留输入对象字段顺序，并补嵌套对象顺序回归测试 |
| 2026-05-06 | 修复 Structured Text Editor 后端 Format 静默失败：Tauri 结构化文本 DTO 改为 camelCase 与前端一致，Format 后端失败时回退前端格式化并显示诊断 |
| 2026-05-06 | 修复 Structured Text Editor JSON key 引号自动修复：前端与后端统一改为结构感知扫描，支持同一行对象、嵌套对象和数组内对象的裸 key / 单引号 key 修复，并避免改写字符串值或数组值 |
| 2026-05-06 | 世界书条目列表卡片补充启用药丸开关与红色删除按钮；删除条目需确认，开关/删除操作不再触发行选中 |
| 2026-05-06 | 收敛世界书 / 预设资源页导航：左侧上下文面板改为文件选择器，选择后下方显示世界书条目或预设分区；右侧页面移除文件列表层级，只保留当前条目 / 分区编辑与文件级操作 |
| 2026-05-06 | 完成日志页面规划：补充 `/logs` 信息架构、筛选与详情面板、Trace 双向跳转、容量/清理/导出边界、Tauri 命令建议与 MVP 验收切片 |
| 2026-05-06 | 修复资源页无法打开：共享 `ConfigManager` 引用了不存在的 `UploadOutline` 图标，导致角色卡 / 世界书 / 预设页面动态加载失败；已改用 `CloudUploadOutline` 并通过前端构建 |
| 2026-05-06 | 收紧应用数据目录边界：`app_data_root` 禁止非 C 盘安装时把数据目录覆盖到 C 盘；Tauri 主窗口改为手动创建并将 WebView localStorage/cache/cookies/IndexedDB 固定到 `data/webview` |
| 2026-05-06 | 接通 Agent 技能真实数据链路：`CharacterFacet(KnownAbility/HiddenAbility)` 可解析为 runtime `Skill`，`ReactionWindow` 改为按技能归属与 `EffectValidator` 预检发放 passive/interrupt 选项，并补模型 / runtime / reaction 定向测试 |
| 2026-05-06 | 补强 Agent Reaction/Effect 热路径：ReactionWindow 改为透传 explicit target，接入 `objective_relationships` 盟友援护资格与 runtime 派生 passive/interrupt 反应选项；`validation::EffectValidator` 新增 passive/interrupt 契约硬失败校验，并补 reaction/effect 定向测试 |
| 2026-05-06 | 修复 PNG 角色卡导入识别：后端解析同时支持前置/后置 `tEXt`、`zTXt`、`iTXt` metadata，兼容 base64 与直接 JSON 载荷，并补角色卡导入回归测试 |
| 2026-05-06 | Agent 首页从固定 `default World` 摘要提升为真正的 World 列表/选择入口：新增 `list_agent_worlds` 命令与前端 world 列表状态，首页改为真实 World 列表、当前 World 摘要与联动最近会话；模式选择页与 Agent 相关页面不再硬编码回退到 `default` |
| 2026-05-02 | 初始化任务清单；阶段一全部完成 |
| 2026-05-02 | 完成任务 2.1 角色卡 V3 管理、2.2 头像上传与显示 |
| 2026-05-02 | 完成任务 2.3-2.7 世界书编辑器（基础 CRUD、分组管理、概率控制、递归扫描、时间控制） |
| 2026-05-02 | 完成任务 2.8-2.10 关键词触发系统（基础匹配、正则匹配、匹配目标扩展） |
| 2026-05-02 | 完成任务 2.11-2.15 Regex 扩展兼容（global/preset/scoped 脚本、prompt-only/display-only、内嵌脚本授权） |
| 2026-05-05 | 完善 Agent 模式：开放会话聊天入口，新增 process_agent_turn / list_agent_session_turns 命令与 session_turns 持久化；ParallelCognitiveExecutor 接入 KnowledgeAccessProtocol + SceneFilter 真实 L2 输入；EffectValidator 增加目标/距离/材料硬约束 |
| 2026-05-02 | 完成任务 2.16-2.21 预设系统（Sampler/Instruct/Context/SystemPrompt/Reasoning 预设） |
| 2026-05-02 | 完成任务 2.25 运行时组装（GlobalAppState、RuntimeContext、RequestAssembler、ProviderRequestMapper） |
| 2026-05-02 | 完成任务 2.26 世界书注入流程（WorldInfoInjector、来源合并、排序、扫描、落槽） |
| 2026-05-02 | 阶段三开始：完成 SQLite 表结构（补充 llm_call_logs/llm_stream_chunks/app_event_logs/log_retention_state） |
| 2026-05-02 | 完成三层语义隔离机制（KnowledgeAccessResolver 实现 GodOnly hard deny） |
| 2026-05-02 | 完成 WorldMainlineCursor/AgentSession/SessionTurn 存储层 |
| 2026-05-02 | 完成 KnowledgeStore CRUD 和访问派生索引 |
| 2026-05-02 | 完成 CharacterRecord 存储层 |
| 2026-05-02 | 清理任务清单重复行；更新 3.33-3.42 已完成任务状态（EmbodimentState/FilteredSceneView/CognitivePass I/O 等） |
| 2026-05-02 | 完成 TemporalStateRecord、ObjectiveRelationship、WorldStateAt 数据模型（session.rs） |
| 2026-05-02 | 完成 HistoricalEventContent 和 KnowledgeEntry content sub-schemas（knowledge.rs） |
| 2026-05-02 | 完成 SceneInitializationDraft、SurfaceRealizerInput 及相关类型（scene.rs） |
| 2026-05-02 | 完成 SubjectiveStateReducer 完整实现（subjective_reducer.rs） |
| 2026-05-02 | 完成 Agent World Editor 基础 CRUD、paused-only 提交、commit journal（world_editor/） |
| 2026-05-02 | **阶段三全部完成！** 数据模型层 52 个任务全部实现 |
| 2026-05-03 | 阶段四开始：完成任务 4.1-4.15（KnowledgeAccessProtocol、LocationResolver、RoutePlanner、AttributeResolver、EmbodimentResolver、SceneFilter、InputAssembly 等） |
| 2026-05-03 | 完成任务 4.16、4.17、4.20-4.22（PhysicsResolver 骨架、CombatMathResolver、KnowledgeRevealEvent、HistoricalTruthResolver、TemporalConsistencyValidator）；SceneFilter 与 EffectValidator 保留进行中状态 |
| 2026-05-03 | 阶段四大部分完成；程序化核心仍需补齐 SceneFilter 感知派生与 EffectValidator 硬约束 |
| 2026-05-03 | 阶段五开始：完成任务 5.1-5.5（PromptBuilder、五类节点静态契约、动态 JSON 输入、schema 注入、prompt version/hash、预算报告与统一消息布局） |
| 2026-05-04 | **阶段五全部完成！** 完成任务 5.9（JSON 输出容错修复器）、5.18（LLM 调用日志 stream chunks）、5.20（Agent LLM 节点配置选择 UI） |
| 2026-05-04 | 完成任务 5.6、5.7、5.10、5.11：实现 SceneInitializer、SceneStateExtractor、OutcomePlanner、SurfaceRealizer 四个 Agent LLM 节点，补充基础输出校验与 runtime 最小接线 |
| 2026-05-04 | 根据实际代码对齐任务状态：补记 5.8、5.12-5.17、5.19 为已完成；阶段六改为”部分完成 + 大量骨架进行中”，不再标记为全未开始 |
| 2026-05-04 | 完成阶段一任务 1.14 Structured Text Editor：落地共享 CodeMirror 6 Plain / JSON / YAML 编辑器，并接入 ST 世界书与 Agent Knowledge 原型编辑 |
| 2026-05-04 | 阶段六大部分结构已落地：AgentRuntime 主循环骨架、DirtyFlags、ActiveSet、StateCommitter 事务模块、ReactionWindow 结构与 TurnValidationResult 统一汇总；运行时热路径仍未闭环 |
| 2026-05-04 | 完成任务 6.7 TemporalCanon：实现 RequiredOutcome/ForbiddenOutcome/KnownAfterEffect 三类约束检查，支持过去线冲突检测 |
| 2026-05-04 | 完成任务 6.16 调用预算监控：实现 BudgetMonitor/BudgetConfig/BudgetTraceEntry，集成到 AgentRuntime 主循环，支持阈值监控与 Trace 记录 |
| 2026-05-04 | 完成任务 6.9 固定快照机制与 6.20 过去线冲突 UX；6.10 并行 CognitivePass 和 6.21 Agent Trace 写入点降为进行中，等待真实 L2 输入与 SQLite 落库接线 |
| 2026-05-04 | 修复 `src-tauri/src/agent/runtime` 当前编译/测试报错：对齐 `TraceRecorder` 测试参数类型、修正 `effective_max_context` 测试期望值，并复核 `cargo check` / `cargo test` 全通过 |
| 2026-05-04 | 修复 ST 安全/逻辑偏差：移除绕过 runtime gate 的公开聊天命令；附件日志改为摘要脱敏；接入 Regex prompt-only 到运行时组装；角色世界书绑定改为优先使用稳定 `rst_world_lore_id`；provider preview 改为复用真实映射链路；SQLite 日志库启动改为同步初始化，消除冷启动首笔请求漏记 |
| 2026-05-04 | 细化阶段七设计文档：明确 `Character/Director` 会话视角、RP 输入轻量标记与预解析、`/scene`/`/back` 命令面板交互，以及对应的运行时 / 持久化 / 前端边界；仅更新 spec，未变更阶段七任务完成状态 |
| 2026-05-04 | 扩展阶段七命令设计：新增 `/fork`，定义为复制当前 World 副本并进入；同步更新应用数据目录、时间线、Scene I/O、运行时、前端交互与实现计划文档；仅更新 spec，未变更任务状态 |
| 2026-05-05 | **阶段七开始！** 完成任务 7.1 用户角色选择：实现 PlayerMode 枚举（Character/Director），更新 AgentSession 结构添加 player_mode 字段，创建 SessionCreateDialog 组件，添加后端 Tauri 命令 |
| 2026-05-05 | 完成任务 7.2 用户输入预解析器：实现 InputPreparser，支持 *...*/[[...]]/引号//command 分段解析，生成 PreparsedUserInput 结构，含测试覆盖 |
| 2026-05-05 | 完成任务 7.3、7.4 DirectorHint 导演权：实现 OutcomeBias（结果规划偏置）和 StyleOverride（文风覆盖）结构，支持方向/角色偏向/事件建议/张力/节奏/基调/视角/细节级别等提示 |
| 2026-05-05 | 完成任务 7.5 同一 World 多时期会话创建：创建 agent store（`src/stores/agent.ts`），增强 AgentWorldView 展示主线/过去线/未来预演会话列表，Session Launcher 支持时间锚点选择和 session_kind 自动推导 |
| 2026-05-05 | 完成任务 7.6 过去线补完 HistoricalEvent 开放细节：实现 ProvisionalTruthManager 管理候选事实的创建/校验/提升，增强 HistoricalTruthResolver 从 HistoricalEventContent 提取 open_detail_slots 和 hard_constraints，添加 Tauri 命令和前端 API |
| 2026-05-05 | **阶段七全部完成！** 完成任务 7.7 正史资格提升机制：实现 CanonStatusManager 管理会话正史资格判定和候选事实提升，集成 TemporalConsistencyValidator 校验，添加 evaluate_canon_eligibility/promote_to_canon/get_session_conflicts 命令 |
| 2026-05-05 | **阶段八开始！** 完成任务 8.1 性能优化 - 缓存：实现 TurnScopedCache（回合内缓存）、KnowledgeAccessCache（知识访问缓存）、DerivedAttributeCache（属性派生缓存）、SceneDerivedCache（场景派生缓存），集成到 KnowledgeAccessProtocol/AttributeResolver/SceneFilter/EmbodimentResolver |
| 2026-05-05 | 任务 8.2 进入进行中：BatchLogWriter / BatchTraceWriter / BatchStateWriter 结构与队列已实现，但 SQLite 批量事务仍未落地 |
| 2026-05-05 | 任务 8.3 进入进行中：资源工作台与 API 配置页已接入真实数据；Agent 工作区、日志 UI、Regex / Preset 页面仍有原型或占位界面 |
| 2026-05-05 | 审查后修正任务状态：AgentRuntime `commit_state`、Scene 初始化、Reaction pass、Trace 落库、批量落库、EffectValidator 完整硬约束和测试矩阵尚未闭环；总完成数调整为 154/172 |
| 2026-05-05 | 完善 AgentRuntime 最小运行闭环：接通 `StateCommitter` 单写提交、Trace/Step Trace SQLite 落库、真实 user_message 持久化与 deterministic fallback；新增 `process_turn_persists_commit_and_trace` 测试，任务 6.12/6.21 标记完成，总完成数调整为 156/172 |
| 2026-05-05 | 完成 AgentRuntime 主循环补强：按 session 恢复最近 SceneSnapshot、提交完整 SceneModel 快照、串联 parent_turn_id、加载角色 temporary_state、生成 ObservableEventDelta、补齐有效属性派生与 deterministic cognitive/reaction fallback；双回合测试覆盖 scene 连续性与 trace/commit 落库，任务 6.8/6.18 标记完成，总完成数调整为 158/172 |
| 2026-05-05 | 处理高风险测试与性能/安全项：补 GodOnly/StateCommitter/附件脱敏/Provider fail-fast 可执行测试；附件按 magic bytes 校验并改异步读写；Gemini 上传不把 key 拼进 URL 字符串；CSP 收窄到固定本机 dev 端口；Vite 拆分 editor/naive-ui chunk；BatchStateWriter 增加真实 SQLite 批量事务 |
| 2026-05-05 | 补强 EffectValidator 热路径：StateUpdatePlan 技能更新按 actor/target 校验冷却、材料、目标数量、范围、视线与激活条件；补 4 个定向单元测试；顺手补齐 DeepSeek/Claude Code `list_models` 与 Anthropic capabilities 序列化以恢复 Rust 编译通路 |
| 2026-05-05 | 调整前端 Shell 与工作区布局：主导航固定为图标栏，ST / Agent / API 页面不再叠加全局上下文列表，补充 flex/grid `min-width` / `min-height` 约束以减少视口溢出 |
| 2026-05-05 | 收敛世界书界面多级导航：`/resources/worldbooks` 使用页面内表格承担文件打开 / 删除，不再叠加 Shell 上下文列表 |
| 2026-05-05 | 修复 API 配置页 Key 持久化链路：获取模型前先保存草稿连接信息到 `data/api_configs`，并让 settings store 向页面抛出保存失败 |
| 2026-05-05 | 修复 API 配置页保存按钮禁用条件：保存连接配置不再强制要求先填写模型，模型校验留给实际请求路径 |
| 2026-05-05 | 修复 ST 聊天入口空会话误显示永久加载：拆分初始化加载状态与空列表状态，并补齐 `sessionId` 路由会话同步 |
| 2026-05-06 | 修复角色卡资源页：列表接口返回真实角色资源 ID，前端编辑 / 导出 / 删除不再误用数组下标；移除角色卡页外层上下文列表，收敛卡片内容预览，并让编辑加载失败显示错误 |
| 2026-05-06 | 修复角色卡详情弹窗加载状态：详情读取改为组件本地 loading / error，不再复用角色卡列表全局 `isLoading` 导致点击卡片时只看到整页加载遮罩 |
| 2026-05-06 | 调整角色卡资源页 UI：左侧上下文列表负责角色选择，右侧内容区直接展示当前角色卡详情与导入 / 导出 / 删除操作，移除卡片网格和编辑弹窗路径 |
| 2026-05-06 | 角色卡详情接入自动保存：字段变更标记 dirty，右侧编辑区整体失焦、切换角色或卸载离开时自动保存当前角色卡 |
| 2026-05-06 | 依据 `E:\AIPlay\cccode` 源码修正 Claude Code Interface：契约改为 Messages API `/v1/messages` + `Authorization: Bearer`，实际 provider 改用顶层 system blocks、user/assistant messages、tool schema 结构化输出和 Anthropic Messages 响应解析 |
| 2026-05-06 | 修复 ST 预设宏与世界书注入断链：新增统一 `st/macros` 替换层，恢复 `{{char}}` / `{{user}}` / `{0}` / `{{wi}}` 等宏在 preset、角色卡字段、Persona、世界书关键词与内容中的运行时展开；世界书扫描补上 role->角色/Persona 名回退；默认 prompt_order 将 `personaDescription` 重新纳入主链，并补 Rust 回归测试 |
| 2026-05-06 | 补齐 Agent World 创建链路：新增 `create_agent_world` Tauri 命令，初始化 `data/worlds/<world_id>/`、`world.sqlite`、`world_base.yaml` 与主线光标；前端 Agent 首页 / World 页增加新建 World 入口与零 World 空状态修复，不再出现“以 World 为顶层却无法创建 World”的断头流程 |
| 2026-05-07 | 修复 ST 会话流式发送路径：补充 Tauri event listen/unlisten capability，发送 store 等待真实 stream end 后再结束生成状态，并用响应式替换方式更新 assistant 消息气泡内容与滚动位置 |
| 2026-05-07 | 修复 ST 聊天响应为空与配置选择问题：provider 流式请求补 `stream` 字段并改用带缓冲 SSE 解码，ST 会话页可直接选择当前预设 / 主 API，发送时尊重预设流式开关，运行时按当前 API 契约过滤和近似映射采样参数 |
| 2026-05-07 | 将 `E:\AIPlay\cards\夏瑾DS预设v0.40.json` 对齐为本应用预设实际标准：文档改为 ST 扁平 `PresetFile` 主标准，预设编辑页补齐顶层 ST 字段与 PromptItem 注入元数据编辑，运行时改为按 `prompts + prompt_order + chatHistory` 前后切分装配 system prompt 与消息链 |
| 2026-05-07 | 修复预设 `ContextList` 提示词条目拖拽失效：确认 `prompt_order` 数据层无禁排约束后，将拖拽源收敛回拖拽手柄，恢复交互区事件隔离，并在列表项容器上显式处理 `dragover/drop`，避免整卡拖拽导致排序无法落到目标条目 |
