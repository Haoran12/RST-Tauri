# 10 Agent 数据模型

本文档承载 Agent 模式的数据语义与结构化模型：

- 三层数据语义（L1 Truth / L2 Per-Character Access / L3 Subjective）
- Layer 1 客观真相：SceneModel / KnowledgeEntry / CharacterRecord
- Layer 2 逐角色可触及视图：EmbodimentState / FilteredSceneView / AccessibleKnowledge
- Layer 3 主观状态：Belief / Emotion / Relation / Goals

程序化派生与对抗解算见 [12_agent_simulation.md](12_agent_simulation.md)。LLM 节点 I/O 契约见 [13_agent_llm_io.md](13_agent_llm_io.md)。SQLite 持久化见 [14_agent_persistence.md](14_agent_persistence.md)。运行时主循环见 [11_agent_runtime.md](11_agent_runtime.md)。LLM/程序边界铁律见 [01_architecture.md](01_architecture.md)。

---

## 1. 三层数据语义（强制隔离）

为避免"客观真相"与"主观认知"在代码中混淆，运行时数据严格分为三层。**层间只能通过定义好的派生关系流动，禁止跨层直接读写。**

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 1 — Truth Store（客观真相，仅编排器与结果规划/验证层访问）│
│  ├── SceneModel              场景客观状态                    │
│  ├── KnowledgeEntry[*]       统一知识库（含世界/势力/角色档 │
│  │                           案/记忆，带访问策略）           │
│  └── 角色 baseline_body_profile（物种/感官基线/灵觉基线）    │
│      + temporary_body_state  伤势/疲惫/痛感/灵力消耗等当前态 │
│  约束：只有声明 God-read 的编排类节点可读此层；              │
│        CognitivePassInput / SurfaceRealizerInput 不出现       │
│        Layer 1 原始对象。                                    │
└──────────────────────────────────────────────────────────────┘
                  │ 经 KnowledgeAccessResolver + SceneFilter 派生
                  ▼
┌──────────────────────────────────────────────────────────────┐
│  Layer 2 — Per-Character Access（角色可触及的客观，每回合重 │
│            建，无独立持久化）                                │
│  ├── FilteredSceneView       能感知的场景 + observable facets│
│  ├── AccessibleKnowledge[*]  通过访问控制过滤的 KnowledgeEntry │
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
│  └── CurrentGoals            目标（含 hidden）               │
│  约束：本层是 LLM 的输出领地；任何关于"我相信 B 是好人"      │
│        的命题进入 RelationModels 而非 BeliefState（避免重复）│
└──────────────────────────────────────────────────────────────┘
```

**信息流向（线性，单向）**：

```
SceneSeed + 公开世界 / 地点 / 人物上下文
    → SceneInitializer(LLM, 公开上下文受控补全) → SceneInitializationDraft 候选
    → Validator + StateCommitter → 新 SceneModel(L1)

最近用户自由文本 + 当前 SceneModel(L1)
    → SceneStateExtractor(LLM, 场景域 God-read) → SceneUpdate / UserInputDelta 候选
    → Validator + StateApplier → World Truth (L1)
    → Embodiment 计算 (L1 baseline + L1 temp + L1 scene → L2 embodiment)
    → SceneFilter (L1 scene + L2 embodiment → L2 filtered_view)
    → KnowledgeAccess (SQLite 访问索引预筛 + KnowledgeAccessResolver 裁剪 → L2 accessible_knowledge)
    → InputAssembly (L2 全部 + L3 prior → CognitivePassInput)
    → CognitivePass(LLM) → Output(perception/belief/intent)
    → Validator (扫描 Output 引用是否 ⊆ L2 输入)
    → OutcomePlanner (LLM 可 God-read + 技能契约/程序硬约束 → 候选行为后果 / 状态更新计划)
    → SurfaceRealizer (渲染叙事)
    → StateCommitter (更新 L1 + L3，处理 KnowledgeRevealEvent)
```

God-read 只表示可读取客观真相用于编排，不表示可直接写入 Layer 1。所有写入必须落成结构化 delta / plan，并由程序校验后提交。

---

## 2. Layer 1 — Truth Store

### 2.1 Scene Model

```rust
pub struct SceneModel {
    pub scene_id: String,
    pub scene_turn_id: String,
    pub time_context: TimeContext,             // 时间/天气/可见度
    pub spatial_layout: SpatialLayout,         // 空间布局/障碍物/入口
    pub lighting: LightingState,               // 光照/光源/阴影/逆光
    pub acoustics: AcousticsState,             // 环境噪声/反射特性
    pub olfactory_field: OlfactoryField,       // 气味场/气流/气味源
    pub scene_mood: SceneMood,                 // 场景基调/氛围（紧张/肃穆/欢庆/敌对/亲密/诡异...），可被角色主观感知
    pub physical_conditions: PhysicalConditions, // 物理环境：气温/地表/空气颗粒/降水/风
    pub mana_field: ManaField,                 // 灵力场（玄幻扩展）
    pub entities: Vec<SceneEntity>,            // 在场实体（id + 位置 + 姿态）
    pub observable_signals: ObservableSignals,
    pub event_stream: Vec<SceneEvent>,
    pub uncertainty_notes: Vec<String>,
}
```

### 2.2 Physical Conditions（物理环境）

承载客观、可量化、直接影响行动与感知的物理量。属于 Layer 1 真相，由 `SceneInitializer` 候选初始化、`SceneStateExtractor` 候选更新与 StateCommitter 提交维护，凡人/修士均可被影响。

```rust
pub struct PhysicalConditions {
    pub temperature: Temperature,                  // 气温（含灵力/法术修正）
    pub surface_state: SurfaceState,               // 地面状态：湿滑/积水/积雪/碎石/血迹
    pub airborne: AirborneEffects,                 // 空气中的颗粒与能见度：雾/烟/扬尘/灵雾
    pub precipitation: Option<Precipitation>,      // 降水：雨/雪/雹/沙暴/灵雨
    pub wind: WindState,                           // 风向/风力（影响声/味传播、远程命中、火势蔓延）
}

pub struct Temperature {
    pub ambient_celsius: f64,                      // 环境基准温度
    pub felt_celsius: f64,                         // 最终感受温度（含修正后的值；EmbodimentResolver 据此计算冷暖耐受）
    pub modifiers: Vec<TemperatureModifier>,       // 局部温变叠加项
}

pub struct TemperatureModifier {
    pub source_id: String,                         // 来源：角色/法术/灵脉/阵法/物品
    pub delta_celsius: f64,                        // 正为升温，负为降温
    pub radius_m: f64,                             // 影响半径
    pub kind: TemperatureModifierKind,             // 物理热源 / 灵力升温 / 灵力冰寒 / 法术屏障
}

pub struct SurfaceState {
    pub slipperiness: f64,                         // 0.0-1.0，影响平衡与移动力
    pub wetness: f64,                              // 0.0-1.0
    pub debris: Vec<String>,                       // 碎石/积雪/灰烬/血迹/法器残骸
    pub notes: String,
}

pub struct AirborneEffects {
    pub fog_density: f64,                          // 0.0-1.0
    pub dust_density: f64,
    pub smoke_density: f64,
    pub visibility_range_m: f64,                   // 综合能见度（米），SceneFilter 据此衰减视觉
    pub mana_haze: Option<ManaHaze>,               // 灵雾：影响灵觉而非视觉，属性继承自 ManaField
}

pub enum Precipitation {
    Rain { intensity: f64 },                       // intensity 0.0-1.0
    Snow { intensity: f64 },
    Hail { intensity: f64 },
    Sandstorm { intensity: f64 },                  // 扬尘/沙暴
    SpiritRain { attribute: ManaAttribute, intensity: f64 },  // 灵雨（带属性，可与 ManaField 联动）
}

pub struct WindState {
    pub direction_deg: f64,                        // 0-360，正北为 0
    pub speed_ms: f64,                             // 米/秒
    pub gust: bool,                                // 是否阵风
}
```

**与其他场景字段的耦合**（由各派生层处理，不在此结构中冗余）：

- `airborne.visibility_range_m` 由 `SceneFilter` 取与 `lighting` 较小者作为最终视觉距离。
- `wind` 影响 `acoustics`（声音传播方向/衰减）与 `olfactory_field`（气味扩散方向）。
- `temperature.modifiers` 中 `kind == 灵力*` 的项必须能在 `ManaField.mana_sources` 或事件流中找到对应来源（一致性由 Validator 检查）。
- `precipitation::SpiritRain` 与 `airborne.mana_haze` 的属性应与 `ManaField.ambient_attribute` 兼容。

### 2.3 Mana Field（玄幻扩展）

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

### 2.4 KnowledgeEntry（统一知识模型）

`KnowledgeEntry` 是 Layer 1 的核心，承载世界设定 / 地区设定 / 势力设定 / 角色档案分面 / 历史事件（Memory）。所有"谁能读取什么 Knowledge"的判断由它的 `access_policy` 字段决定，由 `KnowledgeAccessResolver` 统一计算。

```rust
pub struct KnowledgeEntry {
    pub knowledge_id: String,
    pub kind: KnowledgeKind,
    pub subject: KnowledgeSubject,
    pub content: serde_json::Value,                  // 客观真相（结构化）
    pub apparent_content: Option<serde_json::Value>, // 表象（伪装/欺骗时给观察者看的版本）
    pub access_policy: AccessPolicy,
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

pub struct AccessPolicy {
    // 三谓词，OR 关系（任一为真即可访问）。
    // 例外：scope 含 GodOnly 时为 hard deny，优先级高于 known_by / scope / conditions。
    pub known_by: Vec<String>,                  // 名单制
    pub scope: Vec<AccessScope>,            // 标签制
    pub conditions: Vec<AccessCondition>,   // 条件制（运行时求值）
}

pub enum AccessScope {
    Public,                  // 所有原住民
    GodOnly,                 // 仅编排器（无人可知；hard deny）
    Region(String),          // 在该地区的角色
    Faction(String),         // 该势力成员
    Realm(String),           // 修为门槛及以上
    Role(String),            // 担任该职位
    Bloodline(String),       // 该血脉
    // 可扩展
}

pub enum AccessCondition {
    InSameSceneObservable,                                    // 同场景且能感知
    SocialAccessAtLeast { target: String, threshold: f64 }, // L1 客观关系/授权阈值；禁止读取 L3 relation_models
    HasSkill(String),                                      // 拥有特定技能
    CultivationAtLeast(String),                            // 修为达到
    CustomPredicate(AccessExpression),                 // 结构化 DSL AST 扩展点；禁止自然语言表达式
    // 可扩展
}

pub enum AccessExpression {
    All(Vec<AccessExpression>),
    Any(Vec<AccessExpression>),
    Not(Box<AccessExpression>),
    HasTag { subject_id: String, tag: String },
    NumericAtLeast { path: String, value: f64 },
    BooleanFlag { path: String, expected: bool },
}

pub enum SubjectAwareness {
    /// 默认：subject 自己知道关于自己的这条 facet。
    /// 在为 subject 构建 accessible_knowledge 时，content 直接可访问。
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

1. `content` 永远不进入 LLM，除非 `KnowledgeAccessResolver` 判定该角色对该 entry 拥有完整访问权限。
2. `subject == Character{id: A}` 且 `subject_awareness == Unaware{self_belief}` 时：A 的 accessible_knowledge 中只见 `self_belief`，看不到 `content`。
3. `apparent_content` 存在时：观察者（非 subject）默认看到 `apparent_content`；只有满足"揭穿条件"或在 `known_by` 中的角色才看到 `content`。
4. `access_policy.scope` 含 `GodOnly` 表示仅编排器可读，对所有角色拒绝访问；`KnowledgeAccessResolver` 必须先检查 `GodOnly`，命中后直接拒绝，不再计算 `known_by` / 其他 scope / conditions。
5. `GodOnly` 启用态下 `access_policy.known_by` 必须为空；Validator / StateCommitter 自动检查并拒绝 `GodOnly + known_by 非空` 的状态。
6. 若故事推进后 OutcomePlanner 候选 + EffectValidator 确认某条 `GodOnly` 知识可被角色获知，必须通过 `KnowledgeRevealEvent` 先移除 `GodOnly` 或降级为其他 scope，再追加 `known_by`；禁止在 `GodOnly` 仍存在时直接写入 `known_by`。
7. `MemoryEntry` 不再独立存在；历史事件以 `KnowledgeEntry { kind: Memory }` 形式统一存储。
8. Layer 1 的 Knowledge 内容由编排器/作者/StateCommitter 写入；CognitivePass 不可写。

#### 2.4.1 访问策略存储与查询索引

`KnowledgeEntry.access_policy` 是权威结构，必须完整保存在 `knowledge_entries.access_policy` JSON 中，用于导入导出、回滚、schema 校验和 `KnowledgeAccessResolver` 最终判定。

为避免在高频 `KnowledgeAccess` 中扫描全库，`known_by` 与 `scope` 同时维护为 SQLite 派生索引：

- `knowledge_access_known_by(knowledge_id, character_id)`：展开 `access_policy.known_by`。
- `knowledge_access_scopes(knowledge_id, scope_type, scope_value)`：展开 `access_policy.scope`，`Public` / `GodOnly` 等无值 scope 使用空字符串。
- `character_scope_memberships(character_id, scope_type, scope_value)`：角色当前所属地区、势力、修为门槛、职位、血脉等可查询 membership。

派生索引只用于候选预筛，不是第二套访问规则。`KnowledgeAccess` 的固定流程为：

1. 根据当前角色、场景、可观察实体、近期事件与 `character_scope_memberships` 查询候选 `knowledge_id`。
2. 批量读取候选 `KnowledgeEntry`。
3. 对每条候选调用 `KnowledgeAccessResolver`；`GodOnly` 仍由 Resolver hard deny。
4. 按 `content` / `apparent_content` / `self_belief` 三选一生成 `AccessibleEntry.accessible_content`。

任何写入 `KnowledgeEntry.access_policy`、处理 `KnowledgeRevealEvent`、改变角色地区/势力/职位/血脉/修为等访问身份的操作，必须在同一 SQLite transaction 内同步更新派生索引。索引表可由 `knowledge_entries.access_policy` 全量重建；若重建结果与现有索引不一致，视为存储一致性错误。

### 2.5 Content Schema 约定（核心字段 + extensions 兜底）

`content` / `apparent_content` / `self_belief` 三者共享同一套 sub-schema，按 `kind` × `facet_type` 决定字段定义。

**通用规则**：
- 必含 `summary_text: string`（自由文本简述，供 LLM 快速理解；不参与程序判断）。
- 必含该 kind/facet 的核心结构化字段（用于程序检索 / 访问控制 / 规则匹配）。
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

### 2.6 KnowledgeRevealEvent（访问权限扩展）

访问权限变化必须通过显式事件触发，禁止隐式修改 `access_policy.known_by`：

```rust
pub struct KnowledgeRevealEvent {
    pub event_id: String,
    pub knowledge_id: String,
    pub newly_known_by: Vec<String>,   // 此次新增的知情者
    pub trigger: RevealTrigger,        // 何种触发：witnessed / told / inferred / awakened
    pub scope_change: Option<AccessScopeChange>, // GodOnly 揭示时必须先降级/移除 GodOnly
    pub scene_turn_id: String,
}

pub enum AccessScopeChange {
    RemoveGodOnly,                     // 结果规划 + 程序校验确认该知识已可进入角色可知范围
    ReplaceScopes(Vec<AccessScope>),
}
```

由 StateCommitter 处理：
- 若原 entry 含 `GodOnly`，必须先验证 `scope_change` 已移除/替换 `GodOnly`，否则拒绝追加 `newly_known_by`。
- 更新 `KnowledgeEntry.access_policy.scope` 与 `access_policy.known_by`。
- 在 event_stream 追加事件。
- 创建一条 `KnowledgeEntry { kind: Memory }` 记录"X 何时如何获知 Y"。

### 2.7 角色静态档案（CharacterRecord）

角色不再有大而全的"static_profile" blob。所有"客观属于该角色的事实"都拆为多条 `KnowledgeEntry { kind: CharacterFacet, subject: Character{id, facet} }`，按需查询。

以下三项作为非 Knowledge 的角色基本数据保留在 Layer 1：

```rust
pub struct CharacterRecord {
    pub character_id: String,
    pub baseline_body_profile: BaselineBodyProfile,    // 物种/感官基线/灵觉基线/灵力数值（用于 EmbodimentResolver 与 SceneFilter）
    pub mind_model_card_knowledge_id: String,          // 指向 KnowledgeEntry 中的 MindModelCard，避免双写漂移
    pub temporary_body_state: TemporaryBodyState,       // 当前客观身体/资源状态；每回合由机械演化与 StateCommitter 更新
    pub schema_version: String,
}

pub struct BaselineBodyProfile {
    pub species: String,                           // "人类" / "妖精-狐" / "仙灵-龙" / ...
    pub comfort_temperature_range: (f64, f64),     // 物种舒适带（℃），用于 TemperatureFeelTier 校准
    pub mana_sense_baseline: ManaSenseBaseline,    // 灵觉基线（acuity / overload_threshold / 属性偏向）
    pub base_mana_power: f64,                      // 灵力数值（参考 rp_cards）；无修行凡人 ~100
    pub mana_attribute_affinity: Vec<ManaAttribute>,  // 擅长属性（影响感知 confidence 与施法效率）
    pub size_class: String,                        // "humanoid" / "small_beast" / "kaiju" 等（影响平衡/移动公式）
}

pub struct ManaSenseBaseline {
    pub acuity: f64,                               // 0.0-1.0；凡人 0.0；普通修士 0.4-0.6；高阶仙灵 ~1.0
    pub overload_threshold: f64,                   // 触发感知过载的环境密度阈值（与档位相关）
    pub attribute_bias: Option<ManaAttribute>,     // 天生敏感的属性
}
```

注意：

- `MindModelCard` 只以 `KnowledgeEntry` 形式保存（subject 自我认知层）；`CharacterRecord` 仅保存 `mind_model_card_knowledge_id` 指针，避免同一事实在角色表和知识表双写漂移。
- `temporary_body_state` 主要归入 Layer 1：伤势、疲惫、痛感、灵力消耗、冷却、毒素、短暂身体反应等都属于当前客观/半客观运行态。CognitivePass 只能通过 Layer 2 的 `EmbodimentState` 看到其派生结果，不直接读取原始状态。
- `base_mana_power` 是 raw 数值；当前**有效灵力**还需叠加 L1 中的伤势/压制/突破修正后再喂给 `ManaPotencyTier::from_power`。raw 永远不进入 CognitivePass。
- `comfort_temperature_range` 与 `base_mana_power` 的默认值在角色卡解析时从对应种族卡（如 `humanbeing.yaml` / `yaoguai.yaml`）读取并可被角色级覆盖。

---

## 3. Layer 2 — Per-Character Access

### 3.1 EmbodimentState

```rust
pub struct EmbodimentState {
    pub character_id: String,
    pub scene_turn_id: String,
    pub sensory_capabilities: SensoryCapabilities,  // vision/hearing/smell/touch/proprioception/mana
    pub body_constraints: BodyConstraints,          // 移动力/平衡/痛苦负载/疲惫/认知清晰度 + environmental_strain（环境档位+惩罚）
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

### 3.2 FilteredSceneView

```rust
pub struct FilteredSceneView {
    pub character_id: String,
    pub scene_turn_id: String,
    pub observable_entities: Vec<ObservableEntity>,
    pub audible_signals: Vec<AudibleSignal>,
    pub olfactory_signals: Vec<OlfactorySignal>,
    pub tactile_signals: Vec<TactileSignal>,
    pub mana_signals: Vec<ManaSignal>,
    pub mana_environment: ManaEnvironmentSense,
    pub weather_perception: WeatherPerception,    // 风/温/能见度/降水的档位翻译 + 程序生成的具体描述
    pub spatial_context: SpatialContext,
}

pub struct ObservableEntity {
    pub entity_id: String,
    pub perception_score: f64,
    pub clarity: f64,
    pub observable_facets: Vec<String>,   // KnowledgeEntry IDs（该角色当前对该实体可观察且可访问的 facets）
    pub notes: String,
}
```

`observable_facets` 由 `SceneFilter` 与 `KnowledgeAccessResolver` 共同决定：感官可达 + facet 访问策略通过。

### 3.3 AccessibleKnowledge

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
    pub accessible_content: serde_json::Value,    // 经访问控制裁剪后的内容（content / apparent_content / self_belief 三选一）
    pub source_hint: AccessSource,             // 该角色为何能访问这条（用于调试与 prompt 提示）
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

---

## 4. Layer 3 — Subjective State

```rust
pub struct CharacterSubjectiveState {
    pub character_id: String,
    pub scene_turn_id: String,

    pub belief_state: BeliefState,                                // 关于世界/事件的命题信念
    pub emotion_state: EmotionState,
    pub relation_models: HashMap<String, RelationModel>,          // 对他人的主观印象
    pub current_goals: CurrentGoals,                              // 含 short_term / medium_term / hidden
}
```

**职责边界**：
- "我相信 B 在撒谎" → `relation_models["B"].perceived_intent` 或类似字段。
- "我相信门外有刺客" → `belief_state` 中的命题信念。
- 伤势、疲惫、痛感、灵力消耗等当前身体状态 → Layer 1 `CharacterRecord.temporary_body_state`；L3 只保存角色对此状态的信念、情绪和目标反应。
- 不允许同一命题既写入 `belief_state` 又写入 `relation_models`。

---

