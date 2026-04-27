# 01 总体架构

本文档承载：

- 双模式总体架构图
- 设计原则
- 前/后端模块结构
- LLM 与程序的职责边界总表 + 7 大铁律
- 数据形态铁律（自由文本三关口）

数据契约与程序化派生公式见 [10_agent_data_and_simulation.md](10_agent_data_and_simulation.md)。运行时主循环与验证规则见 [11_agent_runtime.md](11_agent_runtime.md)。

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
│  │  Scene / Embodiment / Filter / Memory / Arbitration      │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Validation Layer (程序化验证)               │  │
│  │  Omniscience / Embodiment / Memory / Mana / Consistency  │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                       Storage Layer                      │  │
│  │   JSON (SillyTavern)   SQLite (Agent)   Trace Log        │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌────────────────┐
                    │  外部 AI APIs  │
                    │ Claude/GPT/etc │
                    └────────────────┘
```

---

## 2. 设计原则

- **Character-Centered Reasoning** — 以角色为中心，而非以场景为中心。
- **Subjective Access** — 角色仅从其能合理获取的信息中推理。
- **Embodied Cognition** — 角色通过当前身体状态感知与推理。
- **Bias Is Causal** — 信念、情绪、关系主动塑造感知与解释。
- **Belief-Driven Action** — 意图源于角色当前信念，即便信念不完整或错误。
- **Structured Handoffs** — 各阶段显式分离并交换结构化输出。
- **Traceable Subjectivity** — 误判、过度反应可在系统层面解释。
- **Truth ≠ Accessible Truth** — 客观真相只有编排器、仲裁、验证可读；角色 LLM 永远经过可见性过滤。
- **Single Source of Visibility** — 所有可见性判断由 VisibilityResolver 集中处理；禁止散落在 prompt builder 或业务代码中。

---

## 3. 数据形态铁律

**自由文本仅允许出现在三处**，所有中间数据节点必须是结构化 JSON。

| 位置 | 形态 | 说明 |
|---|---|---|
| 用户输入 | 自由文本 | 用户对话框输入、扮演角色的言行、元指令 |
| SceneStateExtractor 输入 | 自由文本 | 用户/作者的自然语言叙事描述 |
| SurfaceRealizer 输出 | 自由文本 | 给用户阅读的最终叙事 |

**所有其他中间节点**（Layer 1 / Layer 2 / Layer 3 数据、CognitivePass 输入输出、仲裁输入输出、SurfaceRealizer 输入）**必须为严格 schema JSON**。

### 自由文本进出系统的关口

```
[用户自由文本]
       ↓
SceneStateExtractor (LLM, 严格 schema 输出)
       ↓
[结构化 UserInputDelta]
       ↓
   主循环（全程结构化）
       ↓
[结构化 ArbitrationResult / CognitivePassOutput / ...]
       ↓
SurfaceRealizer (LLM)
       ↓
[自由文本叙事 → 用户]
```

### 例外：作者配置中的描述性字段

文风约束（StyleConstraints）等"作者预设、最终交给 LLM 阅读"的配置中，允许字段值含自由文本字符串。原则：**自由文本字段的值仅作为 LLM 的提示输入，不参与程序逻辑判断 / 检索 / 规则匹配**。

### KnowledgeEntry 内容的结构化要求

`KnowledgeEntry.content` 必须包含核心结构化字段（用于程序判断、可见性、检索），可选包含 `summary_text` 等自由文本辅助字段（仅供 LLM 阅读理解）。详见 [10_agent_data_and_simulation.md](10_agent_data_and_simulation.md) 的 KnowledgeEntry 章节。

---

## 4. LLM 与程序边界总表

| 任务 | 归属 | 形态约束 |
|---|---|---|
| 用户自由文本接收 | 程序（IO） | 自由文本入 |
| 用户输入解析 | **LLM**（SceneStateExtractor） | 输出严格 schema JSON |
| 场景物理状态维护 | 程序 | 全程结构化 |
| 身体状态机械演化（毒衰减/愈合/冷却） | 程序 | 全程结构化 |
| 情绪驱动的身体反应 | LLM（CognitivePass 输出 BodyReactionDelta） | 严格 schema |
| 事件 delta 计算 | 程序 | 全程结构化 |
| 脏标志（客观子集） | 程序 | 仅以下 5 项触发 cognitive pass：directly_addressed / under_threat / reaction_window_open / scene_changed / body_changed |
| 脏标志（主观显著性） | 不作触发条件，仅 prompt hint | - |
| EmbodimentResolver | 程序 | 公式化；含 environmental_strain 档位翻译 |
| 物理量→档位翻译（风/温/能见度/地表/降水/呼吸） | 程序 | 严禁 LLM 从 raw m/s, ℃ 推断后果；档位针对该角色物种已校准；body 侧与 perception 侧共享阈值表 |
| 灵力数值→档位翻译（个体/环境） | 程序 | LLM 不读 raw mana_power；档位边界来自世界配置（默认对 rp_cards 锚点校准） |
| 灵力差距→感知档（Δ 桶） | 程序 | 阈值 200/500/1000/2500 共用同一份表；感知层用 displayed_mana_power |
| 灵力压制/隐匿的"破绽"判定 | 程序 | concealment_suspected 由 (observer.effective vs target.effective − 200) + 灵觉敏锐度计算；不让 LLM 自行猜"他是不是在装弱" |
| 灵力对抗仲裁 | 程序 | 仲裁用 effective_mana_power × 加算修正区 × soul_factor，桶映射到 outcome_tier；不读 displayed |
| 仲裁→社会后果 | 不在仲裁层 | 物理后果写回 L1；恐惧/屈服/记仇由下游 LLM 解读 |
| 可见性判断（VisibilityResolver） | 程序 | 严格禁止 LLM 介入 |
| 场景过滤 + visible_facets | 程序 | 全程结构化 |
| KnowledgeAccess | 程序 | 全程结构化 |
| InputAssembly | 程序 | 类型隔离，禁止 Layer 1 原始对象 |
| 主观感知 / 偏见解释 / 意图生成 | **LLM**（CharacterCognitivePass） | 输出严格 schema JSON；信念变化用离散级别 |
| 客观演绎推理 | 程序（在 Knowledge 中预存事实） | LLM 不擅长长链推理 |
| 物理仲裁（命中/资源/位置/技能） | 程序 | 全程结构化 |
| 认知输出容错（残缺 JSON 修复） | 程序 | 修复常见错误 |
| 认知输出兜底解读 | **LLM**（仲裁层） | 修复失败时启用，输出严格 schema |
| 社会层后果（被骗/被劝服） | 不在仲裁层处理；下游角色 LLM 自行解释 | - |
| 叙事渲染 | **LLM**（SurfaceRealizer） | 输入严格结构化 + StyleConstraints；输出自由文本 |
| NarrativeFactCheck | 程序 | 扫描叙事文本提及事实 ⊆ visible_facts |
| 验证规则 | 程序 | 全程结构化 |
| 状态提交 | 程序 | 全程结构化 |
| 用户扮演输入验证 | 程序（同样跑 Validator） | 一致性 |

### 7 大关键铁律

1. **自由文本仅在三处出现**：用户输入、SceneStateExtractor 输入、SurfaceRealizer 输出。其他全部结构化。
2. **VisibilityResolver 永不调 LLM**：可见性判断必须确定性。
3. **LLM 输出必须严格 schema**：依赖 Provider 的 JSON mode + prompt 模板 + 程序容错。
4. **数值字段不让 LLM 直出**：信念/情绪变化用离散级别，由程序映射为数值。
5. **客观推理交给数据**：长链客观推理通过 Knowledge 预存事实实现，不让 LLM 即兴推理。
6. **社会后果不在仲裁层**：下游角色的 LLM 自行解读社会信号（"我相信他了吗"）。
7. **叙事不引入新事实**：SurfaceRealizer 受 visible_facts 白名单约束，由 NarrativeFactCheck 强制。

---

## 5. 模块结构

### 5.1 前端 (Vue 3)

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
│   │   ├── knowledge.ts                 # KnowledgeEntry / VisibilityPredicate
│   │   ├── embodiment.ts                # EmbodimentState / FilteredSceneView
│   │   ├── accessible.ts                # AccessibleKnowledge
│   │   ├── subjective.ts                # CharacterSubjectiveState
│   │   └── cognitive.ts                 # CognitivePass I/O
│   └── api.ts
├── views/
└── router/
```

### 5.2 后端 (Rust)

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
│   │   │   ├── knowledge.rs             # KnowledgeEntry / VisibilityPredicate / SubjectAwareness
│   │   │   ├── character.rs             # CharacterRecord / BaselineBodyProfile / MindModelCard
│   │   │   ├── embodiment.rs            # EmbodimentState
│   │   │   ├── filtered_view.rs         # FilteredSceneView / VisibleEntity
│   │   │   ├── accessible.rs            # AccessibleKnowledge / AccessibleEntry
│   │   │   ├── subjective.rs            # CharacterSubjectiveState（Layer 3）
│   │   │   ├── cognitive.rs             # CognitivePass I/O
│   │   │   ├── skill.rs
│   │   │   └── dirty_flags.rs
│   │   ├── knowledge/       # 知识子系统（Layer 1 → Layer 2 派生核心）
│   │   │   ├── store.rs                 # KnowledgeStore：CRUD（不做可见性判断）
│   │   │   ├── visibility.rs            # VisibilityResolver：所有可见性逻辑唯一入口
│   │   │   ├── access.rs                # KnowledgeAccessProtocol：构建 AccessibleKnowledge
│   │   │   └── reveal.rs                # KnowledgeRevealEvent 处理
│   │   ├── simulation/      # 程序化核心
│   │   │   ├── scene_extractor.rs
│   │   │   ├── embodiment_resolver.rs
│   │   │   ├── scene_filter.rs          # 含 visible_facets 计算（调用 VisibilityResolver）
│   │   │   ├── input_assembly.rs        # 拼装 CognitivePassInput（保证不漏 Layer 1）
│   │   │   └── arbitration.rs           # 仲裁层（直接读 Layer 1 真相）
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
│   └── models/
└── Cargo.toml
```

### 5.3 模块职责边界（避免屎山）

| 模块 | 唯一职责 | 禁止做的事 |
|---|---|---|
| `knowledge::store` | KnowledgeEntry 的 CRUD | 不做可见性判断，不读 Layer 3 |
| `knowledge::visibility` | 给定 (entry, character, context) → bool | 严禁调 LLM；不读 Layer 3 belief；不修改任何状态 |
| `knowledge::access` | 给定 character → AccessibleKnowledge | 不调 LLM，不修改 belief |
| `knowledge::reveal` | 处理 KnowledgeRevealEvent | 仅追加新 known_by 与生成 Memory，不重写既有 content |
| `simulation::scene_extractor` | 调 LLM 把用户自由文本解析为 UserInputDelta | 不写 Layer 1（写入由 runtime 协调）；不解析中间数据 |
| `simulation::scene_filter` | 当下感官过滤 + 计算 visible_facets | 不读 Knowledge content，仅判断 facet 可见性 |
| `simulation::input_assembly` | 拼装 CognitivePassInput | 不调 LLM，不做语义判断；输入禁止携带 Layer 1 原始对象 |
| `simulation::arbitration` | 物理后果判定 + 认知输出兜底解读（混合层） | 物理判定纯程序；LLM 兜底仅用于解析失败时；不处理社会后果 |
| `cognitive::cognitive_pass` | 调 LLM 输出严格 schema JSON | 不做验证，不直接修改 Layer 1/3 |
| `validation::*` | 检查输入/输出对 | 不修改任何状态；不调 LLM |
| `presentation::surface_realizer` | 调 LLM 渲染叙事 | 受 visible_facts 白名单约束；不引入新事实 |
| `agent::runtime` | 编排上述模块 | 不嵌入业务逻辑（仅做调度） |
