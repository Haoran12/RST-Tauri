# 10 Agent 数据模型与程序化派生

本文档承载：

- 三层数据语义（L1 Truth / L2 Per-Character Access / L3 Subjective）
- 全部 Agent 模式 struct / enum 定义（KnowledgeEntry、Cognitive I/O、UserInput、StyleConstraints、Realizer Input 等）
- 程序化派生：场景 / 物理 / 灵力档位翻译公式
- 仲裁公式（加算修正区 + soul_factor）
- SQLite 表结构

LLM 调用流程、主循环、验证规则见 [11_agent_runtime.md](11_agent_runtime.md)。LLM/程序边界铁律见 [01_architecture.md](01_architecture.md)。

---

## 1. 三层数据语义（强制隔离）

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

承载客观、可量化、直接影响行动与感知的物理量。属于 Layer 1 真相，由 `SceneStateExtractor` 与仲裁层维护，凡人/修士均可被影响。

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
    pub scene_turn_id: String,
}
```

由 StateCommitter 处理：
- 更新 `KnowledgeEntry.visibility.known_by`。
- 在 event_stream 追加事件。
- 创建一条 `KnowledgeEntry { kind: Memory }` 记录"X 何时如何获知 Y"。

### 2.7 角色静态档案（CharacterRecord）

角色不再有大而全的"static_profile" blob。所有"客观属于该角色的事实"都拆为多条 `KnowledgeEntry { kind: CharacterFacet, subject: Character{id, facet} }`，按需查询。

仅以下两项作为非 Knowledge 的角色基本数据保留在 Layer 1：

```rust
pub struct CharacterRecord {
    pub character_id: String,
    pub baseline_body_profile: BaselineBodyProfile,    // 物种/感官基线/灵觉基线/灵力数值（用于 EmbodimentResolver 与 SceneFilter）
    pub mind_model_card: MindModelCard,                // 自我形象/世界观/恐惧触发/防御模式（属于 subject 自我认知，默认 Aware）
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

- `MindModelCard` 在 Layer 1 也以 `KnowledgeEntry` 形式存在（subject 自我认知层），这里只是冗余索引以便 EmbodimentResolver 直接读取，**不允许它脱离 Knowledge 入口被外部读取**。
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
    Hazy,         // 30-100 m
    Limited,      // 5-30 m       仅近距离辨识
    Blind,        // < 5 m        几乎瞎走
}

pub enum PrecipitationIntensityTier {
    None,         // 无降水
    Light,        // 细雨/小雪/零星冰雹
    Moderate,     // 中雨/中雪
    Heavy,        // 大雨/大雪/能见度受影响
    Torrential,   // 暴雨/暴雪/沙暴/能影响行动与视野
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
    pub cold_strain: f64,                // 累积冷损耗（按时间累加，到阈值由仲裁层生成冻伤事件）
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
4. `cold_strain` / `heat_strain` 跨回合累积；到阈值由 `Arbitration` 生成具体伤势事件（冻伤/中暑），写回 Layer 1。
5. `disrupted_actions` 是 LLM 选择行动时的硬约束（在 IntentPlan 验证阶段比对），不是建议。
6. SurfaceRealizer 如需在叙事中提到风速/温度的具体数字，应通过 `SurfaceRealizerInput` 单独传入 raw 值（叙事用），不经 `FilteredSceneView`。
7. **L1 字段须保持自洽**：`physical_conditions` 各子字段间存在因果（暴雨 → wetness↑ → slipperiness↑；沙暴 → dust_density↑ → visibility↓ + respiration 受影响）。`SceneStateExtractor` 在产出 L1 时由 prompt 模板要求一并填齐；档位翻译层只负责把 L1 翻译成档位，不补全 L1 缺失。
8. 翻译公式集中在 `EmbodimentResolver::translate_environment(...)` 与 `SceneFilter::derive_weather_perception(...)`，两者共享同一份阈值表（避免两侧档位不一致）。

---

## 5. 程序化派生：灵力档位翻译

灵力的"档位"用于身份识别（"是凡人/修士/超凡/传说"），灵力的"数值差"用于实力对比（感知层是体感强弱，仲裁层是实际胜负）。两者都不让 LLM 自己估算 raw 数值。

档位边界数值参考 `D:\AI\rp_cards\` 锚点（凡人 100 / 入门 500–800 / 瓶颈 1300–1450 / 大成 2400 / 仙灵修行瓶颈 5000 / 神祇 苍角 8800 / 高阶仙灵 NaN），可在 `world_base.yaml` 中按世界重写。

```rust
pub enum ManaPotencyTier {
    // 单个角色 / 法器 / 法术 / 灵脉的灵力强度档位（默认边界，可由世界配置覆盖）
    Mundane,        // [0, 200)         凡人 / 无修行（锚: 人类无修行 100）
    Awakened,       // [200, 1000)      初醒 / 入门（锚: 妖精入门 500, 人类入门 700, 仙灵诞生 800）
    Adept,          // [1000, 1700)     成熟修士（锚: 妖精瓶颈 1400, 人类瓶颈 1300, 齐松 1450）
    Master,         // [1700, 2500)     大成（锚: 仙灵不修行成型 1800, 人妖大成 2400）
    Transcendent,   // [2500, 5400)     超凡（锚: 仙灵修行瓶颈 5000）
    Legendary,      // [5400, +∞)       传说 / 神祇 / 法则化（锚: 苍角 8800, 高阶仙灵 NaN）
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
    Indistinguishable,       // |Δ| < 200          相若, 难分高下
    SlightlyBelow,           // Δ ∈ [-500, -200)   略弱
    NotablyBelow,            // Δ ∈ [-1000, -500)  显著弱
    FarBelow,                // Δ ∈ [-2500, -1000) 远不及, 基本无力应对（仲裁=Crushing）
    Crushed,                 // Δ < -2500          蝼蚁差距, 无法测度（仲裁=Crushing）
    SlightlyAbove,           // Δ ∈ [200, 500)     略胜
    NotablyAbove,            // Δ ∈ [500, 1000)    显著强
    FarAbove,                // Δ ∈ [1000, 2500)   远胜, 守方基本无力应对（仲裁=Crushing）
    Overwhelming,            // Δ ≥ 2500           压顶, 无法测度（仲裁=Crushing）
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
   - `|Δ| ∈ [1000, 2500)`：可识别 tier，但 attribute 不稳；descriptors 偏向"远胜 / 远不及"。
   - `|Δ| ≥ 2500`：`tier_assessment = None`，descriptors 偏向"无法测度 / 如同蝼蚁"。
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

## 6. Mana Combat Resolution（仲裁层灵力对抗解算）

仲裁层与感知层用的是**不同**输入：

- 感知层：`displayed_mana_power`（含压制）→ 角色"觉得"对方多强。
- 仲裁层：`effective_mana_power`（不含压制；压制只是没主动用全力）→ 实际对抗按真实底力 + 技能 + 身体状态计算。

```rust
pub struct ManaCombatResolution {
    // 仲裁层使用，不进入 CognitivePass
    pub actor_id: String,
    pub target_id: String,
    pub actor_combat_power: f64,         // = effective_mana_power × max(0.1, 1 + Σ_modifiers) × soul_factor
    pub target_combat_power: f64,
    pub combat_delta: f64,               // actor_combat_power − target_combat_power
    pub outcome_tier: CombatOutcomeTier,
    pub disrupting_factors: Vec<String>, // 程序生成: ["攻方处于深度疲惫, 输出折半", "守方擅长水属性, 克制对手火属性"]
}

pub enum CombatOutcomeTier {
    // 由 |combat_delta| 桶映射；与感知层 ManaPerceptionDelta 共享 200/500/1000 三个阈值
    // 仲裁层不再细分 1000 以上：到了"基本无力应对"就够用了，差距更大也只是逃命姿态不同
    Indistinguishable,       // |Δ| < 200       势均力敌, 胜负看技巧/运气
    SlightEdge,              // Δ ∈ [200, 500)  攻方略占上风
    MarkedEdge,              // Δ ∈ [500, 1000) 攻方明显优势
    Crushing,                // Δ ≥ 1000        守方基本无力应对, 仅能逃避或求饶
    // 负向（攻方反吃亏）对称展开
}
```

### 6.1 仲裁公式（程序化）

```
combat_power = effective_mana_power × max(0.1, 1 + Σ_modifiers) × soul_factor
```

仅有**两个独立乘区**：加算修正区（多数因子在此叠加），与灵魂状态乘区（单独成区）。其余因子全部以**加和**方式落到 `Σ_modifiers` 内，不互乘。

1. **基础有效灵力** `effective_mana_power = base_mana_power + L1 状态修正`（突破/中毒/压制解除等，皆为 L1 真相，不含伤势疲惫——后者落入加算修正区）。
2. **加算修正区** `Σ_modifiers`（同区内所有修正以加和方式叠加）：

   **技能**：
   - 本命法术：**+0.10 ~ +0.20**
   - 克制属性：+0.10 ~ +0.30
   - 受克制：-0.10 ~ -0.30
   - mastery_rank：novice -0.10 ~ master +0.15

   **身体**：
   - 轻伤：-0.10
   - **显著疲惫：-0.20**
   - **身体重伤 / 灵力枯竭：-0.20 ~ -0.50**（按伤势严重度落区间）
   - `EnvironmentalStrain.disrupted_actions` 按 disrupted 程度：-0.10 ~ -0.40

   **心境**（来自 Layer 3 EmotionState 与 L1 突发情绪事件，按已有情绪标签程序化映射，不让 LLM 在仲裁时即兴选择）：
   - **亢奋 / 愤怒：+0.05 ~ +0.10**
   - 恐惧 / 迟疑：-0.05 ~ -0.15
   - 崩溃：-0.30 ~ -0.50

   **环境**：
   - 本属性 `Rich/Dense`：**通常 +0.05 ~ +0.15**
   - 本属性 `Saturated`：至 +0.20
   - `mana_haze`：-0.10
   - **明确设定的例外**（特定阵法 / 上古遗迹 / 神祇坐镇地脉等）：由 L1 `KnowledgeEntry { kind: RegionFact / FactionFact }` 的 `content.combat_modifiers` 字段显式给出非常规修正值，直接加入 `Σ_modifiers`，可超出上述区间。

3. **灵魂状态乘区** `soul_factor`（独立乘区，是除加算区外唯一的乘子）：
   - 灵魂完整：1.0
   - **灵魂破损 / 抽离：0.2 ~ 0.7**（按程度落区间，下限对应"魂飞魄散"级）

4. **下限保护**：加算系数以 `max(0.1, 1 + Σ_modifiers)` 截下限，避免修正过深导致 combat_power 趋零或为负而引发除零 / 碾压判定异常。

5. **outcome_tier** 按 `combat_delta = actor_combat_power − target_combat_power` 落桶（200 / 500 / 1000，1000 以上即 Crushing）；细化由 `disrupting_factors` 列出（程序生成的具体说明，例 ["攻方显著疲惫 -0.20", "守方身体重伤 -0.40 + 恐惧 -0.10 + 灵魂破损 ×0.5"]）。

6. 仲裁结果只决定**物理后果**（伤势 / 法力消耗 / 位置变化）写回 L1；**社会层后果**（恐惧 / 屈服 / 记仇）由下游角色 LLM 自行解读。

### 6.2 关键不变量

1. 仲裁公式只读 L1 的 `effective_mana_power`、L1 的身体状态、L1 的技能/属性数据；**不读 displayed_mana_power**（压制是认知层的事，不影响真实对抗）。
2. `combat_delta` 与 `ManaPerceptionDelta` 共享 200/500/1000 三个阈值，保证"我感觉略胜"与"实际略胜"在同一刻度上。仲裁层在 1000 以上不再细分（结果都是 Crushing）；感知层仍区分 `FarAbove(1000-2500)` 与 `Overwhelming(≥2500)`，但两者**对应的对抗结论一致**（皆为"基本无力应对"），区别只在体感（"远胜，难敌" vs "无法测度，压顶之势"）与是否可识别 tier。
3. 当 `disrupting_factors` 与 `outcome_tier` 出现"违和"（例如攻方 base_mana_power 高但身体状态极差导致 combat_delta 反而为负），SurfaceRealizer 必须在叙事中体现这种反差，而不是按"谁灵力高谁赢"硬写。
4. **以弱胜强**在该框架下要求**多个加算修正叠加 + 可能的灵魂状态打击**：守方若同时陷入"显著疲惫 (-0.20) + 身体重伤 (-0.40) + 恐惧 (-0.10) = Σ = -0.70"，加算系数 = max(0.1, 0.30) = 0.30；再叠加灵魂破损 soul_factor = 0.5，总系数 0.15，足以让基础灵力差 1500 的弱者翻盘。"算计 / 偷袭 / 中毒 / 惊扰魂魄"必须落到具体的 L1 状态字段上，由公式自然得出，不允许 LLM 在仲裁口径上手抹平差距。

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
    pub temporary_body_state: TemporaryBodyState,                 // 伤势/疲惫/痛感/灵力消耗
}
```

**职责边界**：
- "我相信 B 在撒谎" → `relation_models["B"].perceived_intent` 或类似字段。
- "我相信门外有刺客" → `belief_state` 中的命题信念。
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

CognitivePassOutput **必须为严格 JSON**，由 prompt 模板与 Provider JSON mode 共同保证。容错路径详见 [11_agent_runtime.md](11_agent_runtime.md)。

---

## 9. UserInputDelta（用户输入解析结果）

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
    pub pov: PointOfView,                  // omniscient / character_focused(id) / objective

    /// 自由文本字段：作者用自然语言书写的硬约束、参考文风、禁忌事项等。
    /// 仅供 LLM 阅读，不参与程序逻辑。
    pub explicit_guidelines: Vec<String>,
    pub reference_excerpts: Vec<String>,   // 参考片段（如"模仿《红楼梦》第三回的笔法"）
}
```

---

## 11. SurfaceRealizerInput（叙事层输入）

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

---

## 12. Dirty Flags（调用预算控制）

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

## 13. Skill Model（最小灵活版）

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

---

## 14. SQLite 表结构

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
