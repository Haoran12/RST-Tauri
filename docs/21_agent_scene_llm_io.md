# 21 Agent 场景 LLM I/O

本文档承载 SceneInitializer、SceneStateExtractor 与 UserInputDelta 的结构化 I/O 契约。

PromptBuilder 通用规则与 CognitivePass I/O 见 [13_agent_llm_io.md](13_agent_llm_io.md)。时间线引导见 [16_agent_timeline_and_canon.md](16_agent_timeline_and_canon.md)。

---

## 1. SceneInitializer I/O

`SceneInitializer` 负责在当前没有可用 `SceneModel`、切换到新场所、时间大幅跳过、或程序判定当前场景锚点已失效时，生成一个可运行的候选场景草案。它不是普通叙事节点，也不替代 `SceneStateExtractor`：前者从结构化种子合成完整舞台，后者从用户自由文本提取本回合变化。

权限采用“公开上下文 + 场景相关私有约束”的受限 God-read：

- 可读：结构化 `SceneSeed`、公开世界 / 地区 / 场所摘要、地点父级链、可访问地区事实、路线提示、相关人物公开外显状态、世界级 schema / 枚举 / 物理约束、程序生成的时间与天气趋势。
- 可读但受限：程序裁剪后的 `private_scene_constraints`，只包含当前场景相关隐藏约束、场景绑定 GodOnly 约束、连续性必须遵守的私有事实或揭示条件。
- 默认不可读：非当前场景的私密历史、未关联本场景的隐藏角色 Knowledge、全局 GodOnly、未公开身份或秘密能力的全量档案。
- 不可写：数据库和持久状态；只能输出候选 `SceneInitializationDraft`，由程序校验后提交为新的 `SceneModel` 或返回用户确认。

```rust
pub struct SceneInitializerInput {
    pub scene_turn_id: String,
    pub world_id: String,
    pub session_context: AgentSessionContext,
    pub seed: SceneSeed,
    pub public_world_context: PublicWorldContext,
    pub location_context: LocationContext,
    pub participant_context: Vec<SceneParticipantSeed>,
    pub continuity_context: Option<SceneContinuityContext>,
    pub private_scene_constraints: Vec<ScenePrivateConstraint>,
    pub truth_guidance: Option<TruthGuidance>,      // 过去线 Truth 引导；只用于一致性和开放细节补完
    pub world_constraints: serde_json::Value,      // semantic: 枚举、schema、物理边界、世界级规则
    pub generation_policy: SceneGenerationPolicy,
}

pub struct AgentSessionContext {
    pub session_id: String,
    pub session_kind: String,                       // mainline / retrospective / future_preview
    pub period_anchor: TimeAnchor,
    pub mainline_time_anchor: TimeAnchor,
    pub player_character_id: Option<String>,
    pub canon_status: String,                       // canon_candidate / partially_canon / noncanon
}

pub struct SceneSeed {
    pub scene_id: String,
    pub transition_reason: SceneTransitionReason,  // initial_scene / location_change / time_skip / rollback_rebuild
    pub time_seed: TimeContextSeed,                // 季节、昼夜、相对时间、天气趋势锚点
    pub location_anchor: LocationAnchor,           // location_id / fallback_region_id / location_type
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
    pub location_id: Option<String>,
    pub fallback_region_id: Option<String>,        // 兼容旧数据；若可解析，应优先使用 location_id
    pub location_type: String,                     // courtyard / forest_path / inn_room / sect_gate / cave ...
    pub known_exits: Vec<String>,                  // semantic ids or direction enums；不是路线图真相
}

pub struct PublicWorldContext {
    pub world_summary: String,                     // llm_readable: 公开世界观摘要
    pub public_rules: Vec<String>,                 // llm_readable: 公开物理 / 灵力 / 风俗约束
    pub ambient_defaults: serde_json::Value,        // semantic: 世界默认温度、灵气、昼夜规则等
}

pub struct LocationContext {
    pub anchor_location_id: Option<String>,
    pub resolved_name: Option<String>,
    pub public_location_summary: String,            // llm_readable
    pub ancestors: Vec<LocationBrief>,              // 从 WorldRoot/Realm 到当前地点的父级链摘要
    pub covering_natural_regions: Vec<LocationBrief>,
    pub sibling_or_nearby_locations: Vec<LocationBrief>,
    pub inherited_public_facts: Vec<AccessibleLocationFact>,
    pub local_public_facts: Vec<AccessibleLocationFact>,
    pub natural_region_facts: Vec<AccessibleLocationFact>,
    pub route_hints: Vec<RouteHint>,
    pub proximity_hints: Vec<ProximityHint>,        // 低置信度提示，不是硬事实
    pub terrain_tags: Vec<String>,                  // semantic
    pub climate_tags: Vec<String>,                  // semantic
    pub known_static_features: Vec<SceneEntitySeed>,
    pub forbidden_features: Vec<String>,            // semantic: 不可生成的特征类型
    pub ambiguity: Vec<LocationAmbiguity>,
}

pub struct LocationBrief {
    pub location_id: String,
    pub name: String,
    pub canonical_level: String,                    // WorldRoot / Realm / NaturalRegion / Polity / ...
    pub type_label: String,                         // 州 / 县 / 城 / 宗门 / 港口 ...
    pub tags: Vec<String>,
}

pub struct AccessibleLocationFact {
    pub knowledge_id: String,
    pub applies_to_location_id: String,
    pub fact_type: String,
    pub summary_text: String,                       // llm_readable
    pub inherited_from_location_id: Option<String>,
    pub confidence: String,
}

pub struct RouteHint {
    pub from_location_id: String,
    pub to_location_id: String,
    pub route_summary: String,                      // llm_readable
    pub travel_mode: String,
    pub distance_km: Option<String>,                // 区间或估算说明；不让 LLM 从 raw 数值推导硬后果
    pub travel_time: Option<String>,
    pub risk_tags: Vec<String>,
    pub confidence: String,
}

pub struct ProximityHint {
    pub location_id: String,
    pub relation: String,                           // same_parent_region / same_level / name_context ...
    pub confidence: String,                         // 通常为 low
    pub notes: String,                              // llm_readable
}

pub struct LocationAmbiguity {
    pub raw_name: String,
    pub candidate_location_ids: Vec<String>,
    pub reason: String,
}

pub struct ScenePrivateConstraint {
    pub constraint_id: String,
    pub source_knowledge_id: Option<String>,
    pub scope: PrivateConstraintScope,
    pub applies_to: Vec<String>,                    // scene_id / entity_id / location_id / participant_id
    pub constraint_kind: String,                    // hidden_presence / trap / disguise / sealed_area / continuity_secret ...
    pub constraint_summary: String,                 // llm_readable; 仅供编排一致性
    pub allowed_uses: Vec<PrivateConstraintUse>,    // initialize_hidden_state / validate_delta / reveal_if_triggered
    pub reveal_conditions: Vec<String>,             // semantic refs；不让 LLM 自行创造揭示条件
}

pub enum PrivateConstraintScope {
    SceneBound,
    LocationBound,
    ParticipantBound,
    ContinuityBound,
}

pub enum PrivateConstraintUse {
    InitializeHiddenState,
    PreserveContinuity,
    ValidateUserDelta,
    RevealIfTriggered,
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
    PrivateSceneConstraint,
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
- 来自 `private_scene_constraints` 的内容只能写入 `ScenePrivateState.hidden_facts` / `ScenePrivateState.reveal_triggers`；若被写入公开可观察字段，权限域检查失败。
- 来自 `truth_guidance.hard_constraints` 的内容只能作为场景一致性约束或隐藏连续性约束；不能让当时角色直接知道未来才揭示的事实。
- 过去线可补完 `truth_guidance.open_detail_slots` 中列出的细节，但不得把补完内容直接写入 canonical Truth；先生成候选，交给 StateCommitter / CanonPromotion 校验。
- 物理子字段、灵力场和可观察信号必须通过与 `SceneStateExtractor` 相同的 ConsistencyRule。

## 2. SceneStateExtractor I/O 与 UserInputDelta

用户的最近一轮自由文本输入由 SceneStateExtractor (LLM) 结合当前结构化场景信息解析。它不是普通角色认知节点，而是场景域编排节点：可读取当前 `SceneModel`，输出候选 `SceneUpdate` 与 `UserInputDelta`，但不直接写入 Layer 1。

用户发言是分级输入权，不是无条件写入权。SceneStateExtractor 必须把自由文本拆成“角色意图 / 主观心理 / 场景候选 / 导演偏置 / 元命令”，并为每个 delta 标注权限等级；越接近用户扮演角色的言行和本回合叙事，越可直接进入 `TurnWorkingState`，越接近正史、隐藏知识、硬数值、他人内心和持久世界结构，越必须降级为候选、假设、歧义或确认请求。

权限采用场景域 God-read：

- 可读：最近一轮自由文本、当前 `SceneModel` 全量、世界级 schema / 枚举 / 物理约束、与当前场景直接相关的公开设定。
- 可读但受限：输入中的 `private_scene_constraints`，用于判断用户动作是否触发、碰撞、揭示或违反当前场景绑定的隐藏事实。
- 默认不可读：非当前场景的私密历史、未关联本场景的隐藏角色 Knowledge、全局 GodOnly，除非后续明确引入"作者编辑 / 导演模式"。
- 不可写：数据库和持久状态；只能输出候选 delta，由程序校验后应用。

```rust
pub struct SceneStateExtractorInput {
    pub scene_turn_id: String,
    pub session_context: AgentSessionContext,
    pub recent_free_text: String,              // 用户最新输入或最近一轮聊天自由文本
    pub current_scene: SceneModel,             // 当前结构化场景 JSON
    pub private_scene_constraints: Vec<ScenePrivateConstraint>,
    pub truth_guidance: Option<TruthGuidance>, // 过去线冲突检测与开放细节槽提示；不进入角色输入
    pub world_constraints: serde_json::Value,  // semantic: 枚举、schema、物理边界、世界级规则
}

pub struct SceneStateExtractorOutput {
    pub scene_update: Option<SceneUpdate>,     // 候选场景更新
    pub user_input_delta: UserInputDelta,      // 结构化用户意图 / 扮演 / 元指令
    pub provisional_truth_candidates: Vec<ProvisionalTruthCandidate>,
    pub conflict_warnings: Vec<ConflictWarning>,
    pub ambiguity_report: Vec<String>,         // llm_readable: 需要用户澄清但不阻塞的歧义
}

pub struct ProvisionalTruthCandidate {
    pub candidate_id: String,
    pub source_session_id: String,
    pub source_scene_turn_id: String,
    pub story_time_anchor: TimeAnchor,
    pub derived_from_event_id: Option<String>,
    pub candidate_kind: String,                // knowledge_entry / event_detail / relation_detail / location_detail
    pub structured_payload: serde_json::Value,
    pub promotion_policy: String,              // promote_if_consistent / trace_only
}

pub struct ConflictWarning {
    pub warning_id: String,
    pub severity: String,                      // soft / hard
    pub source_constraint_ids: Vec<String>,
    pub conflicting_candidate_refs: Vec<String>,
    pub message: String,                       // llm_readable; 给 UI 展示
}

pub struct SceneUpdate {
    pub scene_turn_id: String,
    pub scene_delta: SceneDelta,
    pub update_reason: Vec<String>,            // trace_only / llm_readable
}

pub struct SceneDelta {
    pub scene_id: String,
    pub entity_deltas: Vec<SceneEntityDelta>,
    pub physical_delta: Option<PhysicalConditionsDelta>,
    pub mana_field_delta: Option<ManaFieldDelta>,
    pub observable_signal_deltas: Vec<ObservableSignalDelta>,
    pub private_state_deltas: Vec<ScenePrivateStateDelta>,
    pub event_appends: Vec<SceneEventDraft>,
}

pub struct SceneEntityDelta {
    pub entity_id: String,
    pub delta_kind: String,                  // enter / leave / move / posture_change / status_marker / transient_spawn
    pub payload: serde_json::Value,
}

pub struct PhysicalConditionsDelta {
    pub field_patches: serde_json::Value,    // semantic patch；Validator 检查物理字段自洽
}

pub struct ManaFieldDelta {
    pub field_patches: serde_json::Value,    // semantic patch；Validator 检查与 mana_sources / events 一致
}

pub struct ObservableSignalDelta {
    pub signal_id: String,
    pub delta_kind: String,
    pub payload: serde_json::Value,
}

pub struct ScenePrivateStateDelta {
    pub private_fact_id: String,
    pub delta_kind: String,                  // add_hidden_fact / update_hidden_fact / add_reveal_trigger / reveal / remove
    pub payload: serde_json::Value,
    pub source_constraint_id: Option<String>,
}

pub struct SceneEventDraft {
    pub event_kind: String,
    pub involved_entity_ids: Vec<String>,
    pub payload: serde_json::Value,
}
```

用户输入统一落为 `UserInputDelta`：

```rust
pub struct UserInputDelta {
    pub turn_id: String,
    pub raw_text: String,                          // 原始用户输入（仅用于 trace）
    pub authority_class: UserInputAuthorityClass,
    pub authority_notes: Vec<UserInputAuthorityNote>,
    pub kind: UserInputKind,
}

pub enum UserInputAuthorityClass {
    PlayerCharacterIntent,     // 用户扮演角色的外显意图；强输入，但不保证行动成功
    PlayerSubjectiveState,     // 用户扮演角色的主观心理；只影响该角色 L3
    SceneCandidate,            // 场景旁白候选；需要 SceneUpdate / Validator 仲裁
    DirectorBias,              // 导演偏置；只影响 OutcomePlanner / SurfaceRealizer 倾向
    SessionControl,            // 会话 / 时间 / 场景控制请求
    AmbiguousOrBlocked,        // 无法安全分类，或试图越权
}

pub struct UserInputAuthorityNote {
    pub note_kind: String,                         // applied / downgraded / needs_confirmation / blocked / ambiguous
    pub field_path: Option<String>,
    pub reason: String,                            // llm_readable; Trace/UI 可用，不参与程序硬判断
}

pub enum UserInputKind {
    /// 用户作为旁白 / 作者插入场景描述（如新增 entity / 改变光照）。
    /// 低风险、非持久、非冲突细节可作为候选自动进入工作副本；持久实体、隐藏机关、地理拓扑、天气 / 灵力硬状态等必须由 Validator 确认、降级或阻止。
    SceneNarration { scene_delta: SceneDelta },

    /// 用户扮演角色 X 的言行：直接写入 X 的 IntentPlan，跳过 X 的 CognitivePass。
    /// 这只表达 X 想说 / 想做什么；命中、伤害、资源消耗、位移、揭示等硬结果仍由 OutcomePlanner + EffectValidator 决定。
    CharacterRoleplay {
        character_id: String,
        intent_plan: IntentPlan,
        spoken_dialogue: Option<String>,
        actions: Vec<CharacterAction>,
        subjective_input: Option<PlayerSubjectiveInput>, // 用户扮演角色的心理活动 / 情绪 / 目标声明；只能影响该角色 L3，由 SubjectiveStateReducer 处理
    },

    /// 元指令：跳过时间 / 切场景 / 重置 / 暂停。
    /// 只生成结构化运行指令或 SceneSeed；不得直接提交 canonical Truth。
    MetaCommand { command: MetaCommandKind },

    /// 引导结果规划与文风（用户对当前回合的"导演"权）。
    /// outcome_bias 必须结构化；自由文本导演说明只能进入 style_override.explicit_guidelines 或 trace。
    /// 它不能强制 NPC 违背认知、技能、物理、正史或隐藏真相。
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

用户扮演输入分类规则：

- “我害怕 / 我怀疑他在撒谎 / 我决定撤退”等心理、情绪、目标声明进入 `CharacterRoleplay.subjective_input`。
- “我知道密室里有机关”这类客观隐藏事实断言，若该角色本回合 L2 无来源，只能作为 `PlayerBeliefSource::NewHypothesis` 的主观猜测，或拆为 `DirectorHint` / `SceneNarration` 候选等待 Validator；不得直接变成 Knowledge 访问权限或 L1 真相。
- 用户以导演身份声明的事实必须进入 `SceneNarration` / `DirectorHint`，并继续经过 SceneUpdate / OutcomePlanner / Validator；不得伪装成角色内心从而绕过访问控制。
- “这里有一把普通木椅”这类低风险、非持久场景细节可标为 `SceneCandidate`；“这里有上古神器 / 隐藏传送阵 / 某 NPC 一直在场”这类持久或高影响断言必须标为 needs_confirmation、provisional_truth_candidate、ambiguity 或 blocked。
- “让他相信我 / 让战斗轻一点 / 文风更压抑”只能成为角色目标、`DirectorHint.outcome_bias` 或 `style_override`；不得直接覆盖他人 L3 或保证硬结果。
- “忽略规则 / 直接设为正史 / 改掉隐藏设定”必须标为 `AmbiguousOrBlocked` 或 `MetaCommand` 请求；不得改变节点权限、KnowledgeAccessResolver、EffectValidator、TemporalConsistencyValidator 或 StateCommitter 边界。

---
