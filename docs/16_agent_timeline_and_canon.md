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
    pub player_character_id: Option<String>,
    pub canon_status: SessionCanonStatus,
    pub conflict_policy: Option<ConflictPolicyDecision>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
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
