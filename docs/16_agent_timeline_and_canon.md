# 16 Agent 时间线与正史资格

本文档承载 Agent World 的时间锚点、会话时期、主线光标、过去线正史资格与冲突处理规则。

三层数据语义见 [10_agent_data_model.md](10_agent_data_model.md)。运行时处理见 [11_agent_runtime.md](11_agent_runtime.md)。SQLite 持久化见 [14_agent_persistence.md](14_agent_persistence.md)。

---

## 1. 时间锚点、会话与正史资格

Agent World 使用一条 canonical Truth 作为正史。多份聊天记录不是多套世界状态，而是同一 World 下的 `AgentSession`：用户选择特定时期、地点、扮演人物和叙事视角进入世界。

```rust
pub struct WorldMainlineCursor {
    pub world_id: String,
    pub timeline_id: String,                 // 第一版固定为 "main"
    pub mainline_head_turn_id: Option<String>,
    pub mainline_time_anchor: TimeAnchor,
    pub updated_at: DateTime,
}

pub struct AgentSession {
    pub session_id: String,
    pub world_id: String,
    pub title: String,
    pub session_kind: AgentSessionKind,
    pub period_anchor: TimeAnchor,
    pub player_mode: PlayerMode,
    pub player_character_id: Option<String>,
    pub canon_status: SessionCanonStatus,
    pub conflict_policy: Option<ConflictPolicyDecision>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

pub enum PlayerMode {
    Character,          // 玩家扮演当前 World 内某个 character_id
    Director,           // 玩家以世界外导演身份输入；不直接扮演角色
}

pub enum AgentSessionKind {
    Mainline,
    Retrospective,       // period_anchor < WorldMainlineCursor.mainline_time_anchor
    FuturePreview,       // period_anchor > WorldMainlineCursor.mainline_time_anchor；默认不入正史
}

pub enum SessionCanonStatus {
    CanonCandidate,      // 尚未发生硬冲突，候选细节可经校验提升
    PartiallyCanon,      // 冲突前可提升，冲突回合及之后非正史
    NonCanon,            // 整条会话非正史
}

pub enum SessionTurnCanonStatus {
    CanonCandidate,
    CanonPromoted,
    ConflictWarned,
    NonCanon,
}

pub enum RuntimeTurnCanonStatus {
    Canon,                 // 当前主线 canonical 回合；必须有 state_commit_records
    ProvisionalPromoted,   // 过去线候选已提升为 canonical；必须有 state_commit_records
    ProvisionalOnly,       // 过去线候选运行回合；可关联 Trace / Logs / provisional truth，但未提交 canonical
    NonCanon,              // 非正史运行回合；不得提交 canonical
    FuturePreview,         // 未来预演运行回合；不得提交 canonical
}

pub enum ConflictPolicyDecision {
    NonCanonAfterConflict,
    WholeSessionNonCanon,
}
```

`player_mode` 是会话级权限边界，不是 UI 辅助字段：

- `Character`：`player_character_id` 必填，且必须引用当前 World 内有效角色。
- `Director`：`player_character_id` 必须为空；导演输入只能走 `SceneNarration` / `DirectorHint` / `MetaCommand`，不得直接写任意 NPC 的 `IntentPlan` 或 L3 内心。
- 不允许用 `player_character_id = null` 同时表达“尚未选择角色”和“导演模式”；两者必须显式区分。

三类正史状态不得混用：

- `SessionCanonStatus` 描述整条 `AgentSession` 的正史资格。
- `SessionTurnCanonStatus` 描述会话消息在 UI / 聊天顺序中的状态。
- `RuntimeTurnCanonStatus` 描述 `world_turns` 运行回合是否产生 canonical 提交。

canonical Truth 的判定不只看状态字符串：只有 `world_turns.runtime_turn_status in (Canon, ProvisionalPromoted)` 且存在对应 `state_commit_records`，才表示本回合已经提交 canonical Layer 1 / Layer 3 / Knowledge 变化。典型映射：

| 场景 | SessionCanonStatus | SessionTurnCanonStatus | RuntimeTurnCanonStatus | state_commit_records |
|---|---|---|---|---|
| 当前主线成功推进 | CanonCandidate | CanonPromoted | Canon | 必须存在 |
| 过去线尚未提升 | CanonCandidate | CanonCandidate | ProvisionalOnly | 不存在 |
| 过去线细节提升入正史 | CanonCandidate / PartiallyCanon | CanonPromoted | ProvisionalPromoted | 必须存在 |
| 过去线硬冲突但继续游玩 | PartiallyCanon / NonCanon | ConflictWarned / NonCanon | NonCanon | 不存在 |
| 未来预演 | NonCanon | NonCanon | FuturePreview | 不存在 |

`TimeAnchor` 必须是程序可比较的结构化时间锚点，而不是只给 LLM 阅读的自然语言。不同 World 可在 `world_base.yaml` 中定义日历，但编译后的运行时必须能比较同一 World 内两个锚点的先后：

```rust
pub struct TimeAnchor {
    pub calendar_id: String,
    pub ordinal: i64,                         // World 内可排序时间刻度
    pub precision: TimePrecision,             // exact / day / period / era
    pub display_text: String,                 // llm_readable；不参与排序
}
```

可变化的 Layer 1 事实必须支持有效时间，至少在写入元数据中保存 `valid_from` / `valid_until` 或来源事件时间。角色位置、伤势、临时状态、客观关系 / 授权、Knowledge 揭示、地点状态和历史事件结果不得只覆盖“当前值”。运行时通过 `WorldStateAt(period_anchor)` 构建某一会话的工作视图，禁止过去线读取未来状态。

`WorldStateAt(period_anchor)` 的数据来源：

- `knowledge_entries.valid_from / valid_until`：世界事实、历史事件、记忆与可访问 Knowledge 的有效时间。
- `temporal_state_records`：角色位置、角色临时状态、地点状态、物品状态、客观关系 / 授权等可变化 L1 状态的时态权威。
- `objective_relationships`：当前主线客观关系与授权的 materialized cache / 高频索引；必须能追溯到对应 `temporal_state_records` 或来源 Knowledge。非当前时间点的 `WorldStateAt(period_anchor)` 不得只读取此表。
- `character_subjective_snapshots`：只用于读取某一时期的角色主观快照；非正史快照不得覆盖 canonical 当前心智。

`character_records.temporary_state`、`location_nodes.status`、`objective_relationships` 等当前态字段只是主线最新状态的 materialized cache，用于当前主线热路径读取；过去线、回滚复盘和冲突检测不得只读取这些当前态字段。

## 2. 会话视角与回退命令

阶段七的玩家入口采用“显式会话视角”：

- 用户在创建 `AgentSession` 时必须先选择 `player_mode`。
- `Character` 模式下，从当前 World 的 `CharacterRecord` 列表中选择一个角色；后续回合沿用该绑定，不支持在同一会话中悄然切换扮演对象。
- `Director` 模式下，不绑定角色；默认让全部角色继续按 Active Set + CognitivePass 运行，玩家只提供场景候选、导演偏置和元命令。

`/back` 的语义是“回退当前会话到某个历史 turn 并截断其后会话内容”，不是无条件物理删库：

- 若目标之后只包含当前会话的非 canonical / `provisional_only` / `future_preview` 内容，可安全截断会话显示顺序与非正史运行记录。
- 若目标之后包含 canonical turn、已提升的 `provisional_session_truth`、其他会话依赖或 `world_mainline_cursor` 依赖，必须先做依赖检查；存在依赖时默认阻止，不得把它降格成“只删聊天消息”。
- UI 可以把这类操作表述为“回退到此轮 / 截断此后会话”，底层仍按 rollback 与依赖校验执行。

`/fork` 的语义是“复制当前 World 副本并进入”，不是在同一 World 内增加第二条主线：

- 第一版 `timeline_id` 仍固定为 `main`；`/fork` 不改变这一约束。
- 复制后的新 World 拥有自己的 `world_id`、`WorldMainlineCursor`、会话列表、canonical Truth、Trace 与后续提交。
- 因此 `/fork` 适用于“保留当前世界，另开一个副本继续尝试”，而过去线 / 非正史则仍留在同一 World 的多时期会话语义内。
