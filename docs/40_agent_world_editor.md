# 40 Agent 世界编辑器

本文档承载 Agent 模式 World Editor 的产品边界、数据写入规则、前后端接口、校验规则与首版验收标准。

World Editor 是作者与维护者编辑 Agent World 结构化 Truth 的入口。它服务于开局前创建世界，也服务于运行中暂停后的安全修订；它不是 LLM 节点，不参与回合推理，不绕过运行时验证边界。

相关基础文档：

- 总体架构、LLM / 程序边界与关键铁律见 [01_architecture.md](01_architecture.md)。
- 应用数据目录、前后端模块结构见 [02_app_data_and_modules.md](02_app_data_and_modules.md)。
- Agent 数据模型见 [10_agent_data_model.md](10_agent_data_model.md)。
- SQLite 表结构与 editor commit journal 见 [14_agent_persistence.md](14_agent_persistence.md)。
- 地点系统见 [15_agent_location_system.md](15_agent_location_system.md)。
- Knowledge 模型见 [17_agent_knowledge_model.md](17_agent_knowledge_model.md)。
- 角色模型见 [18_agent_character_model.md](18_agent_character_model.md)。
- 结构化文本编辑器见 [42_structured_text_editor.md](42_structured_text_editor.md)。

---

## 1. 目标与边界

### 1.1 首版目标

World Editor v1 支持四类结构化 CRUD：

1. **World settings**：World 显示信息、`world_base.yaml` 安全子集、Agent LLM profile 绑定摘要与跳转入口。
2. **LocationGraph**：`LocationNode`、`LocationAlias`、`LocationSpatialRelation`、`LocationEdge`、`LocationPolityTemplate`。
3. **KnowledgeEntry**：世界事实、地区事实、势力事实、角色分面、历史事件、记忆，以及访问策略、表象与自我认知。
4. **CharacterRecord**：基础属性、身体基线、灵力显露长期倾向、当前临时状态基础字段、`MindModelCard` 指针。

首版同时允许维护 L1 客观关系 / 授权：

- 当前主线 materialized cache：`objective_relationships`。
- 带故事时间的权威记录：`temporal_state_records(state_kind=objective_relation|authorization)`。
- 角色 scope membership：`character_scope_memberships`，仅作为 Knowledge 访问候选索引的输入之一。

### 1.2 明确不做

- 不做自由文本 / 旧资料的 LLM 自动抽取导入。
- 不直接编辑 `SceneModel` / `scene_snapshots`；场景由运行时、回合工作副本和 StateCommitter 管理。
- 不编辑 Layer 2 派生视图；Layer 2 每回合从 Layer 1 / Layer 3 重建。
- 不直接编辑 Layer 3 `character_subjective_snapshots`；主观状态由 CognitivePass / SubjectiveStateReducer / 用户扮演输入产生。
- 不编辑 Agent Trace、LLM Logs 或运行日志；日志只观察，不驱动业务。
- 不提供“强制改写正在执行的回合”能力。

---

## 2. 核心原则

1. **Editor 写的是结构化 Truth，不写 prompt 文本。** 所有持久化内容必须落到既有模型：Location、Knowledge、Character、TemporalState、ObjectiveRelationship 或 World rules。
2. **运行中修改只允许在 paused 状态。** 只要 World 有 active turn、未完成 LLM call、打开的写事务、正在提交的 StateCommitter 或未处理的回滚任务，World Editor 不允许提交。
3. **编辑器不是第二个 StateCommitter。** Editor commit 使用独立 journal 记录作者编辑，但仍必须复用同一套 schema 校验、访问派生索引维护、地点一致性校验和单写事务规则。
4. **表单是主入口，结构化文本编辑器是兜底。** 核心字段必须有类型化 UI；`content`、`apparent_content`、`self_belief`、`extensions`、`metadata`、低频 Provider 扩展字段可用 Structured Text Editor 的 JSON / YAML 模式编辑，但保存前必须解析为结构化值并通过业务 schema 校验。Plain 模式只允许用于 schema 声明为 string 的叶子字段。
5. **派生索引可重建，权威结构不可双写。** `knowledge_access_known_by`、`knowledge_access_scopes`、`character_scope_memberships` 是候选索引；`KnowledgeEntry.access_policy` 仍是权威。
6. **删除必须先做影响分析。** 删除地点、Knowledge、角色或关系前必须报告引用、继承、访问索引、历史事件、MindModelCard 指针、TemporalState 和 ObjectiveRelationship 影响。
7. **运行时安全优先于编辑便利。** 无法证明安全的运行中修改必须阻止，或要求用户先归档/回滚/关闭相关会话后再提交。

---

## 3. 编辑模型

### 3.1 UI 信息架构

首版使用四区结构：

- **World 导航**：World 列表、当前 paused/running 状态、主线光标摘要、最近 editor commit。
- **实体导航**：Locations、Knowledge、Characters、Relationships、World Rules。
- **编辑区**：类型化表单 + 只读派生摘要 + JSON 兜底字段。
- **校验与影响面板**：schema errors、blocking validation、warnings、impact summary、planned commit diff。

地点编辑需要同时支持树视图与关系视图：

- `parent_id` 树只表达层级归属。
- `LocationSpatialRelation` 视图表达自然地理覆盖 / 穿过 / 重叠。
- `LocationEdge` 视图表达路线 / 相邻 / 通行边。

Knowledge 编辑需要优先暴露访问策略：

- `known_by` 名单。
- `scope` 标签。
- `conditions` 结构化 DSL。
- `GodOnly` hard deny 状态。
- `apparent_content` 与 `subject_awareness` 分流预览。

角色编辑必须把“角色基础档案”和“角色事实分面”分开：

- `CharacterRecord` 只编辑六项基础属性、身体基线、灵力显露长期倾向、临时状态基础字段和 `MindModelCard` 指针。
- 外貌、身份、真名、血脉、能力、背景、动机、创伤、认知基线卡等写为 `KnowledgeEntry { kind: CharacterFacet }`。

### 3.2 保存流程

所有保存动作统一走 patch：

```rust
pub struct WorldEditorPatch {
    pub world_id: String,
    pub base_editor_revision: i64,
    pub operations: Vec<WorldEditorOperation>,
    pub author_note: Option<String>,
}

pub enum WorldEditorOperation {
    UpsertLocationNode(LocationNodeDraft),
    DeleteLocationNode { location_id: String },
    UpsertLocationAlias(LocationAliasDraft),
    DeleteLocationAlias { normalized_alias: String, location_id: String },
    UpsertLocationSpatialRelation(LocationSpatialRelationDraft),
    DeleteLocationSpatialRelation { relation_id: String },
    UpsertLocationEdge(LocationEdgeDraft),
    DeleteLocationEdge { edge_id: String },
    UpsertLocationPolityTemplate(LocationPolityTemplateDraft),
    DeleteLocationPolityTemplate { polity_id: String },
    UpsertKnowledgeEntry(KnowledgeEntryDraft),
    DeleteKnowledgeEntry { knowledge_id: String },
    UpsertCharacterRecord(CharacterRecordDraft),
    DeleteCharacterRecord { character_id: String },
    UpsertObjectiveRelationship(ObjectiveRelationshipDraft),
    DeleteObjectiveRelationship { relation_id: String },
    UpsertTemporalStateRecord(TemporalStateRecordDraft),
    DeleteTemporalStateRecord { state_record_id: String },
    UpsertWorldRules(WorldRulesDraft),
}
```

保存流程固定为：

1. 前端维护未提交 draft，不直接写 SQLite。
2. `validate_world_editor_patch` 返回 `ValidationReport` 与 `ImpactSummary`。
3. 若存在 blocker，禁止提交。
4. 用户确认 warning 与 destructive impact 后调用 `commit_world_editor_patch`。
5. 后端重新读取当前 revision 并复跑校验，防止 stale patch。
6. 单个 SQLite 写事务内写入权威表、同步派生索引、写入 editor commit journal。
7. 提交成功后前端刷新 snapshot 与 revision。

### 3.3 后端命令

```rust
pub async fn get_world_editor_snapshot(world_id: String) -> WorldEditorSnapshot;
pub async fn validate_world_editor_patch(world_id: String, patch: WorldEditorPatch) -> WorldEditorValidationResult;
pub async fn commit_world_editor_patch(world_id: String, patch: WorldEditorPatch) -> WorldEditorCommitResult;
pub async fn rebuild_world_editor_indexes(world_id: String) -> WorldEditorIndexRebuildResult;
```

`get_world_editor_snapshot` 返回编辑所需的结构化摘要，不返回运行日志全文或 LLM 请求响应。Knowledge content 可按列表摘要 + 单条详情懒加载实现；接口层不改变访问规则，因为 Editor 是作者权限入口，但 UI 必须清楚标出 GodOnly、私密和表象字段。

`validate_world_editor_patch` 只读数据库，不写入任何表。它必须在 paused / running 两种状态都可运行，便于用户预览修改影响；只有 `commit_world_editor_patch` 受 paused-only 硬限制。

`rebuild_world_editor_indexes` 只允许重建可派生索引并报告差异，不得根据索引反向改写 `KnowledgeEntry.access_policy`。

---

## 4. 校验规则

### 4.1 World 状态门禁

提交前必须满足：

- World 已打开或可独占加载。
- 无 active turn。
- 无 pending / running LLM call。
- 无 StateCommitter 写入中。
- 无 unresolved rollback task。
- 当前 editor revision 与 patch base revision 一致。

若 World 未启动运行时，仅需通过文件 / SQLite 独占写入检查。

### 4.2 Location 校验

- 每个 World 有且只有一个 `WorldRoot`。
- 除 `WorldRoot` 外，持久地点必须有 `parent_id`，除非 `status = pending_confirmation`。
- `parent_id` 不得形成环。
- `canonical_level` 必须与父级模板兼容；不兼容必须写入结构化 override reason。
- `type_label` 只影响显示，不改变程序层级。
- `NaturalRegion` 跨多个行政节点时必须用 `LocationSpatialRelation` 表达，不允许多父级或复制节点。
- 同一 alias 可指向多个地点；保存后 LocationResolver 必须保留 ambiguity。
- `LocationEdge.bidirectional = false` 时不得生成反向通行。
- 路程、耗时、置信度和来源必须可追溯。

### 4.3 Knowledge 校验

- `content`、`apparent_content`、`self_belief` 必须符合 `kind × facet_type` 子 schema；`summary_text` 只供 LLM 阅读，不参与程序判断。
- `access_policy.scope` 含 `GodOnly` 时，`known_by` 必须为空。
- 给 GodOnly 知识追加知情者必须通过运行时 `KnowledgeRevealEvent`；Editor 只能直接修改作者设定，但必须记录 editor commit，并在影响面板标出它绕过了剧情揭示链。
- `RegionFact.applies_to_location_id` 必须指向存在的 LocationNode。
- `CharacterFacet` 的 `subject_id` 必须指向存在的 CharacterRecord。
- `HistoricalEvent` 的 required / forbidden / known_after_effects 必须是结构化字段，不能只写自然语言。
- 保存 `access_policy` 时必须同事务更新 `knowledge_access_known_by` 与 `knowledge_access_scopes`。
- 索引全量重建结果必须与当前索引一致；不一致时提交失败并提示修复。

### 4.4 Character 校验

- 六项 `base_attributes` 存储为 f64；普通 UI 可按整数展示，但保存不得因展示取整改写未编辑的小数。
- `mind_model_card_knowledge_id` 必须指向同一角色的 `KnowledgeEntry { kind: CharacterFacet, facet = MindModelCard }`。
- `mana_expression_tendency` 只表示长期倾向，不表示当前场景封息 / 外放。
- `temporary_state.mana_expression.display_ratio` 与 `pressure_ratio` 若被编辑，必须能由 WorldRulesSnapshot 或合法来源解释；普通作者 UI 默认只暴露 mode / intentionality / source / expiry。
- 角色删除前必须报告所有 CharacterFacet、Memory、HistoricalEvent participants、ObjectiveRelationship、TemporalStateRecord、session player_character_id 和 scene entity 引用。

### 4.5 关系与时态校验

- `objective_relationships` 是当前主线 materialized cache；按 TimeAnchor 查询必须依赖 `temporal_state_records`。
- 编辑授权、职位、通行许可、誓约关系时，必须同时维护可追溯的 temporal state，除非该关系被明确标为非时态导入草稿。
- `SocialAccessAtLeast` 只能读取 L1 客观关系 / 授权，不允许引用 L3 relation_models。
- 修改角色 scope membership 后，必须重新验证相关 Knowledge 访问候选索引。

---

## 5. Commit Journal

World Editor 的作者提交不属于 runtime turn，因此不写 `state_commit_records`。后端必须写入独立 editor commit journal，用于审计、影响追踪和未来回滚：

```rust
pub struct WorldEditorCommit {
    pub commit_id: String,
    pub world_id: String,
    pub base_editor_revision: i64,
    pub resulting_editor_revision: i64,
    pub changed_location_ids: Vec<String>,
    pub changed_knowledge_ids: Vec<String>,
    pub changed_character_ids: Vec<String>,
    pub changed_relationship_ids: Vec<String>,
    pub changed_temporal_state_ids: Vec<String>,
    pub changed_config_keys: Vec<String>,
    pub rollback_patch: serde_json::Value,
    pub author_note: Option<String>,
    pub validation_summary: serde_json::Value,
    pub created_at: DateTime,
}
```

Editor commit 与 runtime commit 的边界：

- `state_commit_records`：只记录由回合推进产生的 canonical 状态提交，必须有 `scene_turn_id`。
- `world_editor_commits`：只记录作者编辑产生的结构化变更，不伪造 `scene_turn_id`。
- 后续如果需要把 editor commit 回滚为剧情事件，必须通过单独的修正 / 重放流程，不直接把 editor commit 当成 world turn。

---

## 6. 前端实现边界

建议模块结构：

```text
src/components/agent/world-editor/
├── WorldEditorShell.vue
├── WorldEditorEntityNav.vue
├── LocationGraphEditor.vue
├── KnowledgeEntryEditor.vue
├── CharacterRecordEditor.vue
├── RelationshipEditor.vue
├── WorldRulesEditor.vue
├── ValidationPanel.vue
└── ImpactSummaryPanel.vue

src/stores/
└── agentWorldEditor.ts

src/types/agent/
└── worldEditor.ts
```

UI 状态要求：

- draft、validation result、commit result 分开保存。
- 列表摘要与详情编辑分开加载。
- destructive operation 必须显示影响摘要。
- paused 状态不满足时，保存按钮禁用，但仍可编辑 draft 和运行 validation。
- 对 GodOnly、private、apparent、self_belief 字段使用明显标识，避免作者误以为这是角色可见内容。
- KnowledgeEntry 的 `content`、`apparent_content`、`self_belief` 使用 Structured Text Editor；顶层 structured content 不允许以 Plain 模式提交。
- World Rules 高级编辑视图使用 YAML 模式，但保存仍必须经过 `ConfigValidator` 和 World Editor validation。

---

## 7. 测试与验收

首版必须覆盖：

- 运行中有 active turn / LLM call 时提交失败。
- 无 active turn 时同一 patch 单事务提交成功，并写入 editor commit journal。
- stale `base_editor_revision` 提交失败。
- Location parent 环检测、WorldRoot 唯一检测、NaturalRegion 跨域关系检测。
- alias 一对多保存后 LocationResolver 返回 ambiguity。
- Knowledge GodOnly + known_by 被拒绝。
- `access_policy` 保存后派生索引全量重建一致。
- RegionFact 指向不存在 LocationNode 时提交失败。
- CharacterRecord 的 MindModelCard 指针非法时提交失败。
- 删除被引用地点 / 角色 / Knowledge 时返回 blocking impact。
- editor rollback patch 可恢复权威表与派生索引。
- `KnowledgeEntry.content` / `apparent_content` / `self_belief` 的 JSON / YAML 文本解析失败时禁止提交；Plain 顶层提交 structured content 时返回 blocker。

文档阶段验证：

- `git diff -- README.md docs AGENTS.md`
- `git status --short`
