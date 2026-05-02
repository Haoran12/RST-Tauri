# 91 测试矩阵与验证方案

本文档承载按阶段组织的测试用例、验证方案和可执行质量门禁。

风险登记见 [90_pitfalls_and_tests.md](90_pitfalls_and_tests.md)。实现阶段见 [implementation_plan.md](implementation_plan.md)。

---

## 1. 测试用例 / 验证方案

### 阶段一：基础框架

- [ ] 应用启动 / 发送消息 / JSON 存储。
- [ ] 启动后默认进入 `/library`，资源工作台显示最近资源、最近会话、API 配置健康状态和 Agent World 摘要。
- [ ] App Shell 在桌面显示一级导航、上下文列表区和主工作区；窄屏下上下文列表区与右侧检查面板进入抽屉且不遮挡主工作区。
- [ ] 资源列表空状态、搜索无结果、数据目录不可读分别显示直接动作，不使用阻断式向导。
- [ ] 未保存 draft 在切换资源、离开页面或覆盖导入前提示保留 / 丢弃 / 取消，取消后 draft 不丢失。
- [ ] 破坏性操作显示 ImpactSummary；运行中被锁定的操作显示 disabled 原因和可行下一步。
- [ ] 亮色 / 暗色主题下状态徽标同时使用颜色以外的 icon、文本或 tooltip，窄屏动作栏文本不重叠。
- [ ] Structured Text Editor 支持 Plain / JSON / YAML 模式切换，切换失败不丢失原文本；JSON / YAML blocker 会阻止父级保存并定位到对应行列。
- [ ] Structured Text Editor 基于 CodeMirror 6 `EditorView` 封装；组件卸载、切换资源和切换模式不会泄漏 view/listener 或丢失父级 draft。
- [ ] Structured Text Editor 语言包注册表只允许 builtin / bundled / trusted plugin languageId；普通配置不能加载任意 JavaScript 语言包。
- [ ] `storageKind=json_value` 字段只能选择可稳定转换为 JSON-compatible value 的语言包；其他语言包只能用于 string 字段。
- [ ] Structured Text Editor 自动格式化 JSON / YAML 时使用稳定 2 spaces 缩进，Plain 不做结构缩进并保留文本语义。
- [ ] Structured Text Editor 的 JSON 模式在对象上下文输入未加引号 key 时自动修正为英文双引号；歧义 key 只给 quick fix。
- [ ] Structured Text Editor 的 YAML 模式在 `key:` 后换行自动缩进 2 spaces，list item 与 block scalar 换行缩进符合常见 YAML 写法。

### 阶段二：SillyTavern 模式

- [ ] 导入 SillyTavern 角色卡 V3。
- [ ] 世界书词条完整触发（含正则 / 概率 / 时间）。
- [ ] 预设正确应用。
- [ ] 切换 API 配置只改变下一次请求的连接与 Provider 映射，不改变 active presets、自动预设选择结果、世界书来源合并、`chat_metadata.world_info`、`world_info.globalSelect`、角色卡世界书绑定或资源文件内容。
- [ ] ST 世界书来源合并与去重使用 RST 内部稳定 `lore_id`，ST 文件名 / 显示名只影响导入导出和 UI 展示。
- [ ] Regex 脚本按 global -> preset -> scoped 顺序运行，且 `markdownOnly` / `promptOnly` 不写回聊天 JSON。
- [ ] 角色卡和预设内嵌 Regex 未获允许前不运行，允许状态按 avatar / RST `preset_key` 正确持久化；切换 API 配置不会改变同一预设的授权状态。
- [ ] Regex 导入导出、移动作用域、Regex Preset 应用后启用状态与顺序保持 ST 兼容。
- [ ] ST 世界书 / 预设 content 选择 JSON / YAML 时可作为 LLM 可读的结构化文本注入，但保存形态仍是 string，不改变 ST 兼容文件 schema。
- [ ] Regex `findRegex` 的 Plain 括号 / 引号诊断不替代 Regex 编译校验；非法正则由 Regex validator 拒绝。

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
- [ ] **SocialAccessAtLeast 读取 `objective_relationships` / `temporal_state_records`**，缺失 L1 客观授权时不会因 L3 好感提升而放行。
- [ ] **`objective_relationships` 只作为当前主线 materialized cache**；过去线、回滚复盘和按 `TimeAnchor` 查询的 `SocialAccessAtLeast` 读取 `temporal_state_records(state_kind=objective_relation|authorization)`。
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

#### Agent 世界编辑器

- [ ] **World Editor 提交必须 paused-only**：存在 active turn、pending / running LLM call、StateCommitter 写入或未处理回滚任务时拒绝提交。
- [ ] **WorldEditorPatch 使用 `base_editor_revision` 做乐观并发控制**；stale revision 提交失败，validation 可继续运行。
- [ ] **World Editor 保存 LocationGraph 时复用地点一致性校验**：WorldRoot 唯一、parent 无环、NaturalRegion 跨域必须用 LocationSpatialRelation。
- [ ] **World Editor 保存 KnowledgeEntry 时同步维护 `knowledge_access_known_by` 与 `knowledge_access_scopes`**，全量重建结果必须一致。
- [ ] **World Editor 拒绝 `GodOnly + known_by 非空` 的 KnowledgeEntry。**
- [ ] **World Editor 保存 CharacterRecord 时校验 `mind_model_card_knowledge_id` 指向同一角色的 MindModelCard KnowledgeEntry。**
- [ ] **World Editor 删除地点 / 角色 / Knowledge 前返回 blocking ImpactSummary**，覆盖引用、继承、历史事件、关系授权、TemporalState 和 MindModelCard 指针。
- [ ] **World Editor 提交在单个 SQLite 事务内写权威表、派生索引和 `world_editor_commits`**，任一环节失败时全部回滚。
- [ ] **World Editor 不写 `world_turns` 或 `state_commit_records`，也不伪造 `scene_turn_id`。**
- [ ] **World Editor 的 rollback_patch 能恢复权威表与派生索引。**
- [ ] **World Editor 表单展示取整不会改写未编辑的 f64 属性值。**
- [ ] **World Editor 可编辑 draft 并运行 validation，但 paused 状态不满足时提交按钮不可用。**
- [ ] **World Editor 的 KnowledgeEntry structured content 使用 JSON / YAML 解析为 `serde_json::Value` 后再进入业务 schema 校验；Plain 顶层提交返回 blocker。**
- [ ] **World Rules YAML 高级编辑保存前同时通过 Structured Text diagnostics 与 ConfigValidator。**

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
- [ ] 非正史 / FuturePreview 回合会写入非 canonical `world_turns` 以关联 SessionTurn、Trace 与 Logs，但不会写 `state_commit_records` 或 canonical Truth。
- [ ] `agent_sessions.canon_status`、`session_turns.canon_status` 与 `world_turns.runtime_turn_status` 使用不同 enum；只有 `runtime_turn_status in (canon, provisional_promoted)` 且存在 `state_commit_records` 才视为 canonical 提交。
- [ ] 非正史状态不会自动恢复；如需入正史，必须通过新的修正 / 重放流程重新校验。
- [ ] FuturePreview 默认只写会话、provisional truth 与 Trace，不写 canonical Truth。
- [ ] canonical 回滚前检查后续 facts、已提升 provisional truth、其他会话和主线光标依赖；存在依赖时阻止并生成影响报告。
- [ ] `WorldStateAt(period_anchor)` 从 `knowledge_entries.valid_from/valid_until` 与 `temporal_state_records` 重建过去线工作视图，不读取主线当前态缓存作为唯一来源。
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
- [ ] `TemporaryCharacterState.environmental_exposure` 跨回合累计冷 / 热 / 呼吸暴露；`EmbodimentState.environmental_strain` 只包含本回合档位、惩罚和 exposure delta。
- [ ] CharacterCognitivePass 输入只含 L2 + prior L3，不含 Layer 1 原始 SceneEvent；本回合事件使用 `ObservableEventDelta`。
- [ ] CharacterCognitivePassOutput 不直接覆盖 L3；SubjectiveStateReducer 根据 prior L3 与离散 shift 生成下一份 `character_subjective_snapshots`。
- [ ] 用户扮演角色跳过 CognitivePass 时，`CharacterRoleplay.subjective_input` 可更新该角色自己的 L3；未提供心理输入时保留 prior L3。
- [ ] 用户扮演输入中的隐藏客观事实断言若无 L2 来源，只能成为该角色 `NewHypothesis`、`DirectorHint` / `SceneNarration` 候选或 ambiguity，不能提升 Knowledge 访问权限或直接写 L1。
- [ ] `UserInputDelta` 为每次用户输入记录 `authority_class` 与 `authority_notes`；越权、降级、需要确认和歧义原因能在 Trace 中复查。
- [ ] 用户扮演角色的外显行动会写入该角色 IntentPlan 并跳过 CognitivePass，但命中、伤害、资源、位移、揭示等硬效果仍必须通过 OutcomePlanner + EffectValidator。
- [ ] 低风险、非持久 `SceneNarration`（如普通家具 / 氛围细节）可进入工作副本；持久实体、隐藏机关、重要资源、地理拓扑、天气 / 灵力硬状态必须确认、降级或阻止。
- [ ] `DirectorHint` 只能影响 outcome / style 偏置；“让他相信我 / 让某人害怕”等输入不得直接覆盖他人 L3。
- [ ] “忽略规则 / 直接设为正史 / 改掉隐藏设定”等用户发言不能改变 PromptBuilder 节点契约、KnowledgeAccessResolver、EffectValidator、TemporalConsistencyValidator 或 StateCommitter 边界。
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
- [ ] ReactionWindow 开启后会独立派生 `eligible_reactors` 并临时标记 `reaction_window_open`；未进入 primary cognitive pass 预算裁剪的援护者仍可进行 reaction pass。
- [ ] Tier A/B 角色 `knowledge_revealed` 会触发 CognitivePass；离场角色只记录 pending knowledge，入场或被硬触发时消费。
- [ ] `CharacterFocused` 叙事只能引用该角色可观察或可访问事实；`ObjectiveCamera` 叙事不能进入任何角色内心；`DirectorView` 默认仍剔除 GodOnly。
- [ ] `SurfaceRealizerInput` 只接收 `NarrativeCharacterView[]`，不接收完整 `CharacterCognitivePassOutput[]`。
- [ ] SurfaceRealizer 使用 `chat_structured` 返回 `SurfaceRealizerOutput { narrative_text, used_fact_ids }`；不得用裸 `chat` / `chat_stream` 绕过 schema。
- [ ] 第一版 SurfaceRealizer 不记录裸 `chat_stream` chunk；若未来启用叙事流式，必须走 `chat_structured_stream` 或“草稿流式 + 最终结构化确认”方案。
- [ ] `SurfaceRealizerOutput.used_fact_ids` 必须是 `OutcomePlan.narratable_facts.fact_id` 子集；文本抽查发现新增硬事实时判为 NarrativeFactCheck 失败。
- [ ] Dirty Flags 正确过滤无变化角色。
- [ ] Primary cognitive pass 默认控制在每场景 0-3 次；reaction pass 只对 eligible reactors 执行，并受每窗口/每角色/深度预算限制。
- [ ] active characters 达到可配置 `tiering_start_active_characters` 后启用分层调度；`max_primary_cognitive_passes` 默认 3，可由 `RuntimeConfigSnapshot.request_budget` 覆盖。
- [ ] active + dirty 角色超过 primary cognitive pass 预算时，运行时按威胁/反应/直接交互/心智相关性优先级稳定裁剪，未执行角色记录 `budget_deferred` 且 dirty flags 不丢失。
- [ ] 未执行完整 CognitivePass 且无可复用 IntentPlan 的次要人物以 `MinorActorSlot` 交给 OutcomePlanner 补全外显行为；不得写 L3、生成隐藏知识推断或长期目标变化。
- [ ] PromptBuilder 估算输入 token；达到 16K 触发确定性压缩 / 裁剪；超过有效最大上下文时继续裁剪到上限内，并记录 `PromptBudgetReport`。
- [ ] `prior_subjective_state` 的心智模型焦点在 CharacterCognitivePass 中进入 P1 以上优先级；长内心文本被压缩为结构化摘要，不无限回灌。
- [ ] SQLite 使用 WAL + 读连接池 + 单写提交；等待远程 LLM 时没有打开写事务。
- [ ] 应用启动 / 打开 World 时生成 `RuntimeConfigSnapshot` / `WorldRulesSnapshot`，同一回合内配置变更不影响正在执行的 Resolver。
- [ ] Agent 回合、Trace、LLM Logs 和异常事件同时记录 `runtime_config_snapshot_id` 与 `world_rules_snapshot_id`；ST / 全局运行事件只需要 runtime 配置 ID。
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
- [ ] 全局 `app_logs.sqlite` 只创建日志相关表，且不设置指向 `agent_sessions` / `world_turns` / `turn_traces` 的外键；World 内 `world.sqlite` 使用完整 Agent schema。
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
- [ ] Trace、LLM Logs、异常事件记录必要的 `runtime_config_snapshot_id` / `world_rules_snapshot_id`，能回查当时采用的预算与阈值摘要。
- [ ] 普通清理任务不会删除 Agent Trace 或仍被 `state_commit_records.trace_ids` 引用的记录。
- [ ] 30 天未更新且日志较大的 World 只产生提示事件，不自动删除。

### 阶段七：用户角色扮演

- [ ] 用户能扮演特定角色并影响结果规划。

### 阶段八：优化与扩展

- [ ] 性能优化不改变 AgentRuntime 的快照、并行 CognitivePass、单写提交和验证顺序。
- [ ] Trace 可视化只读取 Agent Trace / Logs，不参与 PromptBuilder、Resolver、Validator 或 StateCommitter 判断。
- [ ] 日志管理 UI 能展示全局 Logs 与 World Trace / Logs 的容量摘要，手动清理 / 导出前必须显示影响范围并要求确认。
- [ ] 日志管理 UI 不会删除仍被 `state_commit_records.trace_ids` 引用的 Agent Trace，也不会自动删除长期未更新 World。
- [ ] 30 天未更新且日志较大的 World 只产生提示事件；用户确认前不清理 World 内 Trace / Logs。
- [ ] 插件系统不能绕过 API 配置脱敏、KnowledgeAccessResolver、EffectValidator、TemporalConsistencyValidator、World Editor paused-only 或日志边界。
