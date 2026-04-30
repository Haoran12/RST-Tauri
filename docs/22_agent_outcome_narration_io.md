# 22 Agent 结果规划与叙事 I/O

本文档承载 StyleConstraints、OutcomePlanner、ReactionWindow 与 SurfaceRealizerInput 的结构化 I/O 契约。

PromptBuilder 通用规则、CognitivePass I/O 与 Dirty Flags 见 [13_agent_llm_io.md](13_agent_llm_io.md)。对抗解算与技能契约见 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)。

---

## 1. StyleConstraints（叙事层文风约束）

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

## 2. OutcomePlanner I/O（结果规划与状态更新计划）

OutcomePlanner 是编排类 LLM 节点，可以拥有 God 读取权限。它负责把场景真相、人物情绪与言行意图、技能契约/设定约束综合成"实际可能发生什么"和"候选需要更新哪些数据"。但它不直接写入数据库；输出必须被 EffectValidator 按技能契约和程序硬边界裁剪后，再交给 StateCommitter。

```rust
pub struct OutcomePlannerInput {
    pub scene_turn_id: String,
    pub session_context: AgentSessionContext,
    pub truth_guidance: Option<TruthGuidance>,

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
    pub provisional_truth_candidates: Vec<ProvisionalTruthCandidate>,
}

pub struct OutcomePlannerOutput {
    pub outcome_plan: OutcomePlan,
    pub state_update_plan: StateUpdatePlan,
    pub knowledge_reveal_events: Vec<KnowledgeRevealEvent>,
    pub conflict_reports: Vec<ConflictReport>,
}

pub struct ConflictReport {
    pub conflict_id: String,
    pub session_id: String,
    pub scene_turn_id: String,
    pub severity: String,                      // soft / hard
    pub source_constraint_ids: Vec<String>,
    pub affected_candidate_ids: Vec<String>,
    pub suggested_policy_choices: Vec<String>, // noncanon_after_conflict / whole_session_noncanon
    pub summary: String,                       // llm_readable
}

pub struct OutcomePlan {
    pub outward_actions: Vec<OutwardAction>,          // semantic: 已发生/尝试发生的外显行动
    pub resulting_state_changes: serde_json::Value,   // semantic: 候选硬变化摘要，真实提交以 StateUpdatePlan 为准
    pub narratable_facts: Vec<NarratableFact>,        // 按 NarrationScope 派生的结构化叙事事实白名单
    pub soft_effects: Vec<SoftEffect>,                // llm_readable: 可叙述但不写 L1
    pub blocked_effects: Vec<BlockedEffect>,          // semantic + trace: 被程序边界阻止的效果
}

pub struct OutwardAction {
    pub action_id: String,
    pub actor_id: String,
    pub action_kind: String,              // dialogue / movement / attack / skill_use / reaction / failed_attempt ...
    pub target_refs: Vec<String>,
    pub narratable_fact_refs: Vec<String>,
    pub status: String,                   // occurred / attempted / blocked / softened
}

pub struct NarratableFact {
    pub fact_id: String,
    pub fact_kind: String,                  // action / injury / position / resource / reveal / blocked_effect / soft_effect ...
    pub subject_refs: Vec<String>,          // character_id / entity_id / location_id / knowledge_id
    pub source_refs: Vec<String>,           // event_id / action_id / state_delta_id / knowledge_id
    pub allowed_claim: String,              // llm_readable: SurfaceRealizer 可以表达的最小事实命题
    pub narration_scope: NarrationScope,
}

pub struct StateUpdatePlan {
    pub scene_delta: Option<SceneDelta>,
    pub character_state_deltas: Vec<CharacterStateDelta>,
    pub subjective_update_refs: Vec<String>,       // semantic: 对应角色 cognitive output / fallback output
    pub new_memory_entries: Vec<KnowledgeEntry>,   // kind 必须为 Memory
    pub soft_effects: Vec<SoftEffect>,             // llm_readable: 可叙述但不写入 L1 的软效果
    pub blocked_effects: Vec<BlockedEffect>,       // semantic + trace: 因超出契约/硬规则被阻止的效果
    pub validation_warnings: Vec<String>,          // trace_only: 程序裁剪或降级原因
    pub consistency_notes: Vec<String>,            // llm_readable / trace_only
}

pub struct CharacterStateDelta {
    pub character_id: String,
    pub temporary_state_delta: serde_json::Value,      // semantic: 伤势/疲惫/资源/冷却/心智压制/魂伤等结构化变更
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

### 2.1 ReactionWindow（有限反应窗口）

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

pub enum ReactionEligibilityReason {
    Target,
    AllyGuard,
    AreaProtector,
    PassiveField,
    InterruptSkill,
}

pub enum ReactionKind {
    Dodge,
    Block,
    Counter,
    ProtectAlly,
    Interrupt,
    PassiveMitigation,
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

- `OutcomePlanner` 可读 L1 / GodOnly 用于判断，但 `outcome_plan.narratable_facts` 必须按 `NarrationScope` 派生为结构化 `NarratableFact`，不能把 GodOnly 直接给叙事层。
- `StateUpdatePlan` 中的数值、资源、位置、伤势、访问权限变更必须能被程序公式、技能契约或 Validator 校验；校验失败时不反复调用 LLM，非法硬效果进入 `blocked_effects` 或降级为 `soft_effects`，不得写入 L1。
- `BodyReactionDelta` 只作为候选身体反应；如需改变 `temporary_state`，必须由 OutcomePlanner/EffectValidator 转成合法 `CharacterStateDelta` 后经 StateCommitter 提交。
- 角色是否"相信 / 接受 / 记恨"不由 OutcomePlanner 直接写入 L3，除非它来自该角色本回合 `CharacterCognitivePassOutput`；否则作为外显事件进入下一轮认知输入。
- OutcomePlanner 必须把 `reaction_windows` 中的原行动与 `reaction_intents` 一起结算；禁止在结算中再自由打开无限新窗口。

---

## 3. SurfaceRealizerInput（叙事层输入）

叙事层 LLM 仅接受以下四类结构化输入，**不再读取角色档案 / 世界设定 / 完整角色心智**（它们已体现在情景、结果计划与叙事投影视图中）。

```rust
pub struct SurfaceRealizerInput {
    pub scene_turn_id: String,

    /// 0. 叙事披露边界：决定 SceneNarrativeView 与 narratable_facts 的生成范围。
    pub narration_scope: NarrationScope,

    /// 1. 情景提取结果：本回合场景的客观状态视图（叙事层视角下的"舞台"）。
    pub scene_view: SceneNarrativeView,

    /// 2. 各角色 Agent 的叙事投影视图；不得包含完整 belief_update 或隐藏动机。
    pub character_views: Vec<NarrativeCharacterView>,

    /// 3. 结果计划：本回合的外显行动、合法硬后果摘要、软效果与受阻效果。
    pub outcome_plan: OutcomePlan,

    /// 文风约束（含自由文本指引）。
    pub style: StyleConstraints,
}

pub struct SurfaceRealizerOutput {
    pub narrative_text: String,
    pub used_fact_ids: Vec<String>,
}

pub struct SceneNarrativeView {
    pub scene_id: String,
    pub scene_turn_id: String,
    pub narration_scope: NarrationScope,
    pub visible_entities: Vec<NarrativeEntityView>,
    pub visible_environment: serde_json::Value,     // 已按 scope 裁剪的天气/光照/地形/灵力体感
    pub visible_events: Vec<NarrativeEventView>,
    pub allowed_private_refs: Vec<String>,          // 默认空；仅调试/作者私有视图可填
}

pub struct NarrativeEntityView {
    pub entity_id: String,
    pub display_name: String,
    pub observable_facts: Vec<String>,              // narratable fact refs
    pub outward_state: Vec<String>,                 // llm_readable，但须来自 narratable_facts
}

pub struct NarrativeCharacterView {
    pub character_id: String,
    pub display_name: String,
    pub outward_actions: Vec<String>,               // narratable fact refs
    pub outward_reactions: Vec<String>,             // narratable fact refs
    pub allowed_inner_summary: Option<String>,      // 仅 CharacterFocused / DirectorView 且 NarrationScope 允许时填充
}

pub struct NarrativeEventView {
    pub event_id: String,
    pub event_kind: String,
    pub narratable_fact_refs: Vec<String>,
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

`SceneNarrativeView` 是 SceneModel 按 `narration_scope` 派生的叙事视图：`CharacterFocused` 只能使用该角色的 Layer 2 可观察事实与可访问 Knowledge；`ObjectiveCamera` 只能使用外显事实；`DirectorView` 可使用编排器可访问事实但默认仍剔除 `GodOnly`。`NarrativeCharacterView` 由 `CharacterCognitivePassOutput` 和 OutcomePlan 投影而来，只保留可叙述行动、外显反应和允许的内心摘要。

`OutcomePlan` 包含：每个角色的 outward_action（已发生的事）、resulting_state_changes（伤势/位置/资源等候选硬变化）、结构化 `narratable_facts`、soft_effects 与 blocked_effects。SurfaceRealizer 内部返回 `SurfaceRealizerOutput { narrative_text, used_fact_ids }`；UI 只展示 `narrative_text`。NarrativeFactCheck 先校验 `used_fact_ids` 是 `narratable_facts.fact_id` 子集，再对叙事文本做保守抽查。SurfaceRealizer 可以叙述软效果和受阻结果，但 NarrativeFactCheck 与 StateCommitter 只承认已通过程序校验的硬变化。

---
