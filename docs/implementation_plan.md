# RST-Tauri 实现计划

Ran's SmartTavern：基于 Tauri 的双模式 AI 聊天应用。

> 本文仅承载**项目路线图**：阶段、里程碑、技术栈选型、关键决策。
> 数据模型、架构细节、运行时主循环等 spec 内容已拆分到独立文档，见 [README.md](README.md) 索引。

## 项目概述

- **SillyTavern 模式**：复刻 SillyTavern 体验，支持角色卡 V3 + 世界书 + 预设 + API 配置，JSON 文件存储。详见 [02_st_mode.md](02_st_mode.md)。
- **Agent 模式**：基于 RP Agent 架构的高级角色扮演系统，分层"客观世界 / 人物具身状态 / 主观认知与意图 / 仲裁与状态更新 / 叙事输出"，SQLite 存储。详见 [10_agent_data_and_simulation.md](10_agent_data_and_simulation.md) + [11_agent_runtime.md](11_agent_runtime.md)。

> **架构基础**：参考 `D:\Projects\RST-flutter\docs\rp_agent_*` 系列文档（成熟的角色扮演 Agent 架构），本项目在其基础上为 Tauri + Rust + Vue 3 技术栈做适配。

---

## 1. 技术栈选型

| 层 | 选型 | 理由 |
|---|---|---|
| 前端框架 | Vue 3 + TypeScript | 生态成熟、组合式 API、Pinia 类型友好 |
| UI 组件库 | Naive UI | Vue 3 原生支持，组件丰富，TypeScript 友好，暗色主题完善 |
| 状态管理 | Pinia | 内置、类型安全 |
| 路由 | Vue Router | 标准方案 |
| 后端 | Tauri + Rust | 小型二进制、跨平台、安全 |
| 存储 - ST 模式 | JSON 文件 | 与 SillyTavern 兼容 |
| 存储 - Agent 模式 | SQLite | 结构化查询、事务、性能 |
| 日志与 Trace | SQLite | LLM 请求响应、Agent Trace、异常事件需要按 turn/request 查询 |
| AI 后端 | 多 Provider | Claude / GPT / Gemini / Ollama |

---

## 2. 实现阶段

### 阶段一：基础框架 (MVP)

1. 初始化 Tauri + Vue 3 + TypeScript + Naive UI。
2. 配置 Vue Router + Pinia。
3. 实现 JSON 存储层。
4. 基础聊天 + AI Provider 抽象。
5. 集成 OpenAI Response API, OpenAI ChatCompletion API, Deepseek API。
6. 全局运行 Logs：`./data/logs/app_logs.sqlite` + Provider logging wrapper + 1GB 默认清理。

### 阶段二：SillyTavern 模式

1. 角色卡 V3 管理（创建 / 编辑 / 导入 / 导出）。
2. 世界书编辑器（含分组 / 概率 / 递归 / 时间控制）。
3. 关键词触发系统（含正则 / 匹配目标扩展）。
4. 预设系统。
5. 多 API 支持（Claude / Gemini / Ollama / Deepseek）。

### 阶段三：Agent 模式 — 数据模型层

1. SQLite 表结构 + 三层语义隔离（Layer 1 / Layer 3 / Trace）。
2. SceneModel + ManaField + PhysicalConditions 完整定义（Layer 1）。
3. KnowledgeEntry 体系（kind / subject / visibility / subject_awareness / apparent_content）。
4. KnowledgeEntry content sub-schemas（每种 facet/fact 类型的核心字段 + extensions 兜底）。
5. CharacterRecord（baseline_body_profile + mind_model_card + temporary_body_state）。
6. CharacterSubjectiveState（Layer 3）。
7. EmbodimentState / FilteredSceneView / AccessibleKnowledge（Layer 2 派生类型）。
8. CognitivePass I/O 类型（含 ConfidenceShift / BodyReactionDelta）。
9. UserInputDelta / StyleConstraints / SurfaceRealizerInput / ArbitrationResult。
10. Agent Trace / LLM Logs / app_event_logs 表结构。

### 阶段四：Agent 模式 — 程序化核心

1. KnowledgeStore（Layer 1 CRUD）。
2. VisibilityResolver（统一可见性判断，三谓词合并）。
3. KnowledgeAccessProtocol（构建 AccessibleKnowledge）。
4. EmbodimentResolver（含灵觉 + 环境档位翻译）。
5. SceneFilter（含 visible_facets 计算 + WeatherPerception + ManaSignal）。
6. InputAssembly（拒绝 Layer 1 原始对象）。
7. ActionArbitration（仲裁层读 Layer 1 真相 + Mana Combat Resolution 公式）。
8. KnowledgeRevealEvent 处理。

### 阶段五：Agent 模式 — 认知与叙事层

1. PromptBuilder（结构化 prompt + JSON schema 注入）。
2. SceneStateExtractor（用户输入 → UserInputDelta，严格 schema）。
3. CharacterCognitivePass（融合调用，严格 schema 输出）。
4. JSON 输出容错修复器（缺字段补默认 / 修复常见结构错误）。
5. Arbitration LLM 兜底（认知输出修复失败时启用）。
6. SurfaceRealizer（叙事生成，受 visible_facts 约束）。
7. AI Provider 的 chat_structured 实现（OpenAI/Anthropic/Gemini/Ollama/DeepSeek 各自的 structured output / tool schema / JSON 降级路径）。
8. LLM 调用日志：request / response / schema / stream chunks / readable_text。

### 阶段六：Agent 模式 — 验证与运行时

1. 9 大验证规则（含 SelfAwareness / GodOnly / ApparentVsTrue / NarrativeFactCheck / SchemaConformance）。
2. AgentRuntime 主循环。
3. Dirty Flags + Active Set。
4. 角色 Tier 分级。
5. 调用预算监控。
6. Agent Trace 写入点：Active Set、Dirty Flags、Layer 2 派生、CognitivePass、验证、仲裁、提交。

### 阶段七：用户角色扮演

1. 用户角色选择。
2. 用户输入心理活动 / 言行。
3. 用户对仲裁 / 文风的"导演"权（DirectorHint）。

### 阶段八：优化与扩展

1. 性能优化（缓存 / 事件批处理）。
2. UI / UX 改进。
3. Trace 可视化。
4. 测试覆盖。
5. 插件系统。
6. 日志管理 UI：大小统计、30 天未更新 World 提示、手动清理 / 导出。

---

## 3. 关键文件

### 前端

- `src/stores/agent.ts` — Agent 状态管理。
- `src/services/api.ts` — Tauri IPC 封装。
- `src/components/agent/CharacterMindView.vue` — 心智视图。
- `src/components/agent/ValidationReport.vue` — 验证报告。

### 后端

- `src-tauri/src/agent/runtime.rs` — 主循环。
- `src-tauri/src/agent/knowledge/store.rs` — 知识库 CRUD。
- `src-tauri/src/agent/knowledge/visibility.rs` — 可见性唯一入口。
- `src-tauri/src/agent/knowledge/access.rs` — AccessibleKnowledge 构建。
- `src-tauri/src/agent/knowledge/reveal.rs` — 揭示事件处理。
- `src-tauri/src/agent/cognitive/cognitive_pass.rs` — 融合调用。
- `src-tauri/src/agent/simulation/scene_filter.rs` — 场景过滤（含 visible_facets）。
- `src-tauri/src/agent/simulation/input_assembly.rs` — 拒绝 Layer 1 泄露。
- `src-tauri/src/agent/simulation/arbitration.rs` — 仲裁层（直接读真相 + ManaCombatResolution）。
- `src-tauri/src/agent/validation/validator.rs` — 验证器入口。
- `src-tauri/src/agent/models/knowledge.rs` — KnowledgeEntry 定义。
- `src-tauri/src/agent/models/scene.rs` — 场景模型。
- `src-tauri/src/agent/models/mana_field.rs` — 灵力场。
- `src-tauri/src/storage/sqlite_store.rs` — 存储层。
- `src-tauri/src/logging/llm_logger.rs` — Provider logging wrapper。
- `src-tauri/src/logging/event_logger.rs` — 应用异常事件日志。
- `src-tauri/src/logging/retention.rs` — 1GB 默认清理策略。

---

## 4. 验证与里程碑

每阶段交付的验证清单见 [90_pitfalls_and_tests.md](90_pitfalls_and_tests.md)。

---

## 5. 关键决策记录

> 文档维护原则：修改时直接更新最新版，不保留历史对比、版本演进或"改进前后"标记。重大架构决策与变更走 git commit 记录，本节仅总结当前生效的核心选择。

| 主题 | 决策 | 理由 |
|---|---|---|
| 数据形态铁律 | 自由文本仅在三处出现：用户输入、SceneStateExtractor 输入、SurfaceRealizer 输出。其他全程结构化 JSON | 避免规则匹配失效与"屎山"起点 |
| 三层数据语义 | Layer 1 (Truth) / Layer 2 (Per-Character Access) / Layer 3 (Subjective)，强制隔离 | LLM 永不接触 Layer 1 原始对象，杜绝全知泄露 |
| 知识统一模型 | KnowledgeEntry 统一承载世界/势力/角色档案/记忆，按 visibility 谓词控制 | 单一可见性入口（VisibilityResolver），避免散落 |
| 灵力档位 | 6 档（Mundane / Awakened / Adept / Master / Ascendant / Transcendent），边界对 `D:\AI\rp_cards\` 锚点校准 | 用档位识别身份，用数值差识别实力 |
| 感知 vs 仲裁分离 | 感知用 `displayed_mana_power`（含压制），仲裁用 `effective_mana_power`（不含压制） | 压制是认知层欺骗手段，不影响真实对抗 |
| 仲裁公式 | `combat_power = effective × max(0.1, 1 + Σ_modifiers) × soul_factor`，加算修正区 + 灵魂独立乘区 | 多因子加和可控，灵魂破损保留乘性凸显质变打击 |
| 跨档差阈值 | 感知层 150 / 300 / 1000 / 2000；仲裁层共享 150 / 300 / 1000，1000+ 即 Crushing | 150 起感觉差距、1000+ 基本无力应对，2000+ 进入无法测度的体感描述 |
| LLM 数值字段 | 用 ConfidenceShift 等离散级别，由程序映射为数值 | LLM 直出浮点不稳定 |
| 仲裁层 LLM 兜底范围 | 仅在 CognitivePass 输出失败时启用；物理判定永远走程序 | 防止"什么都让 LLM 仲裁" |
| Agent Trace 与运行 Logs | Agent Trace 随 World 保存，运行 Logs 记录应用观测；两者只通过 ID 关联，不参与业务判断 | 保证复盘、清理、审计边界清晰 |
| 日志存储 | 全局 Logs 位于 `./data/logs/app_logs.sqlite`，Agent Trace 与世界内 LLM Logs 位于 `world.sqlite` | ST 与 Agent 生命周期不同，World 迁移需要自包含 Trace |
| 日志清理 | 默认按 1GB 清理全局运行 Logs；Agent Trace 不自动删除；30 天未更新 World 只提示用户 | 控制空间占用，同时避免误删复盘资料 |
