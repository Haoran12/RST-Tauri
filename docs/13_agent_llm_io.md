# 13 Agent LLM I/O 契约

本文档承载 Agent 模式 LLM 节点的提示词契约、CognitivePass I/O、拆分后的节点 I/O 索引，以及影响调用预算的输入组装信号：

- PromptBuilder 与各 LLM 节点提示词契约
- CognitivePass I/O
- SceneInitializer / SceneStateExtractor I/O 索引
- StyleConstraints / OutcomePlanner / SurfaceRealizer I/O 索引
- Dirty Flags

数据模型见 [10_agent_data_model.md](10_agent_data_model.md)。地点层级、地区事实继承与路线图见 [15_agent_location_system.md](15_agent_location_system.md)。程序化派生见 [12_agent_simulation.md](12_agent_simulation.md)，技能契约和硬规则解算见 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)。场景节点 I/O 见 [21_agent_scene_llm_io.md](21_agent_scene_llm_io.md)，结果规划与叙事 I/O 见 [22_agent_outcome_narration_io.md](22_agent_outcome_narration_io.md)。运行时主循环见 [11_agent_runtime.md](11_agent_runtime.md)。

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
8. 不从 raw 物理量、raw 基础属性或 raw 灵力值自行推导后果；只能使用程序提供的 tier / delta / expression_assessment / pressure_hints / effect_hints / constraints。
9. 角色可以在行动意图中请求 `requested_mana_expression`（封息/抑制/自然/外放/威压），但不得输出 display_ratio、pressure_ratio、displayed_mana_power 或任何 raw 数值；持久的内敛/一般/外放倾向由角色档案提供，不能被单次意图改写。
10. 只有 `SceneInitializer` 可按 `generation_policy` 对缺省场景细节做受控补全；补全必须落在允许域内，并写入来源、置信度与假设说明。

### 0.2 SceneInitializer 提示词契约

节点身份：场景初始化器。任务是在新建场景、切场景或大幅跳时后，根据结构化场景种子、公开世界约束、场景相关私有约束、时间、场所和相关人物，生成候选 `SceneInitializationDraft`。

静态提示词必须强调：

- 只在 `generation_policy.allowed_detail_domains` 中补齐细节，例如空间布局、光照、声场、气味、天气、地表、环境灵气、场景基调和临时背景实体。
- 可以读取输入中 `private_scene_constraints` 给出的场景相关隐藏约束 / GodOnly 约束，但只能用于保持客观一致性；不得主动索取、推断或全库补全隐藏 Knowledge。
- 过去线可读取 `truth_guidance` 中的既有正史约束，用于让场景贴合已知人物、事件和结果；这些约束不得自动变成角色可知信息。
- 不把私有约束写成外显事实；若私有约束需要进入 `SceneModel`，必须落入 `ScenePrivateState.hidden_facts` 或 `ScenePrivateState.reveal_triggers`，并在 `assumptions` 中标明来源。
- 对 `truth_guidance.open_detail_slots` 的补完只能作为候选细节返回，不能声称已经写入 canonical Truth。
- 不创造新的命名重要人物、持久地点、路线边、势力秘密、隐藏机关、关键道具、历史真相或剧情硬事实；若输入中没有对应公开上下文或私有约束，写入 `blocked_additions` 或 `ambiguity_report`。
- 可创建短生命周期、无持久身份的背景实体，但必须受 `max_generated_background_entities` 和 `allow_transient_background_entities` 约束。
- 每个生成性补全都必须写入 `assumptions`，标明来源类型、置信度和影响字段；程序可据此审计、回滚或要求用户确认。
- 输出只是候选草案，不写数据库；最终 SceneModel 必须通过 SceneInitializerValidator / ConsistencyRule 后才可提交。
- 场景细节应与时间、季节、昼夜、地点类型、地点父级链、天气、人物状态和世界物理 / 灵力规则自洽；不确定时降低置信度，不强行定死。

### 0.3 SceneStateExtractor 提示词契约

节点身份：场景输入解析器。任务是把用户最近自由文本与当前 `SceneModel` 对齐，产出候选 `SceneUpdate` 与 `UserInputDelta`。

静态提示词必须强调：

- 用户自由文本可能是场景旁白、角色扮演、元指令或导演提示；必须按 `UserInputKind` 分类。
- 只输出候选 delta，不写数据库，不决定最终生效。
- 保留 `raw_text` 到 `UserInputDelta.raw_text`，但不要让 raw_text 参与后续业务判断。
- 场景域 God-read 只覆盖 `current_scene` 与 `private_scene_constraints`；可以用场景绑定的隐藏状态解释用户动作，但不得读取、发明或改写无关隐藏 Knowledge / 全局 GodOnly。
- 过去线可用 `truth_guidance` 判断用户输入是否触碰既有历史约束；发生硬冲突时输出 `ConflictWarning`，不得打断或拒绝解析用户输入。
- 不把隐藏状态或私有约束泄露到 `UserInputDelta.raw_text`、公开 delta 说明或后续叙事字段；若触发揭示，只输出结构化候选并交给 Validator / EffectValidator。
- 能补完正史开放细节的内容写入 `provisional_truth_candidates`；普通对白、氛围描写和无长期影响细节不应提升为候选 Truth。
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
- 过去线必须读取 `truth_guidance.hard_constraints` 仲裁结果；若用户行为与既有正史硬冲突，输出 `ConflictReport` 并继续给出可游玩的叙事后果。
- 必须区分硬状态变化、软效果和被阻止效果。无法被技能契约或程序边界支持的内容进入 `blocked_effects` 或 `soft_effects`。
- 冲突后的候选变化不得直接写入 canonical Truth；由会话正史策略决定保留为冲突后非正史或整条会话非正史。
- 不直接写入角色内心相信、接受、记恨；这些应成为外显事件或下一轮 CognitivePass 输入。
- 反应窗口必须一次性结算；不得在结果规划中自由开启无限新窗口。
- `narratable_facts` 必须是 SurfaceRealizer 可叙述事实的结构化白名单，不能直接暴露 GodOnly 或超出 `NarrationScope` 的事实。
- 数值、资源、位置、伤势、冷却等硬效果必须能对应到输入中的技能契约、物理公式或已有状态。

### 0.6 SurfaceRealizer 提示词契约

节点身份：最终叙事渲染器。任务是把结构化结果转成给用户阅读的叙事文本，并声明本次使用了哪些叙事事实。

静态提示词必须强调：

- 只叙述 `SurfaceRealizerInput` 中提供的场景视图、角色投影视图、OutcomePlan、StyleConstraints。
- 不引入新事实；具体位置、伤势、资源、身份揭露、技能命中等必须来自 `outcome_plan.narratable_facts` 或已通过校验的结果，并在 `used_fact_ids` 中列出。
- 严格遵守 `NarrationScope`：角色聚焦视角不能写该角色不可观察或不可访问的事实，客观镜头不能进入内心，DirectorView 默认仍不暴露 GodOnly。
- 可以润色节奏、气氛、动作和对话，但不能改变已发生事件的因果。
- `blocked_effects` 可以叙述为尝试失败、被抵消或未能奏效；不得把 blocked 的硬效果写成已经发生。
- 内部输出是严格 schema JSON：`SurfaceRealizerOutput { narrative_text, used_fact_ids }`；UI 只展示 `narrative_text`。

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

pub struct IntentPlan {
    pub character_id: String,
    pub intent_kind: String,                       // speak / move / attack / defend / investigate / cast_skill / wait ...
    pub target_refs: Vec<String>,                  // entity_id / location_id / knowledge_id / scene_private_fact_id
    pub intended_actions: Vec<CharacterAction>,
    pub priority: String,                          // low / normal / high / urgent
    pub commitment: String,                        // tentative / committed / desperate / coerced
    pub rationale: String,                         // llm_readable；不参与硬规则判断
}

pub struct CharacterAction {
    pub action_id: String,
    pub action_kind: String,                       // dialogue / gesture / movement / skill_use / item_use / observation
    pub target_refs: Vec<String>,
    pub spoken_text: Option<String>,
    pub skill_id: Option<String>,
    pub requested_mana_expression: Option<ManaExpressionMode>, // 只能请求运行时离散状态：封息/抑制/自然/外放/威压；倍率由程序派生
    pub declared_effect_refs: Vec<String>,         // semantic refs；必须能被 SkillEffectContract / Validator 解释
    pub outward_description: String,               // llm_readable；仅供叙事层候选
}

pub struct BodyReactionDelta {
    pub character_id: String,
    pub reaction_kind: String,                     // flinch / blush / tremble / freeze / breath_change ...
    pub intensity: String,                         // slight / clear / strong
    pub outward_signal: String,                    // llm_readable；不直接写 L1
    pub possible_state_effect: Option<String>, // semantic hint；若要写 temporary_state，必须转为 CharacterStateDelta
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

## 2. 场景节点 I/O

SceneInitializer、SceneStateExtractor 与 UserInputDelta 的结构化 I/O 契约已拆分到 [21_agent_scene_llm_io.md](21_agent_scene_llm_io.md)。

## 3. 结果规划与叙事 I/O

StyleConstraints、OutcomePlanner、ReactionWindow 与 SurfaceRealizerInput 的结构化 I/O 契约已拆分到 [22_agent_outcome_narration_io.md](22_agent_outcome_narration_io.md)。

## 4. Dirty Flags（调用预算控制）

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
