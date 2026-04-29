# 01 总体架构

本文档承载：

- 双模式总体架构图
- 设计原则
- 前/后端模块结构
- 应用数据目录约束
- 日志 / Trace 的系统边界
- LLM 与程序的职责边界总表 + 关键铁律
- 数据形态铁律（自由文本三关口）

数据契约见 [10_agent_data_model.md](10_agent_data_model.md)。地点层级、地区事实继承与路线图见 [15_agent_location_system.md](15_agent_location_system.md)。程序化派生公式与硬规则解算见 [12_agent_simulation.md](12_agent_simulation.md)。LLM 节点提示词与 I/O 契约见 [13_agent_llm_io.md](13_agent_llm_io.md)。运行时主循环与验证规则见 [11_agent_runtime.md](11_agent_runtime.md)。日志与可观测性见 [30_logging_and_observability.md](30_logging_and_observability.md)。

---

## 1. 总体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                    前端层 (Vue 3 + Naive UI)                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │ 聊天视图 │ │ 角色管理 │ │  世界书  │ │Agent 调试│          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              状态管理层 (Pinia Stores)                   │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │ Tauri IPC
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Tauri 后端 (Rust)                         │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  Presentation Layer                      │  │
│  │   SurfaceRealizer        AgentRuntime 主循环             │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Cognitive Layer (模型调用)                  │  │
│  │      CharacterCognitivePass (融合调用)                   │  │
│  │   Perception + Belief + Intent → 单次模型调用            │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Simulation Core (程序化核心)                │  │
│  │  Scene / Embodiment / Filter / Memory / Outcome Planning │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Validation Layer (程序化验证)               │  │
│  │  Omniscience / Embodiment / Memory / Mana / Consistency  │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                       Storage Layer                      │  │
│  │ JSON (ST)  SQLite (Agent)  Agent Trace  Runtime Logs     │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌────────────────┐
                    │  外部 AI APIs  │
                    │ OpenAI/Gemini/ │
                    │ Anthropic/etc  │
                    └────────────────┘
```

---

## 2. 设计原则

- **Character-Centered Reasoning** — 以角色为中心。
- **Subjective Access** — 角色仅从其能合理获取的信息中推理。
- **Embodied Cognition** — 角色通过当前身体状态感知与推理。
- **Bias Is Causal** — 信念、情绪、关系主动塑造感知与解释。
- **Belief-Driven Action** — 意图源于角色当前信念，即便信念不完整或错误。
- **Structured Handoffs** — 各阶段显式分离并交换结构化输出。
- **Traceable Subjectivity** — 误判、过度反应可在系统层面解释。
- **Truth ≠ Accessible Truth** — 客观真相只有编排器、结果规划、验证可读；角色 LLM 永远经过 KnowledgeAccessResolver 过滤。
- **Single Source of Knowledge Access** — 所有 Knowledge 访问权限判断由 KnowledgeAccessResolver 集中处理；SQLite 索引只做候选预筛，禁止散落在 prompt builder 或业务代码中。
- **Location Graph Is Structured Truth** — 地点层级由 `LocationNode.parent_id` 决定，自然地理覆盖由 `LocationSpatialRelation` 决定，路线与路程由 `LocationEdge` 带权图决定；同父级或同自然地理带只能给低置信度提示，不能自动写成硬事实。
- **Prompt Contracts Are Control Plane** — Agent LLM 提示词只定义节点身份、权限、任务和输出要求；世界事实只能来自本次结构化 input。
- **Logs Are Observations** — Agent Trace 与运行 Logs 只用于调试、审计、回放定位；不得作为业务判断来源或 LLM 输入来源。

---

## 3. 数据形态铁律

**自由文本作为顶层 I/O 仅允许出现在三处**，所有中间数据节点必须是严格 schema JSON；中间 JSON 内允许少量 LLM-readable 文本叶子字段，但这些字段不得参与程序判断、检索、访问控制或规则匹配。

### 术语边界：Access / Observable / Narratable

为避免把 Knowledge 权限误解为视觉可见，文档统一使用三组术语：

- **Access**：Knowledge 访问权限，即角色能否读取某条 `KnowledgeEntry`。核心类型为 `AccessPolicy` / `AccessScope` / `AccessCondition` / `KnowledgeAccessResolver`。
- **Observable**：感官可观察，包括视觉、听觉、嗅觉、触觉和灵觉；由 `SceneFilter` 与 `FilteredSceneView` 表达。
- **Narratable**：叙事可披露，即 SurfaceRealizer 能写给用户的事实白名单；由 `NarrationScope` 和 `narratable_facts` 表达。

| 位置 | 形态 | 说明 |
|---|---|---|
| 用户输入 | 自由文本 | 用户对话框输入、扮演角色的言行、元指令 |
| SceneStateExtractor 输入中的最近自由文本 | 自由文本 | 聊天记录最近一轮自由文本，通常包含用户最新输入；同一请求还会携带既有结构化 Scene JSON |
| SurfaceRealizer 输出 | 自由文本 | 给用户阅读的最终叙事 |

**所有其他中间节点**（Layer 1 / Layer 2 / Layer 3 数据、CognitivePass 输入输出、OutcomePlanner 输入输出、SurfaceRealizer 输入）**必须为严格 schema JSON**。若字段值是文本，必须显式标注用途：

- `semantic`：程序可读，必须使用枚举、ID、数值、布尔、结构体等，不允许自然语言。
- `llm_readable`：仅供 LLM 阅读理解，如 `summary_text` / `effect_hints` / `descriptors` / `notes`；禁止用于程序判断。
- `trace_only`：仅调试回放，如 `raw_text`；禁止进入业务逻辑。

### 自由文本进出系统的关口

```
[用户自由文本 + 既有结构化 Scene JSON]
       ↓
SceneStateExtractor (LLM, 严格 schema 输出：SceneUpdate + UserInputDelta)
       ↓
[结构化 SceneUpdate / UserInputDelta]
       ↓
   主循环（全程结构化）
       ↓
[结构化 OutcomePlan / CognitivePassOutput / ...]
       ↓
SurfaceRealizer (LLM)
       ↓
[自由文本叙事 → 用户]
```

新建场景、切场景和大幅跳时使用独立的 `SceneInitializer`。它不接收用户原始自由文本，而接收程序整理后的结构化 `SceneSeed`、公开世界 / 地点 / 人物上下文、场景相关私有约束和生成策略，输出严格 schema 的 `SceneInitializationDraft`。因此它不打破自由文本关口规则。

### 例外：LLM-readable 文本字段

文风约束（StyleConstraints）、KnowledgeEntry 的 `summary_text`、程序生成的 `effect_hints` / `descriptors` / `notes` 等字段允许包含自然语言。原则：**文本字段的值仅作为 LLM 的提示输入或 trace，不参与程序逻辑判断 / 检索 / 规则匹配**。

### KnowledgeEntry 内容的结构化要求

`KnowledgeEntry.content` 必须包含核心结构化字段（用于程序判断、访问控制、检索），可选包含 `summary_text` 等自由文本辅助字段（仅供 LLM 阅读理解）。详见 [10_agent_data_model.md](10_agent_data_model.md) 的 KnowledgeEntry 章节。

---

## 4. LLM 与程序边界总表

### 4.1 Agent 模式 LLM 节点分型

Agent 模式不再把"LLM"视为单一权限主体。每个 LLM 节点必须声明输入域、输出域、Knowledge 访问权限、叙事披露范围和提交权限。

| LLM 节点 | 输入 | 输出 | 权限边界 |
|---|---|---|---|
| SceneInitializer（场景初始化器） | 结构化 SceneSeed + 公开世界 / 地点 / 人物上下文 + 场景相关私有约束 + 生成策略 | 结构化 SceneInitializationDraft / SceneModel 草案 | 可读公开上下文，并可读取程序裁剪后的当前场景相关隐藏约束 / GodOnly 约束以保持客观一致性；不得全库检索隐藏 Knowledge，不得把私有约束写成外显事实；不得直接提交状态 |
| SceneStateExtractor（场景提取器） | 最近一轮自由文本 + 当前结构化 Scene JSON + 场景相关私有约束 + 必要的世界级结构化约束 | 结构化 SceneUpdate / UserInputDelta | 场景域 God-read：可读当前 SceneModel 全量与程序裁剪后的场景相关隐藏约束；默认不可读非当前场景私密历史、未关联本场景的隐藏角色 Knowledge 或全局 GodOnly；不得直接提交状态 |
| CharacterCognitivePass（人物认知与意图生成器） | 程序派生的该角色 L2 视图 + prior L3；字段值可含 `llm_readable` 文本 | 结构化心理活动、情绪、言行意图；字段值可含 `llm_readable` 文本 | 严格受 KnowledgeAccessResolver 过滤；不得读取 L1 原始对象或 GodOnly 知识 |
| OutcomePlanner（结果规划器） | L1 场景真相、角色情绪与言行意图、技能契约/知识/规则设定、DirectorHint 的结构化部分 | 结构化 OutcomePlan、StateUpdatePlan、KnowledgeRevealEvent 候选 | 可拥有 God 读取权限；但输出只是候选结果与候选更新，最终提交由 EffectValidator + StateCommitter 程序执行 |
| SurfaceRealizer（叙事文本输出器） | NarrationScope 派生的 SceneNarrativeView、角色心理/情绪摘要、实际言行、交互/对抗结果、文风/格式/叙事倾向 | 面向用户的自由文本叙事 | 不得突破 NarrationScope / narratable_facts；不得引入新事实 |

权限规则：

- **God 读取权限不等于提交权限**：SceneInitializer / SceneStateExtractor / OutcomePlanner 即使读取 L1、公开世界上下文或场景相关私有约束，也只能输出严格 schema 的候选 draft / delta / plan；写库只由程序提交。
- **场景域 God-read 不是全库 God-read**：SceneInitializer / SceneStateExtractor 只能读取程序按场景锚点、参与者、连续性和当前 SceneModel 裁剪出的私有约束；全局 GodOnly、无关角色秘密和非当前场景私密历史默认不进入输入。
- **受限 LLM**：CharacterCognitivePass 和 SurfaceRealizer 是主要防泄露对象，必须只接收过滤后的输入。
- **程序验证永远在提交前**：任何 LLM 产出的状态变化都必须经过 schema 校验、一致性校验、访问权限 / 叙事披露校验与 StateCommitter。
- **自由文本字段不驱动程序判断**：LLM 输出里的心理活动、叙事倾向、说明文本只能作为 `llm_readable` 或 `trace_only`，程序判断依赖结构化字段。

### 4.2 职责边界总表

| 任务 | 归属 | 形态约束 |
|---|---|---|
| 用户自由文本接收 | 程序（IO） | 自由文本入 |
| 场景初始化 / 切场景补全 | **LLM**（SceneInitializer）+ 程序校验 | 输入为结构化 SceneSeed + 公开上下文 + 场景相关私有约束；输出严格 schema 的 SceneInitializationDraft；只能按 generation_policy 补全，私有约束只能用于一致性 |
| 用户输入与场景变化提取 | **LLM**（SceneStateExtractor） | 输入为最近自由文本 + 当前结构化 Scene JSON + 场景相关私有约束；输出严格 schema 的 SceneUpdate / UserInputDelta |
| 场景物理状态维护 | 程序 | 全程结构化 |
| 身体状态机械演化（毒衰减/愈合/冷却） | 程序 | 全程结构化 |
| 情绪驱动的身体反应 | LLM（CognitivePass 输出 BodyReactionDelta） | 严格 schema；只作为候选反应/外显信号，不直接写 Layer 1 |
| 事件 delta 计算 | 程序 | 全程结构化 |
| 地点名称解析 / 层级归属 / 地区事实继承 / 自然地理影响 | 程序 | LocationResolver / LocationFactResolver 读取 LocationGraph、LocationSpatialRelation 与 KnowledgeAccessResolver；LLM 不猜 `location_id` |
| 地点相邻与路程估算 | 程序 | RoutePlanner 基于 `LocationEdge` 带权图计算；缺边时只输出低置信度提示，不写硬事实 |
| 脏标志（硬触发） | 程序 | directly_addressed / under_threat / reaction_window_open / scene_changed / body_changed；Tier A/B 的 knowledge_revealed 也触发 |
| 脏标志（主观显著性） | 不作触发条件，仅 prompt hint | received_new_salient_signal / belief_invalidated / relation_changed / intent_invalidated |
| EmbodimentResolver | 程序 | 公式化；含 environmental_strain 档位翻译 |
| 物理量→档位翻译（风/温/能见度/地表/降水/呼吸） | 程序 | 严禁 LLM 从 raw m/s, ℃ 推断后果；档位针对该角色物种已校准；body 侧与 perception 侧共享阈值表 |
| 灵力数值→档位翻译（个体/环境） | 程序 | LLM 不读 raw mana_power；档位边界来自世界配置（默认对 rp_cards 锚点校准） |
| 灵力差距→感知档（Δ 桶） | 程序 | 感知层阈值 150/300/1000/2000；对抗解算共享 150/300/1000，1000+ 即 Crushing；感知层用 displayed_mana_power |
| 灵力压制/隐匿的"破绽"判定 | 程序 | concealment_suspected 由 (observer.effective vs target.effective − 200) + 灵觉敏锐度计算；不让 LLM 自行猜"他是不是在装弱" |
| 灵力/物理硬边界 | 程序（CombatMathResolver / PhysicsResolver） | 用 effective_mana_power × 加算修正区 × soul_factor 等公式产出数值骨架和合法边界；不读 displayed |
| 反应窗口资格与递归上限 | 程序 | ReactionWindow 由可观察威胁/技能契约打开；判定谁能反应、能否援护、资源/距离/视线/感官是否合法；默认不允许 reaction 再开启 reaction |
| 反应意图选择 | **LLM**（CharacterCognitivePass 的受限子任务） | 只在程序给出的合法 reaction_options 内选择；输出 ReactionIntent，不直接结算、不写状态 |
| 复杂技能与外显社会后果 | OutcomePlanner + 程序校验 | LLM 可基于技能契约输出候选结果；程序只提交合法硬状态变化，越界效果降级为 soft_effects / blocked_effects |
| Knowledge 访问权限判断（KnowledgeAccessResolver） | 程序 | 严格禁止 LLM 介入 |
| Knowledge 访问候选索引 | 程序 + SQLite | `access_policy` JSON 是权威结构；`known_by` / `scope` 派生索引只用于缩小候选集，最终仍由 KnowledgeAccessResolver 判定 |
| 场景过滤 + observable_facets | 程序 | 全程结构化 |
| KnowledgeAccess | 程序 | 先用 SQLite 派生索引预筛候选，再调用 KnowledgeAccessResolver 裁剪为 AccessibleKnowledge |
| InputAssembly | 程序 | 类型隔离，禁止 Layer 1 原始对象 |
| 主观感知 / 偏见解释 / 意图生成 | **LLM**（CharacterCognitivePass） | 只读 L2 + prior L3；输出严格 schema JSON；信念变化用离散级别 |
| 客观演绎推理 | 程序（在 Knowledge 中预存事实） | LLM 不擅长长链推理 |
| 结果规划与状态更新计划 | **LLM + 程序**（OutcomePlanner + EffectValidator/StateCommitter） | OutcomePlanner 可 God-read 并输出结构化候选结果；程序裁剪非法硬效果并提交合法部分 |
| 物理公式与硬约束（资源/位置/技能数值/访问权限） | 程序 | 不依赖自由文本；可用于校验、裁剪或阻止候选效果 |
| 认知输出容错（残缺 JSON 修复） | 程序 | 修复常见错误 |
| 认知输出兜底解读 | **LLM**（OutcomePlanner 的子任务） | 修复失败时启用，输出严格 schema；不额外反复调用 LLM |
| 社会层后果（被骗/被劝服） | 可作为 OutcomePlan 的外显事件或下一轮认知输入 | 内心接受/相信仍由对应角色下一次 CharacterCognitivePass 更新 |
| 叙事渲染 | **LLM**（SurfaceRealizer） | 输入严格结构化 + StyleConstraints；受 NarrationScope 限制；输出自由文本 |
| NarrativeFactCheck | 程序 | 扫描叙事文本提及事实 ⊆ 当前 NarrationScope 的 narratable_facts |
| 验证规则 | 程序 | 全程结构化 |
| 状态提交 | 程序 | 全程结构化 |
| 用户扮演输入验证 | 程序（同样跑 Validator） | 一致性 |
| Agent Trace 写入 | 程序 | 记录回合内判断数据；不得改变状态演化 |
| 运行 Logs 写入 | 程序 | 记录 LLM 调用与异常事件；不得作为 LLM 输入 |
| 配置加载 / 校验 / 快照发布 | 程序 | 配置文件只在启动、打开 World、用户保存设置或安全 reload 点读取；热路径只读内存快照 |

### 关键铁律

1. **自由文本顶层 I/O 仅在三处出现**：用户输入、SceneStateExtractor 输入、SurfaceRealizer 输出。SceneInitializer 只能接收结构化 SceneSeed、llm_readable 公开上下文与程序裁剪后的场景相关私有约束，不接收原始用户自由文本；其他中间节点必须为严格 schema JSON；LLM-readable 文本叶子字段只供阅读，不参与程序判断。
2. **KnowledgeAccessResolver 永不调 LLM**：Knowledge 访问权限判断必须确定性；数据库索引只服务查询性能，不承担最终判定。
3. **LLM 输出必须严格 schema**：优先依赖 Provider 的 structured output / tool schema；仅在无强 schema 能力时退化到 JSON mode + schema 校验 + 重试 / 程序容错。
4. **受限 LLM 不读真相**：CharacterCognitivePass 和 SurfaceRealizer 只读过滤后的输入；SceneInitializer / SceneStateExtractor / OutcomePlanner 的 God-read 或公开上下文读取权限必须显式声明。
5. **数值字段不让受限 LLM 直出**：信念/情绪变化用离散级别，由程序映射为数值；对抗解算数值结果必须可被程序公式校验。
6. **God 读取不等于提交权限**：场景初始化、结果规划与场景提取 LLM 只产出候选 JSON，最终状态写入必须由程序校验并提交。
7. **反应窗口有界**：主动行动可打开一次 ReactionWindow；窗口内只收集合法 ReactionIntent，不即时递归结算；默认 `no_reaction_to_reaction = true`、`one_reaction_per_character_per_window = true`。
8. **叙事不引入新事实**：SurfaceRealizer 受 NarrationScope 派生的 narratable_facts 白名单约束，由 NarrativeFactCheck 强制。
9. **PromptBuilder 是 Agent LLM 调用唯一入口**：静态提示词只写节点契约并版本化；动态部分只传对应 `*Input` schema JSON；不得把日志、隐藏事实或临时自然语言说明绕过类型系统塞进 prompt。
10. **日志不驱动业务**：Agent Trace 和运行 Logs 只用于观察、调试、审计、回放定位；不得参与程序判断、检索、访问控制或 LLM prompt 组装。
11. **地点推断不固化弱假设**：`parent_id`、`LocationSpatialRelation`、`LocationEdge` 与结构化 RegionFact 才是地点真相；同父级、同自然地理带、同层级、名称相似、LLM 判断等只能产生带置信度的 `ProximityHint` / `SceneAssumption`，不能自动写入空间关系、路线边或地区事实。
12. **配置不在热路径做 IO**：数值阈值、日志清理上限、运行预算等可配置项不得散落为业务硬编码；程序启动 / World 打开时合并、校验并发布不可变 `RuntimeConfigSnapshot`，一回合内所有 Resolver / Filter / RetentionManager 只读该快照，不读文件或临时查询配置表。
13. **回合内工作副本不等于持久状态**：SceneInitializer / SceneStateExtractor / OutcomePlanner 产出的 draft / delta / plan 只能先应用到本回合 `TurnWorkingState`，供后续派生、验证和叙事组装读取；只有 `StateCommitter` 可在单个 SQLite 写事务中把通过校验的 L1 / L3 / Knowledge / Trace 变更提交为持久状态。

---

## 5. 应用数据目录约束

应用数据根目录固定为应用所在路径下的 `./data/`。默认不得写入 `AppData`、`Application Support`、`~/.config` 等系统用户数据目录，除非用户显式迁移或选择自定义数据目录。理由是让用户可以直接复制、备份、同步和检查完整数据。

### 5.1 通用规则

- 所有用户可迁移数据必须位于 `./data/` 或用户显式选择的数据根目录下。
- API 配置、角色卡、世界书、聊天记录、Agent 世界数据库不得散落在程序目录外。
- 应用启动时由存储层负责创建缺失目录；业务模块只通过 `storage::*` 访问路径。
- 路径中的实体 ID 必须使用安全文件名，禁止 `..`、绝对路径和平台保留字符。
- 日志存储位置见 [30_logging_and_observability.md](30_logging_and_observability.md)：全局运行 Logs 位于 `./data/logs/`，Agent Trace 随 World 位于 `./data/worlds/<world_id>/`。

### 5.2 ST 模式数据布局

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

### 5.3 Agent 模式数据布局与故事线定位

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

### 5.4 配置分层与运行时快照

RST 允许高级用户直接编辑配置文件，也可以后续由 UI 写入同一份配置；但业务热路径不得因配置可编辑而反复做文件 IO、SQLite 查询或 YAML/JSON 解析。

配置来源按以下顺序合并，后者覆盖前者：

1. **内置默认配置**：随应用版本发布（建议放在 `config/defaults/` 并打包为资源），作为缺失文件和迁移失败时的保底来源，不作为业务模块散落常量。
2. **全局运行配置**：`./data/settings/app_runtime.yaml`，保存日志清理上限、后台任务间隔、默认预算等不绑定故事世界的设置。
3. **World 规则配置**：`./data/worlds/<world_id>/world_base.yaml`，保存 `ManaPotencyTier` 边界、`ManaPerceptionDelta` 桶、`CombatOutcomeTier` 桶、压制破绽阈值、环境档位阈值等会改变世界物理刻度的规则。
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
mana_rules:
  potency_tiers:
    mundane: [0, 200]
    awakened: [200, 1000]
    adept: [1000, 1800]
    master: [1800, 2600]
    ascendant: [2600, 5600]
    transcendent: [5600, null]
  perception_delta_thresholds:
    indistinguishable_abs_lt: 150
    slight_abs_lt: 300
    notable_abs_lt: 1000
    far_abs_lt: 2000
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
- `ConfigCompiler` 把合并后的文本配置编译为强类型 `RuntimeConfigSnapshot` / `WorldRulesSnapshot`，预计算排序阈值、查找表、字节上限和配置 hash。
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
    pub mana_rules: serde_json::Value,         // 编译后的灵力规则配置
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

配置文件优先面向“懂行用户可改”，UI 只暴露安全子集。任何可导致旧世界语义大幅变化的配置项（例如灵力档位边界）必须显示警告，并建议在 World 创建初期调整。

---

## 6. 模块结构

### 6.1 前端 (Vue 3)

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

### 6.2 后端 (Rust)

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

### 6.3 模块职责边界（避免屎山）

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
| `simulation::scene_filter` | 当下感官过滤 + 计算 observable_facets | 不读 Knowledge content，仅判断 facet 是否可观察与可访问 |
| `simulation::input_assembly` | 拼装 CognitivePassInput | 不调 LLM，不做语义判断；输入禁止携带 Layer 1 原始对象 |
| `simulation::reaction_window` | 打开 ReactionWindow、判定 eligible_reactors、派发 ReactionOption、限制递归深度 | 不调 LLM；不结算反应后果；不让 reaction 默认再开启 reaction |
| `simulation::physics_resolver` / `combat_math_resolver` | 物理与灵力数值骨架、资源/距离/姿态硬边界 | 不调 LLM；不处理主观相信/记恨 |
| `simulation::effect_validator` | 校验 OutcomePlanner 候选效果是否符合技能契约与世界硬规则 | 不调 LLM；非法硬效果只能裁剪为 blocked_effects / soft_effects，不写 L1 |
| `simulation::outcome_planner` | 调 LLM 生成 OutcomePlan / StateUpdatePlan 候选 | 不直接提交状态；每回合默认最多调用一次 |
| `cognitive::cognitive_pass` | 调 LLM 输出严格 schema JSON | 不做验证，不直接修改 Layer 1/3 |
| `validation::*` | 检查输入/输出对 | 不修改任何状态；不调 LLM |
| `presentation::surface_realizer` | 调 LLM 渲染叙事 | 受 NarrationScope 派生的 narratable_facts 白名单约束；不引入新事实 |
| `agent::runtime` | 编排上述模块 | 不嵌入业务逻辑（仅做调度） |
| `logging::llm_logger` | 包装 Provider 调用并记录请求 / 响应 / stream chunk | 不改写 Provider 结果；不参与 prompt 组装 |
| `logging::event_logger` | 记录应用异常与运行事件 | 不吞异常；不改变业务分支 |
| `logging::retention` | 清理全局运行 Logs | 不自动删除 Agent Trace 或仍被回合引用的记录 |
