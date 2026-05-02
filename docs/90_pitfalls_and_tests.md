# 90 风险登记与测试方案

本文档承载：

- 潜在坑点（按主题分组的风险登记）
- 测试矩阵入口

按阶段组织的测试用例 / 验证方案见 [91_test_matrix.md](91_test_matrix.md)。

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
| SurfaceRealizer 私自添加事实 | 误导用户 / 后续状态不一致 | NarrativeFactCheck 校验 `used_fact_ids` 子集并抽查文本；结构化 narratable_facts 白名单约束 |
| 叙事 POV 泄露隐藏事实 | 角色聚焦叙事写出该角色不可知信息 | `NarrationScope` 先决定 `SceneNarrativeView` 与 `narratable_facts`，StyleConstraints.pov 不得提升叙事披露范围 |
| OutcomePlanner 权限过宽 | God-read 编排节点演变成直接写状态或绕过校验 | God-read 不等于提交权限；输出 StateUpdatePlan 后必须由 EffectValidator + StateCommitter 校验提交 |
| OutcomePlanner 反复修复导致成本失控 | 单回合延迟和费用不可控 | 每回合默认最多 1 次 OutcomePlanner；校验失败时裁剪非法硬效果为 blocked_effects / soft_effects，不二次调用修复 |
| 技能 notes 驱动硬状态 | LLM 根据自然语言即兴改伤势/位置/资源 | SkillEffectContract 定义允许状态域、目标、成本、强度和揭示权限；EffectValidator 只提交契约内硬变化 |
| BodyReactionDelta 直写 L1 | 角色认知 LLM 绕过状态提交路径 | BodyReactionDelta 只作为候选反应；必须转成合法 CharacterStateDelta 后经 StateCommitter 写入 |
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
| 访问权限读取 L3 主观关系 | LLM 输出改变知识访问权限，形成循环 | SocialAccessAtLeast 只读 L1 `objective_relationships` / 授权时态记录，不读 relation_models |
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
| 结构化文本编辑器过度格式化 | 用户的世界书正文、prompt 模板或 YAML 注释被静默改写，导致注入文本变化或配置语义变化 | Structured Text Editor 只在解析成功后格式化；Plain 只做保守缩进；切换模式不自动改写；ST content 仍保存为 string；Agent structured content 仍需业务 validator |
| 文本诊断被误当业务校验 | JSON / YAML 可解析但不符合 Knowledge schema、Regex 语义或 ST 兼容字段 | 文本 diagnostics 与业务 validation 分离；Regex 编译、ST 兼容规则、Agent schema / paused-only / impact analysis 必须在父级模块复跑 |
| 地点层级与行政名称混用 | "县/州/大区" 被当成程序层级，跨国家设定冲突 | 程序只信 `LocationNode.parent_id` 与 `canonical_level`；`type_label` 仅显示；国家模板只做编辑器校验 |
| 同父级地点被误判为可一日抵达 | 行政归属被当成道路距离，导致剧情瞬移 | 路程估算只读 `LocationEdge`；同父级只能生成低置信度 `ProximityHint` |
| 自然地理带被硬塞进行政树 | 山脉 / 平原跨多个州县时破坏单父级层级，或被错误继承为行政事实 | `NaturalRegion` 用 `parent_id` 挂载主地理域，跨域影响用 `LocationSpatialRelation`；行政继承与自然影响分开 |
| 地区事实继承泄露隐藏知识 | 父级私密 RegionFact 被所有子地点角色读到 | LocationFactResolver 只扩展候选，最终仍经 KnowledgeAccessResolver；继承不得提升访问权限 |
| 地点别名歧义被 LLM 猜死 | 多个 `c县` 被错误绑定到同一地点 | `location_aliases` 允许一对多；LocationResolver 多命中必须返回 ambiguity 或要求用户确认 |
| World Editor 直接改 SQLite | 绕过 schema 校验、访问派生索引、地点一致性和审计，导致后续运行时不可复盘 | 所有编辑走 WorldEditorPatch；提交前校验和影响分析；单事务写权威表、派生索引和 `world_editor_commits` |
| 作者编辑伪装成运行回合 | 无 `scene_turn_id` 的修改被写进 `state_commit_records`，破坏回滚与主线判断 | World Editor 使用独立 editor commit journal，不写 `world_turns` / `state_commit_records` |

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
| 运行中作者编辑与回合提交并发 | World Editor 改写 canonical Truth 时，正在执行的回合仍基于旧快照提交，造成状态覆盖 | World Editor 提交必须 paused-only：无 active turn、无 pending LLM call、无 StateCommitter 写入、revision 一致 |
| 流式日志高频抢写 | stream chunk 写入阻塞状态提交 | 日志使用短事务、队列或批量写；状态提交优先级高于调试日志 |

### 1.5 物理 / 属性 / 灵力档位翻译

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| LLM 误读物理量数值 | 50m/s 当成微风、-30℃ 当成凉爽 | 程序在 EmbodimentResolver/SceneFilter 把 raw → tier + effect_hints；FilteredSceneView 不暴露 raw 值给 CognitivePass |
| 物种舒适带未校准 | 同样温度对不同种族应不同感受 | BaselineBodyProfile 含 comfort_temperature_range；档位是相对该范围偏离量计算 |
| 环境压力跨回合丢失 | 长期暴露不发生冻伤 | EnvironmentalStrain 只产出本回合 exposure delta；跨回合累计写入 L1 `TemporaryCharacterState.environmental_exposure`，到阈值后经 OutcomePlanner 候选 + EffectValidator 生成伤势事件 |
| L1 物理子字段不自洽 | 暴雨却地面不湿、沙暴但能见度 100m | SceneInitializer / SceneStateExtractor prompt 模板强制一并填齐；额外 ConsistencyRule 检查（暴雨时 wetness>=阈值，沙暴时 dust_density>=阈值） |
| 档位阈值在两侧不一致 | body 已 Storm 但 perception 仍 Strong | 阈值表集中在已校验配置快照（一份表两侧共享）；改阈值需同时跑两侧单元测试 |
| 配置热路径反复 IO | 每次感知、对抗或日志写入都读 YAML/SQLite，拖慢回合 | 启动 / 打开 World / 保存设置时加载并校验配置，发布 `RuntimeConfigSnapshot`；热路径只读内存快照 |
| UI 取整影响仲裁 | `999.6` 展示为 `1000` 后被当成下一档 | 属性存储和计算用 f64，UI 默认整数展示但不写回；档位按真实值与阈值比较 |
| LLM 误读基础属性数值 | 直接把 physical=1800 写成必定擒拿成功 | AttributeResolver / SceneFilter 把 raw 属性 → AttributeTier / AttributeDelta / descriptors / constraints；FilteredSceneView 不暴露 raw 属性值给 CognitivePass |
| LLM 误读灵力数值 | 8800 当成"高了点"、Δ=3000 当作"略胜" | SceneFilter 把 mana_power → AttributeTier + AttributeDelta；FilteredSceneView 不暴露 raw 数值给 CognitivePass |
| 凡人感知修士细节 | T0 观察者却给出 attribute / 具体档位 | 规则 5/6: T0 灵觉为 0 时 mana_signals 为空; T0 仅能感知 effective ≥ 1000 为"超出常理"，无具体档位 |
| 长期倾向与当前姿态混用 | 角色"天生内敛"被当成每回合都在压制，或"外放体质"不能临时封息 | `ManaExpressionTendency` 持久化三档默认倾向；`ManaExpressionState.mode` 只表达当前封息/抑制/自然/外放/威压，并记录 intentionality/source |
| `display_ratio` 量纲写错 | 把 `display_ratio` 计算成 displayed 数值，导致再次乘 effective 或单位混乱 | `display_ratio = clamp(1 + tendency_factor + mode_factor, 0, 2)`；`displayed_mana_power = effective_mana_power * display_ratio + 合法 L1 修正` |
| 隐匿气息被识破或装太死 | 一律识破 / 一律不识破 | concealment_suspected 由 (observer.effective vs target.effective − 200) + 灵觉敏锐度公式定 |
| 外放被当成真实战力提升 | display_ratio > 1 后对抗也变强 | ManaExpressionTendency / ManaExpressionMode 只影响 displayed、presence pressure、salience/reasoning；CombatMathResolver 只读 effective |
| 威压直接改写他人信念 | 被灵压影响的角色必定臣服/相信 | SceneFilter 只写 pressure_hints 与 reasoning modifiers；恐惧、屈服、误判仍由该角色 CognitivePass 基于 L2 + prior L3 生成 |
| 对抗解算与感知层用同一 mana_power | 持久倾向或运行时封息/抑制/外放直接改写实际对抗 | 对抗解算读 effective（不含显露倾向、运行时状态与压制），感知读 displayed（含显露倾向、运行时状态、压制、伪装）；两层显式分离 |
| 大佬硬吃小弟 | 完全无视技巧/状态导致碾压式叙事 | 加算修正区 × soul_factor 可制造以弱胜强；以毒/偷袭/算计实现而非抹平 mana_power |
| `mana_potency` 与 `mana_power` 双口径 | 同一能力被两套字段/规则重复表达 | 正式字段只用 `mana_power`；`mana_potency` 不作为基础属性或规则名 |
| 不同世界灵力数值无法兼容 | 某些世界无修真 / 数值范围迥异 | AttributeTier 边界与 AttributeDelta 桶阈值存于 world_base.yaml; 不同世界各自一份阈值表; 角色卡解析与档位翻译共用同一 `WorldRulesSnapshot` |
| 灵觉过载处理 | 高灵气环境失真 | 过载阈值 + 感知降级 + 验证 |

### 1.6 调用预算与性能

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 多角色调用成本 | Token 消耗大 | Dirty Flags + 意图复用 + Tier 分级 |
| 并行 Agent 放大 Provider 限流 | 延迟抖动、费用飙升 | 并行度受 Active Set、Tier、场景预算和 Provider 限流器约束 |
| Prompt 输入超出模型上下文 | 请求失败、Provider 截断、角色误判 | `PromptBudgetReport` 估算输入 token；16K 触发压缩 / 裁剪；有效最大上下文来自用户配置与 Provider 窗口；超过时必须继续裁剪到上限内 |
| 预算裁剪删掉关键约束 | LLM 越权、schema 漂移、结果不可校验 | P0/P1/P2/P3 分区；P0 权限、schema、当前任务硬规则和合法选项不可裁剪；裁剪 refs 写 Trace |
| 未全量认知角色突然改变内心 | 次要 NPC 凭空识破秘密、改变信念或长期目标 | 未跑 CognitivePass 的角色只允许复用意图、模板意图、`MinorActorSlot` 外显补全或 crowd behavior；不写 L3 |

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

## 2. 测试矩阵入口

按阶段组织的测试矩阵已拆分到 [91_test_matrix.md](91_test_matrix.md)。

本文件保留风险登记与质量门禁入口；新增可执行测试时优先更新 `91_test_matrix.md`，若暴露新的系统性风险，再回写本文件。
