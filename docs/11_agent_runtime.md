# 11 Agent 运行时

本文档承载：

- 三层运行时（Simulation / Cognitive / Presentation）
- 融合调用 + 角色分级
- Active Set + Dirty Flags（脏标志触发规则）
- 主循环（Per Turn / Per Active Character / Per Turn Global）
- Agent Trace 记录点
- 调用预算
- 9 条验证规则 + 验证时机

数据契约见 [10_agent_data_and_simulation.md](10_agent_data_and_simulation.md)。LLM/程序边界铁律见 [01_architecture.md](01_architecture.md)。日志与 Trace 边界见 [30_logging_and_observability.md](30_logging_and_observability.md)。

---

## 1. 三层运行时

- **Simulation Core**（程序化优先）：场景维护、位置、可见性、身体状态、技能生成、仲裁。
- **Cognitive Layer**（按需调用模型）：主观解释、偏见感知、信念变化、意图生成。
- **Presentation Layer**（输出时调用）：对话、动作叙述、风格化渲染。

## 2. 融合调用

`PerceptionDistributor + BeliefUpdater + IntentAgent` 融合为单次模型调用 `CharacterCognitivePass`，大幅降低成本。

## 3. 角色分级

- **Tier A**（主角 / 重要 NPC）：完整 CognitivePass。
- **Tier B**（次要 NPC）：简化规则，按需升级。
- **Tier C**（背景角色）：纯程序化策略。

---

## 4. Active Set + Dirty Flags

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

---

## 5. 主循环

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
    c. 按 NarrationScope 输出 ArbitrationResult（含 visible_facts 白名单）
14. SurfaceRealizer(LLM) ← {NarrationScope, SceneNarrativeView, CharacterCognitivePassOutput[], ArbitrationResult, StyleConstraints}
    → 自由文本叙事
15. NarrativeFactCheck：扫描叙事文本提及的事实是否 ⊆ 当前 NarrationScope 的 visible_facts
16. StateCommitter:
    - 更新 SceneModel (Layer 1)
    - 处理 KnowledgeRevealEvent（扩展 known_by + 生成对应 Memory）
    - 追加新 KnowledgeEntry { kind: Memory }
    - 应用 BodyReactionDelta 到 Layer 1 的 temporary_body_state
    - 写入 character_subjective_snapshots（Layer 3）
    - 写入 turn_traces / agent_step_traces（调试与回放）
```

---

### 5.1 Trace / Logs 写入点

运行时必须区分 Agent Trace 与运行 Logs：

- Agent Trace 写入 `world.sqlite`，记录回合内程序判断与模型节点产物。
- 运行 Logs 写入全局 `app_logs.sqlite` 或 Agent 世界内的 `llm_call_logs` / `app_event_logs`，记录 LLM 原始请求响应、异常与 Provider 运行状态。
- 两者通过 `scene_turn_id` / `trace_id` / `request_id` 关联，但不得作为后续业务判断或 LLM 输入。

主循环写入规则：

| 步骤 | Agent Trace | 运行 Logs |
|---|---|---|
| 1 | 记录原始用户输入摘要与回合起点 | 输入采集异常 |
| 2 | 记录 SceneStateExtractor 输出、解析状态、修复状态 | LLM request / response / schema / retry / error |
| 3 | 记录 UserInputDelta 应用摘要 | 状态应用异常 |
| 4-5 | 记录机械演化与事件 delta 摘要 | 状态演化异常 |
| 6 | 记录 Active Set、Dirty Flags、跳过原因 | - |
| 7-10 | 记录 Layer 2 派生摘要与 InputAssembly 结构检查 | 派生或类型检查异常 |
| 11 | 记录 CognitivePass 输出、schema 校验、修复结果 | LLM request / response / schema / retry / error |
| 12 | 记录每条 Validator 结果与失败项 | 验证异常事件 |
| 13a | 记录仲裁层 LLM 兜底输入输出与启用原因 | LLM request / response / error |
| 13b-13c | 记录物理仲裁摘要、资源消耗、命中、visible_facts | 仲裁异常 |
| 14 | 记录 SurfaceRealizer 输入摘要与最终叙事 | LLM request / response；stream chunk 与 readable_text |
| 15 | 记录 NarrativeFactCheck 结果 | fact check 失败事件 |
| 16 | 记录提交索引、rollback patch、trace_ids | SQLite 事务异常、回滚事件 |

---

## 6. CognitivePass 输出容错

CognitivePassOutput **必须为严格 schema JSON**，优先由 Provider structured output / tool schema 保证；JSON mode 仅作为降级路径，且必须在返回后通过 schema 校验。三层容错：

1. **第一层（程序）**：JSON 解析失败时尝试常见修复（缺逗号、未转义引号、缺失非必需字段补默认值、字段名拼写偏差）。
2. **第二层（仲裁层 LLM 兜底）**：程序修复失败时，将原始残缺输出 + 上下文交给仲裁层 LLM，由其推断该角色"实际想做什么"，输出可用的 IntentPlan 替代。
3. **最终降级**：仲裁层也失败时，该角色本回合 fallback 到 Tier B 模板策略（保持上回合意图或执行预设默认动作）。

---

## 7. 调用预算

- 每场景窗口：0-2 次 cognitive passes（重要活跃角色）。
- 0 次 cognitive passes（次要 / 背景角色）。
- 1 次 surface realization（仅当需要可见输出）。
- 1 次 SceneStateExtractor（每次用户输入）。
- 0-1 次 仲裁层兜底（仅在认知输出失败时）。

---

## 8. 验证规则

每条规则只读取已派生的 Layer 2 输入与 LLM 输出对，不修改任何状态。

### 8.1 必备规则

1. **Omniscience Leakage Rule** - CognitivePass 输出引用的所有 entity_id / knowledge_id 必须出现在该回合该角色的 `accessible_knowledge` 或 `filtered_scene_view.visible_entities` 中。
2. **Embodiment Ignored Rule** - 感官失能时，输出不应描述对应感知（如失明却看见）。
3. **Self Awareness Rule** - 当某 `KnowledgeEntry` 的 `subject_awareness == Unaware{self_belief}` 且 subject 是当前角色时：该角色的认知输出**只能**引用 `self_belief`，不可引用 `content` 中独有的事实。
4. **God Only Rule** - `visibility.scope` 含 `GodOnly` 的 KnowledgeEntry 在任何角色输出中均不应出现；`GodOnly` 启用态下 `known_by` 必须为空，故事揭示时必须先通过 `KnowledgeRevealEvent` 解除 `GodOnly` 再追加知情者。
5. **Mana Sense Rule** - 凡人（低灵觉敏锐度）不应清晰感知修士气息。
6. **Consistency Rule** - 跨回合连续性（受伤、关系、目标不应无缘由跳变）。
7. **Apparent vs True Rule** - 当观察者通过 `apparent_content` 看到某 facet 时，输出引用该信息应与 `apparent_content` 一致；引用 `content` 独有信息视为泄露。
8. **Narrative Fact Check Rule** - SurfaceRealizer 输出的叙事文本中提及的具体事实必须 ⊆ `ArbitrationResult.visible_facts` 白名单；不可引入新事实（位置/伤势/状态变化等）。修辞描写不计。
9. **Schema Conformance Rule** - 所有 LLM 输出必须通过 schema 校验；失败触发容错路径（见 §6）。

### 8.2 验证时机

- **InputAssembly 之后、CognitivePass 之前**：扫描 prompt 不含 Layer 1 原始对象（结构性检查）。
- **CognitivePass 之后**：schema 校验（规则 9）+ 语义级泄露检测（规则 1-5、7）。
- **SurfaceRealizer 之后**：NarrativeFactCheck（规则 8）。
- **每回合结束**：跨回合一致性（规则 6）。
