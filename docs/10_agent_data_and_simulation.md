# 10 Agent 数据模型与程序化派生

本文档承载：

- 三层数据语义（L1 Truth / L2 Per-Character Access / L3 Subjective）
- 全部 Agent 模式 struct / enum 定义（KnowledgeEntry、Cognitive I/O、UserInput、StyleConstraints、Realizer Input 等）
- 程序化派生：场景 / 物理 / 灵力档位翻译公式
- 对抗解算公式（加算修正区 + soul_factor）
- SQLite 表结构

LLM 调用流程、主循环、验证规则见 [11_agent_runtime.md](11_agent_runtime.md)。LLM/程序边界铁律见 [01_architecture.md](01_architecture.md)。

---

## 1. 三层数据语义（强制隔离）

为避免"客观真相"与"主观认知"在代码中混淆，运行时数据严格分为三层。**层间只能通过定义好的派生关系流动，禁止跨层直接读写。**

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 1 — Truth Store（客观真相，仅编排器与结果规划/验证层访问）│
│  ├── SceneModel              场景客观状态                    │
│  ├── KnowledgeEntry[*]       统一知识库（含世界/势力/角色档 │
│  │                           案/记忆，带可见性谓词）         │
│  └── 角色 baseline_body_profile（物种/感官基线/灵觉基线）    │
│      + temporary_body_state  伤势/疲惫/痛感/灵力消耗等当前态 │
│  约束：只有声明 God-read 的编排类节点可读此层；              │
│        CognitivePassInput / SurfaceRealizerInput 不出现       │
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
│  └── CurrentGoals            目标（含 hidden）               │
│  约束：本层是 LLM 的输出领地；任何关于"我相信 B 是好人"      │
│        的命题进入 RelationModels 而非 BeliefState（避免重复）│
└──────────────────────────────────────────────────────────────┘
```

**信息流向（线性，单向）**：

```
最近用户自由文本 + 当前 SceneModel(L1)
    → SceneStateExtractor(LLM, 场景域 God-read) → SceneUpdate / UserInputDelta 候选
    → Validator + StateApplier → World Truth (L1)
    → Embodiment 计算 (L1 baseline + L1 temp + L1 scene → L2 embodiment)
    → SceneFilter (L1 scene + L2 embodiment → L2 filtered_view)
    → KnowledgeAccess (L1 knowledge_store + 角色身份/属性 → L2 accessible_knowledge)
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

承载客观、可量化、直接影响行动与感知的物理量。属于 Layer 1 真相，由 `SceneStateExtractor` 候选更新与 StateCommitter 提交维护，凡人/修士均可被影响。

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
    // 三谓词，OR 关系（任一为真即可见）。
    // 例外：scope 含 GodOnly 时为 hard deny，优先级高于 known_by / scope / conditions。
    pub known_by: Vec<String>,                  // 名单制
    pub scope: Vec<VisibilityScope>,            // 标签制
    pub conditions: Vec<VisibilityCondition>,   // 条件制（运行时求值）
}

pub enum VisibilityScope {
    Public,                  // 所有原住民
    GodOnly,                 // 仅编排器（无人可知；hard deny）
    Region(String),          // 在该地区的角色
    Faction(String),         // 该势力成员
    Realm(String),           // 修为门槛及以上
    Role(String),            // 担任该职位
    Bloodline(String),       // 该血脉
    // 可扩展
}

pub enum VisibilityCondition {
    InSameSceneVisible,                                    // 同场景且能感知
    SocialAccessAtLeast { target: String, threshold: f64 }, // L1 客观关系/授权阈值；禁止读取 L3 relation_models
    HasSkill(String),                                      // 拥有特定技能
    CultivationAtLeast(String),                            // 修为达到
    CustomPredicate(VisibilityExpression),                 // 结构化 DSL AST 扩展点；禁止自然语言表达式
    // 可扩展
}

pub enum VisibilityExpression {
    All(Vec<VisibilityExpression>),
    Any(Vec<VisibilityExpression>),
    Not(Box<VisibilityExpression>),
    HasTag { subject_id: String, tag: String },
    NumericAtLeast { path: String, value: f64 },
    BooleanFlag { path: String, expected: bool },
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
4. `visibility.scope` 含 `GodOnly` 表示仅编排器可读，对所有角色不可见；`VisibilityResolver` 必须先检查 `GodOnly`，命中后直接拒绝，不再计算 `known_by` / 其他 scope / conditions。
5. `GodOnly` 启用态下 `visibility.known_by` 必须为空；Validator / StateCommitter 自动检查并拒绝 `GodOnly + known_by 非空` 的状态。
6. 若故事推进后 OutcomePlanner 候选 + EffectValidator 确认某条 `GodOnly` 知识可被角色获知，必须通过 `KnowledgeRevealEvent` 先移除 `GodOnly` 或降级为其他 scope，再追加 `known_by`；禁止在 `GodOnly` 仍存在时直接写入 `known_by`。
7. `MemoryEntry` 不再独立存在；历史事件以 `KnowledgeEntry { kind: Memory }` 形式统一存储。
8. Layer 1 的 Knowledge 内容由编排器/作者/StateCommitter 写入；CognitivePass 不可写。

### 2.5 Content Schema 约定（核心字段 + extensions 兜底）

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

### 2.6 KnowledgeRevealEvent（可见性扩展）

可见性变化必须通过显式事件触发，禁止隐式修改 `visibility.known_by`：

```rust
pub struct KnowledgeRevealEvent {
    pub event_id: String,
    pub knowledge_id: String,
    pub newly_known_by: Vec<String>,   // 此次新增的知情者
    pub trigger: RevealTrigger,        // 何种触发：witnessed / told / inferred / awakened
    pub scope_change: Option<VisibilityScopeChange>, // GodOnly 揭示时必须先降级/移除 GodOnly
    pub scene_turn_id: String,
}

pub enum VisibilityScopeChange {
    RemoveGodOnly,                     // 结果规划 + 程序校验确认该知识已可进入角色可知范围
    ReplaceScopes(Vec<VisibilityScope>),
}
```

由 StateCommitter 处理：
- 若原 entry 含 `GodOnly`，必须先验证 `scope_change` 已移除/替换 `GodOnly`，否则拒绝追加 `newly_known_by`。
- 更新 `KnowledgeEntry.visibility.scope` 与 `visibility.known_by`。
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
    pub visible_entities: Vec<VisibleEntity>,
    pub audible_signals: Vec<AudibleSignal>,
    pub olfactory_signals: Vec<OlfactorySignal>,
    pub tactile_signals: Vec<TactileSignal>,
    pub mana_signals: Vec<ManaSignal>,
    pub mana_environment: ManaEnvironmentSense,
    pub weather_perception: WeatherPerception,    // 风/温/能见度/降水的档位翻译 + 程序生成的具体描述
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

---

## 4. 程序化派生：环境档位翻译

LLM 不擅长把 raw 数值（`50.0 m/s`、`-30.0 ℃`、`视距 8 m`）翻译成行为后果。这一步在程序里做：`EmbodimentResolver` 与 `SceneFilter` 协同把 Layer 1 `physical_conditions` 的原始量映射为**档位 + 具体后果**，分别写入 `EmbodimentState.body_constraints.environmental_strain`（影响该角色行动）和 `FilteredSceneView.weather_perception`（角色对天气的主观感受）。

```rust
pub enum WindImpactTier {
    Calm,         // < 0.5 m/s
    Breeze,       // 0.5-5 m/s
    Moderate,     // 5-10 m/s
    Strong,       // 10-17 m/s    远程命中失准, 头发衣物明显被吹动
    Gale,         // 17-25 m/s    行动困难, 小型投射物偏移严重
    Storm,        // 25-32 m/s    站立困难, 小物件被吹飞, 树枝折断
    Hurricane,    // > 32 m/s     无法稳定站立, 大物件被卷起, 强行移动会被推走
}

pub enum TemperatureFeelTier {
    // 档位是相对该角色 BaselineBodyProfile.comfort_temperature_range 的偏离量映射
    // 同样 -30℃: 对人类是 SevereCold, 对厚毛皮的狐狸精可能只是 Cold
    Sweltering,   // 极易中暑
    Hot,
    Warm,
    Comfortable,
    Cool,
    Cold,         // 需保暖措施, 不耐久暴露
    SevereCold,   // 长时间暴露失温, 暴露皮肤受冻伤
    Lethal,       // 短时间致命
}

pub enum SurfaceImpactTier {
    Stable,
    Slippery,     // 跑动失败概率显著, 急停难
    Treacherous,  // 几乎无法稳定行动
}

pub enum VisibilityTier {
    Clear,        // > 100 m
    Hazy,         // 20-100 m
    Limited,      // 5-20 m       仅近距离辨识
    Blind,        // < 5 m        几乎瞎走
}

pub enum PrecipitationIntensityTier {
    None,         // 无降水
    Light,        // 细雨/小雪/零星冰雹
    Moderate,     // 中雨/中雪 行动与能见度略受影响
    Heavy,        // 大雨/大雪/冰雹 行动与能见度明显受影响，持续暴露有伤害风险
    Torrential,   // 暴雨/暴雪/沙暴/ 对于Transcendent以下的人物来说行动能力与视野能见度几乎归零，持续暴露有伤害风险
}

pub enum RespirationImpactTier {
    // 由 airborne (烟/尘/雾) + precipitation (沙暴) + mana_haze 综合给出
    Free,         // 呼吸顺畅
    Irritating,   // 刺激, 偶尔咳嗽, 长时间暴露不适
    Choking,      // 持续咳嗽, 呼吸吃力, 持续动作受影响
    Suffocating,  // 短时间致命, 必须捂口鼻或脱离
}

pub enum SurfaceVisualState {
    // 给 LLM 的"地面长什么样"; 可叠加（既积雪又结冰）
    Dry,
    Damp,
    Wet,          // 湿润但无积水
    Puddled,      // 积水
    Snowy,        // 积雪
    Icy,          // 结冰
    Bloody,
    Cluttered,    // 碎屑/法器残骸/瓦砾
}

pub struct EnvironmentalStrain {
    // 写入 EmbodimentState.body_constraints；驱动 action_feasibility 与跨回合身体状态
    pub wind_tier: WindImpactTier,
    pub temperature_tier: TemperatureFeelTier,
    pub surface_tier: SurfaceImpactTier,
    pub respiration_tier: RespirationImpactTier,
    pub movement_penalty: f64,           // 0.0-1.0
    pub balance_penalty: f64,            // 0.0-1.0
    pub cold_strain: f64,                // 累积冷损耗（按时间累加，到阈值由 OutcomePlanner 候选 + EffectValidator 生成冻伤事件）
    pub heat_strain: f64,
    pub respiration_strain: f64,         // 累积呼吸损耗（沙暴/浓烟久留触发咳嗽/缺氧伤害）
    pub disrupted_actions: Vec<String>,  // 具体限制说明，例 "无法施展持续吟唱的法术"、"远程瞄准命中-40%"
}

pub struct WeatherPerception {
    // 写入 FilteredSceneView；这是 LLM 在 CognitivePass 中读取的版本
    pub wind_tier: WindImpactTier,
    pub temperature_tier: TemperatureFeelTier,
    pub visibility_tier: VisibilityTier,
    pub respiration_tier: RespirationImpactTier,
    pub surface_visual: Vec<SurfaceVisualState>,    // 同时多种状态: 例 [Snowy, Icy]
    pub surface_tier: SurfaceImpactTier,            // 实际打滑程度（与 EnvironmentalStrain 同源）
    pub precipitation: Option<PrecipitationDescriptor>,
    pub effect_hints: Vec<String>,                  // 程序生成的具体后果描述: ["呼气结成白霜", "细小石子被风卷起拍在脸上", "脚下青苔湿滑"]
}

pub struct PrecipitationDescriptor {
    pub kind: PrecipitationKind,                    // 雨/雪/冰雹/沙暴/灵雨
    pub intensity_tier: PrecipitationIntensityTier,
    pub mana_attribute: Option<ManaAttribute>,      // 仅 SpiritRain 有
}

pub enum PrecipitationKind {
    Rain, Snow, Hail, Sandstorm, SpiritRain,
}
```

**关键不变量**：

1. CognitivePass 的 LLM **只读 tier + effect_hints**，不应从 raw 数值推断后果。`FilteredSceneView` 中不放 raw 数值。
2. 物种差异在档位翻译时已校准（用 `BaselineBodyProfile.comfort_temperature_range`），下游不用再判断"对该角色冷不冷"。
3. 灵力升温/冰寒（`TemperatureModifier.kind = 灵力*`）已在 `Temperature.felt_celsius` 中合并；档位只看最终 felt 值。
4. `cold_strain` / `heat_strain` 跨回合累积；到阈值由 OutcomePlanner 候选 + EffectValidator 生成具体伤势事件（冻伤/中暑），写回 Layer 1。
5. `disrupted_actions` 是 LLM 选择行动时的硬约束（在 IntentPlan 验证阶段比对），不是建议。
6. SurfaceRealizer 如需在叙事中提到风速/温度的具体数字，应通过 `SurfaceRealizerInput` 单独传入 raw 值（叙事用），不经 `FilteredSceneView`。
7. **L1 字段须保持自洽**：`physical_conditions` 各子字段间存在因果（暴雨 → wetness↑ → slipperiness↑；沙暴 → dust_density↑ → visibility↓ + respiration 受影响）。`SceneStateExtractor` 在产出 L1 时由 prompt 模板要求一并填齐；档位翻译层只负责把 L1 翻译成档位，不补全 L1 缺失。
8. 翻译公式集中在 `EmbodimentResolver::translate_environment(...)` 与 `SceneFilter::derive_weather_perception(...)`，两者共享同一份阈值表（避免两侧档位不一致）。

---

## 5. 程序化派生：灵力档位翻译

灵力的"档位"用于身份识别（"是凡人/修士/超凡/传说"），灵力的"数值差"用于实力对比（感知层是体感强弱，对抗解算层是实际胜负）。两者都不让 LLM 自己估算 raw 数值。

档位边界数值参考 `D:\AI\rp_cards\` 锚点（凡人 100 / 入门 500–800 / 瓶颈 1300–1450 / 大成 2400 / 仙灵修行瓶颈 5000 / 神祇 苍角 8800 / 高阶仙灵 NaN），可在 `world_base.yaml` 中按世界重写。

```rust
pub enum ManaPotencyTier {
    // 单个角色 / 法器 / 法术 / 灵脉的灵力强度档位（默认边界，可由世界配置覆盖）
    Mundane,       // [0, 200)         凡人 / 无修行（锚: 人类无修行 100）
    Awakened,      // [200, 1000)      入门（锚: 妖精入门 500, 人类入门 700, 仙灵诞生 800）
    Adept,         // [1000, 1800)     成熟/精英（锚: 妖精瓶颈 1400, 人类瓶颈 1300, 齐松 1450）
    Master,        // [1800, 2600)     大成（锚: 仙灵不修行成型 1800, 人妖大成 2400）
    Ascendant,     // [2600, 5600)     高阶（锚: 仙灵修行瓶颈 5000）
    Transcendent,  // [5600, +∞)       超越/超凡（锚: 苍角 7200, 高阶仙灵 NaN）
}

pub enum AmbientManaDensityTier {
    // 环境灵气浓度档位（ManaField.ambient_density 的翻译）
    Barren,         // 几近无灵气，普通修士难以汲取
    Sparse,         // 寻常人间街市
    Normal,         // 山林荒野默认水平
    Rich,           // 灵山福地，修行加成
    Dense,          // 灵脉所在 / 仙府 / 阵法核心，凡人会有压迫感
    Saturated,      // 神祇驻地 / 上古遗迹，弱者会过载乃至昏厥
}

pub enum ManaPerceptionDelta {
    // Δ = target.displayed_mana_power - observer.effective_mana_power
    // 用于"感觉差距多大"，与档位识别正交（同档可有显著差，跨档也可被技巧/状态拉平）
    Indistinguishable,       // |Δ| < 150          相若, 难分高下
    SlightlyBelow,           // Δ ∈ [-300, -150)   略弱
    NotablyBelow,            // Δ ∈ [-1000, -300)  显著弱
    FarBelow,                // Δ ∈ [-2000, -1000) 远不及, 基本无力应对（对抗解算=Crushing）
    Crushed,                 // Δ < -2000          蝼蚁差距, 无法测度（对抗解算=Crushing）
    SlightlyAbove,           // Δ ∈ [150, 300)     略胜
    NotablyAbove,            // Δ ∈ [300, 1000)    显著强
    FarAbove,                // Δ ∈ [1000, 2000)   远胜, 守方基本无力应对（对抗解算=Crushing）
    Overwhelming,            // Δ ≥ 2000           压顶, 无法测度（对抗解算=Crushing）
}

pub struct PerceivedManaProfile {
    pub source_id: String,                            // 被感知者 / 来源
    pub tier_assessment: Option<ManaPotencyTier>,     // 对方档位识别（被压制时为压制后的档）
    pub delta: ManaPerceptionDelta,                   // 感知差距档位
    pub attribute_assessment: Option<ManaAttribute>,  // 仅 |Δ| < 1000 且未被严重干扰时较准
    pub confidence: f64,                              // 0.0-1.0
    pub concealment_suspected: bool,                  // 感觉对方在压制气息
    pub descriptors: Vec<String>,                     // 程序生成: ["气息浩瀚如海", "似有若无, 形迹诡异"]
}

pub struct ManaSignal {
    // FilteredSceneView.mana_signals 中的单个气息：源于具体实体 / 法术 / 灵脉
    pub source_kind: ManaSourceKind,                  // Character / Artifact / SpellResidue / Formation / SpiritVein
    pub direction_hint: Option<String>,               // 方位与距离的粗化描述（不给精确坐标）
    pub perceived: PerceivedManaProfile,
}

pub struct ManaEnvironmentSense {
    // 整体环境灵气感知（区别于针对单一来源的 ManaSignal）
    pub density_tier: AmbientManaDensityTier,
    pub dominant_attribute: Option<ManaAttribute>,
    pub interferences: Vec<String>,                   // "屏蔽阵法残留", "灵雾阻隔感知"
    pub overload_risk: bool,                          // 灵觉过载风险（高敏锐度撞 Saturated 环境）
    pub descriptors: Vec<String>,                     // ["灵气浓郁如蜜, 呼吸间满是清甜"]
}
```

**感知规则（认知层）**——由 `SceneFilter::derive_mana_perception(...)` 程序化实施：

1. **观察者灵力** = `observer.effective_mana_power`（已含 L1 伤势 / 疲惫 / 突破修正）。
2. **目标显示灵力** `target.displayed_mana_power`：
   - 默认 = `target.effective_mana_power`。
   - 若目标具备压制能力且本回合启用：`displayed = effective - suppression_amount`（压制量来自 L1 状态，不让 LLM 自己定）。
3. **Δ = target.displayed_mana_power − observer.effective_mana_power**，按上述 9 档桶映射到 `ManaPerceptionDelta`。
4. **档位识别**：
   - `|Δ| < 1000`：可识别 `tier_assessment = ManaPotencyTier::from_power(displayed)` 与 `attribute_assessment`，`confidence ≥ 0.7`。
   - `|Δ| ∈ [1000, 2000)`：可识别 tier，但 attribute 不稳；descriptors 偏向"远胜 / 远不及"。
   - `|Δ| ≥ 2000`：`tier_assessment = None`，descriptors 偏向"无法测度 / 如同蝼蚁"。
5. **Mundane (Tier0) 观察者**：仅能将 `effective_mana_power ≥ 1000` 的存在感知为"超出常理"，无具体档位；环境灵气仅给"格外厚重 / 压抑"等体感。
6. **零灵觉**（`SensoryCapabilities.mana.acuity == 0`）：`mana_signals = []`，`mana_environment.density_tier` 由间接体感（呼吸/温度异常）回填，`dominant_attribute = None`。
7. **隐匿 / 压制**：
   - 压制后档位 `displayed_tier = ManaPotencyTier::from_power(displayed)` 直接落在 tier_assessment 上。
   - **破绽判定**（`concealment_suspected`）：当 `observer.effective_mana_power ≥ target.effective_mana_power − 200` 时（即观察者实力已能"接近"压制前的目标），置 true（"似有若无的违和感"）。否则压制看起来天衣无缝，false。
   - 灵觉敏锐度可作为额外破绽来源：`acuity ≥ 0.85` 且 `target.suppression_amount ≥ 1000` 时也强制 `concealment_suspected = true`（高灵觉天然能闻到压制痕迹）。
8. **环境干扰**：`ManaField.interferences` 中的 jam/scramble 按强度降低 `confidence`；`mana_haze` 让该回合所有 mana_signals 的 |Δ| 视为额外 +500（拉远感知，便于隐匿者进出）。
9. **属性相生相克**：观察者擅长属性与目标属性相同 → confidence +；相克 → 易识别（descriptors 含"违逆 / 刺骨"），同时影响 `attribute_assessment` 准确度与 descriptors 色彩。

**关键不变量**：

1. CognitivePass 永远不读 raw `mana_power`，只读 tier / delta / descriptors。`FilteredSceneView` 中不暴露 raw 数值。
2. 档位边界、Δ 桶边界、压制破绽阈值都是**世界配置项**（默认值同上，对 rp_cards 锚点校准），改边界需同时更新角色卡解析与单元测试。
3. 感知层只写**事实级感受**（"远胜 / 难测 / 似有压制"），**不写信念**（"他一定是神祇 / 他在装弱 / 他没安好心"）。这些信念由 CognitivePass 的 LLM 基于感受 + `prior_subjective_state` 自行生成。
4. ManaPotencyTier 同时为 `KnowledgeEntry { facet: CultivationRealm }` 的内部表征：visibility 决定"谁能看到这一档", 跨档感知精度决定"看到的是真档还是被压制的档"。
5. SurfaceRealizer 如需在叙事中提到"修为相差一筹/远胜/碾压"等具体差距文字，从 `ManaSignal.perceived.delta` 与 `tier_assessment` 取，不回查 raw mana_power。

---

## 6. Mana Combat Resolution（程序化灵力对抗解算）

对抗解算层与感知层用的是**不同**输入：

- 感知层：`displayed_mana_power`（含压制）→ 角色"觉得"对方多强。
- 对抗解算层：`effective_mana_power`（不含压制；压制只是没主动用全力）→ 实际对抗按真实底力 + 技能 + 身体状态计算。

```rust
pub struct ManaCombatResolution {
    // 对抗解算层使用，不进入 CognitivePass
    pub actor_id: String,
    pub target_id: String,
    pub actor_combat_power: f64,         // = effective_mana_power × max(0.1, 1 + Σ_modifiers) × soul_factor
    pub target_combat_power: f64,
    pub combat_delta: f64,               // actor_combat_power − target_combat_power
    pub outcome_tier: CombatOutcomeTier,
    pub disrupting_factors: Vec<String>, // 程序生成: ["攻方处于深度疲惫, 输出折半", "守方擅长水属性, 克制对手火属性"]
}

pub enum CombatOutcomeTier {
    // 由 |combat_delta| 桶映射；与感知层 ManaPerceptionDelta 共享 150/300/1000 三个阈值
    // 对抗解算层不再细分 1000 以上：到了"无力应对"就够用了
    Indistinguishable,       // |Δ| < 150       势均力敌, 胜负看临场发挥/技巧
    SlightEdge,              // Δ ∈ [150, 300)  攻方略占上风
    MarkedEdge,              // Δ ∈ [300, 1000) 攻方明显优势
    Crushing,                // Δ ≥ 1000        守方基本无力应对, 仅能逃避或求饶
    // 负向（攻方反吃亏）对称展开
}
```

### 6.1 对抗解算公式（程序化）

```
combat_power = effective_mana_power × max(0.1, 1 + Σ_modifiers) × soul_factor
```

仅有**两个独立乘区**：加算修正区（多数因子在此叠加），与灵魂状态乘区（单独成区）。其余因子全部以**加和**方式落到 `Σ_modifiers` 内，不互乘。

1. **基础有效灵力** `effective_mana_power = base_mana_power + L1 状态修正`（突破/中毒/压制解除等，皆为 L1 真相，不含伤势疲惫——后者落入加算修正区）。
2. **加算修正区** `Σ_modifiers`（同区内所有修正以加和方式叠加）：

   **技能**：
   - 本命法术：**+0.10 ~ +0.15**
   - 克制属性：+0.10 ~ +0.20
   - 受克制：-0.10 ~ -0.20
   - mastery_rank：novice -0.15 ~ master +0.15

   **身体**：
   - 轻伤：-0.05 ~ -0.15
   - **严重疲惫：-0.25**
   - **身体重伤 / 灵力枯竭：-0.20 ~ -0.50**（按伤势严重度落区间）
   - `EnvironmentalStrain.disrupted_actions` 按 disrupted 程度：-0.10 ~ -0.40

   **心境**（来自 Layer 3 EmotionState 与 L1 突发情绪事件，按已有情绪标签程序化映射，不让 LLM 在对抗解算时即兴选择）：
   - **自信 / 愤怒：+0.05 ~ +0.10**
   - 恐惧 / 迟疑：-0.05 ~ -0.15
   - 崩溃：-0.20 ~ -0.40

   **环境**：
   - 本属性 `Rich/Dense`：**通常 +0.1 ~ +0.15**
   - 本属性 `Saturated`：至 +0.20
   - `mana_haze`：-0.10
   - **明确设定的例外**（特定阵法 / 上古遗迹 / 神祇坐镇地脉等）：由 L1 `KnowledgeEntry { kind: RegionFact / FactionFact }` 的 `content.combat_modifiers` 字段显式给出非常规修正值，直接加入 `Σ_modifiers`，可超出上述区间。

3. **灵魂状态乘区** `soul_factor`（独立乘区，是除加算区外唯一的乘子）：
   - 灵魂完整：1.0
   - **灵魂破损 / 抽离：0.2 ~ 0.7**（按程度落区间，下限对应"魂飞魄散"级）

4. **下限保护**：加算系数以 `max(0.1, 1 + Σ_modifiers)` 截下限，避免修正过深导致 combat_power 趋零或为负而引发除零 / 碾压判定异常。

5. **outcome_tier** 按 `combat_delta = actor_combat_power − target_combat_power` 落桶（150 / 300 / 1000，1000 以上即 Crushing）；细化由 `disrupting_factors` 列出（程序生成的具体说明，例 ["攻方显著疲惫 -0.20", "守方身体重伤 -0.40 + 恐惧 -0.10 + 灵魂破损 ×0.5"]）。

6. 程序化对抗解算只决定**可验证物理后果**（伤势 / 法力消耗 / 位置变化）是否可写回 L1；公开退让、站队、敌对升级等外显社会事件可由 OutcomePlanner 候选输出，但内心恐惧 / 屈服 / 记仇仍由下游角色 CognitivePass 解读。

### 6.2 关键不变量

1. 对抗解算公式只读 L1 的 `effective_mana_power`、L1 的身体状态、L1 的技能/属性数据；**不读 displayed_mana_power**（压制是认知层的事，不影响真实对抗）。
2. `combat_delta` 与 `ManaPerceptionDelta` 共享 150/300/1000 三个阈值，保证"我感觉略胜"与"实际略胜"在同一刻度上。对抗解算层在 1000 以上不再细分（结果都是 Crushing）；感知层仍区分 `FarAbove(1000-2000)` 与 `Overwhelming(≥2000)`，但两者**对应的对抗结论一致**（皆为"基本无力应对"），区别只在体感（"远胜，难敌" vs "无法测度，压顶之势"）与是否可识别 tier。
3. 当 `disrupting_factors` 与 `outcome_tier` 出现"违和"（例如攻方 base_mana_power 高但身体状态极差导致 combat_delta 反而为负），SurfaceRealizer 必须在叙事中体现这种反差，而不是按"谁灵力高谁赢"硬写。
4. **以弱胜强**在该框架下要求**多个加算修正叠加 + 可能的灵魂状态打击**：守方若同时陷入"显著疲惫 (-0.20) + 身体重伤 (-0.40) + 恐惧 (-0.10) = Σ = -0.70"，加算系数 = max(0.1, 0.30) = 0.30；再叠加灵魂破损 soul_factor = 0.5，总系数 0.15，足以让基础灵力差 1500 的弱者翻盘。"算计 / 偷袭 / 中毒 / 惊扰魂魄"必须落到具体的 L1 状态字段上，由公式自然得出，不允许 LLM 在对抗解算口径上手抹平差距。

---

## 7. Layer 3 — Subjective State

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

## 8. Cognitive Pass I/O

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

    // 本回合事件 delta（程序过滤后的角色可见事件；不得使用 Layer 1 原始 SceneEvent）
    pub recent_event_delta: Vec<VisibleEventDelta>,
}

pub struct VisibleEventDelta {
    pub event_id: String,
    pub scene_turn_id: String,
    pub event_kind: String,                         // semantic
    pub involved_visible_entities: Vec<String>,     // semantic: entity_id list
    pub visible_effects: serde_json::Value,         // semantic: 结构化后果
    pub sensory_descriptors: Vec<String>,           // llm_readable: 角色可感知的声音/气味/光影等描述
    pub source_hint: AccessSource,                  // semantic / trace
}

pub struct CharacterCognitivePassOutput {
    pub perception_delta: PerceptionDelta,
    pub belief_update: BeliefUpdate,
    pub intent_plan: IntentPlan,
    pub body_reaction_delta: Option<BodyReactionDelta>,  // 情绪驱动的候选身体反应（手抖/脸红/失语）；不直接写 L1
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

CognitivePassOutput **必须为严格 schema JSON**，优先由 Provider structured output / tool schema 保证；JSON mode 仅作为降级路径，且必须在返回后通过 schema 校验。容错路径详见 [11_agent_runtime.md](11_agent_runtime.md)。

---

## 9. SceneStateExtractor I/O 与 UserInputDelta

用户的最近一轮自由文本输入由 SceneStateExtractor (LLM) 结合当前结构化场景信息解析。它不是普通角色认知节点，而是场景域编排节点：可读取当前 `SceneModel`，输出候选 `SceneUpdate` 与 `UserInputDelta`，但不直接写入 Layer 1。

第一版权限采用保守规则：

- 可读：最近一轮自由文本、当前 `SceneModel`、世界级 schema / 枚举 / 物理约束、与当前场景直接相关的公开设定。
- 默认不可读：隐藏角色 Knowledge、GodOnly Knowledge、非当前场景的私密历史，除非后续明确引入"作者编辑 / 导演模式"。
- 不可写：数据库和持久状态；只能输出候选 delta，由程序校验后应用。

```rust
pub struct SceneStateExtractorInput {
    pub scene_turn_id: String,
    pub recent_free_text: String,              // 用户最新输入或最近一轮聊天自由文本
    pub current_scene: SceneModel,             // 当前结构化场景 JSON
    pub world_constraints: serde_json::Value,  // semantic: 枚举、schema、物理边界、世界级规则
}

pub struct SceneStateExtractorOutput {
    pub scene_update: Option<SceneUpdate>,     // 候选场景更新
    pub user_input_delta: UserInputDelta,      // 结构化用户意图 / 扮演 / 元指令
    pub ambiguity_report: Vec<String>,         // llm_readable: 需要用户澄清但不阻塞的歧义
}

pub struct SceneUpdate {
    pub scene_turn_id: String,
    pub scene_delta: SceneDelta,
    pub update_reason: Vec<String>,            // trace_only / llm_readable
}
```

用户输入统一落为 `UserInputDelta`：

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

    /// 引导结果规划与文风（用户对当前回合的"导演"权）。
    /// outcome_bias 必须结构化；自由文本导演说明只能进入 style_override.explicit_guidelines 或 trace。
    DirectorHint { outcome_bias: Option<OutcomeBias>, style_override: Option<StyleConstraints> },
}

pub struct OutcomeBias {
    pub preferred_tone: Option<String>,             // semantic enum in implementation
    pub outcome_pressure: Option<OutcomePressure>,  // semantic
    pub protected_entities: Vec<String>,            // semantic entity_id list
    pub forbidden_outcomes: Vec<String>,            // semantic enum in implementation
    pub notes: Vec<String>,                         // llm_readable; 不参与程序硬判断
}

pub enum OutcomePressure {
    PreserveStatusQuo,
    EscalateConflict,
    DeescalateConflict,
    FavorPlayerIntent,
    FavorSimulationStrictness,
}
```

`SceneStateExtractor` 输出严格遵守此 schema。失败时进入容错路径（见 [11_agent_runtime.md](11_agent_runtime.md)）。

---

## 10. StyleConstraints（叙事层文风约束）

由作者预设或用户 DirectorHint 提供，最终交给 SurfaceRealizer LLM 阅读。

```rust
pub struct StyleConstraints {
    pub register: StyleRegister,           // ancient / modern / casual / formal / poetic
    pub detail_level: DetailLevel,         // sparse / moderate / rich
    pub atmosphere: Atmosphere,            // tense / serene / ominous / melancholic / ...
    pub pacing: Pacing,                    // fast / measured / slow
    pub pov: PointOfView,                  // omniscient / character_focused(id) / objective；不得覆盖 narration_scope 的可见性上限

    /// 自由文本字段：作者用自然语言书写的硬约束、参考文风、禁忌事项等。
    /// 仅供 LLM 阅读，不参与程序逻辑。
    pub explicit_guidelines: Vec<String>,
    pub reference_excerpts: Vec<String>,   // 参考片段（如"模仿《红楼梦》第三回的笔法"）
}
```

---

## 11. OutcomePlanner I/O（结果规划与状态更新计划）

OutcomePlanner 是编排类 LLM 节点，可以拥有 God 读取权限。它负责把场景真相、人物情绪与言行意图、技能契约/设定约束综合成"实际可能发生什么"和"候选需要更新哪些数据"。但它不直接写入数据库；输出必须被 EffectValidator 按技能契约和程序硬边界裁剪后，再交给 StateCommitter。

```rust
pub struct OutcomePlannerInput {
    pub scene_turn_id: String,

    // God-read: 结果规划需要的 L1 真相与规则
    pub scene_model: SceneModel,
    pub character_records: Vec<CharacterRecord>,
    pub relevant_knowledge: Vec<KnowledgeEntry>,
    pub skills: Vec<Skill>,

    // 来自受限认知节点或用户扮演输入
    pub character_outputs: Vec<CharacterCognitivePassOutput>,
    pub user_roleplay_intents: Vec<IntentPlan>,
    pub reaction_windows: Vec<ReactionWindow>,
    pub reaction_intents: Vec<ReactionIntent>,
    pub director_hint: Option<OutcomeBias>,
}

pub struct OutcomePlannerOutput {
    pub outcome_plan: OutcomePlan,
    pub state_update_plan: StateUpdatePlan,
    pub knowledge_reveal_events: Vec<KnowledgeRevealEvent>,
}

pub struct OutcomePlan {
    pub outward_actions: Vec<OutwardAction>,          // semantic: 已发生/尝试发生的外显行动
    pub resulting_state_changes: serde_json::Value,   // semantic: 候选硬变化摘要，真实提交以 StateUpdatePlan 为准
    pub visible_facts: Vec<String>,                   // semantic: 按 NarrationScope 派生的叙事事实白名单
    pub soft_effects: Vec<SoftEffect>,                // llm_readable: 可叙述但不写 L1
    pub blocked_effects: Vec<BlockedEffect>,          // semantic + trace: 被程序边界阻止的效果
}

pub struct StateUpdatePlan {
    pub scene_delta: Option<SceneDelta>,
    pub character_body_deltas: Vec<CharacterBodyDelta>,
    pub subjective_update_refs: Vec<String>,       // semantic: 对应角色 cognitive output / fallback output
    pub new_memory_entries: Vec<KnowledgeEntry>,   // kind 必须为 Memory
    pub soft_effects: Vec<SoftEffect>,             // llm_readable: 可叙述但不写入 L1 的软效果
    pub blocked_effects: Vec<BlockedEffect>,       // semantic + trace: 因超出契约/硬规则被阻止的效果
    pub validation_warnings: Vec<String>,          // trace_only: 程序裁剪或降级原因
    pub consistency_notes: Vec<String>,            // llm_readable / trace_only
}

pub struct CharacterBodyDelta {
    pub character_id: String,
    pub temporary_body_state_delta: serde_json::Value, // semantic: 伤势/疲惫/资源/冷却等结构化变更
    pub outward_body_signals: Vec<String>,             // llm_readable: 可被叙事层使用的外显身体反应
}

pub struct SoftEffect {
    pub source_id: String,
    pub target_id: Option<String>,
    pub effect_kind: String,             // semantic enum in implementation
    pub description: String,             // llm_readable
}

pub struct BlockedEffect {
    pub source_id: String,
    pub target_id: Option<String>,
    pub attempted_state_domain: String,  // semantic enum in implementation
    pub reason_code: String,             // semantic enum in implementation
    pub fallback_soft_effect: Option<SoftEffect>,
}
```

### 11.1 ReactionWindow（有限反应窗口）

ReactionWindow 解决"被攻击者及其伙伴是否能即时反应"的问题，但它不是递归事件链。程序打开窗口后，只收集合格角色的 `ReactionIntent`，再由同一次 OutcomePlanner 调用把原行动与所有反应意图一起结算。

```rust
pub struct ReactionWindow {
    pub window_id: String,
    pub scene_turn_id: String,
    pub source_event_id: String,
    pub source_action_id: String,
    pub threat_source_id: String,
    pub primary_targets: Vec<String>,
    pub observable_threat: VisibleEventDelta,
    pub eligible_reactors: Vec<ReactionEligibility>,
    pub max_reaction_depth: u8,                 // 默认 1；只有 interrupt 契约可显式提高到 2
    pub no_reaction_to_reaction: bool,          // 默认 true
    pub one_reaction_per_character: bool,       // 默认 true
}

pub struct ReactionEligibility {
    pub character_id: String,
    pub reason: ReactionEligibilityReason,      // target / ally_guard / area_protector / passive_field / interrupt_skill
    pub available_reaction_options: Vec<ReactionOption>,
    pub sensory_basis: Vec<AccessSource>,       // 看见/听见/灵觉/链接等结构化依据
    pub constraints: Vec<String>,               // semantic: distance / line_of_effect / cooldown / control_state 等
}

pub struct ReactionOption {
    pub option_id: String,
    pub skill_id: Option<String>,
    pub reaction_kind: ReactionKind,             // dodge / block / counter / protect_ally / interrupt / passive_mitigation
    pub target_scope: Vec<String>,
    pub cost_preview: CostProfile,
    pub legality_basis: Vec<String>,            // trace_only: 由哪个技能契约/姿态/被动规则允许
}

pub struct ReactionIntent {
    pub window_id: String,
    pub character_id: String,
    pub chosen_option_id: String,
    pub target_ids: Vec<String>,
    pub intent_rationale: String,                // llm_readable
}

pub struct ReactionPassInput {
    pub character_id: String,
    pub scene_turn_id: String,
    pub filtered_scene_view: FilteredSceneView,
    pub embodiment_state: EmbodimentState,
    pub accessible_knowledge: AccessibleKnowledge,
    pub prior_subjective_state: CharacterSubjectiveState,
    pub reaction_window: ReactionWindow,
    pub available_reaction_options: Vec<ReactionOption>,
}
```

不变量：

1. ReactionWindow 的开启、资格、距离/视线/感官、资源、冷却、援护关系与 `max_reaction_depth` 全由程序判定；LLM 不能自行把旁观者加入窗口。
2. `ReactionIntent` 只表达"打算如何反应"，不立即产生新的 `OutwardAction` 或 `StateUpdatePlan`；反应造成的反击、格挡、援护统一进入 OutcomePlanner 的一次性结算。
3. 默认 `no_reaction_to_reaction = true`。B 的反击不再为 A 打开新的普通反应窗口；只有 SkillEffectContract 明确声明 interrupt/反制反击，且深度未超过上限时才允许进入第二层。
4. 每个角色在同一窗口默认最多提交一个 `ReactionIntent`；未提交或 LLM 失败时，OutcomePlanner 可按 `passive`/默认防御策略兜底，但必须写 trace。
5. 旁观者或伙伴能否反应取决于 `observable_threat` 对该角色是否可见，以及其 `ReactionOption` 是否覆盖目标、距离、通道和资源；"站在场上"本身不构成反应资格。

硬约束：

- `OutcomePlanner` 可读 L1 / GodOnly 用于判断，但 `outcome_plan.visible_facts` 必须按 `NarrationScope` 派生，不能把 GodOnly 直接给叙事层。
- `StateUpdatePlan` 中的数值、资源、位置、伤势、可见性变更必须能被程序公式、技能契约或 Validator 校验；校验失败时不反复调用 LLM，非法硬效果进入 `blocked_effects` 或降级为 `soft_effects`，不得写入 L1。
- `BodyReactionDelta` 只作为候选身体反应；如需改变 `temporary_body_state`，必须由 OutcomePlanner/EffectValidator 转成合法 `CharacterBodyDelta` 后经 StateCommitter 提交。
- 角色是否"相信 / 接受 / 记恨"不由 OutcomePlanner 直接写入 L3，除非它来自该角色本回合 `CharacterCognitivePassOutput`；否则作为外显事件进入下一轮认知输入。
- OutcomePlanner 必须把 `reaction_windows` 中的原行动与 `reaction_intents` 一起结算；禁止在结算中再自由打开无限新窗口。

---

## 12. SurfaceRealizerInput（叙事层输入）

叙事层 LLM 仅接受以下四类结构化输入，**不再读取角色档案 / 世界设定 / 角色心智**（它们已体现在情景与认知结果中）。

```rust
pub struct SurfaceRealizerInput {
    pub scene_turn_id: String,

    /// 0. 叙事可见性边界：决定 SceneNarrativeView 与 visible_facts 的生成范围。
    pub narration_scope: NarrationScope,

    /// 1. 情景提取结果：本回合场景的客观状态视图（叙事层视角下的"舞台"）。
    pub scene_view: SceneNarrativeView,

    /// 2. 各角色 Agent 的认知和意图结果（仅 Tier A/B 中实际进行了 cognitive pass 的角色）。
    pub character_outputs: Vec<CharacterCognitivePassOutput>,

    /// 3. 结果计划：本回合的外显行动、合法硬后果摘要、软效果与受阻效果。
    pub outcome_plan: OutcomePlan,

    /// 文风约束（含自由文本指引）。
    pub style: StyleConstraints,
}

pub enum NarrationScope {
    /// 仅允许叙述指定角色可观察 / 可推断的事实；用于角色聚焦 POV。
    CharacterFocused { character_id: String },

    /// 仅允许叙述场上外显事实，不进入任何角色内心，不暴露隐藏 Knowledge。
    ObjectiveCamera,

    /// 作者/编排器视角；仍不得包含 GodOnly，除非该输出明确标记为调试/作者私有视图。
    DirectorView,
}
```

`SceneNarrativeView` 是 SceneModel 按 `narration_scope` 派生的叙事视图：`CharacterFocused` 只能使用该角色的 Layer 2 可见事实；`ObjectiveCamera` 只能使用外显事实；`DirectorView` 可使用编排器可见事实但默认仍剔除 `GodOnly`。
`OutcomePlan` 包含：每个角色的 outward_action（已发生的事）、resulting_state_changes（伤势/位置/资源等候选硬变化）、visible_facts（按 `narration_scope` 生成的叙事事实白名单）、soft_effects 与 blocked_effects。SurfaceRealizer 可以叙述软效果和受阻结果，但 NarrativeFactCheck 与 StateCommitter 只承认已通过程序校验的硬变化。

---

## 13. Dirty Flags（调用预算控制）

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

触发规则详见 [11_agent_runtime.md](11_agent_runtime.md)。

---

## 14. Skill Model（契约 + LLM）

```rust
pub struct Skill {
    pub skill_id: String,
    pub name: String,
    pub trigger_mode: TriggerMode,         // active / reaction / passive / channeled
    pub delivery_channel: DeliveryChannel, // gaze / voice / touch / projectile / scent / spiritual_link / ritual / field
    pub impact_scope: ImpactScope,         // body / perception / mind / soul / scene
    pub effect_contract: SkillEffectContract,
    pub notes: String,                     // llm_readable: 技能意象、限制、常见表现；不直接参与程序判断
}

pub struct SkillEffectContract {
    pub allowed_target_kinds: Vec<TargetKind>,
    pub allowed_state_domains: Vec<StateDomain>,      // body / resource / position / perception / mind / soul / scene / knowledge_reveal
    pub cost_profile: CostProfile,                    // semantic: 法力/体力/冷却/材料等成本
    pub max_intensity_tier: EffectIntensityTier,      // 程序校验硬效果强度上限
    pub allows_injury: bool,
    pub allows_position_change: bool,
    pub allows_knowledge_reveal: bool,
    pub requires_line_of_effect: bool,
    pub duration_policy: DurationPolicy,
    pub opens_reaction_window: bool,
    pub allows_interrupt: bool,
    pub max_reaction_depth_override: Option<u8>,  // 默认 None；若 Some(2) 必须由 EffectValidator 校验
}

pub struct CharacterSkillUseProfile {
    pub character_id: String,
    pub skill_id: String,
    pub mastery_rank: u8,  // 1-5: novice / trained / skilled / expert / master
    pub notes: String,
}
```

技能的"该角色掌握哪些技能"以 `KnowledgeEntry { kind: CharacterFacet, facet: KnownAbility | HiddenAbility }` 表达，统一受可见性约束。OutcomePlanner 可以读取 `notes` 理解复杂效果，但硬状态变化只能落在 `effect_contract` 允许的范围内；超出范围的候选效果由 EffectValidator 转入 `blocked_effects` 或 `soft_effects`。

---

## 15. SQLite 表结构

按三层语义组织。Layer 2 不持久化（每回合重建）；Layer 1 / Layer 3 / Trace 各自独立。Agent 模式以 World 为故事连续性单元，聊天记录删除 / 回退必须从目标回合开始截断后续全部回合，并回滚到一致的世界状态，因此每个回合提交都必须有可追溯记录。

```sql
-- ===== Turn Commit（故事线与回滚锚点） =====

-- 世界故事线回合链；删除聊天时只能按 parent_turn_id 截断后续回合，不能单删中间消息
CREATE TABLE world_turns (
    scene_turn_id TEXT PRIMARY KEY,
    parent_turn_id TEXT,
    user_message TEXT NOT NULL,             -- JSON: 用户输入/扮演输入
    rendered_output TEXT,                   -- SurfaceRealizer 输出
    status TEXT NOT NULL DEFAULT 'active',  -- active / rolled_back
    created_at TEXT NOT NULL,
    rolled_back_at TEXT,
    FOREIGN KEY (parent_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- 每回合状态提交记录；用于定位需要回滚的人物、世界、知识和 trace 变化
CREATE TABLE state_commit_records (
    commit_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    changed_scene_snapshot_ids TEXT NOT NULL,       -- JSON array
    changed_knowledge_ids TEXT NOT NULL,            -- JSON array
    changed_character_ids TEXT NOT NULL,            -- JSON array
    changed_subjective_snapshot_ids TEXT NOT NULL,  -- JSON array
    trace_ids TEXT NOT NULL,                        -- JSON array
    rollback_patch TEXT NOT NULL,                   -- JSON: 反向补丁或变更前镜像
    created_at TEXT NOT NULL,
    rolled_back_at TEXT,
    rollback_reason TEXT,
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

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
    scope_change TEXT,                     -- JSON: VisibilityScopeChange；GodOnly 揭示时必须有值
    scene_turn_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (knowledge_id) REFERENCES knowledge_entries(knowledge_id)
);

-- 角色基本档案（仅 baseline_body_profile + mind_model_card_knowledge_id 指针；其余事实在 knowledge_entries 中）
CREATE TABLE character_records (
    character_id TEXT PRIMARY KEY,
    baseline_body_profile TEXT NOT NULL,   -- JSON
    mind_model_card_knowledge_id TEXT NOT NULL,
    temporary_body_state TEXT NOT NULL,    -- JSON: Layer 1 当前身体/资源状态
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
    created_at TEXT NOT NULL
);

-- ===== Trace / Logs（调试、回放与运行观测） =====

CREATE TABLE turn_traces (
    trace_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    trace_kind TEXT NOT NULL,              -- turn / character / presentation / rollback
    character_id TEXT,                     -- NULL 表示全局回合 trace
    summary TEXT NOT NULL,                 -- JSON: 回合级关键产物索引与摘要
    linked_request_ids TEXT NOT NULL,       -- JSON array: 关联 llm_call_logs.request_id
    linked_event_ids TEXT NOT NULL,         -- JSON array: 关联 app_event_logs.event_id
    created_at TEXT NOT NULL,
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

CREATE TABLE agent_step_traces (
    step_trace_id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    scene_turn_id TEXT NOT NULL,
    character_id TEXT,                     -- NULL 表示全局步骤
    step_name TEXT NOT NULL,               -- active_set / dirty_flags / scene_filter / cognitive_pass / validation / outcome_planning / effect_validation / state_commit 等
    step_status TEXT NOT NULL,             -- started / skipped / succeeded / failed / fallback_used
    input_summary TEXT,                    -- JSON: 结构化输入摘要
    output_summary TEXT,                   -- JSON: 结构化输出摘要
    decision_json TEXT,                    -- JSON: 关键判定值、跳过原因、验证失败项
    linked_request_id TEXT,                -- 对应 llm_call_logs.request_id
    error_event_id TEXT,                   -- 对应 app_event_logs.event_id
    created_at TEXT NOT NULL,
    FOREIGN KEY (trace_id) REFERENCES turn_traces(trace_id),
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

CREATE TABLE llm_call_logs (
    request_id TEXT PRIMARY KEY,
    mode TEXT NOT NULL,                    -- st / agent
    world_id TEXT,
    scene_turn_id TEXT,
    trace_id TEXT,
    character_id TEXT,
    llm_node TEXT NOT NULL,                -- STChat / SceneStateExtractor / CharacterCognitivePass / OutcomePlanner / SurfaceRealizer
    api_config_id TEXT NOT NULL,           -- 调用时实际使用的 API 配置
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    call_type TEXT NOT NULL,               -- chat / chat_structured / chat_stream
    request_json TEXT NOT NULL,            -- JSON: 写入前必须脱敏
    schema_json TEXT,                      -- JSON Schema，仅 structured 调用有值
    response_json TEXT,                    -- JSON: 原始响应或结构化结果
    assembled_text TEXT,                   -- stream chunk 直接拼接文本
    readable_text TEXT,                    -- 仅展示用的段落化文本
    status TEXT NOT NULL,                  -- started / succeeded / failed / cancelled
    latency_ms INTEGER,
    token_usage TEXT,                      -- JSON
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_summary TEXT,
    redaction_applied INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id),
    FOREIGN KEY (trace_id) REFERENCES turn_traces(trace_id)
);

-- Agent LLM 节点配置档案；api_config_id 指向 ./data/api_configs/ 中的 ST/API 配置池
CREATE TABLE agent_llm_profiles (
    profile_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    default_api_config_id TEXT NOT NULL,
    bindings TEXT NOT NULL,                -- JSON: Vec<AgentLlmConfigBinding>
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- World 级 Agent LLM 配置选择；允许每个 World 使用不同的四节点配置
CREATE TABLE world_agent_settings (
    world_id TEXT PRIMARY KEY,
    agent_llm_profile_id TEXT NOT NULL,
    profile_overrides TEXT,                -- JSON: 可选 World 级覆盖
    updated_at TEXT NOT NULL,
    FOREIGN KEY (agent_llm_profile_id) REFERENCES agent_llm_profiles(profile_id)
);

CREATE TABLE llm_stream_chunks (
    chunk_id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    raw_chunk TEXT NOT NULL,
    received_at TEXT NOT NULL,
    FOREIGN KEY (request_id) REFERENCES llm_call_logs(request_id)
);

CREATE TABLE app_event_logs (
    event_id TEXT PRIMARY KEY,
    level TEXT NOT NULL,                   -- debug / info / warn / error / fatal
    event_type TEXT NOT NULL,
    message TEXT NOT NULL,
    source_module TEXT NOT NULL,
    request_id TEXT,
    world_id TEXT,
    scene_turn_id TEXT,
    trace_id TEXT,
    character_id TEXT,
    detail_json TEXT,                      -- JSON: 异常上下文，写入前必须脱敏
    created_at TEXT NOT NULL
);

CREATE TABLE log_retention_state (
    retention_id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,                   -- global / world
    world_id TEXT,
    size_limit_bytes INTEGER NOT NULL DEFAULT 1073741824,
    current_size_bytes INTEGER,
    last_checked_at TEXT,
    last_cleanup_at TEXT,
    cleanup_needed INTEGER NOT NULL DEFAULT 0,
    user_prompt_required INTEGER NOT NULL DEFAULT 0,
    detail_json TEXT
);

-- ===== 索引 =====

CREATE INDEX idx_scene_snapshots_scene ON scene_snapshots(scene_id);
CREATE INDEX idx_world_turns_parent ON world_turns(parent_turn_id);
CREATE INDEX idx_commit_records_turn ON state_commit_records(scene_turn_id);
CREATE INDEX idx_knowledge_kind ON knowledge_entries(kind);
CREATE INDEX idx_knowledge_subject ON knowledge_entries(subject_type, subject_id);
CREATE INDEX idx_knowledge_facet ON knowledge_entries(subject_id, facet_type) WHERE kind = 'character_facet';
CREATE INDEX idx_reveal_knowledge ON knowledge_reveal_events(knowledge_id);
CREATE INDEX idx_subjective_char ON character_subjective_snapshots(character_id, scene_turn_id);
CREATE INDEX idx_traces_turn ON turn_traces(scene_turn_id);
CREATE INDEX idx_step_traces_turn ON agent_step_traces(scene_turn_id);
CREATE INDEX idx_step_traces_trace ON agent_step_traces(trace_id);
CREATE INDEX idx_llm_logs_turn ON llm_call_logs(scene_turn_id);
CREATE INDEX idx_llm_logs_trace ON llm_call_logs(trace_id);
CREATE INDEX idx_llm_logs_api_config ON llm_call_logs(api_config_id);
CREATE INDEX idx_llm_logs_created ON llm_call_logs(created_at);
CREATE INDEX idx_stream_chunks_request ON llm_stream_chunks(request_id, chunk_index);
CREATE INDEX idx_app_events_context ON app_event_logs(world_id, scene_turn_id, trace_id);
CREATE INDEX idx_app_events_created ON app_event_logs(created_at);
```

**说明**：

- `knowledge_entries.visibility` 是 JSON 而非规范化的多表，是因为查询入口永远是 `VisibilityResolver`（程序化逻辑），不靠 SQL 谓词检索。
- `subject_id + facet_type` 联合索引服务"取角色 X 的所有 facets"这一最高频查询。
- `character_subjective_snapshots` 的最新一条即角色当前心智状态；历史快照保留用于回放与一致性验证。
- 没有"memory_records"表，记忆作为 `knowledge_entries.kind = 'memory'` 统一存储。
- `world_turns.parent_turn_id` 定义故事线顺序；用户删除某条 Agent 聊天记录时，必须将该回合及其后续全部回合标记为 `rolled_back`，禁止单独删除中间消息，并按 `state_commit_records.rollback_patch` 恢复到目标父回合的一致 Layer 1 / Layer 3 状态。
- Agent Trace 以 `scene_turn_id` 为主轴，解释回合如何演化；运行 Logs 以 `request_id` / `event_id` 为主轴，解释应用运行时发生了什么。两者可互相关联，但日志不得作为 Agent 判断或 LLM 输入来源。
- Agent 模式允许四类 LLM 节点绑定不同 API 配置；`agent_llm_profiles.bindings` 保存用户选择，`llm_call_logs.api_config_id` 保存每次调用实际使用的配置。
- `llm_call_logs.request_json`、`response_json`、`llm_stream_chunks.raw_chunk` 尽量还原 Provider 原貌；`readable_text` 仅用于流式响应的段落化查看，不替代原始响应。
- `app_event_logs` 同表结构可用于 `./data/logs/app_logs.sqlite`。ST 模式只写全局运行 Logs；Agent 模式与回合相关的记录写入对应 `world.sqlite`，同时可在全局异常日志中保留带 `world_id` / `scene_turn_id` 的索引事件。
- 默认清理上限为 1GB。自动清理只处理全局运行 Logs；Agent Trace 和仍被 `state_commit_records.trace_ids` 引用的记录不自动删除。30 天以上未更新且日志体积较大的 World 只产生提示事件，等待用户确认。
