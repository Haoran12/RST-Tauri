# 13 Agent LLM I/O 契约

本文档承载 Agent 模式所有 LLM 节点的结构化输入输出契约，以及影响调用预算的输入组装信号：

- PromptBuilder 与各 LLM 节点提示词契约
- CognitivePass I/O
- SceneInitializer I/O
- SceneStateExtractor I/O 与 UserInputDelta
- StyleConstraints 与 SurfaceRealizerInput
- OutcomePlanner I/O 与 ReactionWindow
- Dirty Flags

数据模型见 [10_agent_data_model.md](10_agent_data_model.md)。程序化派生、技能契约和硬规则解算见 [12_agent_simulation.md](12_agent_simulation.md)。运行时主循环见 [11_agent_runtime.md](11_agent_runtime.md)。

---

## 0. PromptBuilder 与提示词契约

Agent 模式发送给 LLM 的内容由 `PromptBuilder` 统一组装。提示词分为两层：

1. **静态节点提示词**：程序内置 / 版本化，说明该节点身份、权限、禁止事项、输出格式和失败时的表达方式。
2. **动态结构化输入**：本回合由运行时组装的 schema JSON，即本文后续各节的 `*Input`。

静态提示词属于调用控制面，不是世界状态、角色记忆或业务数据；不得写入 Layer 1 / Layer 3，也不得被 World 逻辑读取。运行 Logs 可以记录实际 request 用于调试，Agent Trace 只记录 `prompt_template_id`、`prompt_version`、`prompt_hash` 与输入摘要。

```rust
pub struct AgentPromptBundle<TInput> {
    pub prompt_template_id: String,
    pub prompt_version: String,
    pub llm_node: AgentLlmNode,

    /// 静态节点契约：身份、权限、禁止事项、输出要求。
    pub system_contract: String,

    /// 本次任务说明：通常只包含节点特定的短指令，不携带世界事实。
    pub task_instructions: Vec<String>,

    /// 动态输入；必须是对应节点的严格 schema JSON。
    pub input: TInput,

    /// structured 节点必填；SurfaceRealizer 可为空。
    pub output_schema_id: Option<String>,
    pub output_schema_json: Option<serde_json::Value>,

    /// 供日志、回放和提示词迁移定位。
    pub prompt_hash: String,
}
```

发送给 Provider 的消息布局固定为：

| 消息 | 内容 |
|---|---|
| system | `system_contract`：节点身份、权限、通用硬规则 |
| developer / system追加 | `task_instructions`：本次任务说明；若 Provider 无 developer role，则追加到 system |
| user | 单个 JSON 对象：`{ "input": <TInput> }`；不得混入未结构化的额外说明 |

`chat_structured` 节点必须同时传入 JSON Schema；不依赖 prompt 中“请输出 JSON”作为唯一约束。JSON mode 降级时可把 schema 摘要追加到 `system_contract`，但返回后仍必须 schema 校验。

### 0.1 通用静态规则

所有 Agent LLM 节点的静态提示词必须包含这些规则：

1. 只使用本次 `input` 中显式提供的信息；不得引用日志、调试信息、训练记忆或未提供的世界事实。
2. 遵守节点权限。受限节点不得假设隐藏真相；God-read 节点可读取真相但不得直接提交状态。
3. 不改写、伪造或猜测 ID；未知实体、地点、技能、知识必须通过 schema 中的 ambiguity / warning / blocked 表达。
4. 结构化节点只输出符合 schema 的 JSON；不得输出 Markdown、解释性前后缀或额外字段。
5. `llm_readable` 文本只用于解释、叙述或 trace；不得把它当作程序规则、检索键或数值依据。
6. 若信息不足，选择保守输出：保留不确定性、降低置信度、给出候选/歧义，而不是补写新事实。
7. 受限角色节点可以误判、偏见和不完整，但误判必须来自其可观察输入 / 可访问 Knowledge 与 prior L3，不能来自隐藏真相。
8. 不从 raw 物理量或 raw 灵力值自行推导后果；只能使用程序提供的 tier / effect_hints / constraints。
9. 只有 `SceneInitializer` 可按 `generation_policy` 对缺省场景细节做受控补全；补全必须落在允许域内，并写入来源、置信度与假设说明。

### 0.2 SceneInitializer 提示词契约

节点身份：场景初始化器。任务是在新建场景、切场景或大幅跳时后，根据结构化场景种子、公开世界约束、时间、场所和相关人物，生成候选 `SceneInitializationDraft`。

静态提示词必须强调：

- 只在 `generation_policy.allowed_detail_domains` 中补齐细节，例如空间布局、光照、声场、气味、天气、地表、环境灵气、场景基调和临时背景实体。
- 不读取隐藏角色 Knowledge / GodOnly；第一版只使用公开世界设定、公开地点设定、当前参与者公开外显信息和程序提供的约束。
- 不创造新的命名重要人物、势力秘密、隐藏机关、关键道具、历史真相或剧情硬事实；若需要这些内容，写入 `blocked_additions` 或 `ambiguity_report`。
- 可创建短生命周期、无持久身份的背景实体，但必须受 `max_generated_background_entities` 和 `allow_transient_background_entities` 约束。
- 每个生成性补全都必须写入 `assumptions`，标明来源类型、置信度和影响字段；程序可据此审计、回滚或要求用户确认。
- 输出只是候选草案，不写数据库；最终 SceneModel 必须通过 SceneInitializerValidator / ConsistencyRule 后才可提交。
- 场景细节应与时间、季节、昼夜、地点类型、天气、人物状态和世界物理 / 灵力规则自洽；不确定时降低置信度，不强行定死。

### 0.3 SceneStateExtractor 提示词契约

节点身份：场景输入解析器。任务是把用户最近自由文本与当前 `SceneModel` 对齐，产出候选 `SceneUpdate` 与 `UserInputDelta`。

静态提示词必须强调：

- 用户自由文本可能是场景旁白、角色扮演、元指令或导演提示；必须按 `UserInputKind` 分类。
- 只输出候选 delta，不写数据库，不决定最终生效。
- 保留 `raw_text` 到 `UserInputDelta.raw_text`，但不要让 raw_text 参与后续业务判断。
- 不读取或发明隐藏 Knowledge / GodOnly；第一版默认只使用当前场景和公开约束。
- 涉及天气、地表、能见度、灵力环境等 Layer 1 物理子字段时，应保持结构自洽；不确定则写入 `ambiguity_report`。
- 不能把用户的文风要求直接塞进世界事实；文风只进入 `DirectorHint.style_override`。

### 0.4 CharacterCognitivePass 提示词契约

节点身份：单个角色的受限主观认知与意图生成器。任务是根据该角色本回合 L2 可观察世界、具身状态、可访问 Knowledge 和 prior L3，更新主观感知、信念倾向、情绪与意图。

静态提示词必须强调：

- 以 `character_id` 对应角色的作沉浸式主观视角推理；不要写旁白全知结论。
- 只能引用 `filtered_scene_view`、`embodiment_state`、`accessible_knowledge`、`recent_event_delta` 和 `prior_subjective_state` 中出现的事实。
- 角色可以误会、忽略、过度反应或保持旧偏见；这种偏差应体现在离散 `ConfidenceShift`、情绪和 `intent_plan` 中。
- 信念变化用 schema 的离散级别表达，不输出任意浮点数。
- `BodyReactionDelta` 只是候选外显反应；不得声称已经改变 Layer 1 硬状态。
- 若输入与 prior L3 冲突，应在 `contradictions_and_tension` 中表达，不直接抹平旧信念。

Reaction pass 复用受限角色视角，但静态提示词必须额外强调：

- 只能从 `available_reaction_options` 中选择一个合法选项；不能新增反应者、技能或目标范围。
- `ReactionIntent` 只表达即时反应意图，不叙述结算结果，不打开新的普通反应窗口。
- 若所有选项都不符合角色动机或状态，应选择 schema 允许的默认防御 / 无反应选项；不得越权创造硬效果。

### 0.5 OutcomePlanner 提示词契约

节点身份：结果规划器。任务是综合 L1 真相、角色意图、反应窗口、技能契约和导演偏置，产出候选外显结果与候选状态更新。

静态提示词必须强调：

- 可以 God-read 输入中的 L1 / Knowledge / Skill，但输出仍只是候选，最终由 EffectValidator / StateCommitter 决定。
- 必须区分硬状态变化、软效果和被阻止效果。无法被技能契约或程序边界支持的内容进入 `blocked_effects` 或 `soft_effects`。
- 不直接写入角色内心相信、接受、记恨；这些应成为外显事件或下一轮 CognitivePass 输入。
- 反应窗口必须一次性结算；不得在结果规划中自由开启无限新窗口。
- `narratable_facts` 必须是 SurfaceRealizer 可叙述事实白名单，不能直接暴露 GodOnly 或超出 `NarrationScope` 的事实。
- 数值、资源、位置、伤势、冷却等硬效果必须能对应到输入中的技能契约、物理公式或已有状态。

### 0.6 SurfaceRealizer 提示词契约

节点身份：最终叙事渲染器。任务是把结构化结果转成给用户阅读的自由文本。

静态提示词必须强调：

- 只叙述 `SurfaceRealizerInput` 中提供的场景视图、角色输出、OutcomePlan、StyleConstraints。
- 不引入新事实；具体位置、伤势、资源、身份揭露、技能命中等必须来自 `outcome_plan.narratable_facts` 或已通过校验的结果。
- 严格遵守 `NarrationScope`：角色聚焦视角不能写该角色不可观察或不可访问的事实，客观镜头不能进入内心，DirectorView 默认仍不暴露 GodOnly。
- 可以润色节奏、气氛、动作和对话，但不能改变已发生事件的因果。
- `blocked_effects` 可以叙述为尝试失败、被抵消或未能奏效；不得把 blocked 的硬效果写成已经发生。
- 输出是面向用户的自由文本，不包含 JSON、调试字段或 schema 说明。

---

## 1. Cognitive Pass I/O

```rust
pub struct CharacterCognitivePassInput {
    pub character_id: String,
    pub scene_turn_id: String,

    // Layer 2（每回合派生）
    pub filtered_scene_view: FilteredSceneView,
    pub embodiment_state: EmbodimentState,
    pub accessible_knowledge: AccessibleKnowledge,    // 含世界/势力/他人 facet/历史 memory，全部经访问控制过滤

    // Layer 3（角色当前心智，作为先验）
    pub prior_subjective_state: CharacterSubjectiveState,

    // 本回合事件 delta（程序过滤后的角色可观察事件；不得使用 Layer 1 原始 SceneEvent）
    pub recent_event_delta: Vec<ObservableEventDelta>,
}

pub struct ObservableEventDelta {
    pub event_id: String,
    pub scene_turn_id: String,
    pub event_kind: String,                         // semantic
    pub involved_observable_entities: Vec<String>,     // semantic: entity_id list
    pub observable_effects: serde_json::Value,         // semantic: 结构化后果
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

## 2. SceneInitializer I/O

`SceneInitializer` 负责在当前没有可用 `SceneModel`、切换到新场所、时间大幅跳过、或程序判定当前场景锚点已失效时，生成一个可运行的候选场景草案。它不是普通叙事节点，也不替代 `SceneStateExtractor`：前者从结构化种子合成完整舞台，后者从用户自由文本提取本回合变化。

第一版权限采用保守规则：

- 可读：结构化 `SceneSeed`、公开世界 / 地区 / 场所摘要、相关人物公开外显状态、世界级 schema / 枚举 / 物理约束、程序生成的时间与天气趋势。
- 默认不可读：隐藏角色 Knowledge、GodOnly Knowledge、非当前场景的私密历史、未公开身份或秘密能力。
- 不可写：数据库和持久状态；只能输出候选 `SceneInitializationDraft`，由程序校验后提交为新的 `SceneModel` 或返回用户确认。

```rust
pub struct SceneInitializerInput {
    pub scene_turn_id: String,
    pub world_id: String,
    pub seed: SceneSeed,
    pub public_world_context: PublicWorldContext,
    pub location_context: LocationContext,
    pub participant_context: Vec<SceneParticipantSeed>,
    pub continuity_context: Option<SceneContinuityContext>,
    pub world_constraints: serde_json::Value,      // semantic: 枚举、schema、物理边界、世界级规则
    pub generation_policy: SceneGenerationPolicy,
}

pub struct SceneSeed {
    pub scene_id: String,
    pub transition_reason: SceneTransitionReason,  // initial_scene / location_change / time_skip / rollback_rebuild
    pub time_seed: TimeContextSeed,                // 季节、昼夜、相对时间、天气趋势锚点
    pub location_anchor: LocationAnchor,           // region_id / location_id / location_type
    pub required_participant_ids: Vec<String>,
    pub requested_mood: Option<SceneMood>,         // semantic enum
    pub required_entities: Vec<SceneEntitySeed>,   // 用户或程序明确要求出现的实体
}

pub enum SceneTransitionReason {
    InitialScene,
    LocationChange,
    TimeSkip,
    RollbackRebuild,
}

pub struct SceneEntitySeed {
    pub entity_id: Option<String>,                 // 已存在持久实体必须提供；背景实体可为空
    pub entity_kind: String,                       // character / prop / terrain_feature / background_actor ...
    pub display_label: Option<String>,             // llm_readable; 非持久背景实体不得当作检索键
    pub persistence: EntityPersistence,
    pub required: bool,
    pub position_hint: Option<String>,             // semantic direction / zone in implementation
}

pub enum EntityPersistence {
    Persistent,
    Transient,
    NonPersistent,
}

pub struct TimeContextSeed {
    pub season: Option<String>,                    // semantic enum in implementation
    pub day_phase: Option<String>,                 // dawn / day / dusk / night / deep_night
    pub absolute_time_hint: Option<String>,         // trace_only / llm_readable，不参与程序排序
    pub elapsed_from_previous: Option<String>,      // semantic duration in implementation
    pub weather_trend: Option<String>,             // clear / rainy / snow / storm / dry / unknown
}

pub struct LocationAnchor {
    pub region_id: Option<String>,
    pub location_id: Option<String>,
    pub location_type: String,                     // courtyard / forest_path / inn_room / sect_gate / cave ...
    pub known_exits: Vec<String>,                  // semantic ids or direction enums
}

pub struct PublicWorldContext {
    pub world_summary: String,                     // llm_readable: 公开世界观摘要
    pub public_rules: Vec<String>,                 // llm_readable: 公开物理 / 灵力 / 风俗约束
    pub ambient_defaults: serde_json::Value,        // semantic: 世界默认温度、灵气、昼夜规则等
}

pub struct LocationContext {
    pub public_location_summary: String,            // llm_readable
    pub terrain_tags: Vec<String>,                  // semantic
    pub climate_tags: Vec<String>,                  // semantic
    pub known_static_features: Vec<SceneEntitySeed>,
    pub forbidden_features: Vec<String>,            // semantic: 不可生成的特征类型
}

pub struct SceneParticipantSeed {
    pub character_id: String,
    pub public_appearance_summary: String,          // llm_readable: 仅公开外显
    pub entry_state: ParticipantEntryState,         // semantic
    pub position_hint: Option<String>,              // semantic direction / zone in implementation
}

pub enum ParticipantEntryState {
    AlreadyPresent,
    Entering,
    ArrivingWithGroup,
    OffstageExpected,
}

pub struct SceneContinuityContext {
    pub previous_scene_summary: String,             // llm_readable
    pub carried_entities: Vec<SceneEntitySeed>,
    pub unresolved_visible_events: Vec<String>,     // semantic event ids or public summaries
}

pub struct SceneGenerationPolicy {
    pub detail_level: DetailLevel,
    pub allowed_detail_domains: Vec<SceneDetailDomain>,
    pub allow_transient_background_entities: bool,
    pub max_generated_background_entities: u32,
    pub forbid_new_named_entities: bool,
    pub require_user_confirmation_above: AssumptionRisk,
}

pub enum SceneDetailDomain {
    SpatialLayout,
    Lighting,
    Acoustics,
    OlfactoryField,
    PhysicalConditions,
    ManaField,
    SceneMood,
    BackgroundEntities,
    ObservableSignals,
}

pub enum AssumptionRisk {
    Low,
    Medium,
    High,
}

pub struct SceneInitializationDraft {
    pub scene_turn_id: String,
    pub scene_model: SceneModel,
    pub assumptions: Vec<SceneAssumption>,
    pub blocked_additions: Vec<BlockedSceneAddition>,
    pub ambiguity_report: Vec<String>,             // llm_readable
    pub validation_hints: Vec<String>,              // trace_only / llm_readable
}

pub struct SceneAssumption {
    pub field_path: String,                         // semantic path，如 physical_conditions.wind
    pub source: SceneAssumptionSource,
    pub confidence: AssumptionConfidence,
    pub risk: AssumptionRisk,
    pub rationale: String,                          // llm_readable
}

pub enum SceneAssumptionSource {
    UserSeed,
    PublicWorldContext,
    LocationContext,
    ParticipantContext,
    ContinuityContext,
    ProgramDefault,
    LlmInferred,
}

pub enum AssumptionConfidence {
    Low,
    Medium,
    High,
}

pub struct BlockedSceneAddition {
    pub attempted_domain: SceneDetailDomain,
    pub reason_code: String,                        // semantic enum in implementation
    pub description: String,                        // llm_readable
}
```

校验要求：

- `SceneInitializationDraft.scene_model.scene_id` 必须等于 `seed.scene_id`，`scene_turn_id` 必须等于输入回合。
- `required_participant_ids` 必须全部出现在 `scene_model.entities` 中；不得新增命名重要角色，除非实体 ID 已由输入提供。
- `forbid_new_named_entities = true` 时，所有生成背景实体必须是 transient / unnamed / non_persistent。
- 任何不来自输入的字段必须有对应 `SceneAssumption`；`risk >= require_user_confirmation_above` 时，运行时不得自动提交。
- 物理子字段、灵力场和可观察信号必须通过与 `SceneStateExtractor` 相同的 ConsistencyRule。

## 3. SceneStateExtractor I/O 与 UserInputDelta

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

## 4. StyleConstraints（叙事层文风约束）

由作者预设或用户 DirectorHint 提供，最终交给 SurfaceRealizer LLM 阅读。

```rust
pub struct StyleConstraints {
    pub register: StyleRegister,           // ancient / modern / casual / formal / poetic
    pub detail_level: DetailLevel,         // sparse / moderate / rich
    pub atmosphere: Atmosphere,            // tense / serene / ominous / melancholic / ...
    pub pacing: Pacing,                    // fast / measured / slow
    pub pov: PointOfView,                  // omniscient / character_focused(id) / objective；不得覆盖 narration_scope 的叙事披露上限

    /// 自由文本字段：作者用自然语言书写的约束、参考文风、禁忌事项等。
    /// 仅供 LLM 阅读，不参与程序逻辑。
    pub explicit_guidelines: Vec<String>,
    pub reference_excerpts: Vec<String>,   // 参考片段（如"模仿《红楼梦》第三回的笔法"）
}
```

---

## 5. OutcomePlanner I/O（结果规划与状态更新计划）

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
    pub narratable_facts: Vec<String>,                   // semantic: 按 NarrationScope 派生的叙事事实白名单
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

### 5.1 ReactionWindow（有限反应窗口）

ReactionWindow 解决"被攻击者及其伙伴是否能即时反应"的问题，但它不是递归事件链。程序打开窗口后，只收集合格角色的 `ReactionIntent`，再由同一次 OutcomePlanner 调用把原行动与所有反应意图一起结算。

```rust
pub struct ReactionWindow {
    pub window_id: String,
    pub scene_turn_id: String,
    pub source_event_id: String,
    pub source_action_id: String,
    pub threat_source_id: String,
    pub primary_targets: Vec<String>,
    pub observable_threat: ObservableEventDelta,
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
5. 旁观者或伙伴能否反应取决于 `observable_threat` 对该角色是否可观察，以及其 `ReactionOption` 是否覆盖目标、距离、通道和资源；"站在场上"本身不构成反应资格。

硬约束：

- `OutcomePlanner` 可读 L1 / GodOnly 用于判断，但 `outcome_plan.narratable_facts` 必须按 `NarrationScope` 派生，不能把 GodOnly 直接给叙事层。
- `StateUpdatePlan` 中的数值、资源、位置、伤势、访问权限变更必须能被程序公式、技能契约或 Validator 校验；校验失败时不反复调用 LLM，非法硬效果进入 `blocked_effects` 或降级为 `soft_effects`，不得写入 L1。
- `BodyReactionDelta` 只作为候选身体反应；如需改变 `temporary_body_state`，必须由 OutcomePlanner/EffectValidator 转成合法 `CharacterBodyDelta` 后经 StateCommitter 提交。
- 角色是否"相信 / 接受 / 记恨"不由 OutcomePlanner 直接写入 L3，除非它来自该角色本回合 `CharacterCognitivePassOutput`；否则作为外显事件进入下一轮认知输入。
- OutcomePlanner 必须把 `reaction_windows` 中的原行动与 `reaction_intents` 一起结算；禁止在结算中再自由打开无限新窗口。

---

## 6. SurfaceRealizerInput（叙事层输入）

叙事层 LLM 仅接受以下四类结构化输入，**不再读取角色档案 / 世界设定 / 角色心智**（它们已体现在情景与认知结果中）。

```rust
pub struct SurfaceRealizerInput {
    pub scene_turn_id: String,

    /// 0. 叙事披露边界：决定 SceneNarrativeView 与 narratable_facts 的生成范围。
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

`SceneNarrativeView` 是 SceneModel 按 `narration_scope` 派生的叙事视图：`CharacterFocused` 只能使用该角色的 Layer 2 可观察事实与可访问 Knowledge；`ObjectiveCamera` 只能使用外显事实；`DirectorView` 可使用编排器可访问事实但默认仍剔除 `GodOnly`。
`OutcomePlan` 包含：每个角色的 outward_action（已发生的事）、resulting_state_changes（伤势/位置/资源等候选硬变化）、narratable_facts（按 `narration_scope` 生成的叙事事实白名单）、soft_effects 与 blocked_effects。SurfaceRealizer 可以叙述软效果和受阻结果，但 NarrativeFactCheck 与 StateCommitter 只承认已通过程序校验的硬变化。

---

## 7. Dirty Flags（调用预算控制）

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
    pub knowledge_revealed: bool,    // 本回合获得了新可访问 Knowledge
}
```

触发规则详见 [11_agent_runtime.md](11_agent_runtime.md)。

---

