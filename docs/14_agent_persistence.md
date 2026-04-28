# 14 Agent SQLite 持久化

本文档承载 Agent 模式的 SQLite 表结构、索引与持久化边界。

数据语义见 [10_agent_data_model.md](10_agent_data_model.md)。Knowledge 访问派生索引的运行规则见 KnowledgeEntry 章节。运行时写入顺序见 [11_agent_runtime.md](11_agent_runtime.md)，日志表与清理策略见 [30_logging_and_observability.md](30_logging_and_observability.md)。

---

## 1. SQLite 表结构

按三层语义组织。Layer 2 不持久化（每回合重建）；Layer 1 / Layer 3 / Trace 各自独立。Agent 模式以 World 为故事连续性单元，聊天记录删除 / 回退必须从目标回合开始截断后续全部回合，并回滚到一致的世界状态，因此每个回合提交都必须有可追溯记录。

```sql
-- ===== Turn Commit（故事线与回滚锚点） =====

-- 世界故事线回合链；删除聊天时只能按 parent_turn_id 截断后续回合，不能单删中间消息
CREATE TABLE world_turns (
    scene_turn_id TEXT PRIMARY KEY,
    parent_turn_id TEXT,
    user_message TEXT NOT NULL,             -- JSON: 用户输入/扮演输入
    rendered_output TEXT,                   -- SurfaceRealizer 输出
    status TEXT NOT NULL DEFAULT 'active',  -- active / rolled_back
    created_at TEXT NOT NULL,
    rolled_back_at TEXT,
    FOREIGN KEY (parent_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- 每回合状态提交记录；用于定位需要回滚的人物、世界、知识和 trace 变化
CREATE TABLE state_commit_records (
    commit_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    changed_scene_snapshot_ids TEXT NOT NULL,       -- JSON array
    changed_knowledge_ids TEXT NOT NULL,            -- JSON array
    changed_character_ids TEXT NOT NULL,            -- JSON array
    changed_subjective_snapshot_ids TEXT NOT NULL,  -- JSON array
    trace_ids TEXT NOT NULL,                        -- JSON array
    rollback_patch TEXT NOT NULL,                   -- JSON: 反向补丁或变更前镜像
    created_at TEXT NOT NULL,
    rolled_back_at TEXT,
    rollback_reason TEXT,
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- ===== Layer 1: Truth Store =====

-- 场景快照（客观场景状态）
CREATE TABLE scene_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    scene_id TEXT NOT NULL,
    scene_turn_id TEXT NOT NULL,
    scene_model TEXT NOT NULL,             -- JSON: SceneModel
    created_at TEXT NOT NULL
);

-- 统一知识库（世界/地区/势力/角色档案/记忆）
CREATE TABLE knowledge_entries (
    knowledge_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,                    -- world_fact / region_fact / faction_fact / character_facet / memory
    subject_type TEXT NOT NULL,            -- world / region / faction / character / event
    subject_id TEXT,                       -- region/faction/character/event 的具体 ID（World 时为 NULL）
    facet_type TEXT,                       -- 仅 character_facet 有值
    content TEXT NOT NULL,                 -- JSON: 客观真相
    apparent_content TEXT,                 -- JSON: 表象（可空）
    access_policy TEXT NOT NULL,           -- JSON: AccessPolicy
    subject_awareness TEXT NOT NULL,       -- JSON: SubjectAwareness（含 Unaware 的 self_belief）
    metadata TEXT NOT NULL,                -- JSON: KnowledgeMetadata
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 知识揭示事件（访问权限扩展轨迹）
CREATE TABLE knowledge_reveal_events (
    event_id TEXT PRIMARY KEY,
    knowledge_id TEXT NOT NULL,
    newly_known_by TEXT NOT NULL,          -- JSON array
    trigger TEXT NOT NULL,                 -- JSON: RevealTrigger
    scope_change TEXT,                     -- JSON: AccessScopeChange；GodOnly 揭示时必须有值
    scene_turn_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (knowledge_id) REFERENCES knowledge_entries(knowledge_id)
);

-- Knowledge 访问派生索引；权威来源仍是 knowledge_entries.access_policy JSON
CREATE TABLE knowledge_access_known_by (
    knowledge_id TEXT NOT NULL,
    character_id TEXT NOT NULL,
    PRIMARY KEY (knowledge_id, character_id),
    FOREIGN KEY (knowledge_id) REFERENCES knowledge_entries(knowledge_id)
);

CREATE TABLE knowledge_access_scopes (
    knowledge_id TEXT NOT NULL,
    scope_type TEXT NOT NULL,              -- public / god_only / region / faction / realm / role / bloodline / ...
    scope_value TEXT NOT NULL DEFAULT '',  -- 无值 scope 使用空字符串
    PRIMARY KEY (knowledge_id, scope_type, scope_value),
    FOREIGN KEY (knowledge_id) REFERENCES knowledge_entries(knowledge_id)
);

CREATE TABLE character_scope_memberships (
    character_id TEXT NOT NULL,
    scope_type TEXT NOT NULL,              -- region / faction / realm / role / bloodline / ...
    scope_value TEXT NOT NULL,
    source_knowledge_id TEXT,              -- 可空；若 membership 源自 KnowledgeEntry，则记录来源
    updated_at TEXT NOT NULL,
    PRIMARY KEY (character_id, scope_type, scope_value)
);

-- 角色基本档案（仅 baseline_body_profile + mind_model_card_knowledge_id 指针；其余事实在 knowledge_entries 中）
CREATE TABLE character_records (
    character_id TEXT PRIMARY KEY,
    baseline_body_profile TEXT NOT NULL,   -- JSON
    mind_model_card_knowledge_id TEXT NOT NULL,
    temporary_body_state TEXT NOT NULL,    -- JSON: Layer 1 当前身体/资源状态
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ===== Layer 3: Subjective State =====

-- 角色主观状态快照（每回合 cognitive pass 后写入）
CREATE TABLE character_subjective_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL,
    scene_turn_id TEXT NOT NULL,
    belief_state TEXT NOT NULL,            -- JSON
    emotion_state TEXT NOT NULL,           -- JSON
    relation_models TEXT NOT NULL,         -- JSON
    current_goals TEXT NOT NULL,           -- JSON
    created_at TEXT NOT NULL
);

-- ===== Trace / Logs（调试、回放与运行观测） =====

CREATE TABLE turn_traces (
    trace_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    trace_kind TEXT NOT NULL,              -- turn / character / presentation / rollback
    character_id TEXT,                     -- NULL 表示全局回合 trace
    summary TEXT NOT NULL,                 -- JSON: 回合级关键产物索引与摘要
    linked_request_ids TEXT NOT NULL,       -- JSON array: 关联 llm_call_logs.request_id
    linked_event_ids TEXT NOT NULL,         -- JSON array: 关联 app_event_logs.event_id
    created_at TEXT NOT NULL,
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

CREATE TABLE agent_step_traces (
    step_trace_id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    scene_turn_id TEXT NOT NULL,
    character_id TEXT,                     -- NULL 表示全局步骤
    step_name TEXT NOT NULL,               -- active_set / dirty_flags / scene_filter / cognitive_pass / validation / outcome_planning / effect_validation / state_commit 等
    step_status TEXT NOT NULL,             -- started / skipped / succeeded / failed / fallback_used
    input_summary TEXT,                    -- JSON: 结构化输入摘要
    output_summary TEXT,                   -- JSON: 结构化输出摘要
    decision_json TEXT,                    -- JSON: 关键判定值、跳过原因、验证失败项
    linked_request_id TEXT,                -- 对应 llm_call_logs.request_id
    error_event_id TEXT,                   -- 对应 app_event_logs.event_id
    created_at TEXT NOT NULL,
    FOREIGN KEY (trace_id) REFERENCES turn_traces(trace_id),
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

CREATE TABLE llm_call_logs (
    request_id TEXT PRIMARY KEY,
    mode TEXT NOT NULL,                    -- st / agent
    world_id TEXT,
    scene_turn_id TEXT,
    trace_id TEXT,
    character_id TEXT,
    llm_node TEXT NOT NULL,                -- STChat / SceneStateExtractor / CharacterCognitivePass / OutcomePlanner / SurfaceRealizer
    api_config_id TEXT NOT NULL,           -- 调用时实际使用的 API 配置
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    call_type TEXT NOT NULL,               -- chat / chat_structured / chat_stream
    request_json TEXT NOT NULL,            -- JSON: 写入前必须脱敏
    schema_json TEXT,                      -- JSON Schema，仅 structured 调用有值
    response_json TEXT,                    -- JSON: 原始响应或结构化结果
    assembled_text TEXT,                   -- stream chunk 直接拼接文本
    readable_text TEXT,                    -- 仅展示用的段落化文本
    status TEXT NOT NULL,                  -- started / succeeded / failed / cancelled
    latency_ms INTEGER,
    token_usage TEXT,                      -- JSON
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_summary TEXT,
    redaction_applied INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id),
    FOREIGN KEY (trace_id) REFERENCES turn_traces(trace_id)
);

-- Agent LLM 节点配置档案；api_config_id 指向 ./data/api_configs/ 中的 ST/API 配置池
CREATE TABLE agent_llm_profiles (
    profile_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    default_api_config_id TEXT NOT NULL,
    bindings TEXT NOT NULL,                -- JSON: Vec<AgentLlmConfigBinding>
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- World 级 Agent LLM 配置选择；允许每个 World 使用不同的四节点配置
CREATE TABLE world_agent_settings (
    world_id TEXT PRIMARY KEY,
    agent_llm_profile_id TEXT NOT NULL,
    profile_overrides TEXT,                -- JSON: 可选 World 级覆盖
    updated_at TEXT NOT NULL,
    FOREIGN KEY (agent_llm_profile_id) REFERENCES agent_llm_profiles(profile_id)
);

CREATE TABLE llm_stream_chunks (
    chunk_id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    raw_chunk TEXT NOT NULL,
    received_at TEXT NOT NULL,
    FOREIGN KEY (request_id) REFERENCES llm_call_logs(request_id)
);

CREATE TABLE app_event_logs (
    event_id TEXT PRIMARY KEY,
    level TEXT NOT NULL,                   -- debug / info / warn / error / fatal
    event_type TEXT NOT NULL,
    message TEXT NOT NULL,
    source_module TEXT NOT NULL,
    request_id TEXT,
    world_id TEXT,
    scene_turn_id TEXT,
    trace_id TEXT,
    character_id TEXT,
    detail_json TEXT,                      -- JSON: 异常上下文，写入前必须脱敏
    created_at TEXT NOT NULL
);

CREATE TABLE log_retention_state (
    retention_id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,                   -- global / world
    world_id TEXT,
    size_limit_bytes INTEGER NOT NULL DEFAULT 1073741824,
    current_size_bytes INTEGER,
    last_checked_at TEXT,
    last_cleanup_at TEXT,
    cleanup_needed INTEGER NOT NULL DEFAULT 0,
    user_prompt_required INTEGER NOT NULL DEFAULT 0,
    detail_json TEXT
);

-- ===== 索引 =====

CREATE INDEX idx_scene_snapshots_scene ON scene_snapshots(scene_id);
CREATE INDEX idx_world_turns_parent ON world_turns(parent_turn_id);
CREATE INDEX idx_commit_records_turn ON state_commit_records(scene_turn_id);
CREATE INDEX idx_knowledge_kind ON knowledge_entries(kind);
CREATE INDEX idx_knowledge_subject ON knowledge_entries(subject_type, subject_id);
CREATE INDEX idx_knowledge_facet ON knowledge_entries(subject_id, facet_type) WHERE kind = 'character_facet';
CREATE INDEX idx_reveal_knowledge ON knowledge_reveal_events(knowledge_id);
CREATE INDEX idx_access_known_by_character ON knowledge_access_known_by(character_id);
CREATE INDEX idx_access_scopes_lookup ON knowledge_access_scopes(scope_type, scope_value);
CREATE INDEX idx_character_scope_lookup ON character_scope_memberships(character_id, scope_type, scope_value);
CREATE INDEX idx_subjective_char ON character_subjective_snapshots(character_id, scene_turn_id);
CREATE INDEX idx_traces_turn ON turn_traces(scene_turn_id);
CREATE INDEX idx_step_traces_turn ON agent_step_traces(scene_turn_id);
CREATE INDEX idx_step_traces_trace ON agent_step_traces(trace_id);
CREATE INDEX idx_llm_logs_turn ON llm_call_logs(scene_turn_id);
CREATE INDEX idx_llm_logs_trace ON llm_call_logs(trace_id);
CREATE INDEX idx_llm_logs_api_config ON llm_call_logs(api_config_id);
CREATE INDEX idx_llm_logs_created ON llm_call_logs(created_at);
CREATE INDEX idx_stream_chunks_request ON llm_stream_chunks(request_id, chunk_index);
CREATE INDEX idx_app_events_context ON app_event_logs(world_id, scene_turn_id, trace_id);
CREATE INDEX idx_app_events_created ON app_event_logs(created_at);
```

**说明**：

- `knowledge_entries.access_policy` JSON 是权威结构；`knowledge_access_known_by`、`knowledge_access_scopes` 与 `character_scope_memberships` 是可重建的查询索引，只用于候选预筛，最终访问权限仍由 `KnowledgeAccessResolver` 判定。
- `subject_id + facet_type` 联合索引服务"取角色 X 的所有 facets"这一最高频查询。
- `character_subjective_snapshots` 的最新一条即角色当前心智状态；历史快照保留用于回放与一致性验证。
- 没有"memory_records"表，记忆作为 `knowledge_entries.kind = 'memory'` 统一存储。
- `world_turns.parent_turn_id` 定义故事线顺序；用户删除某条 Agent 聊天记录时，必须将该回合及其后续全部回合标记为 `rolled_back`，禁止单独删除中间消息，并按 `state_commit_records.rollback_patch` 恢复到目标父回合的一致 Layer 1 / Layer 3 状态。
- Agent Trace 以 `scene_turn_id` 为主轴，解释回合如何演化；运行 Logs 以 `request_id` / `event_id` 为主轴，解释应用运行时发生了什么。两者可互相关联，但日志不得作为 Agent 判断或 LLM 输入来源。
- Agent 模式允许四类 LLM 节点绑定不同 API 配置；`agent_llm_profiles.bindings` 保存用户选择，`llm_call_logs.api_config_id` 保存每次调用实际使用的配置。
- `llm_call_logs.request_json`、`response_json`、`llm_stream_chunks.raw_chunk` 尽量还原 Provider 原貌；`readable_text` 仅用于流式响应的段落化查看，不替代原始响应。
- `app_event_logs` 同表结构可用于 `./data/logs/app_logs.sqlite`。ST 模式只写全局运行 Logs；Agent 模式与回合相关的记录写入对应 `world.sqlite`，同时可在全局异常日志中保留带 `world_id` / `scene_turn_id` 的索引事件。
- 默认清理上限为 1GB。自动清理只处理全局运行 Logs；Agent Trace 和仍被 `state_commit_records.trace_ids` 引用的记录不自动删除。30 天以上未更新且日志体积较大的 World 只产生提示事件，等待用户确认。
