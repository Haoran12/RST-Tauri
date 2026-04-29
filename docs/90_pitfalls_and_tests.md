# 90 风险登记与测试方案

本文档承载：

- 潜在坑点（按主题分组的风险登记）
- 测试用例 / 验证方案

---

## 1. 潜在坑点

### 1.1 LLM 输出与 Prompt 漂移

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 模型输出非结构化 | 解析失败 | structured output / tool schema + 重试 + schema 验证 |
| LLM 输出不符合 schema | 解析失败 / 主循环中断 | 优先使用 Provider structured output / tool schema；JSON mode 仅作降级，并必须配合 schema 校验 + 重试 + 程序容错修复；必要时由 OutcomePlanner 兜底 |
| LLM 数值不稳定 | belief/emotion 数值跳变 | LLM 输出离散级别（ConfidenceShift），程序映射为数值 |
| Prompt 漂移 | 模型行为变化 | 固定 prompt 版本 + A/B 测试 + 监控 |
| PromptBuilder 混入世界事实 | 静态模板携带隐藏设定，或动态输入绕过 schema | 静态节点提示词只写权限/任务/输出规则；动态部分只允许 `{ input: <TInput> }`；Trace 记录 prompt_template_id/version/hash |
| 不同 Agent 节点误用同一 API 配置 | 结构化节点用到不支持 schema 的模型，或叙事节点误用低质量便宜模型 | AgentLlmProfile 按节点绑定 API 配置；调用前校验结构化能力；日志记录 api_config_id |
| 中间数据混入可判定自由文本 | 屎山起点；规则匹配失效 | 数据形态铁律 + 类型隔离；允许 `summary_text` / `effect_hints` 等 LLM-readable 文本叶子字段，但禁止参与程序判断 |
| SurfaceRealizer 私自添加事实 | 误导用户 / 后续状态不一致 | NarrativeFactCheck 强制扫描；narratable_facts 白名单约束 |
| 叙事 POV 泄露隐藏事实 | 角色聚焦叙事写出该角色不可知信息 | `NarrationScope` 先决定 `SceneNarrativeView` 与 `narratable_facts`，StyleConstraints.pov 不得提升叙事披露范围 |
| OutcomePlanner 权限过宽 | God-read 编排节点演变成直接写状态或绕过校验 | God-read 不等于提交权限；输出 StateUpdatePlan 后必须由 EffectValidator + StateCommitter 校验提交 |
| OutcomePlanner 反复修复导致成本失控 | 单回合延迟和费用不可控 | 每回合默认最多 1 次 OutcomePlanner；校验失败时裁剪非法硬效果为 blocked_effects / soft_effects，不二次调用修复 |
| 技能 notes 驱动硬状态 | LLM 根据自然语言即兴改伤势/位置/资源 | SkillEffectContract 定义允许状态域、目标、成本、强度和揭示权限；EffectValidator 只提交契约内硬变化 |
| BodyReactionDelta 直写 L1 | 角色认知 LLM 绕过状态提交路径 | BodyReactionDelta 只作为候选反应；必须转成合法 CharacterBodyDelta 后经 StateCommitter 写入 |
| 反应事件递归 | A 攻击 B，B 反击又触发 A 反应，形成无限循环 | ReactionWindow 只收集 ReactionIntent；默认 no_reaction_to_reaction + one_reaction_per_character_per_window；interrupt 必须有显式深度上限 |
| 伙伴援护缺失或越权 | B 被攻击时伙伴不能救，或不可观察角色凭空救场 | eligible_reactors 由程序按感知可达性、Knowledge 访问权限、关系/姿态、距离、通道、资源与 SkillEffectContract 判定；LLM 只能在合法 ReactionOption 中选择 |
| SceneStateExtractor 权限不清 | 用户一句话触发隐藏 Knowledge 泄露或改写私密设定 | 场景域 God-read 只读当前 SceneModel 与 `private_scene_constraints`；无关隐藏 Knowledge / 全局 GodOnly 默认不可读，作者编辑模式另行设计 |
| SceneInitializer 过度创作 | 切场景时凭空生成重要 NPC、秘密机关或剧情硬事实 | 只读公开上下文与程序裁剪后的场景相关私有约束；通过 generation_policy 限定可补全域；所有 LLM 推断写 assumptions，高风险需用户确认 |
| 场景私有约束外显化 | 初始化或提取时把隐藏机关、伪装身份、GodOnly 约束写成可见事实 | `private_scene_constraints` 只能用于一致性、隐藏状态或揭示条件；公开字段写入私有来源时权限域检查失败 |
| SceneInitializer 与 SceneStateExtractor 职责重叠 | 一个节点既解析用户输入又补全完整场景，权限和 trace 混乱 | SceneInitializer 只处理结构化 SceneSeed；SceneStateExtractor 只处理最近自由文本到 delta / UserInputDelta |
| 用户输入 LLM 解析失败 | 用户操作丢失 | 显示原始输入 + 提示重写；保留 raw_text 供 trace |

### 1.2 全知与 Knowledge 访问权限

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 全知泄露难检测 | 行为不符设定 | 输入过滤 + 输出验证 + 访问日志审计 |
| Layer 1 泄露至受限 LLM | 全知 / 屎山起点 | InputAssembly 类型隔离（仅接受 Layer 2 类型）+ 单元测试断言；God-read 节点必须显式记录权限域 |
| 访问权限逻辑散落 | 多处不一致 | KnowledgeAccessResolver 是唯一入口；所有判断必须经它 |
| 访问索引替代 Resolver | SQL 查询结果被当成最终访问权限，绕过 GodOnly / apparent / self_belief 规则 | `known_by` / `scope` 派生索引只做候选预筛；所有候选必须再经 KnowledgeAccessResolver |
| 访问派生索引漂移 | `access_policy` JSON 与索引表不一致，导致该读的读不到或隐藏知识泄露 | KnowledgeStore / StateCommitter 同事务维护索引；提供从 JSON 全量重建并比对的校验 |
| 访问权限读取 L3 主观关系 | LLM 输出改变知识访问权限，形成循环 | SocialAccessAtLeast 只读 L1 客观关系/授权等级，不读 relation_models |
| Subject self-belief 被外部读 | 暴露真相 | `KnowledgeEntry.content` 与 `self_belief` 在类型层面分离；访问 API 强制经过 awareness 检查 |
| Knowledge 揭示无追溯 | 不知何时谁知道了什么 | 所有访问权限变更必须经 KnowledgeRevealEvent；持久化到独立表，包含 scope_change |
| Belief 与 RelationModel 重复 | 同一命题两处存储 | 文档约定 + lint 规则：关于人的命题写 RelationModel，关于事件/世界的写 BeliefState |

### 1.3 数据 schema 与持久化

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| KnowledgeEntry 字段膨胀 | 单表过宽难查询 | content 用 JSON 列；高频查询用 (subject_id, facet_type) 索引；不在表层加新列 |
| Schema 漂移 | 旧数据无法兼容 | 每个 KnowledgeEntry 含 `schema_version`；StateCommitter 写入时校验；提供迁移脚本 |
| Rust-TS 类型同步 | 两端定义不一致 | 代码生成 + 共享 schema + 单元测试 |
| 状态爆炸 | 长对话状态过大 | 增量更新 + 周期压缩 + Knowledge metadata 衰减 |
| 地点层级与行政名称混用 | "县/州/大区" 被当成程序层级，跨国家设定冲突 | 程序只信 `LocationNode.parent_id` 与 `canonical_level`；`type_label` 仅显示；国家模板只做编辑器校验 |
| 同父级地点被误判为可一日抵达 | 行政归属被当成道路距离，导致剧情瞬移 | 路程估算只读 `LocationEdge`；同父级只能生成低置信度 `ProximityHint` |
| 自然地理带被硬塞进行政树 | 山脉 / 平原跨多个州县时破坏单父级层级，或被错误继承为行政事实 | `NaturalRegion` 用 `parent_id` 挂载主地理域，跨域影响用 `LocationSpatialRelation`；行政继承与自然影响分开 |
| 地区事实继承泄露隐藏知识 | 父级私密 RegionFact 被所有子地点角色读到 | LocationFactResolver 只扩展候选，最终仍经 KnowledgeAccessResolver；继承不得提升访问权限 |
| 地点别名歧义被 LLM 猜死 | 多个 `c县` 被错误绑定到同一地点 | `location_aliases` 允许一对多；LocationResolver 多命中必须返回 ambiguity 或要求用户确认 |

### 1.3.1 多时期会话与过去线

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 用提交时间判断过去线 | 今天补玩十年前剧情被误认为当前剧情，或未来预演污染正史 | 只用 `period_anchor` / `story_time_anchor` 与 `WorldMainlineCursor.mainline_time_anchor` 比较；`created_at` 只作审计 |
| 聊天记录等同世界状态 | 删除会话误删正史，或非正史聊天改写 canonical Truth | `AgentSession` / `SessionTurn` 与 `world_turns` 分离；会话删除不自动删除 canonical Truth |
| 过去线读到未来知识 | 角色提前知道后续身份揭示、伤势或记忆 | `WorldStateAt(period_anchor)` + KnowledgeAccessResolver 时间参数；未来事实只进 TruthGuidance 的 God-read 域 |
| 既有历史简述被当成硬规则 | `summary_text` 含糊文本导致程序误判冲突 | 只有 `HistoricalEvent` 的结构化 `required_outcomes` / `forbidden_outcomes` / `known_after_effects` 可作硬约束 |
| 过去线冲突中断游玩 | 用户为了探索 if 线被系统强行挡住 | 硬冲突只生成警告和 `ConflictReport`；用户选择冲突后非正史或整条非正史，剧情继续 |
| 非正史内容污染正史 | 冲突后的剧情改写角色记忆、主线光标或 Knowledge | `canon_status` 控制 StateCommitter；非正史只写 SessionTurn / provisional truth / Trace，不写 canonical Layer 1 / Layer 3 / Knowledge |
| 冲突前可用细节被全丢 | 一次后期冲突导致前面已兼容的补完也无法利用 | 用户可选择 `NonCanonAfterConflict`，保留冲突前已校验通过的 provisional truth 提升资格 |
| 非正史自动恢复 | 后续剧情“绕回来”导致系统错误把已冲突分支重新入正史 | 非正史不可自动恢复；必须通过新修正 / 重放流程重新校验 |
| 回滚过去线破坏后续依赖 | 删除早期补完事实后，后续正史仍引用该细节 | canonical 回滚先做依赖检查；有后续 facts、已提升 provisional truth 或主线光标依赖时阻止并生成影响报告 |

### 1.4 SQLite 并发与派生索引

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 等待 LLM 时持有写事务 | SQLite 写锁阻塞 UI、日志和后续提交 | 并行认知阶段只读快照；远程调用期间不持有写事务；最终 StateCommitter 单写提交 |
| 多个 Agent 并发写状态 | L1/L3 顺序不确定，回滚困难 | CognitivePass 只产出候选；统一验证后由 OutcomePlanner 协调，StateCommitter 单事务写入 |
| 流式日志高频抢写 | stream chunk 写入阻塞状态提交 | 日志使用短事务、队列或批量写；状态提交优先级高于调试日志 |

### 1.5 物理 / 灵力档位翻译

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| LLM 误读物理量数值 | 50m/s 当成微风、-30℃ 当成凉爽 | 程序在 EmbodimentResolver/SceneFilter 把 raw → tier + effect_hints；FilteredSceneView 不暴露 raw 值给 CognitivePass |
| 物种舒适带未校准 | 同样温度对不同种族应不同感受 | BaselineBodyProfile 含 comfort_temperature_range；档位是相对该范围偏离量计算 |
| 环境压力跨回合丢失 | 长期暴露不发生冻伤 | EnvironmentalStrain.cold_strain/heat_strain/respiration_strain 在 EmbodimentResolver 累加，到阈值后经 OutcomePlanner 候选 + EffectValidator 生成伤势事件 |
| L1 物理子字段不自洽 | 暴雨却地面不湿、沙暴但能见度 100m | SceneInitializer / SceneStateExtractor prompt 模板强制一并填齐；额外 ConsistencyRule 检查（暴雨时 wetness>=阈值，沙暴时 dust_density>=阈值） |
| 档位阈值在两侧不一致 | body 已 Storm 但 perception 仍 Strong | 阈值表集中在已校验配置快照（一份表两侧共享）；改阈值需同时跑两侧单元测试 |
| 配置热路径反复 IO | 每次感知、对抗或日志写入都读 YAML/SQLite，拖慢回合 | 启动 / 打开 World / 保存设置时加载并校验配置，发布 `RuntimeConfigSnapshot`；热路径只读内存快照 |
| LLM 误读灵力数值 | 8800 当成"高了点"、Δ=3000 当作"略胜" | SceneFilter 把 mana_power → ManaPotencyTier + ManaPerceptionDelta；FilteredSceneView 不暴露 raw 数值给 CognitivePass |
| 凡人感知修士细节 | T0 观察者却给出 attribute / 具体档位 | 规则 5/6: T0 灵觉为 0 时 mana_signals 为空; T0 仅能感知 effective ≥ 1000 为"超出常理"，无具体档位 |
| 隐匿气息被识破或装太死 | 一律识破 / 一律不识破 | concealment_suspected 由 (observer.effective vs target.effective − 200) + 灵觉敏锐度公式定 |
| 对抗解算与感知层用同一 mana_power | 压制就直接弱化实际对抗 | 对抗解算读 effective（不含压制），感知读 displayed（含压制）；两层显式分离 |
| 大佬硬吃小弟 | 完全无视技巧/状态导致碾压式叙事 | 加算修正区 × soul_factor 可制造以弱胜强；以毒/偷袭/算计实现而非抹平 mana_power |
| 不同世界灵力数值无法兼容 | 某些世界无修真 / 数值范围迥异 | ManaPotencyTier 边界与 Δ 桶阈值存于 world_base.yaml; 不同世界各自一份阈值表; 角色卡解析与档位翻译共用同一 `WorldRulesSnapshot` |
| 灵觉过载处理 | 高灵气环境失真 | 过载阈值 + 感知降级 + 验证 |

### 1.6 调用预算与性能

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 多角色调用成本 | Token 消耗大 | Dirty Flags + 意图复用 + Tier 分级 |
| 并行 Agent 放大 Provider 限流 | 延迟抖动、费用飙升 | 并行度受 Active Set、Tier、场景预算和 Provider 限流器约束 |

### 1.6.1 ST API 切换副作用

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| ST 预设身份继续沿用 `apiId` | 切换 API 配置后预设丢失、自动切换或 Regex 授权失效 | RST 运行时使用稳定 `preset_key`；`source_api_id` / `apiId` 只用于导入导出兼容 |
| 世界书选择按 Provider 分组 | 切换 API 配置后 Global lore / Chat lore / Character lore 变化，导致同一聊天上下文漂移 | 世界书文件、`chat_metadata.world_info`、`world_info.globalSelect`、`charLore` 均不读取 `active_api_config_id` |
| API 切换触发资源保存 | 用户只是换连接，却意外改写预设、世界书或聊天 metadata | API 切换只保存 `active_api_config_id`；资源文件只在用户显式编辑对应资源时保存 |

### 1.7 日志与 Trace

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| Agent Trace 与运行 Logs 混用 | 回放、审计、清理边界不清 | Agent Trace 以 `scene_turn_id` 为主轴；运行 Logs 以 `request_id` / `event_id` 为主轴，只通过 ID 关联 |
| 日志反向进入 prompt | 调试信息污染角色认知 | 架构铁律：日志只观察，不参与业务判断或 LLM 输入 |
| LLM 原始响应丢失 | 难以定位 Provider / prompt 问题 | 保存 request / response / schema / stream chunk；额外生成 readable_text |
| 流式输出只保存最终文本 | 无法复现分片、断流、重复 chunk | `llm_stream_chunks` 按序保存原始 chunk |
| 凭证写入日志 | API Key 泄露 | 写入前脱敏，测试覆盖 Authorization / x-api-key / Provider secret |
| 自动清理误删 Agent Trace | 旧回合无法复盘或定位回滚 | 默认只清理全局运行 Logs；Agent Trace 随 World 保留 |
| 长期未更新 World 体积膨胀 | 用户磁盘压力 | 30 天未更新且体积较大时提示用户，不自动删除 |
| 日志清理上限硬编码 | 高级用户无法按磁盘空间调整，或测试环境难以覆盖 | `app_runtime.yaml` 配置清理上限，RetentionManager 只读内存快照；`log_retention_state` 记录当次采用值 |

---

## 2. 测试用例 / 验证方案

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
- [ ] `temporary_body_state` 存储在 Layer 1，并只能通过 `EmbodimentState` 派生进入 CognitivePass。
- [ ] CharacterCognitivePass 输入只含 L2 + prior L3，不含 Layer 1 原始 SceneEvent；本回合事件使用 `ObservableEventDelta`。
- [ ] PromptBuilder 为五类 Agent LLM 节点生成固定消息布局：system 静态契约、developer/task 指令、user 单个 `{ input }` JSON；Trace 记录 prompt_template_id/version/hash。
- [ ] 静态节点提示词不包含世界事实、角色秘密、Knowledge 内容或日志摘要；动态输入必须匹配对应 `*Input` schema。
- [ ] 多个 active + dirty 角色的 CognitivePass 可以并行执行；并行阶段不写 Layer 1 / Layer 3 / Knowledge。
- [ ] 并行 CognitivePass 输出按稳定顺序进入 Validator；LLM 返回顺序变化不改变最终 StateCommitter 提交结果。
- [ ] OutcomePlanner 可 God-read，但输出必须是 `OutcomePlannerOutput` / `StateUpdatePlan`，并在 StateCommitter 前通过 EffectValidator 硬约束校验。
- [ ] OutcomePlanner 每回合默认最多调用一次；候选效果校验失败时不会反复调用 LLM 修复。
- [ ] LLM 候选技能效果超出 SkillEffectContract 时，硬状态不提交，转入 `blocked_effects` 或 `soft_effects`。
- [ ] `BodyReactionDelta` 不直接写入 Layer 1；只有合法 `CharacterBodyDelta` 能修改 `temporary_body_state`。
- [ ] A 攻击 B 时，B 与满足资格的伙伴/守护者会进入同一个 `ReactionWindow`，各自最多提交一个 `ReactionIntent`。
- [ ] B 的 `ReactionIntent` 为反击时，默认不会为 A 再打开普通 ReactionWindow；只有显式 interrupt 契约且未超过 `max_reaction_depth` 时才允许第二层。
- [ ] 不可观察威胁、距离/通道不满足、资源或冷却不足的角色不会出现在 `eligible_reactors`。
- [ ] Tier A/B 角色 `knowledge_revealed` 会触发 CognitivePass；离场角色只记录 pending knowledge，入场或被硬触发时消费。
- [ ] `CharacterFocused` 叙事只能引用该角色可观察或可访问事实；`ObjectiveCamera` 叙事不能进入任何角色内心；`DirectorView` 默认仍剔除 GodOnly。
- [ ] Dirty Flags 正确过滤无变化角色。
- [ ] Primary cognitive pass 控制在每场景 0-2 次；reaction pass 只对 eligible reactors 执行，并受每窗口/每角色/深度预算限制。
- [ ] active + dirty 角色超过 primary cognitive pass 预算时，运行时按威胁/反应/直接交互优先级稳定裁剪，未执行角色记录 `budget_deferred` 且 dirty flags 不丢失。
- [ ] SQLite 使用 WAL + 读连接池 + 单写提交；等待远程 LLM 时没有打开写事务。
- [ ] 应用启动 / 打开 World 时生成 `RuntimeConfigSnapshot` / `WorldRulesSnapshot`，同一回合内配置变更不影响正在执行的 Resolver。
- [ ] ManaPotencyTier、ManaPerceptionDelta、CombatOutcomeTier 使用同一份 World 配置快照；修改 `world_base.yaml` 后从下一回合生效。
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
