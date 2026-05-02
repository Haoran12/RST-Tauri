# RST-Tauri 实现计划

Ran's SmartTavern：基于 Tauri 的双模式 AI 聊天应用。

> 本文仅承载**项目路线图**：阶段、里程碑、技术栈选型、关键决策。
> 数据模型、架构细节、运行时主循环等 spec 内容已拆分到独立文档，见 [README.md](../README.md) 索引。

## 项目概述

- **SillyTavern 模式**：复刻 SillyTavern 体验，支持角色卡 V3 PNG / JSON、外部世界书、预设、Regex 与 API 配置。ST 兼容资源使用 SillyTavern 文件形态：角色卡以 PNG metadata 为主，外部世界书以 `entries: Record<string, WorldInfoEntry>` JSON 保存，API 连接配置与角色卡 / 世界书 / 预设解耦。总览见 [70_st_mode.md](70_st_mode.md)，角色卡见 [71_st_character_cards.md](71_st_character_cards.md)，世界书模型见 [72_st_worldbook_model.md](72_st_worldbook_model.md)，注入流程见 [73_st_worldbook_injection.md](73_st_worldbook_injection.md)，预设见 [74_st_presets.md](74_st_presets.md)，运行时组装见 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)，Regex 见 [76_st_regex.md](76_st_regex.md)。
- **Agent 模式**：基于 RP Agent 架构的高级角色扮演系统，分层"客观世界 / 人物具身状态 / 主观认知与意图 / 结果规划与状态更新 / 叙事输出"，SQLite 存储。数据模型入口见 [10_agent_data_model.md](10_agent_data_model.md)，运行时见 [11_agent_runtime.md](11_agent_runtime.md)，程序化派生见 [12_agent_simulation.md](12_agent_simulation.md)，对抗技能见 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)，LLM I/O 入口见 [13_agent_llm_io.md](13_agent_llm_io.md)，场景节点见 [21_agent_scene_llm_io.md](21_agent_scene_llm_io.md)，结果/叙事节点见 [22_agent_outcome_narration_io.md](22_agent_outcome_narration_io.md)，持久化见 [14_agent_persistence.md](14_agent_persistence.md)，地点系统见 [15_agent_location_system.md](15_agent_location_system.md)，世界编辑器见 [40_agent_world_editor.md](40_agent_world_editor.md)。
- **前端 UI**：应用 Shell、一级页面、路由、资源列表、检查面板、主题 token 与关键工作流见 [41_frontend_interaction.md](41_frontend_interaction.md)。跨 ST / Agent 的 Plain / JSON / YAML 结构化文本编辑器见 [42_structured_text_editor.md](42_structured_text_editor.md)。

> **架构基础**：参考 `D:\Projects\RST-flutter\docs\rp_agent_*` 系列文档（成熟的角色扮演 Agent 架构），本项目在其基础上为 Tauri + Rust + Vue 3 技术栈做适配。

---

## 1. 技术栈选型

| 层 | 选型 | 理由 |
|---|---|---|
| 前端框架 | Vue 3 + TypeScript | 生态成熟、组合式 API、Pinia 类型友好 |
| UI 组件库 | Naive UI | Vue 3 原生支持，组件丰富，TypeScript 友好，暗色主题完善 |
| 文本编辑器 | CodeMirror 6 | 成熟的浏览器编辑器内核；支持 language extensions、lint diagnostics、bracket matching、undo history、keymap 和主题扩展 |
| 状态管理 | Pinia | 内置、类型安全 |
| 路由 | Vue Router | 标准方案 |
| 后端 | Tauri + Rust | 小型二进制、跨平台、安全 |
| 存储 - ST 模式 | PNG 角色卡 + JSON 资源文件 | 与 SillyTavern 兼容；角色卡 PNG metadata、世界书 JSON、预设 JSON 分别保存 |
| 存储 - Agent 模式 | SQLite | 结构化查询、事务、性能 |
| 日志与 Trace | SQLite | LLM 请求响应、Agent Trace、异常事件需要按 turn/request 查询 |
| AI 后端 | 多 Provider / 多协议 | OpenAI Responses / OpenAI Chat Completions / Gemini / Anthropic / DeepSeek / Claude Code Interface |

---

## 2. 实现阶段

### 阶段一：基础框架 (MVP)

1. 初始化 Tauri + Vue 3 + TypeScript + Naive UI。
2. 配置 Vue Router + Pinia。
3. 实现 App Shell、资源工作台默认首页、基础主题和响应式布局。
4. 实现 JSON 存储层。
5. 基础聊天 + AI Provider 抽象。
6. 集成 OpenAI Responses API、OpenAI Chat Completions API、DeepSeek API。
7. 全局运行 Logs：`./data/logs/app_logs.sqlite` + Provider logging wrapper + 可配置清理上限（默认 1GB）。
8. 基于 CodeMirror 6 的结构化文本编辑器：Plain / JSON / YAML 模式、语言包注册表、编辑时缩进、括号 / 引号诊断、JSON key quick fix 和父级 draft 集成。

### 阶段二：SillyTavern 模式

1. ST 资源存储骨架：角色卡目录、头像 / 缩略图缓存、世界书目录、预设目录、Regex 设置和 RST 内部 ID 到 ST 文件名 / avatar stem 的映射。
2. 角色卡 V3 管理：创建 / 编辑 / 删除；PNG `tEXt` metadata 解析与写入；`ccv3` 优先、`chara` 兜底；JSON 导入 / 导出；未知顶层字段、`data` 字段和 `data.extensions` 字段完整保留。
3. 头像上传、存储与显示：PNG 角色卡本身作为头像容器；JSON 导入必须选择或生成 PNG 容器；更换头像时重写当前 TavernCard metadata；角色列表和聊天头像使用同一 PNG 或派生缩略图。
4. 世界书数据层：外部世界书持久化为 `entries: Record<string, WorldInfoEntry>`；`WorldInfoEntry[] / WorldbookEntry[]` 只作为 UI / 排序 / 运行时扫描临时视图；补齐 `WorldInfoEntry` 全字段和 `WorldInfoLogic / WorldInfoPosition / ExtensionPromptRole` 枚举。
5. 角色卡内嵌 CharacterBook 导入：保留 `data.character_book`；用户执行 Import Card Lore 时转换为外部世界书并写入 `originalData`，再把 `data.extensions.world` 绑定到世界书名称；未导入前不参与运行时扫描。
6. 世界书编辑器（含分组 / 概率 / 递归 / 时间控制 / characterFilter / triggers / depth / outlet）。
7. 世界书注入运行时：复刻 ST 的 Chat lore / Persona lore / Global lore / Character lore 来源合并、去重、排序、扫描、递归、预算和 Prompt 落槽；来源选择不得读取 API 配置。
8. 关键词触发系统（含正则 / 匹配目标扩展）。
9. Regex 扩展兼容（global / preset / scoped 脚本、prompt-only / display-only、内嵌脚本授权）。
10. 预设系统：Sampler / Instruct / Context / System Prompt / Reasoning / Prompt Preset 导入导出；保留 ST 原字段，但运行时与 API Provider 连接配置解耦。
11. 多 API 支持（加入 Anthropic Messages / Gemini / Claude Code Interface；GLM、本地模型或 Ollama 可作为后续扩展）；切换 API 配置不得改写角色卡、世界书、预设、Regex allow list 或聊天 metadata。

### 阶段三：Agent 模式 — 数据模型层

1. SQLite 表结构 + 三层语义隔离（Layer 1 / Layer 3 / Trace）。
2. WorldMainlineCursor + AgentSession / SessionTurn：同一 World 多时期、多人物会话入口，聊天顺序与 canonical Truth 分离。
3. TimeAnchor / story_time_anchor / WorldStateAt 基础类型：提交时间与故事时间分离。
4. SceneModel + ManaField + PhysicalConditions 完整定义（Layer 1）。
5. LocationGraph（LocationNode / LocationSpatialRelation / LocationEdge / alias / polity template）+ 地区事实继承字段。
6. KnowledgeEntry 体系（kind / subject / access_policy / subject_awareness / apparent_content）+ 访问派生索引表 + L1 客观关系 / 授权表。
7. HistoricalEvent / TruthGuidance / provisional_session_truth / ConflictReport：过去线 Truth 引导、开放细节补完和非正史分支记录。
8. KnowledgeEntry content sub-schemas（每种 facet/fact 类型的核心字段 + extensions 兜底）。
9. CharacterRecord（base_attributes + baseline_body_profile + mind_model_card + temporary_state）。
10. TemporalStateRecord / WorldStateAt：角色位置、临时状态、地点状态、客观关系等可变化 L1 状态的时态记录。
11. CharacterSubjectiveState（Layer 3）+ SubjectiveStateReducer。
12. EmbodimentState / FilteredSceneView / AccessibleKnowledge（Layer 2 派生类型）。
13. CognitivePass I/O 类型（含 ConfidenceShift / BodyReactionDelta）。
14. SceneInitializationDraft / SceneStateExtractorOutput / UserInputDelta / StyleConstraints / OutcomePlannerOutput / SurfaceRealizerInput。
15. Agent Trace / LLM Logs / app_event_logs 表结构。
16. AgentLlmProfile / World Agent settings（五类 Agent LLM 节点独立绑定 API 配置）。
17. Agent World Editor：结构化 World settings / LocationGraph / KnowledgeEntry / CharacterRecord CRUD、paused-only 提交、影响分析与 editor commit journal。

### 阶段四：Agent 模式 — 程序化核心

1. KnowledgeStore（Layer 1 CRUD）。
2. KnowledgeAccessResolver（统一 Knowledge 访问权限最终判定，三谓词合并）。
3. KnowledgeAccessProtocol（SQLite 派生索引预筛 + KnowledgeAccessResolver 裁剪，构建 AccessibleKnowledge；支持按 TimeAnchor 查询当时可知信息）。
4. LocationResolver / LocationFactResolver / RoutePlanner（地点消歧、父级事实继承、自然地理影响、路线与路程估算）。
5. AttributeResolver + EmbodimentResolver（含基础属性有效值、灵觉 + 环境档位翻译）。
6. SceneFilter（含 observable_facets 计算 + WeatherPerception + ManaSignal）。
7. InputAssembly（拒绝 Layer 1 原始对象）。
8. PhysicsResolver / CombatMathResolver（Mana Combat Resolution 公式等）+ EffectValidator（技能契约与硬约束校验）。
9. KnowledgeRevealEvent 处理。
10. HistoricalTruthResolver + TemporalConsistencyValidator：过去线 TruthGuidance 生成、冲突检测和正史资格判定。

### 阶段五：Agent 模式 — 认知与叙事层

1. PromptBuilder（`AgentPromptBundle`、五类节点静态提示词、动态 JSON 输入、JSON schema 注入、prompt version/hash、`PromptBudgetReport`、输入 token 估算与 P0/P1/P2/P3 裁剪）。
2. SceneInitializer（SceneSeed + 公开上下文 + 场景相关私有约束 + generation_policy → SceneInitializationDraft，严格 schema）。
3. SceneStateExtractor（最近自由文本 + 当前 Scene JSON + 场景相关私有约束 → SceneStateExtractorOutput，严格 schema）。
4. CharacterCognitivePass（融合调用，严格 schema 输出；prior L3 心智模型焦点作为高优先级输入）。
5. JSON 输出容错修复器（缺字段补默认 / 修复常见结构错误）。
6. OutcomePlanner（结果规划与状态更新计划，God-read 但不直接提交）。
7. SurfaceRealizer（叙事生成，`chat_structured` 返回 `SurfaceRealizerOutput { narrative_text, used_fact_ids }`，受结构化 narratable_facts 约束）。
8. AI Provider 的 chat_structured 实现（OpenAI Responses / OpenAI Chat Completions / Anthropic / Gemini / DeepSeek / Claude Code Interface 各自的 structured output / tool schema / JSON 降级路径）。
9. LLM 调用日志：request / response / schema / stream chunks / readable_text。
10. Agent LLM 节点配置选择 UI：SceneInitializer / SceneStateExtractor / CharacterCognitivePass / OutcomePlanner / SurfaceRealizer 分别选择 API 配置。

### 阶段六：Agent 模式 — 验证与运行时

1. 验证规则（含 SelfAwareness / GodOnly / ApparentVsTrue / ReactionWindow / NarrativeFactCheck / SchemaConformance / TemporalCanon）。
2. AgentRuntime 主循环（固定快照、并行 CognitivePass、统一验证、单写提交）。
3. Dirty Flags + Active Set。
4. 角色 Tier 分级。
5. 调用预算监控（默认 primary CognitivePass 0-3 次、可配置分层阈值、16K 软上限、可配置最大上下文、MinorActorSlot 兜底）。
6. ReactionWindow 有界反应窗口：资格判定、ReactionIntent 收集、递归深度限制。
7. 过去线冲突 UX：硬冲突只警告不中断，用户选择冲突后非正史或整条非正史。
8. Agent Trace 写入点：Active Set、Dirty Flags、Layer 2 派生、CognitivePass、ReactionWindow、验证、结果规划、提交。

### 阶段七：用户角色扮演

1. 用户角色选择。
2. 用户输入心理活动 / 言行。
3. 用户对结果规划 / 文风的"导演"权（DirectorHint）。
4. 同一 World 下创建当前主线、过去线和未来预演会话。
5. 过去线补完既有 HistoricalEvent 的开放细节，并按正史资格提升或保留为非正史。

### 阶段八：优化与扩展

1. 性能优化（缓存 / 事件批处理）。
2. UI / UX 改进。
3. 高级 Trace 可视化。
4. 测试覆盖。
5. 插件系统。
6. 日志管理 UI 增强：大小统计、30 天未更新 World 提示、手动清理 / 导出。

---

## 3. 关键文件

### 前端

- `src/stores/agent.ts` — Agent 状态管理。
- `src/stores/agentWorldEditor.ts` — Agent 世界编辑器 draft、validation、impact 与提交状态。
- `src/services/api.ts` — Tauri IPC 封装。
- `src/components/shared/structured-text-editor/StructuredTextEditor.vue` — ST / Agent 共用的 CodeMirror 6 Plain / JSON / YAML 大文本编辑器。
- `src/components/shared/structured-text-editor/cm6Setup.ts` — CodeMirror 6 extension 组合、theme、language compartments、lint 与 keymap 配置。
- `src/components/shared/structured-text-editor/languageRegistry.ts` — Structured Text Editor 语言包注册表，管理 builtin / bundled / trusted_plugin language support。
- `src/types/structuredText.ts` — StructuredTextBinding / Draft / Diagnostic 类型。
- `src/components/agent/world-editor/WorldEditorShell.vue` — Agent 世界编辑器主界面。
- `src/components/agent/CharacterMindView.vue` — 心智视图。
- `src/components/agent/ValidationReport.vue` — 验证报告。

### 后端

- `src-tauri/src/agent/runtime.rs` — 主循环。
- `src-tauri/src/agent/knowledge/store.rs` — 知识库 CRUD。
- `src-tauri/src/agent/knowledge/access_policy.rs` — Knowledge 访问权限唯一入口。
- `src-tauri/src/agent/knowledge/access.rs` — AccessibleKnowledge 构建。
- `src-tauri/src/agent/knowledge/reveal.rs` — 揭示事件处理。
- `src-tauri/src/agent/location/resolver.rs` — 地点别名解析与父级链构建。
- `src-tauri/src/agent/location/fact_resolver.rs` — 地区事实继承、自然地理影响与访问裁剪。
- `src-tauri/src/agent/location/route_planner.rs` — 路线图与路程估算。
- `src-tauri/src/agent/cognitive/cognitive_pass.rs` — 融合调用。
- `src-tauri/src/agent/simulation/scene_filter.rs` — 场景过滤（含 observable_facets）。
- `src-tauri/src/agent/simulation/attribute_resolver.rs` — 基础属性 effective 值、AttributeTier / AttributeDelta 派生。
- `src-tauri/src/agent/simulation/input_assembly.rs` — 拒绝 Layer 1 泄露。
- `src-tauri/src/agent/simulation/reaction_window.rs` — 有界反应窗口资格判定与 ReactionOption 派发。
- `src-tauri/src/agent/simulation/physics_resolver.rs` — 物理与灵力数值骨架。
- `src-tauri/src/agent/simulation/effect_validator.rs` — 技能契约与候选效果硬校验。
- `src-tauri/src/agent/simulation/outcome_planner.rs` — OutcomePlanner LLM 编排候选结果。
- `src-tauri/src/agent/validation/validator.rs` — 验证器入口。
- `src-tauri/src/agent/world_editor/validator.rs` — World Editor patch 校验。
- `src-tauri/src/agent/world_editor/commit.rs` — World Editor paused-only 单事务提交与 editor commit journal。
- `src-tauri/src/agent/models/knowledge.rs` — KnowledgeEntry 定义。
- `src-tauri/src/agent/models/location.rs` — LocationNode / LocationSpatialRelation / LocationEdge 定义。
- `src-tauri/src/agent/models/scene.rs` — 场景模型。
- `src-tauri/src/agent/models/mana_field.rs` — 灵力场。
- `src-tauri/src/storage/sqlite_store.rs` — 存储层。
- `src-tauri/src/config/loader.rs` / `validator.rs` / `registry.rs` — 运行配置加载、校验与快照发布。
- `src-tauri/src/text_format/json.rs` / `yaml.rs` — 保存前 JSON / YAML 复检、格式化与 diagnostics。
- `src-tauri/src/logging/llm_logger.rs` — Provider logging wrapper。
- `src-tauri/src/logging/event_logger.rs` — 应用异常事件日志。
- `src-tauri/src/logging/retention.rs` — 读取 `RuntimeConfigSnapshot` 的日志清理策略。

---

## 4. 验证与里程碑

风险登记见 [90_pitfalls_and_tests.md](90_pitfalls_and_tests.md)，每阶段交付的验证清单见 [91_test_matrix.md](91_test_matrix.md)。

---

## 5. 关键决策记录

> 文档维护原则：修改时直接更新最新版，不保留历史对比、版本演进或"改进前后"标记。重大架构决策与变更走 git commit 记录，本节仅总结当前生效的核心选择。

| 主题 | 决策 | 理由 |
|---|---|---|
| 数据形态铁律 | 自由文本仅在三处出现：用户输入、SceneStateExtractor 输入、SurfaceRealizer 输出。SceneInitializer 只接收结构化 SceneSeed、llm_readable 公开上下文与场景相关私有约束，其他全程结构化 JSON | 避免规则匹配失效与"屎山"起点 |
| 三层数据语义 | Layer 1 (Truth) / Layer 2 (Per-Character Access) / Layer 3 (Subjective)，强制隔离 | 受限 LLM 不接触 Layer 1 原始对象；God-read 节点只产出候选更新，防止全知泄露与直接写状态 |
| 同 World 多时期会话 | World 维护 `WorldMainlineCursor`；`AgentSession.period_anchor` 早于主线光标时进入过去线，同一 World 多份聊天共享 canonical Truth | 用户可扮演不同时期 / 不同人物补完世界，但聊天会话不等于独立世界状态 |
| 过去线正史资格 | 过去线读取结构化 TruthGuidance 引导场景与仲裁；硬冲突只警告不中断，用户选择冲突后非正史或整条非正史 | 保留自由游玩体验，同时防止矛盾内容污染正史 |
| 正史与 provisional truth | 过去线新细节先写 `provisional_session_truth`；只有无冲突且仍具正史资格时才提升为 canonical Knowledge / Event | 避免生成过程中把后续可能废弃的细节直接写入世界真相 |
| 知识统一模型 | KnowledgeEntry 统一承载世界/势力/角色档案/记忆，按 access_policy 谓词控制，并维护 SQLite 访问派生索引 | 单一访问权限入口（KnowledgeAccessResolver），索引只做候选预筛 |
| ST 资源兼容形态 | ST 模式角色卡以 PNG metadata 为主，JSON 只作为导入/导出形态；外部世界书持久化必须是 `entries: Record<string, WorldInfoEntry>`，数组只作 UI / 运行时临时视图；预设和 Regex 保留 ST 原字段 | 避免导入导出与 SillyTavern 真实代码不兼容，同时防止编辑器内部视图污染文件格式 |
| ST API 解耦边界 | API 配置只保存 provider、endpoint、model、key、代理和超时；角色卡、世界书、预设、聊天 metadata 与 Regex allow list 不以 API 配置分组，切换 API 配置不改写任何 ST 资源文件 | 支持同一 ST 资源跨 Provider 使用，避免连接配置切换造成角色/世界/预设副作用 |
| 结构化文本编辑器 | ST 与 Agent 共用基于 CodeMirror 6 的 Structured Text Editor，首版内置 Plain / JSON / YAML，并通过受控语言包注册表支持后续扩展；ST content 即使以 JSON / YAML 或其他文本语言编辑也保存为 string，Agent structured content 必须解析为 `serde_json::Value` 后再交给业务 validator | 复用成熟编辑器能力，避免自研光标、选择、undo、缩进和 lint UI；第三方语言包属于可执行前端代码，必须走受信插件或预装机制 |
| 地点系统 | LocationNode.parent_id 表达层级归属；LocationSpatialRelation 表达自然地理覆盖 / 穿过 / 重叠；LocationEdge 带权图表达相邻 / 路线；RegionFact 可沿 parent 链继承 | 支持地点归属、自然地理影响、地区事实默认适用与路程估算，同时避免把弱推断固化成硬设定 |
| Agent 世界编辑器 | 首版只做结构化 CRUD：World settings、LocationGraph、KnowledgeEntry、CharacterRecord 与 L1 关系 / 授权；运行中提交必须 paused-only，并写独立 editor commit journal | 支持开局前建世界与运行中安全修订，同时避免作者编辑伪装成运行回合或绕过派生索引 / 校验边界 |
| 场景域 God-read | SceneInitializer / SceneStateExtractor 可读取程序裁剪后的当前场景相关私有约束；不得全库读取隐藏 Knowledge / GodOnly，也不得把私有约束写成外显事实 | 避免初始化和输入解析与隐藏真相冲突，同时限制泄露面 |
| 基础属性档位 | 6 档（Mundane / Awakened / Adept / Master / Ascendant / Transcendent），默认边界对 `rp_cards\` 的 mana_power 锚点校准 | 六项基础属性共用档位/差距机制；raw f64 不进受限 LLM |
| 灵力显露两层模型 | 持久倾向 3 档（Inward / Neutral / Expressive）+ 运行时状态 5 档（Sealed / Suppressed / Natural / Released / Dominating） | 区分人物体质/性格/修行体系导致的默认外显倾向，和场景中有意/无意/被迫做出的封息、抑制、自然、外放、威压状态 |
| 感知 vs 对抗解算分离 | 感知用 `displayed_mana_power`（含显露倾向、运行时状态、压制、伪装），对抗解算用 `effective_mana_power`（不含显露倾向、运行时状态与压制） | 压制/外放是认知层与环境压力手段，不直接改变真实对抗 |
| 对抗解算公式 | `combat_power = effective × max(0.1, 1 + Σ_modifiers) × soul_factor`，加算修正区 + 灵魂独立乘区 | 多因子加和可控，灵魂破损保留乘性凸显质变打击 |
| 跨档差阈值 | 感知层 150 / 300 / 1000 / 2000；对抗解算共享 150 / 300 / 1000，1000+ 即 Crushing | 150 起感觉差距、1000+ 基本无力应对，2000+ 进入无法测度的体感描述 |
| 运行配置策略 | 默认配置 + `app_runtime.yaml` + World `world_base.yaml` 合并校验后发布 `RuntimeConfigSnapshot` / `WorldRulesSnapshot` | 阈值和清理策略可配置，但 Resolver / Filter / Retention 热路径不做配置 IO |
| LLM 数值字段 | 用 ConfidenceShift 等离散级别，由程序映射为数值 | LLM 直出浮点不稳定 |
| Agent Prompt 契约 | 静态节点提示词版本化；动态输入只传对应 schema JSON；Trace 记录 prompt_template_id/version/hash | 防止提示词漂移、隐藏事实混入 prompt、回放无法定位 |
| Agent 输入 token 预算 | 默认 8K 关键注意力带、16K 软上限、可配置最大上下文；PromptBuilder 按 P0/P1/P2/P3 估算、压缩和裁剪 | 控制成本与上下文溢出，同时保证权限、schema、当前任务硬规则和角色心智焦点不被误删 |
| Agent 多人物分层 | active characters 达到可配置阈值后启用；primary CognitivePass 默认最多 3 个，未全量认知人物复用意图、模板或 MinorActorSlot | 控制等待时间和费用，避免次要人物凭空更新深层心理 |
| 反应窗口 | 主动威胁打开有限 ReactionWindow；合格目标/伙伴/守护者各最多提交一个 ReactionIntent；默认 reaction 不再触发 reaction | 支持即时援护与反击，同时避免无限递归和调用成本失控 |
| Agent LLM API 配置 | 五类 Agent LLM 节点可分别绑定 `api_configs/` 中的配置，未配置时继承默认 Agent 配置 | 不同节点对成本、速度、结构化输出能力、叙事质量要求不同 |
| OutcomePlanner 权限 | 可 God-read 并输出实际言行、交互结果和 StateUpdatePlan 候选；EffectValidator 裁剪非法硬效果后才提交 | 支持复杂技能与叙事结果规划，同时避免 LLM 直接改写世界状态 |
| Agent Trace 与运行 Logs | Agent Trace 随 World 保存，运行 Logs 记录应用观测；两者只通过 ID 关联，不参与业务判断 | 保证复盘、清理、审计边界清晰 |
| 日志存储 | 全局 Logs 位于 `./data/logs/app_logs.sqlite`，Agent Trace 与世界内 LLM Logs 位于 `world.sqlite` | ST 与 Agent 生命周期不同，World 迁移需要自包含 Trace |
| 日志清理 | 默认按 1GB 清理全局运行 Logs，实际上限来自 `app_runtime.yaml`；Agent Trace 不自动删除；30 天未更新 World 只提示用户 | 控制空间占用，同时避免误删复盘资料 |
