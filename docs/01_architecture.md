# 01 总体架构

本文档承载：

- 双模式总体架构图
- 设计原则
- 架构层文档导航
- 跨系统约束与边界索引
- 日志 / Trace 的系统边界
- LLM 与程序的职责边界总表 + 关键铁律
- 数据形态铁律（自由文本三关口）

应用数据目录与模块结构见 [02_app_data_and_modules.md](02_app_data_and_modules.md)。数据契约见 [10_agent_data_model.md](10_agent_data_model.md)。地点层级、地区事实继承与路线图见 [15_agent_location_system.md](15_agent_location_system.md)。程序化派生见 [12_agent_simulation.md](12_agent_simulation.md)，对抗解算与技能契约见 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)。LLM 节点提示词与 I/O 契约入口见 [13_agent_llm_io.md](13_agent_llm_io.md)，场景节点见 [21_agent_scene_llm_io.md](21_agent_scene_llm_io.md)，结果规划与叙事节点见 [22_agent_outcome_narration_io.md](22_agent_outcome_narration_io.md)。运行时主循环与验证规则见 [11_agent_runtime.md](11_agent_runtime.md)。日志与可观测性见 [30_logging_and_observability.md](30_logging_and_observability.md)。

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
- **Narratable**：叙事可披露，即 SurfaceRealizer 能写给用户的结构化事实白名单；由 `NarrationScope`、`NarratableFact` 和 `narratable_facts` 表达。

| 位置 | 形态 | 说明 |
|---|---|---|
| 用户输入 | 自由文本 | 用户对话框输入、扮演角色的言行、元指令 |
| SceneStateExtractor 输入中的最近自由文本 | 自由文本 | 聊天记录最近一轮自由文本，通常包含用户最新输入；同一请求还会携带既有结构化 Scene JSON |
| SurfaceRealizer 输出 | 严格 schema JSON + 自由文本叶子 | 内部为 `SurfaceRealizerOutput { narrative_text, used_fact_ids }`，UI 只展示 `narrative_text` |

**所有其他中间节点**（Layer 1 / Layer 2 / Layer 3 数据、CognitivePass 输入输出、OutcomePlanner 输入输出、SurfaceRealizer 输入输出）**必须为严格 schema JSON**。若字段值是文本，必须显式标注用途：

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
[结构化 OutcomePlan / NarrativeCharacterView / ...]
       ↓
SurfaceRealizer (LLM)
       ↓
[SurfaceRealizerOutput.narrative_text → 用户]
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
| SurfaceRealizer（叙事文本输出器） | NarrationScope 派生的 SceneNarrativeView、NarrativeCharacterView、实际言行、交互/对抗结果、文风/格式/叙事倾向 | `SurfaceRealizerOutput { narrative_text, used_fact_ids }`；UI 只展示自由文本叙事 | 不得突破 NarrationScope / narratable_facts；不得引入新事实 |

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
| 基础属性有效值派生 | 程序（AttributeResolver） | `physical` / `agility` / `endurance` / `insight` / `mana_power` / `soul_strength` raw 值为 Layer 1；存储和计算用 f64，UI 默认整数展示；伤势/状态/技能/环境只改 effective，不改 base |
| EmbodimentResolver | 程序 | 公式化；含 environmental_strain 档位翻译 |
| 物理量→档位翻译（风/温/能见度/地表/降水/呼吸） | 程序 | 严禁 LLM 从 raw m/s, ℃ 推断后果；档位针对该角色物种已校准；body 侧与 perception 侧共享阈值表 |
| 基础属性→档位翻译 | 程序 | LLM 不读 raw 属性值；AttributeTier 边界来自世界配置（默认对 rp_cards 的 mana_power 锚点校准）；UI 取整不影响落档 |
| 基础属性差距→感知档（Δ 桶） | 程序 | 感知层阈值 150/300/1000/2000；对抗解算共享 150/300/1000，1000+ 即 Crushing；mana_power 感知层用 displayed_mana_power |
| 灵力显露倾向与运行时状态 | 程序 | 持久层 `ManaExpressionTendency` 只表达体质/性格/修行体系导致的默认倾向（内敛/一般/外放），可有人物级 `tendency_factor` 覆盖；运行时 `ManaExpressionMode` 表达当前场景封息/抑制/自然/外放/威压；`display_ratio = clamp(1 + tendency_factor + mode_factor, 0, 2)`，再派生 displayed_mana_power、局部灵压和认知压力，不改变 effective_mana_power；LLM 只能请求离散运行时状态，不输出倍率 |
| 灵力压制/隐匿的"破绽"判定 | 程序 | concealment_suspected 由 (observer.effective vs target.effective − 200) + 灵觉敏锐度 + 显露倾向/当前状态反差计算；不让 LLM 自行猜"他是不是在装弱" |
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
| 叙事渲染 | **LLM**（SurfaceRealizer） | 输入严格结构化 + StyleConstraints；受 NarrationScope 限制；输出 `narrative_text + used_fact_ids` |
| NarrativeFactCheck | 程序 | 校验 used_fact_ids ⊆ 当前 NarrationScope 的 narratable_facts，并保守抽查叙事文本 |
| 验证规则 | 程序 | 全程结构化 |
| 状态提交 | 程序 | 全程结构化 |
| 用户扮演输入验证 | 程序（同样跑 Validator） | 一致性 |
| Agent Trace 写入 | 程序 | 记录回合内判断数据；不得改变状态演化 |
| 运行 Logs 写入 | 程序 | 记录 LLM 调用与异常事件；不得作为 LLM 输入 |
| 配置加载 / 校验 / 快照发布 | 程序 | 配置文件只在启动、打开 World、用户保存设置或安全 reload 点读取；热路径只读内存快照 |

### 关键铁律

1. **自由文本顶层 I/O 仅在三处出现**：用户输入、SceneStateExtractor 输入、SurfaceRealizerOutput.narrative_text。SceneInitializer 只能接收结构化 SceneSeed、llm_readable 公开上下文与程序裁剪后的场景相关私有约束，不接收原始用户自由文本；其他中间节点必须为严格 schema JSON；LLM-readable 文本叶子字段只供阅读，不参与程序判断。
2. **KnowledgeAccessResolver 永不调 LLM**：Knowledge 访问权限判断必须确定性；数据库索引只服务查询性能，不承担最终判定。
3. **LLM 输出必须严格 schema**：优先依赖 Provider 的 structured output / tool schema；仅在无强 schema 能力时退化到 JSON mode + schema 校验 + 重试 / 程序容错。
4. **受限 LLM 不读真相**：CharacterCognitivePass 和 SurfaceRealizer 只读过滤后的输入；SceneInitializer / SceneStateExtractor / OutcomePlanner 的 God-read 或公开上下文读取权限必须显式声明。
5. **数值字段不让受限 LLM 直出**：基础属性与 `mana_power` 存储/计算可用 f64，但受限 LLM 只读 tier / delta / expression_assessment / pressure_hints / descriptors / constraints；灵力显露只能区分持久倾向与当前运行时状态，不让 LLM 直出 display/pressure 倍率；信念/情绪变化用离散级别，由程序映射为数值；对抗解算数值结果必须可被程序公式校验。
6. **God 读取不等于提交权限**：场景初始化、结果规划与场景提取 LLM 只产出候选 JSON，最终状态写入必须由程序校验并提交。
7. **反应窗口有界**：主动行动可打开一次 ReactionWindow；窗口内只收集合法 ReactionIntent，不即时递归结算；默认 `no_reaction_to_reaction = true`、`one_reaction_per_character_per_window = true`。
8. **叙事不引入新事实**：SurfaceRealizer 受 NarrationScope 派生的结构化 narratable_facts 白名单约束，由 NarrativeFactCheck 强制。
9. **PromptBuilder 是 Agent LLM 调用唯一入口**：静态提示词只写节点契约并版本化；动态部分只传对应 `*Input` schema JSON；不得把日志、隐藏事实或临时自然语言说明绕过类型系统塞进 prompt。
10. **日志不驱动业务**：Agent Trace 和运行 Logs 只用于观察、调试、审计、回放定位；不得参与程序判断、检索、访问控制或 LLM prompt 组装。
11. **地点推断不固化弱假设**：`parent_id`、`LocationSpatialRelation`、`LocationEdge` 与结构化 RegionFact 才是地点真相；同父级、同自然地理带、同层级、名称相似、LLM 判断等只能产生带置信度的 `ProximityHint` / `SceneAssumption`，不能自动写入空间关系、路线边或地区事实。
12. **配置不在热路径做 IO**：数值阈值、日志清理上限、运行预算等可配置项不得散落为业务硬编码；程序启动 / World 打开时合并、校验并发布不可变 `RuntimeConfigSnapshot`，一回合内所有 Resolver / Filter / RetentionManager 只读该快照，不读文件或临时查询配置表。
13. **回合内工作副本不等于持久状态**：SceneInitializer / SceneStateExtractor / OutcomePlanner 产出的 draft / delta / plan 只能先应用到本回合 `TurnWorkingState`，供后续派生、验证和叙事组装读取；只有 `StateCommitter` 可在单个 SQLite 写事务中把通过校验的 L1 / L3 / Knowledge / Trace 变更提交为持久状态。

---

## 5. 应用数据目录与模块结构

应用数据目录、配置分层、运行时快照、前后端模块结构和模块职责边界已拆分到 [02_app_data_and_modules.md](02_app_data_and_modules.md)。

本文件只保留跨系统架构、数据形态铁律、LLM/程序边界总表和关键铁律。
