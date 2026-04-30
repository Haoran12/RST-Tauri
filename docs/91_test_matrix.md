# 91 测试矩阵与验证方案

本文档承载按阶段组织的测试用例、验证方案和可执行质量门禁。

风险登记见 [90_pitfalls_and_tests.md](90_pitfalls_and_tests.md)。实现阶段见 [implementation_plan.md](implementation_plan.md)。

---

## 1. 测试用例 / 验证方案

### 阶段一：基础框架

- [ ] 应用启动 / 发送消息 / JSON 存储。

### 阶段二：SillyTavern 模式

- [ ] 导入 SillyTavern 角色卡 V3。
- [ ] 世界书词条完整触发（含正则 / 概率 / 时间）。
- [ ] 预设正确应用。
- [ ] 切换 API 配置只改变下一次请求的连接与 Provider 映射，不改变 active presets、自动预设选择结果、世界书来源合并、`chat_metadata.world_info`、`world_info.globalSelect`、角色卡世界书绑定或资源文件内容。
- [ ] Regex 脚本按 global -> preset -> scoped 顺序运行，且 `markdownOnly` / `promptOnly` 不写回聊天 JSON。
- [ ] 角色卡和预设内嵌 Regex 未获允许前不运行，允许状态按 avatar / RST `preset_key` 正确持久化；切换 API 配置不会改变同一预设的授权状态。
- [ ] Regex 导入导出、移动作用域、Regex Preset 应用后启用状态与顺序保持 ST 兼容。

### 阶段三-六：Agent 模式（参考 `D:\Projects\RST-flutter\docs\rp_agent_filtering_example.md`）

#### 感官与访问权限

- [ ] 失明角色 `observable_entities` 为空。
- [ ] 狐狸精能闻到细微血腥味，普通人闻不到。
- [ ] 凡人无法清晰感知修士气息。

#### Knowledge 访问权限体系

- [ ] **私密 Knowledge 仅 known_by 中的角色能访问。**
- [ ] **`knowledge_access_known_by` 只作为候选索引**；删除索引命中后的 Resolver 调用会导致测试失败。
- [ ] **`knowledge_access_scopes` + `character_scope_memberships` 能预筛 scope 候选**，但最终 accessible_knowledge 与 KnowledgeAccessResolver 全量扫描结果一致。
- [ ] **GodOnly 知识不出现在任何角色的 accessible_knowledge 中。**
- [ ] **GodOnly 即使存在于派生索引候选中，也会被 KnowledgeAccessResolver hard deny。**
- [ ] **GodOnly 启用态下 known_by 必须为空；若故事揭示，KnowledgeRevealEvent 必须先解除 GodOnly 再追加知情者。**
- [ ] **subject_awareness=Unaware 时，subject 自我描述只能引用 self_belief**（如被封印记忆的狐狸精仍自称人类）。
- [ ] **观察者通过 apparent_content 看到的伪装信息与 content 真相一致地分流**（伪装方与揭穿方分别得到不同 accessible_content）。
- [ ] **scope:faction:玄天宗 的 KnowledgeEntry 仅对该势力成员可访问。**
- [ ] **同场景观察可获得他人 Appearance facet，但获取不到 TrueName facet**（无关系阈值）。
- [ ] **KnowledgeRevealEvent 触发后**，被揭示者的下一回合输入包含新可访问 Knowledge。
- [ ] **GodOnly 揭示事件持久化 scope_change**，回滚后 scope 与 known_by 恢复到揭示前状态。
- [ ] **KnowledgeEntry.access_policy JSON 与派生索引全量重建结果一致**；不一致时报告存储一致性错误。
- [ ] **SocialAccessAtLeast 只读取 L1 客观关系/授权等级**，L3 `relation_models` 改变不会影响 Knowledge 访问权限。
- [ ] **CustomPredicate 访问条件只能使用结构化 AccessExpression AST，不接受自然语言表达式。**

#### 地点系统

- [ ] **每个 Agent World 有且只有一个 `WorldRoot` LocationNode。**
- [ ] **LocationNode.parent_id 不允许形成环。**
- [ ] **`type_label` 变化不会改变程序层级判断；层级判断只读 `canonical_level` 与 `parent_id`。**
- [ ] **NaturalRegion 可通过 LocationSpatialRelation 覆盖 / 穿过多个 MajorRegion / LocalRegion，而不需要多重 parent_id。**
- [ ] **NaturalRegion 的气候 / 地形事实进入 LocationContext.natural_region_facts，不进入 inherited_public_facts。**
- [ ] **行政 RegionFact 继承与自然地理影响同时存在时，LocationContext 保留两个来源，不自动互相覆盖。**
- [ ] **同名地点别名解析返回 ambiguity，直到上下文锚点或用户确认能确定唯一 `location_id`。**
- [ ] **`c县 -> b州 -> a国` 的父级链可由 LocationResolver 稳定返回。**
- [ ] **b州可继承 RegionFact 会出现在 c县 LocationContext 的 inherited_public_facts 中。**
- [ ] **c县本地 RegionFact 按 `child_overrides_parent` 覆盖 b州同类可继承事实。**
- [ ] **私密 / GodOnly RegionFact 即使可继承，也不会绕过 KnowledgeAccessResolver 进入角色输入。**
- [ ] **LocationEdge 多跳路线能按 travel_mode 估算距离、耗时、风险和置信度。**
- [ ] **无连通 LocationEdge 时，RoutePlanner 不返回硬距离；同父级只返回 low-confidence ProximityHint。**
- [ ] **单向 LocationEdge 不会被反向路线查询误用。**
- [ ] **回滚 Agent 回合后，新增 / 修改的 location_nodes、location_spatial_relations、location_edges、location_aliases 与地区事实恢复到父回合状态。**

#### 状态与运行时

- [ ] World 打开时加载 `WorldMainlineCursor`，`period_anchor < mainline_time_anchor` 的会话自动标记为 `RetrospectiveSession`。
- [ ] `world_turns.created_at` 不参与故事时间判断；今天提交的过去线回合仍按 `story_time_anchor` 参与 `WorldStateAt` 查询。
- [ ] 同一 World 可创建多份 `AgentSession`，每份会话有独立聊天顺序，但共享同一套 canonical Truth。
- [ ] 会话删除 / 归档不会删除 canonical `world_turns`、`knowledge_entries` 或主线光标。
- [ ] 过去线根据 `period_anchor + location + participants` 生成 `TruthGuidance`，并记录相关 `HistoricalEvent` 来源。
- [ ] `HistoricalEvent.summary_text` 不会被程序当作硬约束；只有结构化 `required_outcomes` / `forbidden_outcomes` / `known_after_effects` 参与冲突检测。
- [ ] 过去线不能让角色读取主线之后才揭示的 Knowledge、伤势、身份真相或记忆；这些只能作为 God-read 约束进入 TruthGuidance。
- [ ] 过去线补完的长期细节先写入 `provisional_session_truth`，不会在生成中途直接写 canonical Knowledge。
- [ ] 过去线产生硬冲突时，系统弹出警告但继续游玩，并写入 `ConflictReport`。
- [ ] 用户选择“冲突后非正史”时，冲突前已校验通过的 provisional truth 仍可提升，冲突回合及之后不得提升。
- [ ] 用户选择“整条会话非正史”时，该会话所有 provisional truth 都不得提升。
- [ ] 非正史会话继续游玩时不会改变 canonical Layer 1 / Layer 3 / Knowledge、`WorldMainlineCursor` 或后续正史判断。
- [ ] 非正史状态不会自动恢复；如需入正史，必须通过新的修正 / 重放流程重新校验。
- [ ] FuturePreview 默认只写会话、provisional truth 与 Trace，不写 canonical Truth。
- [ ] canonical 回滚前检查后续 facts、已提升 provisional truth、其他会话和主线光标依赖；存在依赖时阻止并生成影响报告。
- [ ] SceneStateExtractor 输入包含最近自由文本、当前结构化 Scene JSON 与场景相关私有约束，输出 `SceneStateExtractorOutput`，不直接提交 Layer 1。
- [ ] SceneStateExtractor 只能使用当前 SceneModel 与 `private_scene_constraints` 做场景域 God-read；读取或引用无关隐藏 Knowledge / 全局 GodOnly 时权限域检查失败。
- [ ] SceneInitializer 输入只包含结构化 SceneSeed、公开世界 / 地点 / 人物上下文、场景相关私有约束和 generation_policy，不包含用户原始自由文本。
- [ ] SceneInitializer 输出 `SceneInitializationDraft`，所有非输入来源字段都有 `SceneAssumption`，且风险达到阈值时不会自动提交。
- [ ] SceneInitializer 在 `forbid_new_named_entities=true` 时不能生成新的命名持久实体；背景实体只能是 transient / unnamed / non_persistent。
- [ ] SceneInitializer 可以使用 `private_scene_constraints` 初始化隐藏状态、保持连续性或设置揭示条件，但不能把私有约束写入公开可观察字段。
- [ ] SceneInitializer / SceneStateExtractor 的 Trace 记录 `private_scene_constraints` 来源、God-read 使用范围和权限域检查结果。
- [ ] 受伤状态跨回合保持。
- [ ] `character_records` 持久化包含 `base_attributes`、`mana_expression_tendency`、可选 `mana_expression_tendency_factor_override` 与 `temporary_state`，且不再出现独立 `mana_potency` 或旧 `temporary_body_state` 字段。
- [ ] `temporary_state` 存储在 Layer 1，并只能通过 `EmbodimentState` 派生进入 CognitivePass。
- [ ] CharacterCognitivePass 输入只含 L2 + prior L3，不含 Layer 1 原始 SceneEvent；本回合事件使用 `ObservableEventDelta`。
- [ ] PromptBuilder 为五类 Agent LLM 节点生成固定消息布局：system 静态契约、developer/task 指令、user 单个 `{ input }` JSON；Trace 记录 prompt_template_id/version/hash。
- [ ] 静态节点提示词不包含世界事实、角色秘密、Knowledge 内容或日志摘要；动态输入必须匹配对应 `*Input` schema。
- [ ] 多个 active + dirty 角色的 CognitivePass 可以并行执行；并行阶段不写 Layer 1 / Layer 3 / Knowledge。
- [ ] 并行 CognitivePass 输出按稳定顺序进入 Validator；LLM 返回顺序变化不改变最终 StateCommitter 提交结果。
- [ ] OutcomePlanner 可 God-read，但输出必须是 `OutcomePlannerOutput` / `StateUpdatePlan`，并在 StateCommitter 前通过 EffectValidator 硬约束校验。
- [ ] OutcomePlanner 每回合默认最多调用一次；候选效果校验失败时不会反复调用 LLM 修复。
- [ ] LLM 候选技能效果超出 SkillEffectContract 时，硬状态不提交，转入 `blocked_effects` 或 `soft_effects`。
- [ ] `BodyReactionDelta` 不直接写入 Layer 1；只有合法 `CharacterStateDelta` 能修改 `temporary_state`。
- [ ] A 攻击 B 时，B 与满足资格的伙伴/守护者会进入同一个 `ReactionWindow`，各自最多提交一个 `ReactionIntent`。
- [ ] B 的 `ReactionIntent` 为反击时，默认不会为 A 再打开普通 ReactionWindow；只有显式 interrupt 契约且未超过 `max_reaction_depth` 时才允许第二层。
- [ ] 不可观察威胁、距离/通道不满足、资源或冷却不足的角色不会出现在 `eligible_reactors`。
- [ ] Tier A/B 角色 `knowledge_revealed` 会触发 CognitivePass；离场角色只记录 pending knowledge，入场或被硬触发时消费。
- [ ] `CharacterFocused` 叙事只能引用该角色可观察或可访问事实；`ObjectiveCamera` 叙事不能进入任何角色内心；`DirectorView` 默认仍剔除 GodOnly。
- [ ] `SurfaceRealizerInput` 只接收 `NarrativeCharacterView[]`，不接收完整 `CharacterCognitivePassOutput[]`。
- [ ] `SurfaceRealizerOutput.used_fact_ids` 必须是 `OutcomePlan.narratable_facts.fact_id` 子集；文本抽查发现新增硬事实时判为 NarrativeFactCheck 失败。
- [ ] Dirty Flags 正确过滤无变化角色。
- [ ] Primary cognitive pass 控制在每场景 0-2 次；reaction pass 只对 eligible reactors 执行，并受每窗口/每角色/深度预算限制。
- [ ] active + dirty 角色超过 primary cognitive pass 预算时，运行时按威胁/反应/直接交互优先级稳定裁剪，未执行角色记录 `budget_deferred` 且 dirty flags 不丢失。
- [ ] SQLite 使用 WAL + 读连接池 + 单写提交；等待远程 LLM 时没有打开写事务。
- [ ] 应用启动 / 打开 World 时生成 `RuntimeConfigSnapshot` / `WorldRulesSnapshot`，同一回合内配置变更不影响正在执行的 Resolver。
- [ ] AttributeTier、AttributeDelta、CombatOutcomeTier 使用同一份 World 配置快照；修改 `world_base.yaml` 后从下一回合生效。
- [ ] 基础属性和 mana_power 存储 / 计算为 f64；普通 UI 取整展示不改变存储值，除非用户显式编辑保存。
- [ ] `ManaExpressionTendency` 三档持久化在角色档案中，按默认或人物级覆盖的 `tendency_factor` 参与 `display_ratio`，不表示当前场景正在压制或外放。
- [ ] `ManaExpressionMode` 五档运行时状态由配置限幅；LLM 只能请求封息/抑制/自然/外放/威压，不能输出 display_ratio / pressure_ratio。
- [ ] `display_ratio = min(2.0, max(0.0, 1.0 + tendency_factor + mode_factor))`，默认 factor 为 Inward=-0.5、Neutral=-0.2、Expressive=0.1、Sealed=-0.7、Suppressed=-0.3、Natural=0、Released=0.2、Dominating=0.4。
- [ ] 无额外隐匿/伪装/阵法修正时，`displayed_mana_power = effective_mana_power * display_ratio`；额外 flat/bonus/penalty 必须来自 L1 来源。
- [ ] `ManaExpressionState.intentionality` 能区分 Intentional / Unintentional / Forced，并能追溯 actor intent、情绪/伤势/突破、禁制/法器/他人压制等来源。
- [ ] 外放/威压只影响 `displayed_mana_power`、`ManaField.character_presences`、salience / reasoning modifiers，不改变 CombatMathResolver 使用的 `effective_mana_power`。
- [ ] `Sealed` / `Suppressed` 角色在同阶或高灵觉观察者面前能触发 `concealment_suspected`，但不会向 CognitivePass 暴露 raw effective 值。
- [ ] AttributeResolver 独立写入 Trace，记录属性修正来源、effective/displayed、expression tendency、runtime expression mode、intentionality、presence pressure、tier/delta 与异常修正。
- [ ] CognitivePassInput 不包含 raw `BaseAttributes`、`effective_attributes`、`effective_mana_power` 或 `displayed_mana_power`，只包含 tier / delta / expression_assessment / pressure_hints / descriptors / constraints。
- [ ] 非单调阈值、负数清理上限、未知配置字段会被 ConfigValidator 拒绝，并保留上一份有效快照。
- [ ] 五类 Agent LLM 节点可分别选择 `api_configs/` 中的不同配置；未配置节点继承默认 Agent 配置。
- [ ] `chat_structured` 节点绑定到不支持结构化输出的配置时，按文档降级或报错，不静默绕过 schema 校验。

#### 日志与 Trace

- [ ] ST 模式 LLM 调用只写全局 `./data/logs/app_logs.sqlite`。
- [ ] Agent 模式任意 `scene_turn_id` 能查到完整 `turn_traces` / `agent_step_traces`。
- [ ] Agent Trace 能通过 `request_id` 跳转到对应 LLM request / response。
- [ ] SceneInitializer / SceneStateExtractor / CognitivePass / OutcomePlanner / SurfaceRealizer 的 request、response、schema、状态、耗时都被记录。
- [ ] 每条 Agent LLM 调用日志都记录实际 `api_config_id`、provider、model。
- [ ] 流式输出保存原始 chunk 顺序，并生成 `assembled_text` / `readable_text`。
- [ ] API Key、Authorization header、Provider secret、代理认证不会进入 SQLite。
- [ ] API 适配改动覆盖一等目标：OpenAI Responses、OpenAI Chat Completions、Google Gemini、Anthropic、DeepSeek、Claude Code Interface；请求字段、流式解析、结构化输出、错误响应与日志脱敏均有对应行为。
- [ ] CognitivePass schema 失败、程序修复、OutcomePlanner 兜底都有 Trace 与异常事件。
- [ ] Agent 回滚后世界状态回退，运行 Logs 保留为审计记录。
- [ ] 全局 Logs 超过当前配置上限（默认 1GB）后后台清理旧运行日志。
- [ ] 修改 `app_runtime.yaml` 的日志清理上限后，下一次后台 retention 检查使用新上限；日志写入路径不读取配置文件。
- [ ] Trace、LLM Logs、异常事件记录 `config_snapshot_id`，能回查当时采用的阈值摘要。
- [ ] 普通清理任务不会删除 Agent Trace 或仍被 `state_commit_records.trace_ids` 引用的记录。
- [ ] 30 天未更新且日志较大的 World 只产生提示事件，不自动删除。

### 阶段七：用户角色扮演

- [ ] 用户能扮演特定角色并影响结果规划。
