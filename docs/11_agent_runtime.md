# 11 Agent 运行时

本文档承载：

- 三层运行时（Simulation / Cognitive / Presentation）
- 融合调用 + 角色分级
- Active Set + Dirty Flags（脏标志触发规则）
- 主循环（Per Turn / Per Active Character / Per Turn Global）
- Agent Trace 记录点
- 调用预算
- 验证规则 + 验证时机

数据契约见 [10_agent_data_model.md](10_agent_data_model.md)。程序化派生见 [12_agent_simulation.md](12_agent_simulation.md)，对抗解算与技能契约见 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)。LLM 节点 I/O 契约入口见 [13_agent_llm_io.md](13_agent_llm_io.md)，场景节点见 [21_agent_scene_llm_io.md](21_agent_scene_llm_io.md)，结果规划与叙事节点见 [22_agent_outcome_narration_io.md](22_agent_outcome_narration_io.md)。LLM/程序边界铁律见 [01_architecture.md](01_architecture.md)。日志与 Trace 边界见 [30_logging_and_observability.md](30_logging_and_observability.md)。

---

## 1. 三层运行时

- **Simulation Core**（程序化优先）：场景维护、位置、Knowledge 访问权限、身体状态、技能生成、结果规划硬边界。
- **Cognitive Layer**（按需调用模型）：主观解释、偏见感知、信念变化、意图生成。
- **Presentation Layer**（输出时调用）：对话、动作叙述、风格化渲染。

## 2. Agent 模式 LLM 节点

Agent 模式包含五类 LLM 节点，权限不同：

1. **SceneInitializer（场景初始化器）**：输入结构化 SceneSeed + 公开世界 / 地点 / 人物上下文 + 场景相关私有约束 + 生成策略；输出候选 `SceneInitializationDraft`。它用于新建场景、切场景和大幅跳时，可读取程序裁剪后的当前场景相关隐藏约束 / GodOnly 约束以保持客观一致性，但不得全库读取隐藏 Knowledge 或把私有约束写成外显事实。
2. **SceneStateExtractor（场景提取器）**：输入最近一轮自由文本 + 当前结构化 Scene JSON + 场景相关私有约束；输出候选 `SceneUpdate` 与 `UserInputDelta`。它是场景域 God-read，可读当前场景真相和场景绑定的隐藏状态，但默认不读非当前场景私密历史、未关联本场景的隐藏角色 Knowledge 或全局 GodOnly。
3. **CharacterCognitivePass（人物认知与意图生成器）**：输入该角色可接触的 L2 场景、具身感官、可访问设定与 prior L3；输出结构化心理活动、情绪、言行意图。它是强访问限制节点。
4. **OutcomePlanner（结果规划器）**：输入 L1 场景真相、人物情绪与意图、技能契约/设定、结构化 DirectorHint；输出实际言行、交互/对抗结果、状态更新计划候选。它可以 God-read，但不能直接提交状态。
5. **SurfaceRealizer（叙事文本输出）**：输入 NarrationScope 限制后的场景、人物叙事投影视图、实际言行、交互结果、文风/格式要求；内部输出 `SurfaceRealizerOutput { narrative_text, used_fact_ids }`，UI 只展示自由文本叙事。它是强叙事披露限制节点。

`God-read` 只用于编排判断；SceneInitializer / SceneStateExtractor 的 God-read 范围限于本场景输入域，OutcomePlanner 可读取结果规划所需的更宽 L1 / Knowledge / Skill，但都必须记录使用范围。所有 LLM 输出必须先通过 schema + 业务校验，再由程序提交。

每类节点可绑定不同 API 配置。运行时在调用节点前按 `AgentLlmProfile` 解析实际 `api_config_id`：节点绑定优先，其次 World 覆盖，最后全局默认 Agent 配置。API 配置只影响 Provider / model / 参数，不改变节点权限。

运行时同时固定一份配置快照。打开 World 时，`ConfigLoader` 合并全局 `app_runtime.yaml` 与该 World 的 `world_base.yaml`，校验后发布 `RuntimeConfigSnapshot` / `WorldRulesSnapshot`。每个 Agent 回合开始时捕获 `config_snapshot_id`；本回合所有 `SceneFilter`、`EmbodimentResolver`、`CombatMathResolver`、PromptBuilder 预算判断和日志 retention 判断只读这份快照。用户在回合执行中保存配置时，新配置只能从下一回合生效。

## 3. 融合调用

`PerceptionDistributor + BeliefUpdater + IntentAgent` 融合为单次模型调用 `CharacterCognitivePass`，大幅降低成本。

## 4. 角色分级

- **Tier A**（主角 / 重要 NPC）：完整 CognitivePass。
- **Tier B**（次要 NPC）：简化规则，按需升级。
- **Tier C**（背景角色）：纯程序化策略。

---

## 5. Active Set + Dirty Flags

仅对**活跃且脏**的角色执行认知传递。

**触发 cognitive pass 的硬条件（程序可判定）**——以下条件任一为真即触发：
- `directly_addressed`：被对话方直接称呼/提问。
- `under_threat`：被攻击或处于即时威胁。
- `reaction_window_open`：技能/事件开放了反应窗口。
- `scene_changed`：所在场景的可观察状态发生显著变化。
- `body_changed`：自身身体状态发生显著变化。
- `knowledge_revealed`：Tier A/B 角色本回合获得新可访问 Knowledge。

**主观显著性标志**（不作为触发条件，仅作为 prompt hint 提示 LLM "你刚听到/看到 X"）：
- `received_new_salient_signal`、`belief_invalidated`、`relation_changed`、`intent_invalidated`。

离开当前活跃场景的角色若收到 KnowledgeRevealEvent，不立即调用 CognitivePass；运行时只记录 pending knowledge，等该角色进入活跃场景或被其他硬条件触发时消费。

跳过用户当前扮演的角色（其行为由 UserInputDelta 直接给出）。

当 active + dirty 角色数超过本场景 primary cognitive pass 预算（默认 2，来自 `RuntimeConfigSnapshot.request_budget`）时，运行时按确定性优先级裁剪：

1. `under_threat` / `reaction_window_open` / `directly_addressed` 的 Tier A 角色优先。
2. 与用户扮演角色直接交互、或其 intent 会打开 ReactionWindow 的角色优先。
3. `knowledge_revealed`、`scene_changed`、`body_changed` 按 salience 分数排序；同分时按 `character_id` 稳定排序。
4. 超出预算的 Tier A/B 角色本回合降级为 Tier B 模板策略或沿用上一可用 IntentPlan，并在 Trace 中记录 `budget_deferred`；其 dirty flags 不丢弃，下一回合仍满足硬条件时继续参与调度。
5. Reaction pass 不占 primary cognitive pass 预算，但受独立的 reaction window 深度、每窗口人数和 Provider 限流预算约束。

---

### 5.1 有界反应窗口

反应窗口用于处理"角色 A 的言行或攻击会被 B、B 的伙伴、守护者、被波及者即时感知并尝试反应"。它是单回合内的有限收集阶段，不是事件递归。

程序在两类时机打开 `ReactionWindow`：

1. 用户扮演或 SceneStateExtractor 解析出的外显行动已经构成即时威胁、直接称呼、区域波及或技能契约声明的可反应事件。
2. NPC 的 `CharacterCognitivePassOutput.intent_plan` 表示将要发动攻击、打断、强制位移、揭示、控制等可能被反应的行动。

窗口打开后，程序只做资格判定与选项派发：

- primary target、盟友/守护者、领域/被动技能拥有者、被波及者都可以成为候选反应者，但必须能通过本回合 `FilteredSceneView` / `ObservableEventDelta` 感知到威胁。
- 候选反应者必须满足距离、视线/通道、感官、冷却、资源、姿态、控制状态与 SkillEffectContract。
- 对候选者额外执行一次受限 reaction pass，输入只包含该窗口、合法 `ReactionOption`、该角色 L2 视图与 prior L3，输出 `ReactionIntent`。
- `ReactionIntent` 只表示"选择怎么反应"，不立即结算，也不生成新的普通 ReactionWindow。

递归上限：

- 默认 `no_reaction_to_reaction = true`，B 的反击不会再为 A 打开普通反应窗口。
- 默认 `one_reaction_per_character_per_window = true`。
- 默认 `max_reaction_depth = 1`；只有技能契约明确 `allows_interrupt` 且 `max_reaction_depth_override` 允许时，可进入深度 2。深度耗尽后，后续效果只能作为同一 OutcomePlanner 结算项处理。

OutcomePlanner 每回合仍默认最多 1 次：它接收原行动、所有 `ReactionWindow`、所有 `ReactionIntent`，一次性输出最终 OutcomePlan / StateUpdatePlan。

---

## 6. 主循环

```
== Per Turn ==

0. 加载 AgentSession 与 WorldMainlineCursor：
   - 比较 session.period_anchor 与 mainline_time_anchor，判定 Mainline / Retrospective / FuturePreview
   - RetrospectiveSession 调用 HistoricalTruthResolver，生成 TruthGuidance
   - canon_status == noncanon 时，本回合只能写会话记录、Trace 与 provisional truth，不能写 canonical Truth
1. 收集用户输入（自由文本）
1a. 若当前没有可用 SceneModel，或上一回合 MetaCommand / 程序事件要求切场景、大幅跳时：
   - 程序组装 SceneSeed、AgentSessionContext、公开世界 / 地点 / 人物上下文、场景相关私有约束、TruthGuidance、SceneGenerationPolicy
   - SceneInitializer(LLM) → SceneInitializationDraft（结构化）
   - SceneInitializerValidator + ConsistencyRule 校验；高风险假设需用户确认，否则提交为新的 SceneModel
2. SceneStateExtractor(LLM) ← {AgentSessionContext, 最近自由文本, 当前结构化 Scene JSON, 场景相关私有约束, TruthGuidance}
   → SceneStateExtractorOutput（结构化）
   - 输出 SceneUpdate 候选 + UserInputDelta
   - 过去线输出 provisional_truth_candidates 与 conflict_warnings
   - UserInputDelta 可为 SceneNarration / CharacterRoleplay / MetaCommand / DirectorHint
   - 解析失败 → 容错修复 → 仍失败则提示用户重写
3. Validator + StateApplier 将 SceneUpdate / UserInputDelta 应用到本回合 `TurnWorkingState`：
   - SceneUpdate / SceneNarration → 更新工作副本中的 SceneModel
   - CharacterRoleplay → 写入对应角色的 IntentPlan（跳过其 CognitivePass）
   - MetaCommand → 时间/场景控制
   - DirectorHint → 暂存结构化 outcome_bias 与 style_override
   - hard conflict warning → UI 弹出“冲突后非正史 / 整条会话非正史”，不中断本回合
4. 更新身体 / 资源 / 状态 / 冷却（Layer 1，机械演化）
4a. AttributeResolver 从 `base_attributes + temporary_state + scene/status/skill modifiers` 派生 `effective_attributes`：
   - `physical` / `agility` / `endurance` / `insight` / `mana_power` / `soul_strength` 存储和计算均为 f64，UI 展示取整不参与仲裁
   - `effective_mana_power` 用于实际施法、压制破绽与对抗解算
   - `CharacterRecord.mana_expression_tendency` 提供长期默认倾向（内敛/一般/外放），可用人物级 `tendency_factor` 覆盖，不表示当前场景动作
   - `ManaExpressionState` 将当前场景灵力状态归一为 `Sealed / Suppressed / Natural / Released / Dominating`，并记录 Intentional / Unintentional / Forced 来源；`display_ratio = clamp(1 + tendency_factor + mode_factor, 0, 2)`
   - `displayed_mana_power` 只用于感知层，受长期倾向、当前运行时状态、隐匿/压制、伪装/放大影响
   - `ManaField.character_presences` 由外放/威压姿态派生，用于环境灵压、salience 和 reasoning modifiers
   - raw base / effective / displayed 数值不得进入 CognitivePassInput
   - Agent Trace 独立记录修正来源、effective/displayed、expression tendency、runtime expression mode、intentionality、tier/delta 与异常修正摘要
5. 生成事件 delta
6. 计算活跃集 + 脏标志 + 初始 ReactionWindow（来自用户扮演/场景事件）

== Per Active & Dirty Character (跳过用户已扮演的角色) ==

7. EmbodimentResolver 读取 `effective_attributes` → embodiment_state（Layer 2）
8. SceneFilter 读取 `effective_attributes`、displayed/effective mana 差异与 `ManaField.character_presences`（含 observable_facets 计算）→ filtered_scene_view（Layer 2）
9. KnowledgeAccess → accessible_knowledge（Layer 2；SQLite 索引预筛候选后由 KnowledgeAccessResolver 最终过滤）
10. InputAssembly → CognitivePassInput（保证不含 Layer 1 原始对象）
11. CharacterCognitivePass(LLM) → 严格 schema JSON
    - 解析失败 → 程序容错（修复常见 JSON 错误）
    - 修复失败 → 标记进入 OutcomePlanner 兜底
12. Validator 扫描输入/输出对（OmniscienceLeakage / SelfAwareness / GodOnly / Embodiment / 一致性）
    - 验证失败 → 标记进入 OutcomePlanner 兜底

== Reaction Collection (bounded, optional) ==

12a. 程序从用户扮演 intent、场景事件、NPC IntentPlan 中打开 ReactionWindow
     - 只允许 active/dirty 场景内可感知威胁的角色进入候选
     - 援护者/伙伴/被动领域按 SkillEffectContract 与感知可达性 / Knowledge 访问权限判断资格
12b. 对每个 eligible reactor 组装 ReactionPassInput（L2 视图 + ReactionWindow + 合法 ReactionOption）
12c. CharacterCognitivePass 的 reaction 子任务输出 ReactionIntent
     - 每角色每窗口最多一个
     - reaction 不再打开普通 reaction；interrupt 例外必须受 max_reaction_depth 限制
12d. ReactionIntent 通过 Validator / EffectValidator 预检；非法选项丢弃或降级为默认防御/无反应，并写 trace

== Per Turn (Global) ==

13. OutcomePlanner（结果规划器，God-read，每回合默认最多 1 次）：
    a. 收集 AgentSessionContext、TruthGuidance、L1 场景真相、角色记录、相关 Knowledge / Skill、角色输出、用户扮演 intent、ReactionWindow / ReactionIntent、DirectorHint、provisional_truth_candidates
    b. 必要时处理 step 11/12 中标记失败的角色，推断可用 IntentPlan
    c. 输出 OutcomePlan + StateUpdatePlan + KnowledgeRevealEvent 候选 + ConflictReport
    d. EffectValidator 校验资源/位置/技能契约/数值/Knowledge 访问权限/GodOnly 规则；合法硬效果保留，非法或越界硬效果转入 blocked_effects 或 soft_effects，不反复调用 LLM 修复
    d2. TemporalConsistencyValidator 校验过去线候选是否违反 TruthGuidance；硬冲突只改变 canon_status，不阻止生成叙事
    e. 按 NarrationScope 派生结构化 narratable_facts 白名单
14. SurfaceRealizer(LLM) ← {NarrationScope, SceneNarrativeView, NarrativeCharacterView[], OutcomePlan, StyleConstraints}
    → SurfaceRealizerOutput { narrative_text, used_fact_ids }
15. NarrativeFactCheck：校验 used_fact_ids ⊆ 当前 NarrationScope 的 narratable_facts，并抽查叙事文本提及事实
16. StateCommitter（本回合唯一持久化写入点）:
    - canon_status 允许时，更新 canonical SceneModel (Layer 1)
    - canon_status 允许时，处理 KnowledgeRevealEvent（扩展 known_by + 生成对应 Memory）
    - canon_status 允许时，追加新 KnowledgeEntry { kind: Memory }
    - canon_status 允许时，只应用通过 EffectValidator / TemporalConsistencyValidator 的 StateUpdatePlan；BodyReactionDelta 不直接写 Layer 1
    - RetrospectiveSession 的新长期细节先写 provisional_session_truth；只有 canon_candidate 且校验通过时才提升为 canonical Knowledge / Event
    - noncanon 或冲突后非正史区间只写 SessionTurn、provisional truth、conflict_reports 与 Trace，不改 canonical Layer 1 / Layer 3 / Knowledge
    - 当前主线会话成功推进时，更新 WorldMainlineCursor
    - 写入 character_subjective_snapshots（Layer 3；非正史会话写会话域快照，不覆盖 canonical 当前心智）
    - 写入 turn_traces / agent_step_traces（调试与回放）
```

---

### 6.1 并行 CognitivePass 调度

`Per Active & Dirty Character` 阶段允许并行执行，以降低用户等待时间。并行的边界是**读固定快照 + 本回合工作副本、产出候选、不写持久状态**：

1. step 3-6 完成后，运行时固定本回合 `scene_turn_id`、原始 SceneModel、工作副本 SceneModel、角色记录、Knowledge 版本与 prior L3 快照。
2. 对每个 active + dirty 且非用户扮演的角色，可并行执行 EmbodimentResolver、SceneFilter、KnowledgeAccess、InputAssembly。
3. 多个 `CharacterCognitivePass` 可并行调用不同或相同 Provider；每个调用只读取自己的 `CharacterCognitivePassInput`，只产出 `CharacterCognitivePassOutput` 候选。
4. 并行阶段不得写入 Layer 1、Layer 3、Knowledge、Trace 决策结果或提交记录；LLM 调用日志可以通过异步日志队列或短事务写入。
5. 所有认知输出收集完毕后，按 `character_id` / `request_id` 稳定排序并统一进入 Validator；输出到达顺序不得影响最终结果。
6. 认知冲突、同时攻击、互相打断、社会后果统一交给同一次 OutcomePlanner 结算。
7. StateCommitter 是本回合唯一状态提交点，使用单个 SQLite 写事务写入 L1 / L3 / KnowledgeRevealEvent / Trace 索引。

`TurnWorkingState` 是内存中的回合工作副本，不是第二套持久状态。它只保存本回合已通过结构校验、尚未提交的候选 SceneModel、用户扮演 intent、DirectorHint、ReactionWindow 与待验证 plan。任何需要跨回合保留的变化都必须进入 `StateUpdatePlan` / `KnowledgeRevealEvent`，并由 `StateCommitter` 提交。

### 6.2 过去线正史资格与冲突处理

过去线运行时遵循“继续游玩，分离正史资格”的规则：

1. `HistoricalTruthResolver` 只读取结构化 `HistoricalEvent`、时态 Layer 1 事实和主线光标之前已成立的 canonical Truth，生成 `TruthGuidance`。
2. `TruthGuidance` 进入 SceneInitializer / SceneStateExtractor / OutcomePlanner / Validator 的 God-read 输入域，不进入 CharacterCognitivePass，也不会自动进入角色聚焦叙事。
3. 发现硬冲突时，运行时写入 `ConflictReport` 并提示用户选择：
   - `NonCanonAfterConflict`：冲突前已校验通过的 provisional truth 仍可提升；冲突回合及之后为非正史。
   - `WholeSessionNonCanon`：整条会话降为非正史，所有 provisional truth 不得提升。
4. 冲突不终止当前回合；OutcomePlanner 仍生成可游玩的叙事结果，但 StateCommitter 按会话正史状态决定是否写 canonical Truth。
5. 非正史状态不自动恢复。若用户希望让非正史内容进入正史，必须新开修正 / 重放流程，重新生成会话或候选事实并从头校验。
6. FuturePreview 第一版视为非正史预演，只能写会话和 Trace，不能更新主线光标或 canonical Truth。

SQLite 并发策略：

- 使用 WAL 模式，读连接池服务快照读取和 L2 派生，单写连接或写队列服务提交。
- 不在等待远程 LLM、流式响应或重试期间持有写事务。
- `KnowledgeEntry.access_policy` JSON 与访问派生索引表必须在同一写事务内更新。
- 不需要为并行人物 Agent 更换数据库；除非未来引入多进程远程协作或跨设备实时多人编辑，再重新评估服务端数据库。

### 6.3 Trace / Logs 写入点

运行时必须区分 Agent Trace 与运行 Logs：

- Agent Trace 写入 `world.sqlite`，记录回合内程序判断与模型节点产物。
- 运行 Logs 写入全局 `app_logs.sqlite` 或 Agent 世界内的 `llm_call_logs` / `app_event_logs`，记录 LLM 原始请求响应、异常与 Provider 运行状态。
- 两者通过 `scene_turn_id` / `trace_id` / `request_id` 关联，但不得作为后续业务判断或 LLM 输入。

主循环写入规则：

| 步骤 | Agent Trace | 运行 Logs |
|---|---|---|
| 0 | 记录 session_id、session_kind、period_anchor、mainline_time_anchor、canon_status、TruthGuidance 来源摘要 | 会话加载 / 主线光标读取异常 |
| 1 | 记录原始用户输入摘要、回合起点与 `config_snapshot_id` | 输入采集异常 |
| 1a | 记录 SceneInitializer 输入域、`private_scene_constraints` / `truth_guidance` 来源、生成策略、假设列表、阻止项、确认需求、api_config_id | LLM request / response / schema / retry / error |
| 2 | 记录 SceneStateExtractor 输入域、`private_scene_constraints` / `truth_guidance` 来源、输出、解析状态、修复状态、api_config_id、provisional truth 候选 | LLM request / response / schema / retry / error |
| 3 | 记录 UserInputDelta 应用摘要 | 状态应用异常 |
| 4 | 记录机械演化摘要 | 状态演化异常 |
| 4a | 记录 AttributeResolver 修正来源、effective/displayed 摘要、tier/delta 与异常修正 | 属性派生异常 |
| 5 | 记录事件 delta 摘要 | 事件生成异常 |
| 6 | 记录 Active Set、Dirty Flags、跳过原因 | - |
| 7-10 | 记录 Layer 2 派生摘要、KnowledgeAccess 候选索引命中、KnowledgeAccessResolver 裁剪摘要与 InputAssembly 结构检查 | 派生、索引漂移或类型检查异常 |
| 11 | 记录 CognitivePass 输出、schema 校验、修复结果、api_config_id | LLM request / response / schema / retry / error |
| 12 | 记录每条 Validator 结果与失败项 | 验证异常事件 |
| 13a-13c | 记录 OutcomePlanner 输入域、God-read 使用范围、TruthGuidance 使用范围、输出 plan、ConflictReport、兜底原因、api_config_id | LLM request / response / error |
| 13d-13e | 记录 EffectValidator / TemporalConsistencyValidator 裁剪结果、资源消耗、命中、结构化 narratable_facts、blocked_effects / soft_effects、canon_status 变化 | 结果规划异常 |
| 14 | 记录 SurfaceRealizer 输入摘要、api_config_id、used_fact_ids 与最终叙事 | LLM request / response；stream chunk 与 readable_text |
| 15 | 记录 NarrativeFactCheck 的 fact_id 子集校验与文本抽查结果 | fact check 失败事件 |
| 16 | 记录 canonical 提交索引、provisional truth、rollback patch、trace_ids、主线光标更新或非正史跳过原因 | SQLite 事务异常、回滚事件 |

所有 LLM 调用日志、异常事件和 Trace 条目都应尽量记录同一 `config_snapshot_id`。缺失该 ID 时仍允许写日志，但必须写 `config_snapshot_missing` 事件，避免后续无法判断某次运行使用了哪套阈值与清理策略。

---

## 7. CognitivePass 输出容错

CognitivePassOutput **必须为严格 schema JSON**，优先由 Provider structured output / tool schema 保证；JSON mode 仅作为降级路径，且必须在返回后通过 schema 校验。三层容错：

1. **第一层（程序）**：JSON 解析失败时尝试常见修复（缺逗号、未转义引号、缺失非必需字段补默认值、字段名拼写偏差）。
2. **第二层（OutcomePlanner 兜底）**：程序修复失败时，将原始残缺输出 + 该角色本回合 CognitivePassInput + 必要 L1 结果规划上下文交给 OutcomePlanner，由其在本回合唯一调用中推断该角色"实际想做什么"，输出可用的 IntentPlan 替代。兜底可 God-read，但必须在 trace 中记录使用范围。
3. **最终降级**：OutcomePlanner 也失败时，该角色本回合 fallback 到 Tier B 模板策略（保持上回合意图或执行预设默认动作）。

---

## 8. 调用预算

- 每场景窗口：0-2 次 primary cognitive passes（重要活跃角色）。
- Reaction pass 只对 `ReactionWindow.eligible_reactors` 执行；每角色每窗口最多 1 次，并受 `max_reaction_depth` 与场景级预算限制。
- 0 次 cognitive passes（次要 / 背景角色）。
- 1 次 surface realization（仅当需要叙事输出）。
- 0-1 次 SceneInitializer（仅新建场景、切场景、大幅跳时或回滚重建时）。
- 1 次 SceneStateExtractor（每次用户输入）。
- 0-1 次 OutcomePlanner（每回合需要状态推进时；若无交互可跳过；默认不因校验失败二次调用）。

---

## 9. 验证规则

每条规则只读取已派生的 Layer 2 输入与 LLM 输出对，不修改任何状态。

### 9.1 必备规则

1. **Omniscience Leakage Rule** - CognitivePass 输出引用的所有 entity_id / knowledge_id 必须出现在该回合该角色的 `accessible_knowledge` 或 `filtered_scene_view.observable_entities` 中。
2. **Embodiment Ignored Rule** - 感官失能时，输出不应描述对应感知（如失明却看见）。
3. **Self Awareness Rule** - 当某 `KnowledgeEntry` 的 `subject_awareness == Unaware{self_belief}` 且 subject 是当前角色时：该角色的认知输出**只能**引用 `self_belief`，不可引用 `content` 中独有的事实。
4. **God Only Rule** - `access_policy.scope` 含 `GodOnly` 的 KnowledgeEntry 在任何角色输出中均不应出现；`GodOnly` 启用态下 `known_by` 必须为空，故事揭示时必须先通过 `KnowledgeRevealEvent` 解除 `GodOnly` 再追加知情者。
5. **Mana Sense Rule** - 凡人（低灵觉敏锐度）不应清晰感知修士气息。
6. **Consistency Rule** - 跨回合连续性（受伤、关系、目标不应无缘由跳变）。
7. **Apparent vs True Rule** - 当观察者通过 `apparent_content` 看到某 facet 时，输出引用该信息应与 `apparent_content` 一致；引用 `content` 独有信息视为泄露。
8. **Narrative Fact Check Rule** - SurfaceRealizer 输出的 `used_fact_ids` 必须 ⊆ `OutcomePlan.narratable_facts.fact_id` 白名单；叙事文本中提及的具体事实还需通过保守抽查，不可引入新事实（位置/伤势/状态变化等）。修辞描写不计。
9. **Schema Conformance Rule** - 所有 LLM 输出必须通过 schema 校验；失败触发容错路径（见 §6）。
10. **Reaction Window Rule** - ReactionIntent 的 character_id 必须存在于对应 `ReactionWindow.eligible_reactors`，chosen_option_id 必须来自该角色的 `available_reaction_options`；不得违反 `one_reaction_per_character`、`no_reaction_to_reaction` 与 `max_reaction_depth`。
11. **Temporal Canon Rule** - 过去线候选事实必须与 `TruthGuidance.hard_constraints`、`WorldStateAt(period_anchor)` 和主线光标之后已成立的 canonical facts 相容；硬冲突生成 `ConflictReport`，并按用户选择更新会话正史状态，不打断游玩。

### 9.2 验证时机

- **SceneInitializer 之后**：schema 校验 + 生成策略域检查 + 假设风险检查 + 场景完整性 / 物理一致性检查 + 权限域检查（只能使用输入中的公开上下文与场景相关私有约束；私有约束不得外显化）。
- **SceneStateExtractor 之后**：schema 校验 + 场景 delta 合法性检查 + 权限域检查（只能使用当前 SceneModel 与输入中的场景相关私有约束；不得引用或改写无关隐藏 Knowledge / 全局 GodOnly）。
- **InputAssembly 之后、CognitivePass 之前**：扫描 prompt 不含 Layer 1 原始对象（结构性检查）。
- **CognitivePass 之后**：schema 校验（规则 9）+ 语义级泄露检测（规则 1-5、7）。
- **ReactionIntent 之后**：验证 eligible reactor / option / depth / resource preview（规则 10）；失败丢弃或降级为默认防御/无反应，并写入 trace。
- **OutcomePlanner 之后、StateCommitter 之前**：EffectValidator 校验 StateUpdatePlan 的资源/位置/伤势/技能契约/KnowledgeRevealEvent/GodOnly 规则，裁剪非法硬效果。
- **过去线 OutcomePlanner 之后、StateCommitter 之前**：TemporalConsistencyValidator 校验 provisional truth 与历史硬约束；硬冲突只改变正史资格，不阻止叙事输出。
- **SurfaceRealizer 之后**：NarrativeFactCheck（规则 8）。
- **每回合结束**：跨回合一致性（规则 6）。
