# 17 Agent Knowledge 模型

本文档承载 Agent 统一知识库 KnowledgeEntry、访问策略、内容 schema、过去线 TruthGuidance 与 KnowledgeRevealEvent。

三层数据语义见 [10_agent_data_model.md](10_agent_data_model.md)。地点事实继承见 [15_agent_location_system.md](15_agent_location_system.md)。持久化表结构见 [14_agent_persistence.md](14_agent_persistence.md)。

---

## 1. KnowledgeEntry（统一知识模型）

`KnowledgeEntry` 是 Layer 1 的核心，承载世界设定 / 地点与地区设定 / 势力设定 / 角色档案分面 / 历史事件约束 / 角色记忆。所有"谁能读取什么 Knowledge"的判断由它的 `access_policy` 字段决定，由 `KnowledgeAccessResolver` 统一计算。

```rust
pub struct KnowledgeEntry {
    pub knowledge_id: String,
    pub kind: KnowledgeKind,
    pub subject: KnowledgeSubject,
    pub content: serde_json::Value,                  // 客观真相（结构化）
    pub apparent_content: Option<serde_json::Value>, // 表象（隐藏/伪装/欺骗时对观察者可访问的版本）
    pub access_policy: AccessPolicy,
    pub subject_awareness: SubjectAwareness,         // 仅 subject 为 Character 时有意义
    pub metadata: KnowledgeMetadata,
    pub schema_version: String,
}

pub enum KnowledgeKind {
    WorldFact,        // 世界级设定（宇宙规则、修真体系）
    RegionFact,       // 地点/地区设定（地理、风俗、气候、禁令；subject_id 指向 LocationNode）
    FactionFact,      // 势力设定（玄天宗内规、口诀）
    CharacterFacet,   // 角色档案分面（外貌/身份/能力/血脉/...）
    HistoricalEvent,  // 正史事件约束；用于过去线 Truth 引导与冲突检测
    Memory,           // 记忆/事件（亲历或传闻）
}

pub enum KnowledgeSubject {
    World,
    Region(String),   // LocationNode.location_id；保留 Region 命名表示地理主体
    Faction(String),
    Character { id: String, facet: CharacterFacetType },
    Event { event_id: String },
}

pub enum CharacterFacetType {
    Appearance,        // 外观（可被同场景观察）
    Identity,          // 公开身份/称谓
    TrueName,          // 真实姓名
    Species,           // 种族
    Bloodline,         // 血脉
    CultivationRealm,  // 修为境界
    KnownAbility,      // 已展示的能力
    HiddenAbility,     // 隐藏能力
    Personality,       // 性格特质
    Background,        // 出身背景
    Motivation,        // 动机
    Trauma,            // 创伤
    MindModelCard,     // 认知基线卡：注意力、风险偏好、常用推理模式、价值排序
    // 可扩展
}

pub struct AccessPolicy {
    // 三谓词，OR 关系（任一为真即可访问）。
    // 例外：scope 含 GodOnly 时为 hard deny，优先级高于 known_by / scope / conditions。
    pub known_by: Vec<String>,                  // 名单制
    pub scope: Vec<AccessScope>,            // 标签制
    pub conditions: Vec<AccessCondition>,   // 条件制（运行时求值）
}

pub enum AccessScope {
    Public,                  // 所有原住民
    GodOnly,                 // 仅编排器（无人可知；hard deny）
    Region(String),          // 在该地区的角色
    Faction(String),         // 该势力成员
    Realm(String),           // 修为门槛及以上
    Role(String),            // 担任该职位
    Bloodline(String),       // 该血脉
    // 可扩展
}

pub enum AccessCondition {
    InSameSceneObservable,                                    // 同场景且能感知
    SocialAccessAtLeast { target: String, threshold: f64 }, // L1 客观关系/授权阈值；禁止读取 L3 relation_models
    HasSkill(String),                                      // 拥有特定技能
    CultivationAtLeast(String),                            // 修为达到
    CustomPredicate(AccessExpression),                 // 结构化 DSL AST 扩展点；禁止自然语言表达式
    // 可扩展
}

pub enum AccessExpression {
    All(Vec<AccessExpression>),
    Any(Vec<AccessExpression>),
    Not(Box<AccessExpression>),
    HasTag { subject_id: String, tag: String },
    NumericAtLeast { path: String, value: f64 },
    BooleanFlag { path: String, expected: bool },
}

pub enum SubjectAwareness {
    /// 默认：subject 自己知道关于自己的这条 facet。
    /// 在为 subject 构建 accessible_knowledge 时，content 直接可访问。
    Aware,

    /// subject 不知道客观真相，但有一个"自以为是"的版本。
    /// 在为 subject 构建 accessible_knowledge 时：返回 self_belief，content 保持隐藏。
    Unaware { self_belief: serde_json::Value },
}

pub struct KnowledgeMetadata {
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub valid_from: Option<TimeAnchor>,
    pub valid_until: Option<TimeAnchor>,
    pub source_session_id: Option<String>,
    pub source_scene_turn_id: Option<String>,
    pub derived_from_event_id: Option<String>,
    // Memory 专用（其他 kind 留空）
    pub emotional_weight: Option<f64>,
    pub last_accessed_at: Option<DateTime>,
    pub source: Option<String>,                  // 知识来源：witnessed / told_by / inferred
}
```

**关键不变量**：

1. `content` 永远不进入 LLM，除非 `KnowledgeAccessResolver` 判定该角色对该 entry 拥有完整访问权限。
2. `subject == Character{id: A}` 且 `subject_awareness == Unaware{self_belief}` 时：A 的 accessible_knowledge 中只见 `self_belief`，看不到 `content`。
3. `apparent_content` 存在时：观察者（非 subject）默认看到 `apparent_content`；只有满足"揭穿条件"或在 `known_by` 中的角色才看到 `content`。
4. `access_policy.scope` 含 `GodOnly` 表示仅编排器可读，对所有角色拒绝访问；`KnowledgeAccessResolver` 必须先检查 `GodOnly`，命中后直接拒绝，不再计算 `known_by` / 其他 scope / conditions。
5. `GodOnly` 启用态下 `access_policy.known_by` 必须为空；Validator / StateCommitter 自动检查并拒绝 `GodOnly + known_by 非空` 的状态。
6. 若故事推进后 OutcomePlanner 候选 + EffectValidator 确认某条 `GodOnly` 知识可被角色获知，必须通过 `KnowledgeRevealEvent` 先移除 `GodOnly` 或降级为其他 scope，再追加 `known_by`；禁止在 `GodOnly` 仍存在时直接写入 `known_by`。
7. `HistoricalEvent` 表示正史层面的事件约束与已知结果；`Memory` 表示某个角色亲历、听闻或推断出的主观记忆。两者不能混用。
8. Layer 1 的 Knowledge 内容由编排器/作者/StateCommitter 写入；CognitivePass 不可写。
9. `summary_text` 只供 LLM 阅读，不参与过去线硬约束判断；过去线仲裁只能读取 `HistoricalEvent` 的结构化字段。

#### 2.4.1 访问策略存储与查询索引

`KnowledgeEntry.access_policy` 是权威结构，必须完整保存在 `knowledge_entries.access_policy` JSON 中，用于导入导出、回滚、schema 校验和 `KnowledgeAccessResolver` 最终判定。

为避免在高频 `KnowledgeAccess` 中扫描全库，`known_by` 与 `scope` 同时维护为 SQLite 派生索引：

- `knowledge_access_known_by(knowledge_id, character_id)`：展开 `access_policy.known_by`。
- `knowledge_access_scopes(knowledge_id, scope_type, scope_value)`：展开 `access_policy.scope`，`Public` / `GodOnly` 等无值 scope 使用空字符串。
- `character_scope_memberships(character_id, scope_type, scope_value)`：角色当前所属地区、势力、修为门槛、职位、血脉等可查询 membership。

派生索引只用于候选预筛，不是第二套访问规则。`KnowledgeAccess` 的固定流程为：

1. 根据当前角色、场景、可观察实体、近期事件与 `character_scope_memberships` 查询候选 `knowledge_id`。
2. 批量读取候选 `KnowledgeEntry`。
3. 对每条候选调用 `KnowledgeAccessResolver`；`GodOnly` 仍由 Resolver hard deny。
4. 按 `content` / `apparent_content` / `self_belief` 三选一生成 `AccessibleEntry.accessible_content`。

任何写入 `KnowledgeEntry.access_policy`、处理 `KnowledgeRevealEvent`、改变角色地区/势力/职位/血脉/修为等访问身份的操作，必须在同一 SQLite transaction 内同步更新派生索引。索引表可由 `knowledge_entries.access_policy` 全量重建；若重建结果与现有索引不一致，视为存储一致性错误。

## 2. Content Schema 约定（核心字段 + extensions 兜底）

`content` / `apparent_content` / `self_belief` 三者共享同一套 sub-schema，按 `kind` × `facet_type` 决定字段定义。

**通用规则**：
- 必含 `summary_text: string`（自由文本简述，供 LLM 快速理解；不参与程序判断）。
- 必含该 kind/facet 的核心结构化字段（用于程序检索 / 访问控制 / 规则匹配）。
- 可含 `extensions: Record<String, Any>` 用于扩展（不参与核心程序逻辑，可供 LLM 阅读）。

**示例：CharacterFacet::Appearance**

```json
{
  "summary_text": "高个子瘦削男人，黑发，左脸有疤，蓝袍白裤",
  "height": "tall",
  "build": "lean",
  "hair": {"color": "black", "style": "long_loose"},
  "distinctive_marks": ["scar_on_left_cheek"],
  "clothing": {"upper": "blue_robe", "lower": "white_pants"},
  "extensions": {}
}
```

**示例：CharacterFacet::CultivationRealm**

```json
{
  "summary_text": "金丹中期",
  "realm": "golden_core",
  "stage": "mid",
  "progress_within_stage": 0.6,
  "extensions": {}
}
```

**示例：CharacterFacet::HiddenAbility**

```json
{
  "summary_text": "可短暂凝聚虚空裂隙",
  "ability_id": "void_rift_summon",
  "category": "soul",
  "trigger_condition": "extreme_duress",
  "extensions": {}
}
```

**示例：KnowledgeKind::FactionFact**

```json
{
  "summary_text": "玄天宗执事可调用三品法宝",
  "rule_id": "xuantian_rank_3_artifact",
  "applies_to": {"role": "executor"},
  "extensions": {}
}
```

**示例：CharacterFacet::MindModelCard**

```json
{
  "summary_text": "谨慎、重承诺、遇到未知威胁时先保护同伴再试探",
  "attention_biases": ["protect_allies", "notice_hidden_threats"],
  "risk_tolerance": "low",
  "default_social_strategy": "polite_probe",
  "value_priorities": ["promise", "ally_safety", "truth"],
  "extensions": {}
}
```

**示例：KnowledgeKind::RegionFact**

```json
{
  "summary_text": "云北州民风尚武",
  "fact_type": "customs",
  "applies_to_location_id": "yunbei_state",
  "inheritance": {
    "inheritable": true,
    "applies_to_descendants": true,
    "max_depth": null,
    "blocked_location_ids": [],
    "override_policy": "child_overrides_parent"
  },
  "confidence": "asserted",
  "extensions": {}
}
```

`RegionFact` 的 `applies_to_location_id` 必须指向 `LocationNode.location_id`。继承只沿 `LocationNode.parent_id` 链计算；继承会扩大候选事实范围，但不会绕过 `KnowledgeAccessResolver`，也不会把父级事实复制成子地点的新 KnowledgeEntry。

**示例：KnowledgeKind::Memory**

```json
{
  "summary_text": "在庭院中被 Bob 突然反目袭击",
  "event_type": "betrayal",
  "actor": "bob",
  "target": "self",
  "location": "courtyard",
  "timestamp": "2 weeks ago",
  "key_observations": ["bob_smiled_then_attacked"],
  "extensions": {}
}
```

**示例：KnowledgeKind::HistoricalEvent**

```json
{
  "summary_text": "十年前，乙在青石镇背叛丙，导致丙失去宗门信物。",
  "event_id": "betrayal_qingshi_10y_ago",
  "time_window": {
    "start": {"calendar_id": "main", "ordinal": 12000, "precision": "day", "display_text": "十年前春末"},
    "end": {"calendar_id": "main", "ordinal": 12003, "precision": "day", "display_text": "三日内"}
  },
  "participants": [
    {"character_id": "yi", "role": "betrayer"},
    {"character_id": "bing", "role": "victim"}
  ],
  "required_outcomes": [
    {"outcome_id": "yi_betrays_bing", "domain": "relationship", "subject_id": "yi", "target_id": "bing"},
    {"outcome_id": "token_lost", "domain": "item_state", "subject_id": "sect_token"}
  ],
  "forbidden_outcomes": [
    {"outcome_id": "yi_never_betrays", "domain": "event_negation"},
    {"outcome_id": "bing_dies_here", "domain": "character_life_state", "subject_id": "bing"}
  ],
  "known_after_effects": [
    {"fact_ref": "bing_missing_sect_token", "valid_from_ordinal": 12004}
  ],
  "open_detail_slots": ["betrayal_motive", "exact_dialogue", "who_witnessed"]
}
```

`HistoricalEvent` 的 `required_outcomes` / `forbidden_outcomes` / `known_after_effects` 是过去线冲突检测的结构化依据。`open_detail_slots` 表示允许过去线补完但尚未定死的过程、动机、见证者、对白或局部支线。

**核心字段表**（所有 facet/fact 类型应预定义最小集）由 `models/knowledge_schemas.rs` 维护，每种类型一个 struct。`extensions` 总是 `serde_json::Map<String, serde_json::Value>` 兜底。

## 3. TruthGuidance（过去线引导输入）

`HistoricalTruthResolver` 根据 `period_anchor + location + participants` 收集相关 `HistoricalEvent`、当时可见状态、后续已知结果与禁止矛盾项，生成只读的 `TruthGuidance`。它不是新的持久事实，只是运行时输入。

```rust
pub struct TruthGuidance {
    pub session_id: String,
    pub period_anchor: TimeAnchor,
    pub related_event_ids: Vec<String>,
    pub hard_constraints: Vec<TruthConstraint>,
    pub soft_context: Vec<String>,                 // llm_readable
    pub open_detail_slots: Vec<OpenDetailSlot>,
    pub future_knowledge_warnings: Vec<String>,    // trace_only；不得进入受限角色节点
}

pub struct TruthConstraint {
    pub constraint_id: String,
    pub source_knowledge_id: String,
    pub constraint_kind: String,                   // required_outcome / forbidden_outcome / known_after_effect
    pub applies_to_refs: Vec<String>,
    pub structured_payload: serde_json::Value,
}

pub struct OpenDetailSlot {
    pub slot_id: String,
    pub source_event_id: String,
    pub detail_kind: String,                       // motive / dialogue / witness / route / local_cause ...
    pub promotion_policy: String,                  // promote_if_consistent / trace_only
}
```

`TruthGuidance` 可进入 SceneInitializer、SceneStateExtractor、OutcomePlanner 和冲突检测器的 God-read 输入域，但不得进入 `CharacterCognitivePassInput`。SurfaceRealizer 只能通过 `NarrationScope` 派生出的可叙述事实接触其中允许披露的部分。

## 4. KnowledgeRevealEvent（访问权限扩展）

访问权限变化必须通过显式事件触发，禁止隐式修改 `access_policy.known_by`：

```rust
pub struct KnowledgeRevealEvent {
    pub event_id: String,
    pub knowledge_id: String,
    pub newly_known_by: Vec<String>,   // 此次新增的知情者
    pub trigger: RevealTrigger,        // 何种触发：witnessed / told / inferred / awakened
    pub scope_change: Option<AccessScopeChange>, // GodOnly 揭示时必须先降级/移除 GodOnly
    pub scene_turn_id: String,
}

pub enum AccessScopeChange {
    RemoveGodOnly,                     // 结果规划 + 程序校验确认该知识已可进入角色可知范围
    ReplaceScopes(Vec<AccessScope>),
}
```

由 StateCommitter 处理：
- 若原 entry 含 `GodOnly`，必须先验证 `scope_change` 已移除/替换 `GodOnly`，否则拒绝追加 `newly_known_by`。
- 更新 `KnowledgeEntry.access_policy.scope` 与 `access_policy.known_by`。
- 在 event_stream 追加事件。
- 创建一条 `KnowledgeEntry { kind: Memory }` 记录"X 何时如何获知 Y"。
