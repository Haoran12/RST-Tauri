# RST-Tauri AI 聊天工具实现计划

## 项目概述

基于 Tauri 的双模式 AI 聊天应用Ran's SmartTavern)：

- **SillyTavern 模式**：复刻 SillyTavern 体验，支持角色卡、世界书，预设, API配置的JSON 存储.
- **Agent 模式**：基于 RP Agent 架构的高级角色扮演系统，分层"客观世界 / 人物具身状态 / 主观认知与意图 / 仲裁"架构，SQLite 存储。

> **架构基础**：参考 `D:\Projects\RST-flutter\docs\rp_agent_*` 系列文档（成熟的角色扮演 Agent 架构），本计划在其基础上为 Tauri + Rust + Vue 3 技术栈做适配。

---

## 1. 技术栈

| 层 | 选型 | 理由 |
|---|---|---|
| 前端框架 | Vue 3 + TypeScript | 生态成熟、组合式 API、Pinia 类型友好 |
| UI 组件库 | Naive UI | Vue 3 原生支持，组件丰富，TypeScript 友好，暗色主题完善 |
| 状态管理 | Pinia | 内置、类型安全 |
| 路由 | Vue Router | 标准方案 |
| 后端 | Tauri + Rust | 小型二进制、跨平台、安全 |
| 存储 - ST 模式 | JSON 文件 | 与 SillyTavern 兼容 |
| 存储 - Agent 模式 | SQLite | 结构化查询、事务、性能 |
| AI 后端 | 多 Provider | Claude / GPT / Gemini / Ollama |

---

## 2. 文档体系

参考 `rp_agent_docs_index.md` 的分层文档原则：

```
docs/
├── 01_rst_framework_spec.md            # 架构与设计哲学
├── 02_rst_runtime_protocol_spec.md     # 数据契约与模块接口
├── 03_rst_prompt_skill_spec.md         # Prompt 实现指南
├── 04_rst_persistence_validation_spec.md   # 持久化与验证
├── 05_rst_silly_tavern_compat_spec.md  # SillyTavern 兼容层
└── 06_rst_test_cases.md                # 测试用例
```

**文档维护原则**：

- 每份文档有单一职责边界（设计 / 协议 / Prompt / 持久化）。
- 上层文档定义概念，下层文档操作化为可实现契约。
- **修改时直接更新最新版，不保留历史对比、版本演进或"改进前后"标记**。
- 概念变更从 framework spec 开始，向下传递。

---

## 3. 系统架构

### 3.1 总体架构

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

### 3.2 三层数据语义（强制隔离）

为避免"客观真相"与"主观认知"在代码中混淆，运行时数据严格分为三层。**层间只能通过定义好的派生关系流动，禁止跨层直接读写。**

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 1 — Truth Store（客观真相，仅编排器与仲裁/验证层访问）│
│  ├── SceneModel              场景客观状态                    │
│  ├── KnowledgeEntry[*]       统一知识库（含世界/势力/角色档 │
│  │                           案/记忆，带可见性谓词）         │
│  └── 角色 baseline_body_profile（物种/感官基线/灵觉基线）    │
│  约束：LLM 永远不直接读取此层；CognitivePassInput 不出现     │
│        Layer 1 原始对象。                                    │
└──────────────────────────────────────────────────────────────┘
                  │ 经 VisibilityResolver + SceneFilter 派生
                  ▼
┌──────────────────────────────────────────────────────────────┐
│  Layer 2 — Per-Character Access（角色可触及的客观，每回合重 │
│            建，无独立持久化）                                │
│  ├── FilteredSceneView       能感知的场景 + 可见 facets      │
│  ├── AccessibleKnowledge[*]  通过可见性过滤的 KnowledgeEntry │
│  │                           视图（含表象 / 自以为版本）     │
│  └── EmbodimentState         具身状态（每回合从 baseline +   │
│                              temp + scene 计算）             │
│  约束：每条数据必有可追溯的 Layer 1 来源；不可被 LLM 改写。  │
└──────────────────────────────────────────────────────────────┘
                  │ CognitivePass 输入
                  ▼
┌──────────────────────────────────────────────────────────────┐
│  Layer 3 — Subjective State（角色主观心智，每回合 cognitive │
│            pass 后更新并持久化）                             │
│  ├── BeliefState             关于世界/事件的命题信念         │
│  ├── EmotionState            情绪                            │
│  ├── RelationModels[*]       对他人的主观印象（trust/感知意 │
│  │                           图/主观评价）                   │
│  ├── CurrentGoals            目标（含 hidden）               │
│  └── temporary_body_state    伤势/疲惫/痛感/灵力消耗          │
│  约束：本层是 LLM 的输出领地；任何关于"我相信 B 是好人"      │
│        的命题进入 RelationModels 而非 BeliefState（避免重复）│
└──────────────────────────────────────────────────────────────┘
```

**信息流向（线性，单向）**：

```
World Truth (L1)
    → Embodiment 计算 (L1 baseline + L1 temp + L1 scene → L2 embodiment)
    → SceneFilter (L1 scene + L2 embodiment → L2 filtered_view)
    → KnowledgeAccess (L1 knowledge_store + 角色身份/属性 → L2 accessible_knowledge)
    → InputAssembly (L2 全部 + L3 prior → CognitivePassInput)
    → CognitivePass(LLM) → Output(perception/belief/intent)
    → Validator (扫描 Output 引用是否 ⊆ L2 输入)
    → Arbitration (读 L1 真相 + 多角色意图 → 行为后果)
    → SurfaceRealizer (渲染叙事)
    → StateCommitter (更新 L1 + L3，处理 KnowledgeRevealEvent)
```

### 3.3 设计原则

- **Character-Centered Reasoning** - 以角色为中心，而非以场景为中心。
- **Subjective Access** - 角色仅从其能合理获取的信息中推理。
- **Embodied Cognition** - 角色通过当前身体状态感知与推理。
- **Bias Is Causal** - 信念、情绪、关系主动塑造感知与解释。
- **Belief-Driven Action** - 意图源于角色当前信念，即便信念不完整或错误。
- **Structured Handoffs** - 各阶段显式分离并交换结构化输出。
- **Traceable Subjectivity** - 误判、过度反应可在系统层面解释。
- **Truth ≠ Accessible Truth** - 客观真相只有编排器、仲裁、验证可读；角色 LLM 永远经过可见性过滤。
- **Single Source of Visibility** - 所有可见性判断由 VisibilityResolver 集中处理；禁止散落在 prompt builder 或业务代码中。

### 3.4 数据形态铁律

**自由文本仅允许出现在三处**，所有中间数据节点必须是结构化 JSON。

| 位置 | 形态 | 说明 |
|---|---|---|
| 用户输入 | 自由文本 | 用户对话框输入、扮演角色的言行、元指令 |
| SceneStateExtractor 输入 | 自由文本 | 用户/作者的自然语言叙事描述 |
| SurfaceRealizer 输出 | 自由文本 | 给用户阅读的最终叙事 |

**所有其他中间节点**（Layer 1 / Layer 2 / Layer 3 数据、CognitivePass 输入输出、仲裁输入输出、SurfaceRealizer 输入）**必须为严格 schema JSON**。

#### 自由文本进出系统的关口

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

#### 例外：作者配置中的描述性字段

文风约束（StyleConstraints）等"作者预设、最终交给 LLM 阅读"的配置中，允许字段值含自由文本字符串。原则：**自由文本字段的值仅作为 LLM 的提示输入，不参与程序逻辑判断 / 检索 / 规则匹配**。

#### KnowledgeEntry 内容的结构化要求

`KnowledgeEntry.content` 必须包含核心结构化字段（用于程序判断、可见性、检索），可选包含 `summary_text` 等自由文本辅助字段（仅供 LLM 阅读理解）。详见 §4.2.1。

---

## 4. 数据模型

### 4.1 SillyTavern 模式 (JSON)

#### 角色卡 (TavernCard V3)

基于 SillyTavern `character-card-parser.js` 与 `spec-v2.d.ts`：

```typescript
interface TavernCardV3 {
  spec: 'chara_card_v3';
  spec_version: string;  // >= 3.0 and < 4.0
  data: {
    name: string;
    description: string;
    personality: string;
    scenario: string;
    first_mes: string;
    mes_example: string;
    creator_notes: string;
    system_prompt: string;
    post_history_instructions: string;
    alternate_greetings: string[];
    tags: string[];
    creator: string;
    character_version: string;
    extensions: Record<string, any>;
    character_book?: CharacterBook;
  };
}
```

#### 世界书 (CharacterBook)

完整字段（覆盖 SillyTavern 全部能力）：

- **匹配**：主关键词 / 次关键词 + 触发逻辑（AND_ANY / NOT_ALL / NOT_ANY / AND_ALL）+ 大小写 / 全词匹配 / 正则
- **插入**：position（7 种枚举：BEFORE_CHAR / AFTER_CHAR / AN_TOP / AN_BOTTOM / AT_DEPTH / EM_TOP / EM_BOTTOM）+ depth + order
- **概率**：probability(0-100) + useProbability
- **递归**：excludeRecursion / preventRecursion / delayUntilRecursion
- **分组**：group / groupOverride / groupWeight / useGroupScoring
- **时间**：sticky / cooldown / delay
- **匹配目标扩展**：matchPersonaDescription / matchCharacterDescription / matchScenario / matchCreatorNotes 等
- **其他**：constant（常驻）/ vectorized / automationId / role

### 4.2 Agent 模式

数据模型按三层语义组织。Layer 1 是真相存储，Layer 2 是派生视图，Layer 3 是角色心智。

#### 4.2.1 Layer 1 — Truth Store

##### Scene Model

```rust
pub struct SceneModel {
    pub scene_id: String,
    pub scene_turn_id: String,
    pub time_context: TimeContext,             // 时间/天气/可见度
    pub spatial_layout: SpatialLayout,         // 空间布局/障碍物/入口
    pub lighting: LightingState,               // 光照/光源/阴影/逆光
    pub acoustics: AcousticsState,             // 环境噪声/反射特性
    pub olfactory_field: OlfactoryField,       // 气味场/气流/气味源
    pub physical_atmosphere: PhysicalAtmosphere, // 温湿度/表面状态
    pub mana_field: ManaField,                 // 灵力场（玄幻扩展）
    pub entities: Vec<SceneEntity>,            // 在场实体（id + 位置 + 姿态）
    pub observable_signals: ObservableSignals,
    pub event_stream: Vec<SceneEvent>,
    pub uncertainty_notes: Vec<String>,
}
```

##### Mana Field（玄幻扩展）

```rust
pub struct ManaField {
    pub ambient_density: f64,
    pub ambient_attribute: ManaAttribute,        // 金 木 水 火 土 风 
    pub mana_sources: Vec<ManaSource>,
    pub flow: ManaFlow,
    pub interferences: Vec<ManaInterference>,    // 屏蔽/扰乱/伪装/放大/重定向
}

pub enum ManaSourceType {
    SpiritVein, FormationCore, BarrierNode, SpiritWell,            // 环境源
    CultivatorAura, ArtifactAura, SpiritBeastAura, FormationTrace, // 实体源
    SpellResidue, Breakthrough, Tribulation, Sacrifice,            // 事件源
    Corruption, Seal, VoidRift,                                    // 异常源
}
```

##### KnowledgeEntry（统一知识模型）

`KnowledgeEntry` 是 Layer 1 的核心，承载世界设定 / 地区设定 / 势力设定 / 角色档案分面 / 历史事件（Memory）。所有"谁能知道什么"的判断由它的 `visibility` 字段决定，由 `VisibilityResolver` 统一计算。

```rust
pub struct KnowledgeEntry {
    pub knowledge_id: String,
    pub kind: KnowledgeKind,
    pub subject: KnowledgeSubject,
    pub content: serde_json::Value,                  // 客观真相（结构化）
    pub apparent_content: Option<serde_json::Value>, // 表象（伪装/欺骗时给观察者看的版本）
    pub visibility: VisibilityPredicate,
    pub subject_awareness: SubjectAwareness,         // 仅 subject 为 Character 时有意义
    pub metadata: KnowledgeMetadata,
    pub schema_version: String,
}

pub enum KnowledgeKind {
    WorldFact,        // 世界级设定（宇宙规则、修真体系）
    RegionFact,       // 地区设定（北境地理、风俗）
    FactionFact,      // 势力设定（玄天宗内规、口诀）
    CharacterFacet,   // 角色档案分面（外貌/身份/能力/血脉/...）
    Memory,           // 历史事件（亲历或传闻）
}

pub enum KnowledgeSubject {
    World,
    Region(String),
    Faction(String),
    Character { id: String, facet: CharacterFacetType },
    Event { event_id: String },
}

pub enum CharacterFacetType {
    Appearance,        // 外观（可被同场景观察）
    Identity,          // 公开身份/称谓
    TrueName,          // 真实姓名
    Species,           // 种族
    Bloodline,         // 血脉
    CultivationRealm,  // 修为境界
    KnownAbility,      // 已展示的能力
    HiddenAbility,     // 隐藏能力
    Personality,       // 性格特质
    Background,        // 出身背景
    Motivation,        // 真实动机
    Trauma,            // 创伤
    // 可扩展
}

pub struct VisibilityPredicate {
    // 三谓词，OR 关系（任一为真即可见）
    pub known_by: Vec<String>,                  // 名单制
    pub scope: Vec<VisibilityScope>,            // 标签制
    pub conditions: Vec<VisibilityCondition>,   // 条件制（运行时求值）
}

pub enum VisibilityScope {
    Public,                  // 所有原住民
    GodOnly,                 // 仅编排器（无人可知）
    Region(String),          // 在该地区的角色
    Faction(String),         // 该势力成员
    Realm(String),           // 修为门槛及以上
    Role(String),            // 担任该职位
    Bloodline(String),       // 该血脉
    // 可扩展
}

pub enum VisibilityCondition {
    InSameSceneVisible,                                    // 同场景且能感知
    RelationAtLeast { target: String, threshold: f64 },    // 与目标关系阈值
    HasSkill(String),                                      // 拥有特定技能
    CultivationAtLeast(String),                            // 修为达到
    CustomExpression(String),                              // DSL 扩展点
    // 可扩展
}

pub enum SubjectAwareness {
    /// 默认：subject 自己知道关于自己的这条 facet。
    /// 在为 subject 构建 accessible_knowledge 时，content 直接可见。
    Aware,

    /// subject 不知道客观真相，但有一个"自以为是"的版本。
    /// 在为 subject 构建 accessible_knowledge 时：返回 self_belief，content 保持隐藏。
    Unaware { self_belief: serde_json::Value },
}

pub struct KnowledgeMetadata {
    pub created_at: DateTime,
    pub updated_at: DateTime,
    // Memory 专用（其他 kind 留空）
    pub emotional_weight: Option<f64>,
    pub last_accessed_at: Option<DateTime>,
    pub source: Option<String>,                  // 知识来源：witnessed / told_by / inferred
}
```

**关键不变量**：

1. `content` 永远不进入 LLM，除非 `VisibilityResolver` 判定该角色对该 entry 完全可见。
2. `subject == Character{id: A}` 且 `subject_awareness == Unaware{self_belief}` 时：A 的 accessible_knowledge 中只见 `self_belief`，看不到 `content`。
3. `apparent_content` 存在时：观察者（非 subject）默认看到 `apparent_content`；只有满足"揭穿条件"或在 `known_by` 中的角色才看到 `content`。
4. `visibility.scope` 含 `GodOnly` 表示仅编排器可读，对所有角色不可见。
5. `MemoryEntry` 不再独立存在；历史事件以 `KnowledgeEntry { kind: Memory }` 形式统一存储。
6. Layer 1 的 Knowledge 内容由编排器/作者/StateCommitter 写入；CognitivePass 不可写。

##### Content Schema 约定（核心字段 + extensions 兜底）

`content` / `apparent_content` / `self_belief` 三者共享同一套 sub-schema，按 `kind` × `facet_type` 决定字段定义。

**通用规则**：
- 必含 `summary_text: string`（自由文本简述，供 LLM 快速理解；不参与程序判断）。
- 必含该 kind/facet 的核心结构化字段（用于程序检索 / 可见性 / 规则匹配）。
- 可含 `extensions: Record<String, Any>` 用于扩展（不参与核心程序逻辑，可供 LLM 阅读）。

**示例：CharacterFacet::Appearance**

```json
{
  "summary_text": "高个子瘦削男人，黑发，左脸有疤，蓝袍白裤",
  "height": "tall",
  "build": "lean",
  "hair": {"color": "black", "style": "long_loose"},
  "distinctive_marks": ["scar_on_left_cheek"],
  "clothing": {"upper": "blue_robe", "lower": "white_pants"},
  "extensions": {}
}
```

**示例：CharacterFacet::CultivationRealm**

```json
{
  "summary_text": "金丹中期",
  "realm": "golden_core",
  "stage": "mid",
  "progress_within_stage": 0.6,
  "extensions": {}
}
```

**示例：CharacterFacet::HiddenAbility**

```json
{
  "summary_text": "可短暂凝聚虚空裂隙",
  "ability_id": "void_rift_summon",
  "category": "soul",
  "trigger_condition": "extreme_duress",
  "extensions": {}
}
```

**示例：KnowledgeKind::FactionFact**

```json
{
  "summary_text": "玄天宗执事可调用三品法宝",
  "rule_id": "xuantian_rank_3_artifact",
  "applies_to": {"role": "executor"},
  "extensions": {}
}
```

**示例：KnowledgeKind::Memory**

```json
{
  "summary_text": "在庭院中被 Bob 突然反目袭击",
  "event_type": "betrayal",
  "actor": "bob",
  "target": "self",
  "location": "courtyard",
  "timestamp": "2 weeks ago",
  "key_observations": ["bob_smiled_then_attacked"],
  "extensions": {}
}
```

**核心字段表**（所有 facet/fact 类型应预定义最小集）由 `models/knowledge_schemas.rs` 维护，每种类型一个 struct。`extensions` 总是 `serde_json::Map<String, serde_json::Value>` 兜底。

##### KnowledgeRevealEvent（可见性扩展）

可见性变化必须通过显式事件触发，禁止隐式修改 `visibility.known_by`：

```rust
pub struct KnowledgeRevealEvent {
    pub event_id: String,
    pub knowledge_id: String,
    pub newly_known_by: Vec<String>,   // 此次新增的知情者
    pub trigger: RevealTrigger,        // 何种触发：witnessed / told / inferred / awakened
    pub scene_turn_id: String,
}
```

由 StateCommitter 处理：
- 更新 `KnowledgeEntry.visibility.known_by`。
- 在 event_stream 追加事件。
- 创建一条 `KnowledgeEntry { kind: Memory }` 记录"X 何时如何获知 Y"。

#### 4.2.2 Layer 2 — Per-Character Access

##### EmbodimentState

```rust
pub struct EmbodimentState {
    pub character_id: String,
    pub scene_turn_id: String,
    pub sensory_capabilities: SensoryCapabilities,  // vision/hearing/smell/touch/proprioception/mana
    pub body_constraints: BodyConstraints,          // 移动力/平衡/痛苦负载/疲惫/认知清晰度
    pub salience_modifiers: SalienceModifiers,      // 注意力吸引/厌恶触发/过载风险
    pub reasoning_modifiers: ReasoningModifiers,    // 痛苦偏倚/威胁偏倚/过载偏倚
    pub action_feasibility: ActionFeasibility,      // 物理执行/社交耐心/精细控制/持续注意
}

pub struct SensoryCapability {
    pub availability: f64,  // 0.0-1.0 可用性
    pub acuity: f64,        // 敏锐度
    pub stability: f64,     // 稳定性
    pub notes: String,
}
```

##### FilteredSceneView

```rust
pub struct FilteredSceneView {
    pub character_id: String,
    pub scene_turn_id: String,
    pub visible_entities: Vec<VisibleEntity>,
    pub audible_signals: Vec<AudibleSignal>,
    pub olfactory_signals: Vec<OlfactorySignal>,
    pub tactile_signals: Vec<TactileSignal>,
    pub mana_signals: Vec<ManaSignal>,
    pub mana_environment: ManaEnvironmentSense,
    pub spatial_context: SpatialContext,
}

pub struct VisibleEntity {
    pub entity_id: String,
    pub visibility_score: f64,
    pub clarity: f64,
    pub visible_facets: Vec<String>,   // KnowledgeEntry IDs（该角色当前对该实体可见的 facets）
    pub notes: String,
}
```

`visible_facets` 由 `SceneFilter` 与 `VisibilityResolver` 共同决定：感官可达 + facet 可见性谓词通过。

##### AccessibleKnowledge

```rust
pub struct AccessibleKnowledge {
    pub character_id: String,
    pub scene_turn_id: String,
    pub entries: Vec<AccessibleEntry>,
}

pub struct AccessibleEntry {
    pub knowledge_id: String,
    pub kind: KnowledgeKind,
    pub subject: KnowledgeSubject,
    pub visible_content: serde_json::Value,    // 经可见性裁剪后的内容（content / apparent_content / self_belief 三选一）
    pub source_hint: AccessSource,             // 该角色为何能看到这条（用于调试与 prompt 提示）
}

pub enum AccessSource {
    InKnownBy,              // 名单内
    ScopeMatch(String),     // 标签命中
    ConditionMet(String),   // 条件命中
    SelfFacetAware,         // 自身 facet 且 Aware
    SelfFacetBelief,        // 自身 facet 且 Unaware（看到的是 self_belief）
    ApparentFromObservation, // 通过同场景观察获取的表象
}
```

`AccessibleKnowledge` **完全无 Layer 1 原始引用**。它是为本回合本角色派生的**纯净视图**，可以安全地序列化进 prompt。

#### 4.2.3 Layer 3 — Subjective State

```rust
pub struct CharacterSubjectiveState {
    pub character_id: String,
    pub scene_turn_id: String,

    pub belief_state: BeliefState,                                // 关于世界/事件的命题信念
    pub emotion_state: EmotionState,
    pub relation_models: HashMap<String, RelationModel>,          // 对他人的主观印象
    pub current_goals: CurrentGoals,                              // 含 short_term / medium_term / hidden
    pub temporary_body_state: TemporaryBodyState,                 // 伤势/疲惫/痛感/灵力消耗
}
```

**职责边界**：
- "我相信 B 在撒谎" → `relation_models["B"].perceived_intent` 或类似字段。
- "我相信门外有刺客" → `belief_state` 中的命题信念。
- 不允许同一命题既写入 `belief_state` 又写入 `relation_models`。

#### 4.2.4 角色静态档案（Layer 1 入口）

角色不再有大而全的"static_profile" blob。所有"客观属于该角色的事实"都拆为多条 `KnowledgeEntry { kind: CharacterFacet, subject: Character{id, facet} }`，按需查询。

仅以下两项作为非 Knowledge 的角色基本数据保留在 Layer 1：

```rust
pub struct CharacterRecord {
    pub character_id: String,
    pub baseline_body_profile: BaselineBodyProfile,    // 物种/感官基线/灵觉基线（用于 EmbodimentResolver）
    pub mind_model_card: MindModelCard,                // 自我形象/世界观/恐惧触发/防御模式（属于 subject 自我认知，默认 Aware）
    pub schema_version: String,
}
```

注意：`MindModelCard` 在 Layer 1 也以 `KnowledgeEntry` 形式存在（subject 自我认知层），这里只是冗余索引以便 EmbodimentResolver 直接读取，**不允许它脱离 Knowledge 入口被外部读取**。

#### 4.2.5 Cognitive Pass I/O

```rust
pub struct CharacterCognitivePassInput {
    pub character_id: String,
    pub scene_turn_id: String,

    // Layer 2（每回合派生）
    pub filtered_scene_view: FilteredSceneView,
    pub embodiment_state: EmbodimentState,
    pub accessible_knowledge: AccessibleKnowledge,    // 含世界/势力/他人 facet/历史 memory，全部经可见性过滤

    // Layer 3（角色当前心智，作为先验）
    pub prior_subjective_state: CharacterSubjectiveState,

    // 本回合事件 delta（仅该角色可见的部分）
    pub recent_event_delta: Vec<SceneEvent>,
}

pub struct CharacterCognitivePassOutput {
    pub perception_delta: PerceptionDelta,
    pub belief_update: BeliefUpdate,
    pub intent_plan: IntentPlan,
    pub body_reaction_delta: Option<BodyReactionDelta>,  // 情绪驱动的身体反应（手抖/脸红/失语）
}

/// 信念变化使用离散级别，避免 LLM 直接输出浮点数。
pub enum ConfidenceShift {
    StrongDecrease,
    Decrease,
    Unchanged,
    Increase,
    StrongIncrease,
    Flip,                  // 完全翻转（A 原本相信 X，现在相信非 X）
}

pub struct BeliefUpdate {
    pub stable_beliefs_reinforced: Vec<BeliefShiftEntry>,    // confidence_shift: ConfidenceShift
    pub stable_beliefs_weakened: Vec<BeliefShiftEntry>,
    pub new_hypotheses: Vec<NewHypothesis>,                  // status: tentative|working|strong
    pub revised_models_of_others: Vec<RevisedRelationModel>,
    pub contradictions_and_tension: Vec<ContradictionResolution>,
    pub emotional_shift: EmotionalShiftDelta,                // 离散级别
    pub decision_relevant_beliefs: Vec<String>,
}
```

**LLM 永远只接触上述输入。** 任何尝试把 Layer 1 原始对象塞进 prompt 都被 InputAssembly 拒绝。

##### 输出严格 schema 与容错

CognitivePassOutput **必须为严格 JSON**，由 prompt 模板与 Provider JSON mode 共同保证。容错路径（详见 §6.7）：

1. **第一层（程序）**：JSON 解析失败时尝试常见修复（缺逗号、未转义引号、缺失非必需字段补默认值、字段名拼写偏差）。
2. **第二层（仲裁层 LLM 兜底）**：程序修复失败时，将原始残缺输出 + 上下文交给仲裁层 LLM，由其推断该角色"实际想做什么"，输出可用的 IntentPlan 替代。
3. **最终降级**：仲裁层也失败时，该角色本回合 fallback 到 Tier B 模板策略（保持上回合意图或执行预设默认动作）。

#### 4.2.6 UserInputDelta（用户输入解析结果）

用户的所有自由文本输入由 SceneStateExtractor (LLM) 转为统一的结构化 delta。

```rust
pub struct UserInputDelta {
    pub turn_id: String,
    pub raw_text: String,                          // 原始用户输入（仅用于 trace）
    pub kind: UserInputKind,
}

pub enum UserInputKind {
    /// 用户作为旁白 / 作者插入场景描述（如新增 entity / 改变光照）。
    SceneNarration { scene_delta: SceneDelta },

    /// 用户扮演角色 X 的言行：直接写入 X 的 IntentPlan，跳过 X 的 CognitivePass。
    CharacterRoleplay {
        character_id: String,
        intent_plan: IntentPlan,
        spoken_dialogue: Option<String>,
        actions: Vec<CharacterAction>,
    },

    /// 元指令：跳过时间 / 切场景 / 重置 / 暂停。
    MetaCommand { command: MetaCommandKind },

    /// 引导仲裁与文风（用户对当前回合的"导演"权）。
    DirectorHint { arbitration_bias: Option<String>, style_override: Option<StyleConstraints> },
}
```

`SceneStateExtractor` 输出严格遵守此 schema。失败时进入容错路径（见 §6.7）。

#### 4.2.7 StyleConstraints（叙事层文风约束）

由作者预设或用户 DirectorHint 提供，最终交给 SurfaceRealizer LLM 阅读。

```rust
pub struct StyleConstraints {
    pub register: StyleRegister,           // ancient / modern / casual / formal / poetic
    pub detail_level: DetailLevel,         // sparse / moderate / rich
    pub atmosphere: Atmosphere,            // tense / serene / ominous / melancholic / ...
    pub pacing: Pacing,                    // fast / measured / slow
    pub pov: PointOfView,                  // omniscient / character_focused(id) / objective

    /// 自由文本字段：作者用自然语言书写的硬约束、参考文风、禁忌事项等。
    /// 仅供 LLM 阅读，不参与程序逻辑。
    pub explicit_guidelines: Vec<String>,
    pub reference_excerpts: Vec<String>,   // 参考片段（如"模仿《红楼梦》第三回的笔法"）
}
```

#### 4.2.8 SurfaceRealizerInput（叙事层输入）

叙事层 LLM 仅接受以下三类结构化输入，**不再读取角色档案 / 世界设定 / 角色心智**（它们已体现在情景与认知结果中）。

```rust
pub struct SurfaceRealizerInput {
    pub scene_turn_id: String,

    /// 1. 情景提取结果：本回合场景的客观状态视图（叙事层视角下的"舞台"）。
    pub scene_view: SceneNarrativeView,

    /// 2. 各角色 Agent 的认知和意图结果（仅 Tier A/B 中实际进行了 cognitive pass 的角色）。
    pub character_outputs: Vec<CharacterCognitivePassOutput>,

    /// 3. 仲裁结果：本回合的物理后果与最终行动顺序。
    pub arbitration_result: ArbitrationResult,

    /// 文风约束（含自由文本指引）。
    pub style: StyleConstraints,
}
```

`SceneNarrativeView` 是 SceneModel 面向叙事的精简视图（去除 GodOnly 真相、保留可观察事件）。
`ArbitrationResult` 包含：每个角色的 outward_action（已发生的事）、resulting_state_changes（伤势/位置/资源等）、visible_facts（可被叙事提及的事实白名单）。

#### 4.2.9 Dirty Flags（调用预算控制）

```rust
pub struct DirtyFlags {
    pub scene_changed: bool,
    pub body_changed: bool,
    pub relation_changed: bool,
    pub belief_invalidated: bool,
    pub intent_invalidated: bool,
    pub directly_addressed: bool,
    pub under_threat: bool,
    pub reaction_window_open: bool,
    pub received_new_salient_signal: bool,
    pub knowledge_revealed: bool,    // 本回合获得了新可见知识
}
```

#### 4.2.10 Skill Model（最小灵活版）

```rust
pub struct Skill {
    pub skill_id: String,
    pub name: String,
    pub trigger_mode: TriggerMode,         // active / reaction / passive / channeled
    pub delivery_channel: DeliveryChannel, // gaze / voice / touch / projectile / scent / spiritual_link / ritual / field
    pub impact_scope: ImpactScope,         // body / perception / mind / soul / scene
    pub notes: String,
}

pub struct CharacterSkillUseProfile {
    pub character_id: String,
    pub skill_id: String,
    pub mastery_rank: u8,  // 1-5: novice / trained / skilled / expert / master
    pub notes: String,
}
```

技能的"该角色掌握哪些技能"以 `KnowledgeEntry { kind: CharacterFacet, facet: KnownAbility | HiddenAbility }` 表达，统一受可见性约束。

### 4.3 SQLite 表结构

按三层语义组织。Layer 2 不持久化（每回合重建）；Layer 1 / Layer 3 / Trace 各自独立。

```sql
-- ===== Layer 1: Truth Store =====

-- 场景快照（客观场景状态）
CREATE TABLE scene_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    scene_id TEXT NOT NULL,
    scene_turn_id TEXT NOT NULL,
    scene_model TEXT NOT NULL,             -- JSON: SceneModel
    created_at TEXT NOT NULL
);

-- 统一知识库（世界/地区/势力/角色档案/记忆）
CREATE TABLE knowledge_entries (
    knowledge_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,                    -- world_fact / region_fact / faction_fact / character_facet / memory
    subject_type TEXT NOT NULL,            -- world / region / faction / character / event
    subject_id TEXT,                       -- region/faction/character/event 的具体 ID（World 时为 NULL）
    facet_type TEXT,                       -- 仅 character_facet 有值
    content TEXT NOT NULL,                 -- JSON: 客观真相
    apparent_content TEXT,                 -- JSON: 表象（可空）
    visibility TEXT NOT NULL,              -- JSON: VisibilityPredicate
    subject_awareness TEXT NOT NULL,       -- JSON: SubjectAwareness（含 Unaware 的 self_belief）
    metadata TEXT NOT NULL,                -- JSON: KnowledgeMetadata
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 知识揭示事件（可见性扩展轨迹）
CREATE TABLE knowledge_reveal_events (
    event_id TEXT PRIMARY KEY,
    knowledge_id TEXT NOT NULL,
    newly_known_by TEXT NOT NULL,          -- JSON array
    trigger TEXT NOT NULL,                 -- JSON: RevealTrigger
    scene_turn_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (knowledge_id) REFERENCES knowledge_entries(knowledge_id)
);

-- 角色基本档案（仅 baseline_body_profile + mind_model_card 索引；其余事实在 knowledge_entries 中）
CREATE TABLE character_records (
    character_id TEXT PRIMARY KEY,
    baseline_body_profile TEXT NOT NULL,   -- JSON
    mind_model_card TEXT NOT NULL,         -- JSON（同时在 knowledge_entries 有冗余条目）
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ===== Layer 3: Subjective State =====

-- 角色主观状态快照（每回合 cognitive pass 后写入）
CREATE TABLE character_subjective_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL,
    scene_turn_id TEXT NOT NULL,
    belief_state TEXT NOT NULL,            -- JSON
    emotion_state TEXT NOT NULL,           -- JSON
    relation_models TEXT NOT NULL,         -- JSON
    current_goals TEXT NOT NULL,           -- JSON
    temporary_body_state TEXT NOT NULL,    -- JSON
    created_at TEXT NOT NULL
);

-- ===== Trace（调试与回放） =====

CREATE TABLE turn_traces (
    trace_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    character_id TEXT,                     -- NULL 表示全局回合 trace
    cognitive_pass_input TEXT,             -- JSON（含 Layer 2 派生视图）
    cognitive_pass_output TEXT,            -- JSON
    rendered_output TEXT,
    validation_results TEXT,
    created_at TEXT NOT NULL
);

-- ===== 索引 =====

CREATE INDEX idx_scene_snapshots_scene ON scene_snapshots(scene_id);
CREATE INDEX idx_knowledge_kind ON knowledge_entries(kind);
CREATE INDEX idx_knowledge_subject ON knowledge_entries(subject_type, subject_id);
CREATE INDEX idx_knowledge_facet ON knowledge_entries(subject_id, facet_type) WHERE kind = 'character_facet';
CREATE INDEX idx_reveal_knowledge ON knowledge_reveal_events(knowledge_id);
CREATE INDEX idx_subjective_char ON character_subjective_snapshots(character_id, scene_turn_id);
CREATE INDEX idx_traces_turn ON turn_traces(scene_turn_id);
```

**说明**：

- `knowledge_entries.visibility` 是 JSON 而非规范化的多表，是因为查询入口永远是 `VisibilityResolver`（程序化逻辑），不靠 SQL 谓词检索。
- `subject_id + facet_type` 联合索引服务"取角色 X 的所有 facets"这一最高频查询。
- `character_subjective_snapshots` 的最新一条即角色当前心智状态；历史快照保留用于回放与一致性验证。
- 没有"memory_records"表，记忆作为 `knowledge_entries.kind = 'memory'` 统一存储。

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

#### 模块职责边界（避免屎山）

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

---

## 6. 执行策略

### 6.1 三层运行时

- **Simulation Core**（程序化优先）：场景维护、位置、可见性、身体状态、技能生成、仲裁。
- **Cognitive Layer**（按需调用模型）：主观解释、偏见感知、信念变化、意图生成。
- **Presentation Layer**（输出时调用）：对话、动作叙述、风格化渲染。

### 6.2 融合调用

`PerceptionDistributor + BeliefUpdater + IntentAgent` 融合为单次模型调用 `CharacterCognitivePass`，大幅降低成本。

### 6.3 角色分级

- **Tier A**（主角 / 重要 NPC）：完整 CognitivePass。
- **Tier B**（次要 NPC）：简化规则，按需升级。
- **Tier C**（背景角色）：纯程序化策略。

### 6.4 Active Set + Dirty Flags

仅对**活跃且脏**的角色执行认知传递。

**触发 cognitive pass 的硬条件（程序可判定）**——以下 5 项任一为真即触发：
- `directly_addressed`：被对话方直接称呼/提问。
- `under_threat`：被攻击或处于即时威胁。
- `reaction_window_open`：技能/事件开放了反应窗口。
- `scene_changed`：所在场景的可观察状态发生显著变化。
- `body_changed`：自身身体状态发生显著变化。

**主观显著性标志**（不作为触发条件，仅作为 prompt hint 提示 LLM "你刚听到/看到 X"）：
- `received_new_salient_signal`、`belief_invalidated`、`relation_changed`、`intent_invalidated`、`knowledge_revealed`。

跳过用户当前扮演的角色（其行为由 UserInputDelta 直接给出）。

### 6.5 主循环

```
== Per Turn ==

1. 收集用户输入（自由文本）
2. SceneStateExtractor(LLM) → UserInputDelta（结构化）
   - 自由文本 → SceneNarration / CharacterRoleplay / MetaCommand / DirectorHint
   - 解析失败 → 容错修复 → 仍失败则提示用户重写
3. 应用 UserInputDelta 到 Layer 1：
   - SceneNarration → 更新 SceneModel
   - CharacterRoleplay → 写入对应角色的 IntentPlan（跳过其 CognitivePass）
   - MetaCommand → 时间/场景控制
   - DirectorHint → 暂存供仲裁层与叙事层使用
4. 更新身体 / 资源 / 状态 / 冷却（Layer 1，机械演化）
5. 生成事件 delta
6. 计算活跃集 + 脏标志

== Per Active & Dirty Character (跳过用户已扮演的角色) ==

7. EmbodimentResolver → embodiment_state（Layer 2）
8. SceneFilter (含 visible_facets 计算) → filtered_scene_view（Layer 2）
9. KnowledgeAccess → accessible_knowledge（Layer 2，全部经可见性过滤）
10. InputAssembly → CognitivePassInput（保证不含 Layer 1 原始对象）
11. CharacterCognitivePass(LLM) → 严格 schema JSON
    - 解析失败 → 程序容错（修复常见 JSON 错误）
    - 修复失败 → 标记进入仲裁层兜底
12. Validator 扫描输入/输出对（OmniscienceLeakage / SelfAwareness / GodOnly / Embodiment / 一致性）
    - 验证失败 → 标记进入仲裁层兜底

== Per Turn (Global) ==

13. ActionArbitration（混合层）：
    a. 仲裁层 LLM 兜底：处理 step 11/12 中标记失败的角色，推断可用 IntentPlan
    b. 物理仲裁（程序）：读 Layer 1 真相，对所有角色 IntentPlan 做命中/资源/位置/技能判定
    c. 输出 ArbitrationResult（含 visible_facts 白名单）
14. SurfaceRealizer(LLM) ← {SceneNarrativeView, CharacterCognitivePassOutput[], ArbitrationResult, StyleConstraints}
    → 自由文本叙事
15. NarrativeFactCheck：扫描叙事文本提及的事实是否 ⊆ visible_facts
16. StateCommitter:
    - 更新 SceneModel (Layer 1)
    - 处理 KnowledgeRevealEvent（扩展 known_by + 生成对应 Memory）
    - 追加新 KnowledgeEntry { kind: Memory }
    - 应用 BodyReactionDelta 到 temporary_body_state
    - 写入 character_subjective_snapshots（Layer 3）
    - 写入 turn_traces（调试）
```

### 6.6 调用预算

- 每场景窗口：0-2 次 cognitive passes（重要活跃角色）。
- 0 次 cognitive passes（次要 / 背景角色）。
- 1 次 surface realization（仅当需要可见输出）。
- 1 次 SceneStateExtractor（每次用户输入）。
- 0-1 次 仲裁层兜底（仅在认知输出失败时）。

### 6.7 LLM 与程序边界总表

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
| EmbodimentResolver | 程序 | 公式化 |
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

#### 关键铁律

1. **自由文本仅在三处出现**：用户输入、SceneStateExtractor 输入、SurfaceRealizer 输出。其他全部结构化。
2. **VisibilityResolver 永不调 LLM**：可见性判断必须确定性。
3. **LLM 输出必须严格 schema**：依赖 Provider 的 JSON mode + prompt 模板 + 程序容错。
4. **数值字段不让 LLM 直出**：信念/情绪变化用离散级别，由程序映射为数值。
5. **客观推理交给数据**：长链客观推理通过 Knowledge 预存事实实现，不让 LLM 即兴推理。
6. **社会后果不在仲裁层**：下游角色的 LLM 自行解读社会信号（"我相信他了吗"）。
7. **叙事不引入新事实**：SurfaceRealizer 受 visible_facts 白名单约束，由 NarrativeFactCheck 强制。

---

## 7. 验证规则

每条规则只读取已派生的 Layer 2 输入与 LLM 输出对，不修改任何状态。

### 必备规则

1. **Omniscience Leakage Rule** - CognitivePass 输出引用的所有 entity_id / knowledge_id 必须出现在该回合该角色的 `accessible_knowledge` 或 `filtered_scene_view.visible_entities` 中。
2. **Embodiment Ignored Rule** - 感官失能时，输出不应描述对应感知（如失明却看见）。
3. **Self Awareness Rule** - 当某 `KnowledgeEntry` 的 `subject_awareness == Unaware{self_belief}` 且 subject 是当前角色时：该角色的认知输出**只能**引用 `self_belief`，不可引用 `content` 中独有的事实。
4. **God Only Rule** - `visibility.scope` 含 `GodOnly` 的 KnowledgeEntry 在任何角色输出中均不应出现。
5. **Mana Sense Rule** - 凡人（低灵觉敏锐度）不应清晰感知修士气息。
6. **Consistency Rule** - 跨回合连续性（受伤、关系、目标不应无缘由跳变）。
7. **Apparent vs True Rule** - 当观察者通过 `apparent_content` 看到某 facet 时，输出引用该信息应与 `apparent_content` 一致；引用 `content` 独有信息视为泄露。
8. **Narrative Fact Check Rule** - SurfaceRealizer 输出的叙事文本中提及的具体事实必须 ⊆ `ArbitrationResult.visible_facts` 白名单；不可引入新事实（位置/伤势/状态变化等）。修辞描写不计。
9. **Schema Conformance Rule** - 所有 LLM 输出必须通过 schema 校验；失败触发容错路径。

### 验证时机

- **InputAssembly 之后、CognitivePass 之前**：扫描 prompt 不含 Layer 1 原始对象（结构性检查）。
- **CognitivePass 之后**：schema 校验（规则 9）+ 语义级泄露检测（规则 1-5、7）。
- **SurfaceRealizer 之后**：NarrativeFactCheck（规则 8）。
- **每回合结束**：跨回合一致性（规则 6）。

---

## 8. AI Provider 抽象

需支持三种调用模式：自由对话（SillyTavern 模式 / SurfaceRealizer）、严格 JSON 输出（Agent 模式各 LLM 节点）、流式输出（聊天 UI 体验）。

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    fn models(&self) -> Vec<String>;

    /// 自由文本输出（SillyTavern 模式 / SurfaceRealizer）
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String>;

    /// 严格 JSON 输出（Agent 模式各 LLM 节点）
    /// 使用 Provider 的 JSON mode / structured output 能力。
    async fn chat_structured(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,    // JSON Schema
    ) -> Result<serde_json::Value, String>;

    /// 流式输出（聊天 UI 体验）
    async fn chat_stream(&self, request: ChatRequest)
        -> Result<Box<dyn Stream<Item = String>>, String>;
}

// 实现：OpenAIProvider / AnthropicProvider / GeminiProvider / OllamaProvider
// 各 Provider 通过自身 JSON mode (OpenAI) / tool use (Anthropic) / response_schema (Gemini) /
// format=json (Ollama) 实现 chat_structured。
```

**Agent 模式各 LLM 节点对应的调用类型**：

| LLM 节点 | 调用类型 |
|---|---|
| SceneStateExtractor | `chat_structured`（输出 UserInputDelta schema） |
| CharacterCognitivePass | `chat_structured`（输出 CharacterCognitivePassOutput schema） |
| Arbitration 兜底 | `chat_structured`（输出 IntentPlan schema） |
| SurfaceRealizer | `chat` 或 `chat_stream`（输出自由文本叙事） |

---

## 9. 实现阶段

### 阶段一：基础框架 (MVP)

1. 初始化 Tauri + Vue 3 + TypeScript + Naive UI。
2. 配置 Vue Router + Pinia。
3. 实现 JSON 存储层。
4. 基础聊天 + AI Provider 抽象。
5. 集成 OpenAI API。

### 阶段二：SillyTavern 模式

1. 角色卡 V3 管理（创建 / 编辑 / 导入 / 导出）。
2. 世界书编辑器（含分组 / 概率 / 递归 / 时间控制）。
3. 关键词触发系统（含正则 / 匹配目标扩展）。
4. 预设系统。
5. 多 API 支持（Claude / Gemini / Ollama）。

### 阶段三：Agent 模式 - 数据模型层

1. SQLite 表结构 + 三层语义隔离（Layer 1 / Layer 3 / Trace）。
2. SceneModel + ManaField 完整定义（Layer 1）。
3. KnowledgeEntry 体系（kind / subject / visibility / subject_awareness / apparent_content）。
4. KnowledgeEntry content sub-schemas（每种 facet/fact 类型的核心字段 + extensions 兜底）。
5. CharacterRecord（baseline_body_profile + mind_model_card）。
6. CharacterSubjectiveState（Layer 3）。
7. EmbodimentState / FilteredSceneView / AccessibleKnowledge（Layer 2 派生类型）。
8. CognitivePass I/O 类型（含 ConfidenceShift / BodyReactionDelta）。
9. UserInputDelta / StyleConstraints / SurfaceRealizerInput / ArbitrationResult。

### 阶段四：Agent 模式 - 程序化核心

1. KnowledgeStore（Layer 1 CRUD）。
2. VisibilityResolver（统一可见性判断，三谓词合并）。
3. KnowledgeAccessProtocol（构建 AccessibleKnowledge）。
4. SceneStateExtractor。
5. EmbodimentResolver（含灵觉）。
6. SceneFilter（含 visible_facets 计算）。
7. InputAssembly（拒绝 Layer 1 原始对象）。
8. ActionArbitration（仲裁层读 Layer 1 真相）。
9. KnowledgeRevealEvent 处理。

### 阶段五：Agent 模式 - 认知与叙事层

1. PromptBuilder（结构化 prompt + JSON schema 注入）。
2. SceneStateExtractor（用户输入 → UserInputDelta，严格 schema）。
3. CharacterCognitivePass（融合调用，严格 schema 输出）。
4. JSON 输出容错修复器（缺字段补默认 / 修复常见结构错误）。
5. Arbitration LLM 兜底（认知输出修复失败时启用）。
6. SurfaceRealizer（叙事生成，受 visible_facts 约束）。
7. AI Provider 的 chat_structured 实现（OpenAI/Anthropic/Gemini/Ollama 各自的 JSON mode）。

### 阶段六：Agent 模式 - 验证与运行时

1. 9 大验证规则（含 SelfAwareness / GodOnly / ApparentVsTrue / NarrativeFactCheck / SchemaConformance）。
2. AgentRuntime 主循环。
3. Dirty Flags + Active Set。
4. 角色 Tier 分级。
5. 调用预算监控。

### 阶段七：用户角色扮演

1. 用户角色选择。
2. 用户输入心理活动 / 言行。
3. 用户对仲裁 / 文风的"指导"。

### 阶段八：优化与扩展

1. 性能优化（缓存 / 事件批处理）。
2. UI / UX 改进。
3. Trace 可视化。
4. 测试覆盖。
5. 插件系统。

---

## 10. 关键文件

### 前端

- `src/stores/agent.ts` - Agent 状态管理。
- `src/services/api.ts` - Tauri IPC 封装。
- `src/components/agent/CharacterMindView.vue` - 心智视图。
- `src/components/agent/ValidationReport.vue` - 验证报告。

### 后端

- `src-tauri/src/agent/runtime.rs` - 主循环。
- `src-tauri/src/agent/knowledge/store.rs` - 知识库 CRUD。
- `src-tauri/src/agent/knowledge/visibility.rs` - 可见性唯一入口。
- `src-tauri/src/agent/knowledge/access.rs` - AccessibleKnowledge 构建。
- `src-tauri/src/agent/knowledge/reveal.rs` - 揭示事件处理。
- `src-tauri/src/agent/cognitive/cognitive_pass.rs` - 融合调用。
- `src-tauri/src/agent/simulation/scene_filter.rs` - 场景过滤（含 visible_facets）。
- `src-tauri/src/agent/simulation/input_assembly.rs` - 拒绝 Layer 1 泄露。
- `src-tauri/src/agent/simulation/arbitration.rs` - 仲裁层（直接读真相）。
- `src-tauri/src/agent/validation/validator.rs` - 验证器入口。
- `src-tauri/src/agent/models/knowledge.rs` - KnowledgeEntry 定义。
- `src-tauri/src/agent/models/scene.rs` - 场景模型。
- `src-tauri/src/agent/models/mana_field.rs` - 灵力场。
- `src-tauri/src/storage/sqlite_store.rs` - 存储层。

---

## 11. 验证方案

### 阶段一

- [ ] 应用启动 / 发送消息 / JSON 存储。

### 阶段二

- [ ] 导入 SillyTavern 角色卡 V3。
- [ ] 世界书词条完整触发（含正则 / 概率 / 时间）。
- [ ] 预设正确应用。

### 阶段三-六（参考 `rp_agent_filtering_example.md`）

- [ ] 失明角色 `visible_entities` 为空。
- [ ] 狐狸精能闻到血腥味，普通人闻不到。
- [ ] 凡人无法清晰感知修士气息。
- [ ] **私密 Knowledge 仅 known_by 中的角色能访问。**
- [ ] **GodOnly 知识不出现在任何角色的 accessible_knowledge 中。**
- [ ] **subject_awareness=Unaware 时，subject 自我描述只能引用 self_belief（如被封印记忆的狐狸精仍自称人类）。**
- [ ] **观察者通过 apparent_content 看到的伪装信息与 content 真相一致地分流（伪装方与揭穿方分别得到不同 visible_content）。**
- [ ] **scope:faction:玄天宗 的 KnowledgeEntry 仅对该势力成员可见。**
- [ ] **同场景观察可获得他人 Appearance facet，但获取不到 TrueName facet（无关系阈值）。**
- [ ] **KnowledgeRevealEvent 触发后，被揭示者的下一回合输入包含新可见 Knowledge。**
- [ ] 受伤状态跨回合保持。
- [ ] Dirty Flags 正确过滤无变化角色。
- [ ] 调用预算控制在每场景 0-2 次。

### 阶段七

- [ ] 用户能扮演特定角色并影响仲裁。

---

## 12. 潜在坑点

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 模型输出非结构化 | 解析失败 | JSON / YAML 约束 + 重试 + schema 验证 |
| 全知泄露难检测 | 行为不符设定 | 输入过滤 + 输出验证 + 访问日志审计 |
| 状态爆炸 | 长对话状态过大 | 增量更新 + 周期压缩 + Knowledge metadata 衰减 |
| 多角色调用成本 | Token 消耗大 | Dirty Flags + 意图复用 + Tier 分级 |
| Rust-TS 类型同步 | 两端定义不一致 | 代码生成 + 共享 schema + 单元测试 |
| Prompt 漂移 | 模型行为变化 | 固定 prompt 版本 + A/B 测试 + 监控 |
| 灵觉过载处理 | 高灵气环境失真 | 过载阈值 + 感知降级 + 验证 |
| Layer 1 泄露至 LLM | 全知 / 屎山起点 | InputAssembly 类型隔离（仅接受 Layer 2 类型）+ 单元测试断言 |
| 可见性逻辑散落 | 多处不一致 | VisibilityResolver 是唯一入口；所有判断必须经它 |
| Subject self-belief 被外部读 | 暴露真相 | `KnowledgeEntry.content` 与 `self_belief` 在类型层面分离；访问 API 强制经过 awareness 检查 |
| Knowledge 揭示无追溯 | 不知何时谁知道了什么 | 所有可见性变更必须经 KnowledgeRevealEvent；持久化到独立表 |
| Belief 与 RelationModel 重复 | 同一命题两处存储 | 文档约定 + lint 规则：关于人的命题写 RelationModel，关于事件/世界的写 BeliefState |
| KnowledgeEntry 字段膨胀 | 单表过宽难查询 | content 用 JSON 列；高频查询用 (subject_id, facet_type) 索引；不在表层加新列 |
| LLM 输出非结构化 JSON | 解析失败 / 主循环中断 | Provider 强制 JSON mode + 程序容错修复 + 仲裁层 LLM 兜底 + 最终 Tier B 模板降级 |
| Schema 漂移 | 旧数据无法兼容 | 每个 KnowledgeEntry 含 `schema_version`；StateCommitter 写入时校验；提供迁移脚本 |
| LLM 数值不稳定 | belief/emotion 数值跳变 | LLM 输出离散级别（ConfidenceShift），程序映射为数值 |
| 中间数据混入自由文本 | 屎山起点；规则匹配失效 | §3.4 铁律 + 类型隔离（中间结构禁止 String content 字段；summary_text 仅供阅读不参与判断） |
| SurfaceRealizer 私自添加事实 | 误导用户 / 后续状态不一致 | NarrativeFactCheck 强制扫描；visible_facts 白名单约束 |
| 仲裁层 LLM 兜底范围扩大 | 演变成"什么都让 LLM 仲裁" | 仲裁层 LLM 仅在认知输出失败时启用；物理判定永远走程序 |
| 用户输入 LLM 解析失败 | 用户操作丢失 | 显示原始输入 + 提示重写；保留 raw_text 供 trace |
