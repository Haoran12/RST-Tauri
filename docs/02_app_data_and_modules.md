# 02 应用数据目录与模块结构

本文档承载应用数据目录、配置分层、运行时快照、前后端模块结构与模块职责边界。

总体架构、数据形态铁律与 LLM/程序边界见 [01_architecture.md](01_architecture.md)。Agent 数据契约见 [10_agent_data_model.md](10_agent_data_model.md)。SQLite 表结构见 [14_agent_persistence.md](14_agent_persistence.md)。日志与可观测性见 [30_logging_and_observability.md](30_logging_and_observability.md)。

---

## 1. 应用数据目录约束

应用数据根目录固定为应用所在路径下的 `./data/`。默认不得写入 `AppData`、`Application Support`、`~/.config` 等系统用户数据目录，除非用户显式迁移或选择自定义数据目录。理由是让用户可以直接复制、备份、同步和检查完整数据。

### 1.1 通用规则

- 所有用户可迁移数据必须位于 `./data/` 或用户显式选择的数据根目录下。
- API 配置、角色卡、世界书、聊天记录、Agent 世界数据库不得散落在程序目录外。
- 应用启动时由存储层负责创建缺失目录；业务模块只通过 `storage::*` 访问路径。
- 路径中的实体 ID 必须使用安全文件名，禁止 `..`、绝对路径和平台保留字符。
- 日志存储位置见 [30_logging_and_observability.md](30_logging_and_observability.md)：全局运行 Logs 位于 `./data/logs/`，Agent Trace 随 World 位于 `./data/worlds/<world_id>/`。

### 1.2 ST 模式数据布局

ST 模式使用 JSON 文件存储，目录结构必须与 SillyTavern 兼容目标保持清晰分层：

```
./data/
├── lores/          # 世界书
├── presets/        # 预设（与 API 配置解耦）
│   ├── samplers/   # 采样参数预设
│   ├── instruct/   # 指令模板
│   ├── context/    # 上下文模板
│   ├── sysprompt/  # 系统提示词
│   ├── reasoning/  # 思维链模板
│   └── prompts/    # 提示词预设
├── chats/          # 聊天记录
├── characters/     # 角色卡 V3
├── settings/       # ST 全局扩展设置（含 Regex 全局脚本与 allow list）
└── api_configs/    # AI Provider 配置
```

ST 模式的聊天记录是文本会话数据，不承担 Agent 世界状态演化职责；删除 / 编辑消息不强制触发世界回滚约束。

**API 配置与 ST 资源独立于会话**：
- API 配置（Provider、endpoint、model、鉴权）存储在 `./data/api_configs/`
- 预设（采样参数、提示词模板）存储在 `./data/presets/`
- 世界书、角色卡世界书绑定和聊天 world_info metadata 存储在 `./data/lores/`、`./data/characters/`、`./data/chats/`
- API 配置、预设与世界书均由各自的全局 / 会话状态管理，用户可随时切换 API 配置，不与会话绑定
- 同一预设、同一世界书选择和同一 Regex 授权状态可用于不同 API 配置；切换 Provider 只影响请求发送目标与 Provider 字段映射，不会重新选择、重命名、复制、清空或重新授权预设 / 世界书
- 详见 [74_st_presets.md](74_st_presets.md) 与 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)

`api_configs/` 是全应用共享的 AI Provider 配置池。第一版 Agent 模式不另建一套 Provider 配置，而是从该配置池中为五类 Agent LLM 节点分别选择配置；具体绑定关系存于 Agent profile / World settings，见 [20_backend_contracts.md](20_backend_contracts.md)。

第一版必须把以下 AI Provider / 协议视为一等适配目标：OpenAI Responses API、OpenAI Chat Completions API、Google Gemini GenerateContent、Anthropic Messages、DeepSeek、Claude Code Interface。后续任何 API 配置、请求组装、日志、结构化输出或流式传输相关改动，都必须检查这六类适配面的影响；新增 Provider 不能降低这六类的兼容性要求。

### 1.3 Agent 模式数据布局与故事线定位

Agent 模式以 World 为顶层隔离单元。一个 World 不是普通聊天文件夹，而是一个持续演化的拟真故事世界；世界设定、人物状态、历史事件、聊天记录和回放 trace 共享同一套 canonical Truth。聊天记录只是用户在同一 World 下选择不同时间、不同人物和不同视角进入世界的会话入口，不等同于独立世界状态。

每个 World 维护一个 `WorldMainlineCursor`，记录当前主线的正史前沿：

- `mainline_head_turn_id`：当前主线最后提交的 canonical turn。
- `mainline_time_anchor`：当前主线所在的故事时间锚点。
- `timeline_id`：默认 `main`，第一版不支持平行正史时间线。

会话的 `period_anchor` 与 `mainline_time_anchor` 决定会话语义：

- `period_anchor < mainline_time_anchor`：过去线 `RetrospectiveSession`。运行时读取既有 Truth 引导场景与结果仲裁。
- `period_anchor == mainline_time_anchor`：当前主线会话。可推进 `WorldMainlineCursor`。
- `period_anchor > mainline_time_anchor`：未来线 / 预演。默认不直接写入 canonical Truth，后续另行设计。

过去线用于补完正史细节，而不是默认创建平行 if 线。若过去线与既有结构化 Truth 产生硬冲突，系统只向用户警告，不打断游玩；用户在警告中选择“冲突后非正史”或“整条会话非正史”。非正史会话仍保留聊天、Trace 与 provisional truth，但不得改变 canonical Truth、主线光标、角色 canonical 记忆或后续正史判断。

Agent 世界数据存放在应用数据目录的 `data/worlds/<world_id>/` 下，每个世界独立保存 SQLite 数据库、运行时快照、回放 trace 和必要资源；全局运行 Logs 作为应用观测数据存放在 `data/logs/`：

```
./data/
├── logs/
│   ├── app_logs.sqlite
│   └── archives/
├── settings/
│   └── app_runtime.yaml
└── worlds/
    ├── <world_id>/
    │   ├── world.sqlite
    │   ├── world_base.yaml
    │   ├── traces/
    │   └── assets/
    └── <world_id>/
```

SQLite 内部表结构见 [14_agent_persistence.md](14_agent_persistence.md)。Layer 2 派生视图不持久化，每回合由 Layer 1 / Layer 3 重建。`world_turns.created_at` 只表示提交发生时间，不能用于判断故事先后；所有故事时间判断必须使用 `story_time_anchor` / `period_anchor`。

Agent 会话删除只删除或归档会话消息视图，不能自动删除 canonical Truth。回滚 canonical turn 时必须从目标 canonical 回合开始检查依赖：若后续正史事实、其他会话已提升的细节或主线光标依赖该回合，默认阻止并生成影响报告；只有确认可回滚时才按 `state_commit_records.rollback_patch` 恢复 Layer 1 / Layer 3 / Knowledge / Trace 索引。非正史会话回滚只影响该会话的 provisional truth 和聊天记录。

Agent Trace 是世界调试与回放数据，随 World 保存；运行 Logs 是应用观测数据，用于记录 LLM 请求响应、Provider 错误与异常事件。回滚 Agent 回合时，世界状态与回合 trace 按故事线回退；运行 Logs 默认保留为审计记录，不随剧情回滚物理删除。

### 1.4 配置分层与运行时快照

RST 允许高级用户直接编辑配置文件，也可以后续由 UI 写入同一份配置；但业务热路径不得因配置可编辑而反复做文件 IO、SQLite 查询或 YAML/JSON 解析。

配置来源按以下顺序合并，后者覆盖前者：

1. **内置默认配置**：随应用版本发布（建议放在 `config/defaults/` 并打包为资源），作为缺失文件和迁移失败时的保底来源，不作为业务模块散落常量。
2. **全局运行配置**：`./data/settings/app_runtime.yaml`，保存日志清理上限、后台任务间隔、默认预算等不绑定故事世界的设置。
3. **World 规则配置**：`./data/worlds/<world_id>/world_base.yaml`，保存 `AttributeTier` 边界、`AttributeDelta` 桶、`CombatOutcomeTier` 桶、压制破绽阈值、环境档位阈值等会改变世界物理刻度的规则。
4. **运行期 UI 草稿**：设置界面编辑时先进入 draft，只有通过校验并保存后才发布新快照。

示例形态：

```yaml
# ./data/settings/app_runtime.yaml
schema_version: 1
log_retention:
  global_size_limit_bytes: 1073741824
  check_interval_hours: 24
  world_stale_prompt:
    inactive_days: 30
    size_ratio_of_global_limit: 0.5
```

```yaml
# ./data/worlds/<world_id>/world_base.yaml
schema_version: 1
attribute_rules:
  tier_thresholds:
    mundane: [0, 200]
    awakened: [200, 1000]
    adept: [1000, 1800]
    master: [1800, 2600]
    ascendant: [2600, 5600]
    transcendent: [5600, null]
  delta_thresholds:
    indistinguishable_abs_lt: 150
    slight_abs_lt: 300
    notable_abs_lt: 1000
    far_abs_lt: 2000
mana_rules:
  display_ratio_clamp: [0.0, 2.0]
  tendency_factors:
    inward: -0.5
    neutral: -0.2
    expressive: 0.1
  mode_factors:
    sealed: -0.7
    suppressed: -0.3
    natural: 0.0
    released: 0.2
    dominating: 0.4
  expression_modes:
    sealed: { radius: self_only, pressure_multiplier: 0.0 }
    suppressed: { radius: close, pressure_multiplier: 0.5 }
    natural: { radius: room, pressure_multiplier: 1.0 }
    released: { radius: area, pressure_multiplier: 1.15 }
    dominating: { radius: scene, pressure_multiplier: 1.3 }
  concealment_suspected_gap: 200
combat_rules:
  delta_thresholds:
    indistinguishable_abs_lt: 150
    slight_abs_lt: 300
    marked_abs_lt: 1000
```

加载流程：

- `ConfigLoader` 只在应用启动、打开 World、用户保存配置、显式重新加载配置时读取文件。
- `ConfigValidator` 必须检查 schema version、未知字段、数值范围、阈值单调性、互斥项和迁移规则；失败时保留上一份有效快照，并写入 `app_event_logs`。
- `ConfigCompiler` 把合并后的文本配置编译为强类型 `RuntimeConfigSnapshot` / `WorldRulesSnapshot`，预计算排序阈值、查找表、字节上限和配置 hash；属性 / `mana_power` 阈值在 YAML 中可写整数或小数，编译后统一为 f64。
- `ConfigRegistry` 以 `Arc` / 只读引用发布当前快照；Resolver、Filter、RequestAssembler、RetentionManager 只接收快照引用，不直接依赖文件路径。
- Agent 回合开始时固定 `config_snapshot_id`，本回合内即使用户保存新配置，也只能从下一回合 / 下一次请求组装开始生效。Trace 和 Logs 记录该 ID 以便复盘。

快照只保存强类型、已校验、可复盘的运行配置，不作为新的业务数据源：

```rust
pub struct RuntimeConfigSnapshot {
    pub config_snapshot_id: String,
    pub schema_version: String,
    pub log_retention: serde_json::Value,      // 编译后的日志清理配置
    pub request_budget: serde_json::Value,     // 编译后的调用预算配置
    pub provider_limits: serde_json::Value,
    pub config_hash: String,
}

pub struct WorldRulesSnapshot {
    pub config_snapshot_id: String,
    pub world_id: String,
    pub schema_version: String,
    pub attribute_rules: serde_json::Value,    // 编译后的基础属性档位/差距配置
    pub mana_rules: serde_json::Value,         // 编译后的灵力感知/压制特殊规则
    pub combat_rules: serde_json::Value,       // 编译后的对抗规则配置
    pub environment_rules: serde_json::Value,  // 编译后的环境档位配置
    pub config_hash: String,
}
```

配置变更生效边界：

- ST 模式请求组装：下一次生成请求生效。
- Agent World 规则：下一回合生效；已提交回合不重算，回滚后按目标回合记录的 `config_snapshot_id` 检查是否可复现。
- 日志清理策略：下一次后台 retention 检查生效；日志写入线程只更新内存计数和 `cleanup_needed` 标记，不在写入路径扫描文件大小。
- 文件监听只允许标记“配置可能已变更”，不得在监听回调内直接改运行快照；实际 reload 走同一套校验和发布流程。

配置文件优先面向“懂行用户可改”，UI 只暴露安全子集。任何可导致旧世界语义大幅变化的配置项（例如基础属性档位边界）必须显示警告，并建议在 World 创建初期调整。

---

## 2. 模块结构

### 2.1 前端 (Vue 3)

```
src/
├── components/
│   ├── chat/                # 聊天组件
│   ├── character/           # 角色管理
│   ├── worldbook/           # 世界书
│   ├── agent/               # Agent 模式专用
│   │   ├── SceneInspector.vue
│   │   ├── CharacterMindView.vue
│   │   ├── EmbodimentDebug.vue
│   │   ├── ValidationReport.vue
│   │   └── TurnTraceViewer.vue
│   └── settings/
├── stores/                  # Pinia stores
│   ├── chat.ts
│   ├── characters.ts
│   ├── worldbook.ts
│   ├── agent.ts
│   └── settings.ts
├── services/
│   ├── api.ts               # Tauri IPC 封装
│   └── storage.ts
├── types/
│   ├── character.ts                     # SillyTavern 角色卡
│   ├── worldbook.ts                     # SillyTavern 世界书
│   ├── agent/                           # Agent 模式（与 Rust 端对应）
│   │   ├── scene.ts                     # SceneModel / ManaField
│   │   ├── knowledge.ts                 # KnowledgeEntry / AccessPolicy
│   │   ├── location.ts                  # LocationNode / LocationSpatialRelation / LocationEdge / RouteEstimate
│   │   ├── embodiment.ts                # EmbodimentState / FilteredSceneView
│   │   ├── accessible.ts                # AccessibleKnowledge
│   │   ├── subjective.ts                # CharacterSubjectiveState
│   │   └── cognitive.ts                 # CognitivePass I/O
│   └── api.ts
├── views/
└── router/
```

### 2.2 后端 (Rust)

```
src-tauri/
├── src/
│   ├── main.rs / lib.rs
│   ├── commands/            # Tauri 命令
│   │   ├── chat.rs
│   │   ├── character.rs
│   │   ├── worldbook.rs
│   │   ├── agent.rs
│   │   └── settings.rs
│   ├── api/                 # AI Provider 抽象
│   │   ├── provider.rs
│   │   ├── openai.rs
│   │   ├── anthropic.rs
│   │   ├── gemini.rs
│   │   └── ollama.rs
│   ├── worldinfo/           # SillyTavern 世界书
│   │   ├── matcher.rs
│   │   ├── injector.rs
│   │   └── scanner.rs
│   ├── agent/               # Agent 核心
│   │   ├── mod.rs
│   │   ├── models/          # Layer 1/2/3 数据模型
│   │   │   ├── scene.rs                 # SceneModel + 子结构
│   │   │   ├── mana_field.rs
│   │   │   ├── knowledge.rs             # KnowledgeEntry / AccessPolicy / SubjectAwareness
│   │   │   ├── location.rs              # LocationNode / LocationSpatialRelation / LocationEdge / RouteEstimate
│   │   │   ├── character.rs             # CharacterRecord / BaselineBodyProfile / MindModelCard
│   │   │   ├── embodiment.rs            # EmbodimentState
│   │   │   ├── filtered_view.rs         # FilteredSceneView / ObservableEntity
│   │   │   ├── accessible.rs            # AccessibleKnowledge / AccessibleEntry
│   │   │   ├── subjective.rs            # CharacterSubjectiveState（Layer 3）
│   │   │   ├── cognitive.rs             # CognitivePass I/O
│   │   │   ├── skill.rs
│   │   │   └── dirty_flags.rs
│   │   ├── knowledge/       # 知识子系统（Layer 1 → Layer 2 派生核心）
│   │   │   ├── store.rs                 # KnowledgeStore：CRUD（不做访问权限判断）
│   │   │   ├── access_policy.rs         # KnowledgeAccessResolver：所有 Knowledge 访问逻辑唯一入口
│   │   │   ├── access.rs                # KnowledgeAccessProtocol：构建 AccessibleKnowledge
│   │   │   └── reveal.rs                # KnowledgeRevealEvent 处理
│   │   ├── location/        # 地点层级、自然地理关系、别名解析、地区事实继承与路线估算
│   │   │   ├── store.rs
│   │   │   ├── resolver.rs
│   │   │   ├── fact_resolver.rs
│   │   │   └── route_planner.rs
│   │   ├── simulation/      # 程序化核心
│   │   │   ├── scene_initializer.rs
│   │   │   ├── scene_extractor.rs
│   │   │   ├── attribute_resolver.rs    # 基础属性 effective 值、AttributeTier / AttributeDelta 派生
│   │   │   ├── embodiment_resolver.rs
│   │   │   ├── scene_filter.rs          # 含 observable_facets 计算（调用 KnowledgeAccessResolver）
│   │   │   ├── input_assembly.rs        # 拼装 CognitivePassInput（保证不漏 Layer 1）
│   │   │   ├── reaction_window.rs       # 有界反应窗口资格判定与 ReactionOption 派发
│   │   │   ├── physics_resolver.rs      # 物理 / 灵力数值骨架
│   │   │   ├── effect_validator.rs      # 技能契约与候选效果硬校验
│   │   │   └── outcome_planner.rs       # OutcomePlanner LLM 编排候选结果
│   │   ├── cognitive/       # 认知层（模型调用）
│   │   │   ├── cognitive_pass.rs
│   │   │   └── prompt_builder.rs
│   │   ├── presentation/    # 表现层
│   │   │   └── surface_realizer.rs
│   │   ├── validation/      # 验证规则
│   │   │   ├── validator.rs
│   │   │   ├── omniscience_rule.rs      # 通用全知泄露（覆盖 entity + knowledge）
│   │   │   ├── embodiment_rule.rs
│   │   │   ├── self_awareness_rule.rs   # Unaware facet 不应出现在 subject 自我描述中
│   │   │   ├── god_only_rule.rs         # GodOnly 知识不应出现在任何角色输出中
│   │   │   ├── mana_sense_rule.rs
│   │   │   └── consistency_rule.rs
│   │   ├── runtime.rs       # AgentRuntime 主循环
│   │   └── budget.rs        # 调用预算监控
│   ├── storage/
│   │   ├── json_store.rs
│   │   └── sqlite_store.rs
│   ├── config/
│   │   ├── loader.rs       # app_runtime.yaml / world_base.yaml 加载
│   │   ├── validator.rs    # schema version、范围、单调性校验
│   │   └── registry.rs     # RuntimeConfigSnapshot / WorldRulesSnapshot 发布
│   ├── logging/             # 日志与可观测性
│   │   ├── mod.rs
│   │   ├── context.rs       # LogContext / request_id / trace_id
│   │   ├── llm_logger.rs    # Provider logging wrapper
│   │   ├── event_logger.rs  # app_event_logs
│   │   └── retention.rs     # 读取 RuntimeConfigSnapshot 的清理策略
│   └── models/
└── Cargo.toml
```

### 2.3 模块职责边界（避免屎山）

| 模块 | 唯一职责 | 禁止做的事 |
|---|---|---|
| `knowledge::store` | KnowledgeEntry 的 CRUD；同事务维护访问派生索引 | 不做访问判断，不读 Layer 3 |
| `knowledge::access_policy` | 给定 (entry, character, context) → bool | 严禁调 LLM；不读 Layer 3 belief；不修改任何状态；不依赖 SQL 结果作为最终判定 |
| `knowledge::access` | 用 SQLite 派生索引预筛候选，并经 KnowledgeAccessResolver 构建 AccessibleKnowledge | 不调 LLM，不修改 belief；不绕过 KnowledgeAccessResolver |
| `knowledge::reveal` | 处理 KnowledgeRevealEvent | 追加 known_by 与生成 Memory；若原 scope 含 GodOnly，必须先由 OutcomePlanner 候选 + EffectValidator 确认并解除 GodOnly 后才能追加知情者；不重写既有 content |
| `location::store` | LocationNode / LocationSpatialRelation / LocationEdge / alias / polity template 的 CRUD；维护可重建索引 | 不做 LLM 推断；不把低置信度估算写成硬事实 |
| `location::resolver` | 名称 / 别名 / 上下文锚点 → 候选 LocationNode 与父级链 | 不猜唯一 ID；多命中必须返回 ambiguity |
| `location::fact_resolver` | 沿 parent 链合并可继承 RegionFact，读取自然地理影响，并调用 KnowledgeAccessResolver 裁剪 | 不提升 Knowledge 访问权限；不把自然地理影响混入行政继承 |
| `location::route_planner` | 基于 LocationEdge 带权图计算路线、耗时、风险与置信度 | 不用行政层级直接硬算距离；无连通边时只返回未知或低置信度提示 |
| `simulation::scene_initializer` | 调 LLM 从结构化 SceneSeed、公开上下文与场景相关私有约束生成候选 SceneModel 草案 | 不全库读取隐藏 Knowledge / GodOnly；不把私有约束泄露为外显事实；不直接写 Layer 1；不创建未授权持久实体 |
| `simulation::scene_extractor` | 调 LLM 把用户自由文本解析为 UserInputDelta / SceneUpdate 候选 | 只做场景域 God-read；不读取无关私密 Knowledge；不写 Layer 1（写入由 runtime 协调）；不解析中间数据 |
| `simulation::attribute_resolver` | 从基础属性、身体状态、技能/环境修正派生 effective 属性值、档位和差距骨架 | 不调 LLM；不把 UI 取整值用于仲裁；不修改 base_attributes |
| `simulation::scene_filter` | 当下感官过滤 + 计算 observable_facets | 不读 Knowledge content，仅判断 facet 是否可观察与可访问 |
| `simulation::input_assembly` | 拼装 CognitivePassInput | 不调 LLM，不做语义判断；输入禁止携带 Layer 1 原始对象 |
| `simulation::reaction_window` | 打开 ReactionWindow、判定 eligible_reactors、派发 ReactionOption、限制递归深度 | 不调 LLM；不结算反应后果；不让 reaction 默认再开启 reaction |
| `simulation::physics_resolver` / `combat_math_resolver` | 物理与灵力数值骨架、资源/距离/姿态硬边界 | 不调 LLM；不处理主观相信/记恨 |
| `simulation::effect_validator` | 校验 OutcomePlanner 候选效果是否符合技能契约与世界硬规则 | 不调 LLM；非法硬效果只能裁剪为 blocked_effects / soft_effects，不写 L1 |
| `simulation::outcome_planner` | 调 LLM 生成 OutcomePlan / StateUpdatePlan 候选 | 不直接提交状态；每回合默认最多调用一次 |
| `cognitive::cognitive_pass` | 调 LLM 输出严格 schema JSON | 不做验证，不直接修改 Layer 1/3 |
| `validation::*` | 检查输入/输出对 | 不修改任何状态；不调 LLM |
| `presentation::surface_realizer` | 调 LLM 渲染叙事并返回 used_fact_ids | 受 NarrationScope 派生的 narratable_facts 白名单约束；不引入新事实 |
| `agent::runtime` | 编排上述模块 | 不嵌入业务逻辑（仅做调度） |
| `logging::llm_logger` | 包装 Provider 调用并记录请求 / 响应 / stream chunk | 不改写 Provider 结果；不参与 prompt 组装 |
| `logging::event_logger` | 记录应用异常与运行事件 | 不吞异常；不改变业务分支 |
| `logging::retention` | 清理全局运行 Logs | 不自动删除 Agent Trace 或仍被回合引用的记录 |
