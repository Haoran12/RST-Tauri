# 01 总体架构

本文档承载：

- 双模式总体架构图
- 设计原则
- 前/后端模块结构
- 应用数据目录约束
- 日志 / Trace 的系统边界
- LLM 与程序的职责边界总表 + 关键铁律
- 数据形态铁律（自由文本三关口）

数据契约见 [10_agent_data_model.md](10_agent_data_model.md)。程序化派生公式与硬规则解算见 [12_agent_simulation.md](12_agent_simulation.md)。LLM 节点提示词与 I/O 契约见 [13_agent_llm_io.md](13_agent_llm_io.md)。运行时主循环与验证规则见 [11_agent_runtime.md](11_agent_runtime.md)。日志与可观测性见 [30_logging_and_observability.md](30_logging_and_observability.md)。

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

新建场景、切场景和大幅跳时使用独立的 `SceneInitializer`。它不接收用户原始自由文本，而接收程序整理后的结构化 `SceneSeed`、公开世界 / 地点 / 人物上下文和生成策略，输出严格 schema 的 `SceneInitializationDraft`。因此它不打破自由文本关口规则。

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
| SceneInitializer（场景初始化器） | 结构化 SceneSeed + 公开世界 / 地点 / 人物上下文 + 生成策略 | 结构化 SceneInitializationDraft / SceneModel 草案 | 可读公开设定与当前场景种子；默认不读隐藏 Knowledge / GodOnly。只允许在策略白名单域内补全细节，不得直接提交状态 |
| SceneStateExtractor（场景提取器） | 最近一轮自由文本 + 当前结构化 Scene JSON + 必要的世界级结构化约束 | 结构化 SceneUpdate / UserInputDelta | 可读当前场景真相；是否可读隐藏 Knowledge 待决策。不得直接提交状态，只产出候选 delta |
| CharacterCognitivePass（人物认知与意图生成器） | 程序派生的该角色 L2 视图 + prior L3；字段值可含 `llm_readable` 文本 | 结构化心理活动、情绪、言行意图；字段值可含 `llm_readable` 文本 | 严格受 KnowledgeAccessResolver 过滤；不得读取 L1 原始对象或 GodOnly 知识 |
| OutcomePlanner（结果规划器） | L1 场景真相、角色情绪与言行意图、技能契约/知识/规则设定、DirectorHint 的结构化部分 | 结构化 OutcomePlan、StateUpdatePlan、KnowledgeRevealEvent 候选 | 可拥有 God 读取权限；但输出只是候选结果与候选更新，最终提交由 EffectValidator + StateCommitter 程序执行 |
| SurfaceRealizer（叙事文本输出器） | NarrationScope 派生的 SceneNarrativeView、角色心理/情绪摘要、实际言行、交互/对抗结果、文风/格式/叙事倾向 | 面向用户的自由文本叙事 | 不得突破 NarrationScope / narratable_facts；不得引入新事实 |

权限规则：

- **God 读取权限不等于提交权限**：SceneInitializer / SceneStateExtractor / OutcomePlanner 即使读取 L1 或公开世界上下文，也只能输出严格 schema 的候选 draft / delta / plan；写库只由程序提交。
- **受限 LLM**：CharacterCognitivePass 和 SurfaceRealizer 是主要防泄露对象，必须只接收过滤后的输入。
- **程序验证永远在提交前**：任何 LLM 产出的状态变化都必须经过 schema 校验、一致性校验、访问权限 / 叙事披露校验与 StateCommitter。
- **自由文本字段不驱动程序判断**：LLM 输出里的心理活动、叙事倾向、说明文本只能作为 `llm_readable` 或 `trace_only`，程序判断依赖结构化字段。

### 4.2 职责边界总表

| 任务 | 归属 | 形态约束 |
|---|---|---|
| 用户自由文本接收 | 程序（IO） | 自由文本入 |
| 场景初始化 / 切场景补全 | **LLM**（SceneInitializer）+ 程序校验 | 输入为结构化 SceneSeed + 公开上下文；输出严格 schema 的 SceneInitializationDraft；只能按 generation_policy 补全 |
| 用户输入与场景变化提取 | **LLM**（SceneStateExtractor） | 输入为最近自由文本 + 当前结构化 Scene JSON；输出严格 schema 的 SceneUpdate / UserInputDelta |
| 场景物理状态维护 | 程序 | 全程结构化 |
| 身体状态机械演化（毒衰减/愈合/冷却） | 程序 | 全程结构化 |
| 情绪驱动的身体反应 | LLM（CognitivePass 输出 BodyReactionDelta） | 严格 schema；只作为候选反应/外显信号，不直接写 Layer 1 |
| 事件 delta 计算 | 程序 | 全程结构化 |
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

### 关键铁律

1. **自由文本顶层 I/O 仅在三处出现**：用户输入、SceneStateExtractor 输入、SurfaceRealizer 输出。SceneInitializer 只能接收结构化 SceneSeed 与 llm_readable 公开上下文，不接收原始用户自由文本；其他中间节点必须为严格 schema JSON；LLM-readable 文本叶子字段只供阅读，不参与程序判断。
2. **KnowledgeAccessResolver 永不调 LLM**：Knowledge 访问权限判断必须确定性；数据库索引只服务查询性能，不承担最终判定。
3. **LLM 输出必须严格 schema**：优先依赖 Provider 的 structured output / tool schema；仅在无强 schema 能力时退化到 JSON mode + schema 校验 + 重试 / 程序容错。
4. **受限 LLM 不读真相**：CharacterCognitivePass 和 SurfaceRealizer 只读过滤后的输入；SceneInitializer / SceneStateExtractor / OutcomePlanner 的 God-read 或公开上下文读取权限必须显式声明。
5. **数值字段不让受限 LLM 直出**：信念/情绪变化用离散级别，由程序映射为数值；对抗解算数值结果必须可被程序公式校验。
6. **God 读取不等于提交权限**：场景初始化、结果规划与场景提取 LLM 只产出候选 JSON，最终状态写入必须由程序校验并提交。
7. **反应窗口有界**：主动行动可打开一次 ReactionWindow；窗口内只收集合法 ReactionIntent，不即时递归结算；默认 `no_reaction_to_reaction = true`、`one_reaction_per_character_per_window = true`。
8. **叙事不引入新事实**：SurfaceRealizer 受 NarrationScope 派生的 narratable_facts 白名单约束，由 NarrativeFactCheck 强制。
9. **PromptBuilder 是 Agent LLM 调用唯一入口**：静态提示词只写节点契约并版本化；动态部分只传对应 `*Input` schema JSON；不得把日志、隐藏事实或临时自然语言说明绕过类型系统塞进 prompt。
10. **日志不驱动业务**：Agent Trace 和运行 Logs 只用于观察、调试、审计、回放定位；不得参与程序判断、检索、访问控制或 LLM prompt 组装。

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

**API 配置与预设独立于会话**：
- API 配置（Provider、endpoint、model、鉴权）存储在 `./data/api_configs/`
- 预设（采样参数、提示词模板）存储在 `./data/presets/`
- 两者均由全局应用状态管理，用户可随时切换，不与会话绑定
- 同一预设可用于不同 API 配置，切换 Provider 无需重新选择预设
- 详见 [74_st_presets.md](74_st_presets.md) 与 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)

`api_configs/` 是全应用共享的 AI Provider 配置池。第一版 Agent 模式不另建一套 Provider 配置，而是从该配置池中为五类 Agent LLM 节点分别选择配置；具体绑定关系存于 Agent profile / World settings，见 [20_backend_contracts.md](20_backend_contracts.md)。

第一版必须把以下 AI Provider / 协议视为一等适配目标：OpenAI Responses API、OpenAI Chat Completions API、Google Gemini GenerateContent、Anthropic Messages、DeepSeek、Claude Code Interface。后续任何 API 配置、请求组装、日志、结构化输出或流式传输相关改动，都必须检查这六类适配面的影响；新增 Provider 不能降低这六类的兼容性要求。

### 5.3 Agent 模式数据布局与故事线定位

Agent 模式以 World 为顶层隔离单元。一个 World 不是普通聊天文件夹，而是一个持续演化的拟真故事世界；世界设定、人物状态、历史事件、聊天记录和回放 trace 必须共享同一条故事线。Agent 世界数据存放在应用数据目录的 `data/worlds/<world_id>/` 下，每个世界独立保存 SQLite 数据库、运行时快照、回放 trace 和必要资源；全局运行 Logs 作为应用观测数据存放在 `data/logs/`：

```
./data/
├── logs/
│   ├── app_logs.sqlite
│   └── archives/
└── worlds/
    ├── <world_id>/
    │   ├── world.sqlite
    │   ├── traces/
    │   └── assets/
    └── <world_id>/
```

SQLite 内部表结构见 [14_agent_persistence.md](14_agent_persistence.md)。Layer 2 派生视图不持久化，每回合由 Layer 1 / Layer 3 重建。

用户删除或回退 Agent 聊天记录时，不能只删除消息文本，也不能单独删除中间某一条消息。Agent 模式只允许从目标消息对应回合开始，连同其后的所有回合一起截断；系统必须同步回滚这些回合造成的人物数据、世界数据、知识揭示、主观状态和 trace 记录。因此每个已提交回合必须有 `scene_turn_id`、父回合关系、状态提交记录和可回滚快照；删除操作本质上是回到目标父回合的一致世界状态。

Agent Trace 是世界调试与回放数据，随 World 保存；运行 Logs 是应用观测数据，用于记录 LLM 请求响应、Provider 错误与异常事件。回滚 Agent 回合时，世界状态与回合 trace 按故事线回退；运行 Logs 默认保留为审计记录，不随剧情回滚物理删除。

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
│   ├── logging/             # 日志与可观测性
│   │   ├── mod.rs
│   │   ├── context.rs       # LogContext / request_id / trace_id
│   │   ├── llm_logger.rs    # Provider logging wrapper
│   │   ├── event_logger.rs  # app_event_logs
│   │   └── retention.rs     # 1GB 默认清理策略
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
| `simulation::scene_initializer` | 调 LLM 从结构化 SceneSeed 与公开上下文生成候选 SceneModel 草案 | 不读隐藏 Knowledge / GodOnly；不直接写 Layer 1；不创建未授权持久实体 |
| `simulation::scene_extractor` | 调 LLM 把用户自由文本解析为 UserInputDelta | 不写 Layer 1（写入由 runtime 协调）；不解析中间数据 |
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
