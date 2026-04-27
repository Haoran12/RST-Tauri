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
| LLM 输出不符合 schema | 解析失败 / 主循环中断 | 优先使用 Provider structured output / tool schema；JSON mode 仅作降级，并必须配合 schema 校验 + 重试 + 程序容错修复 + 仲裁层 LLM 兜底 |
| LLM 数值不稳定 | belief/emotion 数值跳变 | LLM 输出离散级别（ConfidenceShift），程序映射为数值 |
| Prompt 漂移 | 模型行为变化 | 固定 prompt 版本 + A/B 测试 + 监控 |
| 中间数据混入可判定自由文本 | 屎山起点；规则匹配失效 | 数据形态铁律 + 类型隔离；允许 `summary_text` / `effect_hints` 等 LLM-readable 文本叶子字段，但禁止参与程序判断 |
| SurfaceRealizer 私自添加事实 | 误导用户 / 后续状态不一致 | NarrativeFactCheck 强制扫描；visible_facts 白名单约束 |
| 叙事 POV 泄露隐藏事实 | 角色聚焦叙事写出该角色不可知信息 | `NarrationScope` 先决定 `SceneNarrativeView` 与 `visible_facts`，StyleConstraints.pov 不得提升可见性 |
| 仲裁层 LLM 兜底范围扩大 | 演变成"什么都让 LLM 仲裁" | 仲裁层 LLM 仅在认知输出失败时启用；物理判定永远走程序 |
| 用户输入 LLM 解析失败 | 用户操作丢失 | 显示原始输入 + 提示重写；保留 raw_text 供 trace |

### 1.2 全知与可见性

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 全知泄露难检测 | 行为不符设定 | 输入过滤 + 输出验证 + 访问日志审计 |
| Layer 1 泄露至 LLM | 全知 / 屎山起点 | InputAssembly 类型隔离（仅接受 Layer 2 类型）+ 单元测试断言 |
| 可见性逻辑散落 | 多处不一致 | VisibilityResolver 是唯一入口；所有判断必须经它 |
| Subject self-belief 被外部读 | 暴露真相 | `KnowledgeEntry.content` 与 `self_belief` 在类型层面分离；访问 API 强制经过 awareness 检查 |
| Knowledge 揭示无追溯 | 不知何时谁知道了什么 | 所有可见性变更必须经 KnowledgeRevealEvent；持久化到独立表 |
| Belief 与 RelationModel 重复 | 同一命题两处存储 | 文档约定 + lint 规则：关于人的命题写 RelationModel，关于事件/世界的写 BeliefState |

### 1.3 数据 schema 与持久化

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| KnowledgeEntry 字段膨胀 | 单表过宽难查询 | content 用 JSON 列；高频查询用 (subject_id, facet_type) 索引；不在表层加新列 |
| Schema 漂移 | 旧数据无法兼容 | 每个 KnowledgeEntry 含 `schema_version`；StateCommitter 写入时校验；提供迁移脚本 |
| Rust-TS 类型同步 | 两端定义不一致 | 代码生成 + 共享 schema + 单元测试 |
| 状态爆炸 | 长对话状态过大 | 增量更新 + 周期压缩 + Knowledge metadata 衰减 |

### 1.4 物理 / 灵力档位翻译

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| LLM 误读物理量数值 | 50m/s 当成微风、-30℃ 当成凉爽 | 程序在 EmbodimentResolver/SceneFilter 把 raw → tier + effect_hints；FilteredSceneView 不暴露 raw 值给 CognitivePass |
| 物种舒适带未校准 | 同样温度对不同种族应不同感受 | BaselineBodyProfile 含 comfort_temperature_range；档位是相对该范围偏离量计算 |
| 环境压力跨回合丢失 | 长期暴露不发生冻伤 | EnvironmentalStrain.cold_strain/heat_strain/respiration_strain 在 EmbodimentResolver 累加，仲裁层到阈值生成伤势事件 |
| L1 物理子字段不自洽 | 暴雨却地面不湿、沙暴但能见度 100m | SceneStateExtractor prompt 模板强制一并填齐；额外 ConsistencyRule 检查（暴雨时 wetness>=阈值，沙暴时 dust_density>=阈值） |
| 档位阈值在两侧不一致 | body 已 Storm 但 perception 仍 Strong | 阈值表集中常量化（一份表两侧共享）；改阈值需同时跑两侧单元测试 |
| LLM 误读灵力数值 | 8800 当成"高了点"、Δ=3000 当作"略胜" | SceneFilter 把 mana_power → ManaPotencyTier + ManaPerceptionDelta；FilteredSceneView 不暴露 raw 数值给 CognitivePass |
| 凡人感知修士细节 | T0 观察者却给出 attribute / 具体档位 | 规则 5/6: T0 灵觉为 0 时 mana_signals 为空; T0 仅能感知 effective ≥ 1000 为"超出常理"，无具体档位 |
| 隐匿气息被识破或装太死 | 一律识破 / 一律不识破 | concealment_suspected 由 (observer.effective vs target.effective − 200) + 灵觉敏锐度公式定 |
| 仲裁层与感知层用同一 mana_power | 压制就直接弱化对方仲裁 | 仲裁读 effective（不含压制），感知读 displayed（含压制）；两层显式分离 |
| 大佬硬吃小弟 | 完全无视技巧/状态导致碾压式叙事 | 加算修正区 × soul_factor 可制造以弱胜强；以毒/偷袭/算计实现而非抹平 mana_power |
| 不同世界灵力数值无法兼容 | 某些世界无修真 / 数值范围迥异 | ManaPotencyTier 边界与 Δ 桶阈值存于 world_base.yaml; 不同世界各自一份阈值表; 角色卡解析与档位翻译共用 |
| 灵觉过载处理 | 高灵气环境失真 | 过载阈值 + 感知降级 + 验证 |

### 1.5 调用预算与性能

| 坑点 | 风险 | 应对策略 |
|---|---|---|
| 多角色调用成本 | Token 消耗大 | Dirty Flags + 意图复用 + Tier 分级 |

### 1.6 日志与 Trace

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

### 阶段三-六：Agent 模式（参考 `D:\Projects\RST-flutter\docs\rp_agent_filtering_example.md`）

#### 感官与可见性

- [ ] 失明角色 `visible_entities` 为空。
- [ ] 狐狸精能闻到细微血腥味，普通人闻不到。
- [ ] 凡人无法清晰感知修士气息。

#### 知识可见性体系

- [ ] **私密 Knowledge 仅 known_by 中的角色能访问。**
- [ ] **GodOnly 知识不出现在任何角色的 accessible_knowledge 中。**
- [ ] **GodOnly 启用态下 known_by 必须为空；若故事揭示，KnowledgeRevealEvent 必须先解除 GodOnly 再追加知情者。**
- [ ] **subject_awareness=Unaware 时，subject 自我描述只能引用 self_belief**（如被封印记忆的狐狸精仍自称人类）。
- [ ] **观察者通过 apparent_content 看到的伪装信息与 content 真相一致地分流**（伪装方与揭穿方分别得到不同 visible_content）。
- [ ] **scope:faction:玄天宗 的 KnowledgeEntry 仅对该势力成员可见。**
- [ ] **同场景观察可获得他人 Appearance facet，但获取不到 TrueName facet**（无关系阈值）。
- [ ] **KnowledgeRevealEvent 触发后**，被揭示者的下一回合输入包含新可见 Knowledge。
- [ ] **CustomPredicate 可见性条件只能使用结构化 VisibilityExpression AST，不接受自然语言表达式。**

#### 状态与运行时

- [ ] 受伤状态跨回合保持。
- [ ] `temporary_body_state` 存储在 Layer 1，并只能通过 `EmbodimentState` 派生进入 CognitivePass。
- [ ] `CharacterFocused` 叙事只能引用该角色可见事实；`ObjectiveCamera` 叙事不能进入任何角色内心；`DirectorView` 默认仍剔除 GodOnly。
- [ ] Dirty Flags 正确过滤无变化角色。
- [ ] 调用预算控制在每场景 0-2 次。

#### 日志与 Trace

- [ ] ST 模式 LLM 调用只写全局 `./data/logs/app_logs.sqlite`。
- [ ] Agent 模式任意 `scene_turn_id` 能查到完整 `turn_traces` / `agent_step_traces`。
- [ ] Agent Trace 能通过 `request_id` 跳转到对应 LLM request / response。
- [ ] SceneStateExtractor / CognitivePass / ArbitrationFallback / SurfaceRealizer 的 request、response、schema、状态、耗时都被记录。
- [ ] 流式输出保存原始 chunk 顺序，并生成 `assembled_text` / `readable_text`。
- [ ] API Key、Authorization header、Provider secret、代理认证不会进入 SQLite。
- [ ] CognitivePass schema 失败、程序修复、仲裁兜底都有 Trace 与异常事件。
- [ ] Agent 回滚后世界状态回退，运行 Logs 保留为审计记录。
- [ ] 全局 Logs 超过 1GB 后后台清理旧运行日志。
- [ ] 普通清理任务不会删除 Agent Trace 或仍被 `state_commit_records.trace_ids` 引用的记录。
- [ ] 30 天未更新且日志较大的 World 只产生提示事件，不自动删除。

### 阶段七：用户角色扮演

- [ ] 用户能扮演特定角色并影响仲裁。
