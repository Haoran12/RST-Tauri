# 10 Agent 数据模型

本文档承载 Agent 模式的数据语义与结构化模型：

- 三层数据语义（L1 Truth / L2 Per-Character Access / L3 Subjective）
- Layer 1 客观真相总览：SceneModel / LocationGraph / KnowledgeEntry / CharacterRecord
- Layer 2 逐角色可触及视图：EmbodimentState / FilteredSceneView / AccessibleKnowledge
- Layer 3 主观状态：Belief / Emotion / Relation / Goals

程序化派生见 [12_agent_simulation.md](12_agent_simulation.md)，对抗解算与技能契约见 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)。LLM 节点 I/O 契约见 [13_agent_llm_io.md](13_agent_llm_io.md)。Knowledge 模型见 [17_agent_knowledge_model.md](17_agent_knowledge_model.md)，角色模型见 [18_agent_character_model.md](18_agent_character_model.md)，时间线规则见 [16_agent_timeline_and_canon.md](16_agent_timeline_and_canon.md)，SQLite 持久化见 [14_agent_persistence.md](14_agent_persistence.md)。运行时主循环见 [11_agent_runtime.md](11_agent_runtime.md)。LLM/程序边界铁律见 [01_architecture.md](01_architecture.md)。

---

## 1. 三层数据语义（强制隔离）

为避免"客观真相"与"主观认知"在代码中混淆，运行时数据严格分为三层。**层间只能通过定义好的派生关系流动，禁止跨层直接读写。**

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 1 — Truth Store（客观真相，仅编排器与结果规划/验证层访问）│
│  ├── SceneModel              场景客观状态                    │
│  ├── LocationGraph           地点层级、别名、路线边与路程估算 │
│  ├── KnowledgeEntry[*]       统一知识库（含世界/势力/角色档 │
│  │                           案/记忆，带访问策略）           │
│  └── CharacterRecord        基础属性 / 身体基线 / 当前临时态 │
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

地点名称 / 场景锚点
    → LocationResolver(LocationGraph) → LocationNode + ancestors / ambiguity
    → LocationFactResolver(LocationGraph + LocationSpatialRelation + RegionFact + KnowledgeAccessResolver)
    → RoutePlanner(LocationEdge 带权图)
    → LocationContext(运行时输入的一部分，不持久化)

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
    pub private_state: ScenePrivateState,       // 隐藏实体/机关/伪装/连续性秘密；仅场景域 God-read 与验证层可读
    pub event_stream: Vec<SceneEvent>,
    pub uncertainty_notes: Vec<String>,
}

pub struct ScenePrivateState {
    pub hidden_facts: Vec<ScenePrivateFact>,
    pub reveal_triggers: Vec<SceneRevealTrigger>,
    pub source_constraint_ids: Vec<String>,
}

pub struct ScenePrivateFact {
    pub fact_id: String,
    pub source_knowledge_id: Option<String>,
    pub applies_to: Vec<String>,              // scene_id / entity_id / location_id / participant_id
    pub fact_kind: String,                    // hidden_presence / trap / disguise / sealed_area / continuity_secret ...
    pub structured_payload: serde_json::Value,
    pub summary_text: String,                 // llm_readable；只供场景域编排一致性
}

pub struct SceneRevealTrigger {
    pub trigger_id: String,
    pub private_fact_id: String,
    pub condition_refs: Vec<String>,          // semantic refs；由程序/技能契约/KnowledgeRevealEvent 校验
    pub reveal_target: RevealTarget,
}

pub enum RevealTarget {
    PublicSceneFact,
    KnowledgeEntry(String),
    CharacterKnownBy(Vec<String>),
}
```

`ScenePrivateState` 属于 Layer 1，但不是公开场景事实。`SceneFilter` 默认不得把它派生进 `FilteredSceneView`；`SurfaceRealizerInput` 默认不得携带它；只有 `SceneInitializer` / `SceneStateExtractor` 的场景域 God-read 输入、`OutcomePlanner`、Validator 与 StateCommitter 可按权限读取。私有事实若被故事揭示，必须转换为公开 `SceneDelta`、`KnowledgeRevealEvent` 或合法 `StateUpdatePlan` 后提交。

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
    pub character_presences: Vec<CharacterManaPresence>, // 人物当前灵力显露对局部环境的影响；由持久倾向 + ManaExpressionState 派生
    pub flow: ManaFlow,
    pub interferences: Vec<ManaInterference>,    // 屏蔽/扰乱/伪装/放大/重定向
}

pub enum ManaSourceType {
    SpiritVein, FormationCore, BarrierNode, SpiritWell,            // 环境源
    CultivatorAura, ArtifactAura, SpiritBeastAura, FormationTrace, // 实体源
    SpellResidue, Breakthrough, Tribulation, Sacrifice,            // 事件源
    Corruption, Seal, VoidRift,                                    // 异常源
}

pub struct CharacterManaPresence {
    pub character_id: String,
    pub source_type: ManaSourceType,              // CultivatorAura / SpiritBeastAura 等
    pub expression_mode: ManaExpressionMode,      // 当前场景状态：封息/抑制/自然/外放/威压
    pub radius_tier: ManaPresenceRadiusTier,      // 影响范围粗档
    pub pressure_delta: AttributeDelta,           // 对同场观察者的压迫/舒缓体感差距档；不含 raw 数值
    pub attribute: Option<ManaAttribute>,
    pub descriptors: Vec<String>,                 // llm_readable: "气息收束如沉水", "威压铺满厅堂"
}
```

### 2.3.1 LocationGraph（地点层级、自然地理与路线图）

`LocationGraph` 是 Layer 1 的结构化地点真相，由地点层级、自然地理覆盖关系和路线关系组成。完整数据模型、解析、地区事实继承与路程估算见 [15_agent_location_system.md](15_agent_location_system.md)。

本文件只保留边界：地点 ID、父级链、自然地理关系和路线图都属于 Layer 1；运行时通过 LocationResolver / LocationFactResolver / RoutePlanner 派生上下文，禁止 LLM 猜测唯一地点或把低置信度估算写成硬事实。

### 2.3.2 时间锚点、会话与正史资格

Agent World 的主线光标、`TimeAnchor`、`AgentSession`、过去线正史资格和冲突处理规则已拆分到 [16_agent_timeline_and_canon.md](16_agent_timeline_and_canon.md)。

### 2.4 KnowledgeEntry（统一知识模型）

`KnowledgeEntry`、访问策略、content schema、`TruthGuidance` 和 `KnowledgeRevealEvent` 已拆分到 [17_agent_knowledge_model.md](17_agent_knowledge_model.md)。

### 2.5 角色静态档案（CharacterRecord）

`CharacterRecord`、基础属性、身体基线、临时状态、灵力显露长期倾向与运行时状态已拆分到 [18_agent_character_model.md](18_agent_character_model.md)。

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

pub struct SensoryCapabilities {
    pub vision: SensoryCapability,
    pub hearing: SensoryCapability,
    pub smell: SensoryCapability,
    pub touch: SensoryCapability,
    pub proprioception: SensoryCapability,
    pub mana: SensoryCapability,
}

pub struct SensoryCapability {
    pub availability: f64,  // 0.0-1.0 可用性
    pub acuity: f64,        // 敏锐度
    pub stability: f64,     // 稳定性
    pub notes: String,
}

pub struct BodyConstraints {
    pub mobility: f64,             // 0.0-1.0
    pub balance: f64,              // 0.0-1.0
    pub fine_control: f64,         // 0.0-1.0
    pub pain_load: f64,            // 0.0-1.0
    pub fatigue_load: f64,         // 0.0-1.0
    pub cognitive_clarity: f64,    // 0.0-1.0
    pub environmental_strain: Vec<String>, // tier / descriptor refs
}

pub struct SalienceModifiers {
    pub attention_biases: Vec<String>,     // llm_readable descriptors
    pub aversion_triggers: Vec<String>,
    pub overload_risk: f64,                // 0.0-1.0
}

pub struct ReasoningModifiers {
    pub pain_bias: f64,        // 0.0-1.0
    pub threat_bias: f64,      // 0.0-1.0
    pub overload_bias: f64,    // 0.0-1.0
    pub notes: Vec<String>,
}

pub struct ActionFeasibility {
    pub physical_execution: f64,    // 0.0-1.0
    pub social_patience: f64,       // 0.0-1.0
    pub fine_control: f64,          // 0.0-1.0
    pub sustained_attention: f64,   // 0.0-1.0
    pub blocked_action_kinds: Vec<String>,
}
```

### 3.2 FilteredSceneView

```rust
pub struct FilteredSceneView {
    pub character_id: String,
    pub scene_turn_id: String,
    pub observable_entities: Vec<ObservableEntity>,
    pub perceived_attributes: Vec<PerceivedAttributeProfile>, // 对可观察实体的属性档位/差距评估；不含 raw 数值
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
- 伤势、疲惫、痛感、灵力消耗、魂伤等当前临时状态 → Layer 1 `CharacterRecord.temporary_state`；L3 只保存角色对此状态的信念、情绪和目标反应。
- 不允许同一命题既写入 `belief_state` 又写入 `relation_models`。

---
