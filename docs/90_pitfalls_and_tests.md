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
| SceneStateExtractor 权限不清 | 用户一句话触发隐藏 Knowledge 泄露或改写私密设定 | 第一版只读当前 SceneModel + 世界级约束；隐藏 Knowledge / GodOnly 默认不可读，作者编辑模式另行设计 |
| SceneInitializer 过度创作 | 切场景时凭空生成重要 NPC、秘密机关或剧情硬事实 | 只读公开上下文；通过 generation_policy 限定可补全域；所有 LLM 推断写 assumptions，高风险需用户确认 |
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
| 档位阈值在两侧不一致 | body 已 Storm 但 perception 仍 Strong | 阈值表集中常量化（一份表两侧共享）；改阈值需同时跑两侧单元测试 |
| LLM 误读灵力数值 | 8800 当成"高了点"、Δ=3000 当作"略胜" | SceneFilter 把 mana_power → ManaPotencyTier + ManaPerceptionDelta；FilteredSceneView 不暴露 raw 数值给 CognitivePass |
| 凡人感知修士细节 | T0 观察者却给出 attribute / 具体档位 | 规则 5/6: T0 灵觉为 0 时 mana_signals 为空; T0 仅能感知 effective ≥ 1000 为"超出常理"，无具体档位 |
| 隐匿气息被识破或装太死 | 一律识破 / 一律不识破 | concealment_suspected 由 (observer.effective vs target.effective − 200) + 灵觉敏锐度公式定 |
| 对抗解算与感知层用同一 mana_power | 压制就直接弱化实际对抗 | 对抗解算读 effective（不含压制），感知读 displayed（含压制）；两层显式分离 |
| 大佬硬吃小弟 | 完全无视技巧/状态导致碾压式叙事 | 加算修正区 × soul_factor 可制造以弱胜强；以毒/偷袭/算计实现而非抹平 mana_power |
| 不同世界灵力数值无法兼容 | 某些世界无修真 / 数值范围迥异 | ManaPotencyTier 边界与 Δ 桶阈值存于 world_base.yaml; 不同世界各自一份阈值表; 角色卡解析与档位翻译共用 |
| 灵觉过载处理 | 高灵气环境失真 | 过载阈值 + 感知降级 + 验证 |

### 1.6 调用预算与性能

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 多角色调用成本 | Token 消耗大 | Dirty Flags + 意图复用 + Tier 分级 |
| 并行 Agent 放大 Provider 限流 | 延迟抖动、费用飙升 | 并行度受 Active Set、Tier、场景预算和 Provider 限流器约束 |

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

---

## 2. 测试用例 / 验证方案

### 阶段一：基础框架

- [ ] 应用启动 / 发送消息 / JSON 存储。

### 阶段二：SillyTavern 模式

- [ ] 导入 SillyTavern 角色卡 V3。
- [ ] 世界书词条完整触发（含正则 / 概率 / 时间）。
- [ ] 预设正确应用。
- [ ] Regex 脚本按 global -> preset -> scoped 顺序运行，且 `markdownOnly` / `promptOnly` 不写回聊天 JSON。
- [ ] 角色卡和预设内嵌 Regex 未获允许前不运行，允许状态按 avatar / preset name 正确持久化。
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

#### 状态与运行时

- [ ] SceneStateExtractor 输入包含最近自由文本与当前结构化 Scene JSON，输出 `SceneStateExtractorOutput`，不直接提交 Layer 1。
- [ ] SceneStateExtractor 第一版不会读取隐藏 Knowledge / GodOnly；若请求使用这些信息，权限域检查失败。
- [ ] SceneInitializer 输入只包含结构化 SceneSeed、公开世界 / 地点 / 人物上下文和 generation_policy，不包含用户原始自由文本。
- [ ] SceneInitializer 输出 `SceneInitializationDraft`，所有非输入来源字段都有 `SceneAssumption`，且风险达到阈值时不会自动提交。
- [ ] SceneInitializer 在 `forbid_new_named_entities=true` 时不能生成新的命名持久实体；背景实体只能是 transient / unnamed / non_persistent。
- [ ] SceneInitializer 第一版不会读取隐藏 Knowledge / GodOnly；若请求用私密设定补场景，权限域检查失败。
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
- [ ] SQLite 使用 WAL + 读连接池 + 单写提交；等待远程 LLM 时没有打开写事务。
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
- [ ] 全局 Logs 超过 1GB 后后台清理旧运行日志。
- [ ] 普通清理任务不会删除 Agent Trace 或仍被 `state_commit_records.trace_ids` 引用的记录。
- [ ] 30 天未更新且日志较大的 World 只产生提示事件，不自动删除。

### 阶段七：用户角色扮演

- [ ] 用户能扮演特定角色并影响结果规划。
