# 14 Agent SQLite 持久化

本文档承载 Agent 模式的 SQLite 表结构、索引与持久化边界。

数据语义见 [10_agent_data_model.md](10_agent_data_model.md)。地点层级、地区事实继承与路线图见 [15_agent_location_system.md](15_agent_location_system.md)。Knowledge 访问派生索引的运行规则见 KnowledgeEntry 章节。运行时写入顺序见 [11_agent_runtime.md](11_agent_runtime.md)，日志表与清理策略见 [30_logging_and_observability.md](30_logging_and_observability.md)。

---

## 1. SQLite 表结构

按三层语义组织。Layer 2 不持久化（每回合重建）；Layer 1 / Layer 3 / Trace 各自独立。Agent 模式以 World 为 canonical Truth 单元；聊天记录是同一 World 下的会话视图，不等同于独立世界状态。canonical 回合、会话消息、过去线 provisional truth 与冲突报告必须分表保存，避免非正史会话污染正史。

```sql
-- ===== Session / Timeline / Turn Commit（会话、主线光标与正史提交） =====

-- 每个 World 的当前主线前沿；第一版 timeline_id 固定为 main
CREATE TABLE world_mainline_cursor (
    world_id TEXT NOT NULL,
    timeline_id TEXT NOT NULL DEFAULT 'main',
    mainline_head_turn_id TEXT,
    mainline_time_anchor TEXT NOT NULL,       -- JSON: TimeAnchor
    updated_at TEXT NOT NULL,
    PRIMARY KEY (world_id, timeline_id)
);

-- 同一 World 下的聊天 / 过去线 / 预演会话
CREATE TABLE agent_sessions (
    session_id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    title TEXT NOT NULL,
    session_kind TEXT NOT NULL,               -- mainline / retrospective / future_preview
    period_anchor TEXT NOT NULL,              -- JSON: TimeAnchor
    player_character_id TEXT,
    canon_status TEXT NOT NULL,               -- canon_candidate / partially_canon / noncanon
    conflict_policy TEXT,                     -- noncanon_after_conflict / whole_session_noncanon
    status TEXT NOT NULL DEFAULT 'active',    -- active / archived / deleted
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 会话内消息顺序；可引用 canonical turn，也可只作为非正史聊天存在
CREATE TABLE session_turns (
    session_turn_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    scene_turn_id TEXT,
    local_index INTEGER NOT NULL,
    role TEXT NOT NULL,                       -- user / assistant / system
    message_json TEXT NOT NULL,
    canon_status TEXT NOT NULL,               -- canon_candidate / canon_promoted / conflict_warned / noncanon
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id),
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- canonical commit journal；提交顺序不等于故事时间顺序
CREATE TABLE world_turns (
    scene_turn_id TEXT PRIMARY KEY,
    parent_turn_id TEXT,
    session_id TEXT,
    timeline_id TEXT NOT NULL DEFAULT 'main',
    story_time_anchor TEXT NOT NULL,          -- JSON: TimeAnchor
    user_message TEXT NOT NULL,             -- JSON: 用户输入/扮演输入
    rendered_output TEXT,                   -- SurfaceRealizerOutput.narrative_text；used_fact_ids 写入 Agent Trace
    canon_status TEXT NOT NULL DEFAULT 'canon', -- canon / provisional_promoted
    status TEXT NOT NULL DEFAULT 'active',  -- active / rolled_back
    created_at TEXT NOT NULL,
    rolled_back_at TEXT,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id),
    FOREIGN KEY (parent_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- 过去线 / 预演会话中尚未或不能提升为 canonical Truth 的候选事实
CREATE TABLE provisional_session_truth (
    provisional_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    source_session_turn_id TEXT NOT NULL,
    source_scene_turn_id TEXT,
    story_time_anchor TEXT NOT NULL,          -- JSON: TimeAnchor
    derived_from_event_id TEXT,
    candidate_kind TEXT NOT NULL,             -- knowledge_entry / event_detail / relation_detail / location_detail
    candidate_payload TEXT NOT NULL,          -- JSON
    promotion_status TEXT NOT NULL,           -- pending / promoted / blocked_conflict / noncanon / trace_only
    promoted_knowledge_id TEXT,
    promoted_scene_turn_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id),
    FOREIGN KEY (source_session_turn_id) REFERENCES session_turns(session_turn_id),
    FOREIGN KEY (source_scene_turn_id) REFERENCES world_turns(scene_turn_id),
    FOREIGN KEY (promoted_knowledge_id) REFERENCES knowledge_entries(knowledge_id),
    FOREIGN KEY (promoted_scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- 过去线硬冲突报告；冲突不阻断游玩，只改变正史资格
CREATE TABLE conflict_reports (
    conflict_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    session_turn_id TEXT NOT NULL,
    scene_turn_id TEXT,
    severity TEXT NOT NULL,                   -- soft / hard
    source_constraint_ids TEXT NOT NULL,      -- JSON array
    affected_provisional_ids TEXT NOT NULL,   -- JSON array
    policy_decision TEXT,                     -- noncanon_after_conflict / whole_session_noncanon
    summary TEXT NOT NULL,                    -- JSON / llm_readable
    created_at TEXT NOT NULL,
    resolved_at TEXT,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id),
    FOREIGN KEY (session_turn_id) REFERENCES session_turns(session_turn_id),
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- 每回合状态提交记录；用于定位需要回滚的人物、世界、知识和 trace 变化
CREATE TABLE state_commit_records (
    commit_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    changed_scene_snapshot_ids TEXT NOT NULL,       -- JSON array
    changed_location_ids TEXT NOT NULL,             -- JSON array: location_nodes / location_edges / spatial_relations / aliases / templates 变更 ID
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

-- 地点节点（层级归属与地点属性）
CREATE TABLE location_nodes (
    location_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    polity_id TEXT,
    parent_id TEXT,
    canonical_level TEXT NOT NULL,          -- world_root / realm / continent / natural_region / polity / major_region / local_region / settlement / district_or_site / room_or_subsite
    type_label TEXT NOT NULL,               -- 州 / 县 / 城 / 宗门 / 港口 ...
    tags TEXT NOT NULL,                     -- JSON array
    status TEXT NOT NULL,                   -- active / deprecated / pending_confirmation
    metadata TEXT NOT NULL,                 -- JSON
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES location_nodes(location_id)
);

-- 地点别名索引；允许一对多，多命中必须返回 ambiguity
CREATE TABLE location_aliases (
    alias TEXT NOT NULL,
    location_id TEXT NOT NULL,
    locale TEXT,
    normalized_alias TEXT NOT NULL,
    PRIMARY KEY (normalized_alias, location_id),
    FOREIGN KEY (location_id) REFERENCES location_nodes(location_id)
);

-- 地点空间覆盖 / 穿过 / 重叠关系；尤其用于自然地理地带影响行政区或场所
CREATE TABLE location_spatial_relations (
    relation_id TEXT PRIMARY KEY,
    source_location_id TEXT NOT NULL,
    target_location_id TEXT NOT NULL,
    relation TEXT NOT NULL,                 -- overlaps / crosses / source_contains_part_of_target / source_partly_within_target / adjacent_to / within_natural_band
    coverage TEXT,                          -- JSON: CoverageEstimate
    confidence TEXT NOT NULL,
    source TEXT NOT NULL,                   -- JSON: FactSource
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_location_id) REFERENCES location_nodes(location_id),
    FOREIGN KEY (target_location_id) REFERENCES location_nodes(location_id)
);

-- 地点路线 / 相邻带权图；parent_id 只表达归属，spatial_relations 只表达覆盖/穿过/重叠，二者都不表达可通行距离
CREATE TABLE location_edges (
    edge_id TEXT PRIMARY KEY,
    from_location_id TEXT NOT NULL,
    to_location_id TEXT NOT NULL,
    relation TEXT NOT NULL,                 -- adjacent / road / river_route / sea_route / mountain_pass / ...
    bidirectional INTEGER NOT NULL,
    distance_km TEXT,                       -- JSON: DistanceEstimate
    travel_time TEXT,                       -- JSON: TravelTimeEstimate
    terrain_cost REAL NOT NULL DEFAULT 1.0,
    safety_cost REAL NOT NULL DEFAULT 1.0,
    seasonal_modifiers TEXT NOT NULL,        -- JSON array
    allowed_modes TEXT NOT NULL,             -- JSON array
    confidence TEXT NOT NULL,
    source TEXT NOT NULL,                    -- JSON: FactSource
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (from_location_id) REFERENCES location_nodes(location_id),
    FOREIGN KEY (to_location_id) REFERENCES location_nodes(location_id)
);

-- 国家 / 政体的显示层级模板；辅助编辑器与导入校验，不是层级归属真相
CREATE TABLE location_polity_templates (
    polity_id TEXT PRIMARY KEY,
    level_labels TEXT NOT NULL,              -- JSON object: canonical_level -> label
    allowed_parent_child TEXT NOT NULL,      -- JSON array
    override_rules TEXT NOT NULL,            -- JSON
    updated_at TEXT NOT NULL,
    FOREIGN KEY (polity_id) REFERENCES location_nodes(location_id)
);

-- 统一知识库（世界/地区/势力/角色档案/历史事件/记忆）
CREATE TABLE knowledge_entries (
    knowledge_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,                    -- world_fact / region_fact / faction_fact / character_facet / historical_event / memory
    subject_type TEXT NOT NULL,            -- world / region / faction / character / event
    subject_id TEXT,                       -- region/faction/character/event 的具体 ID（World 时为 NULL）
    facet_type TEXT,                       -- 仅 character_facet 有值
    content TEXT NOT NULL,                 -- JSON: 客观真相
    apparent_content TEXT,                 -- JSON: 表象（可空）
    access_policy TEXT NOT NULL,           -- JSON: AccessPolicy
    subject_awareness TEXT NOT NULL,       -- JSON: SubjectAwareness（含 Unaware 的 self_belief）
    metadata TEXT NOT NULL,                -- JSON: KnowledgeMetadata
    valid_from TEXT,                       -- JSON: TimeAnchor；可空
    valid_until TEXT,                      -- JSON: TimeAnchor；可空
    source_session_id TEXT,
    source_scene_turn_id TEXT,
    derived_from_event_id TEXT,
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

-- 角色基本档案（base_attributes + baseline_body_profile + mana_expression_tendency + optional factor override + mind_model_card_knowledge_id 指针 + temporary_state；其余事实在 knowledge_entries 中）
CREATE TABLE character_records (
    character_id TEXT PRIMARY KEY,
    base_attributes TEXT NOT NULL,         -- JSON: BaseAttributes，f64 存储和计算
    baseline_body_profile TEXT NOT NULL,   -- JSON
    mana_expression_tendency TEXT NOT NULL, -- enum: Inward / Neutral / Expressive，长期默认显露倾向
    mana_expression_tendency_factor_override REAL, -- 可空；特定人物覆盖 tendency_factor
    mind_model_card_knowledge_id TEXT NOT NULL,
    temporary_state TEXT NOT NULL,         -- JSON: Layer 1 当前身体/资源/跨域临时状态，含 mana_expression
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
    session_id TEXT,
    story_time_anchor TEXT,                -- JSON: TimeAnchor
    canon_status TEXT NOT NULL DEFAULT 'canon',
    belief_state TEXT NOT NULL,            -- JSON
    emotion_state TEXT NOT NULL,           -- JSON
    relation_models TEXT NOT NULL,         -- JSON
    current_goals TEXT NOT NULL,           -- JSON
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id)
);

-- ===== Trace / Logs（调试、回放与运行观测） =====

CREATE TABLE config_snapshots (
    config_snapshot_id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,                   -- global / world
    world_id TEXT,
    schema_version INTEGER NOT NULL,
    config_hash TEXT NOT NULL,
    source_paths TEXT NOT NULL,            -- JSON array: 参与合并的配置来源
    compiled_summary TEXT NOT NULL,        -- JSON: 脱敏后的关键阈值摘要，便于回放定位
    created_at TEXT NOT NULL
);

CREATE TABLE turn_traces (
    trace_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    session_id TEXT,
    story_time_anchor TEXT,                -- JSON: TimeAnchor
    canon_status TEXT NOT NULL DEFAULT 'canon',
    trace_kind TEXT NOT NULL,              -- turn / character / presentation / rollback
    character_id TEXT,                     -- NULL 表示全局回合 trace
    config_snapshot_id TEXT NOT NULL,
    summary TEXT NOT NULL,                 -- JSON: 回合级关键产物索引与摘要
    linked_request_ids TEXT NOT NULL,       -- JSON array: 关联 llm_call_logs.request_id
    linked_event_ids TEXT NOT NULL,         -- JSON array: 关联 app_event_logs.event_id
    created_at TEXT NOT NULL,
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id),
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id),
    FOREIGN KEY (config_snapshot_id) REFERENCES config_snapshots(config_snapshot_id)
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
    session_id TEXT,
    scene_turn_id TEXT,
    trace_id TEXT,
    character_id TEXT,
    llm_node TEXT NOT NULL,                -- STChat / SceneInitializer / SceneStateExtractor / CharacterCognitivePass / OutcomePlanner / SurfaceRealizer
    api_config_id TEXT NOT NULL,           -- 调用时实际使用的 API 配置
    config_snapshot_id TEXT,
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
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id),
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id),
    FOREIGN KEY (trace_id) REFERENCES turn_traces(trace_id),
    FOREIGN KEY (config_snapshot_id) REFERENCES config_snapshots(config_snapshot_id)
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

-- World 级 Agent LLM 配置选择；允许每个 World 使用不同的五节点配置
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
    session_id TEXT,
    scene_turn_id TEXT,
    trace_id TEXT,
    character_id TEXT,
    config_snapshot_id TEXT,
    detail_json TEXT,                      -- JSON: 异常上下文，写入前必须脱敏
    created_at TEXT NOT NULL,
    FOREIGN KEY (config_snapshot_id) REFERENCES config_snapshots(config_snapshot_id)
);

CREATE TABLE log_retention_state (
    retention_id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,                   -- global / world
    world_id TEXT,
    config_snapshot_id TEXT,
    size_limit_bytes INTEGER NOT NULL,       -- 本轮 retention 检查采用的快照值
    current_size_bytes INTEGER,
    last_checked_at TEXT,
    last_cleanup_at TEXT,
    cleanup_needed INTEGER NOT NULL DEFAULT 0,
    user_prompt_required INTEGER NOT NULL DEFAULT 0,
    detail_json TEXT,
    FOREIGN KEY (config_snapshot_id) REFERENCES config_snapshots(config_snapshot_id)
);

-- ===== 索引 =====

CREATE INDEX idx_scene_snapshots_scene ON scene_snapshots(scene_id);
CREATE INDEX idx_sessions_world ON agent_sessions(world_id, session_kind, canon_status);
CREATE INDEX idx_sessions_period ON agent_sessions(world_id, period_anchor);
CREATE INDEX idx_session_turns_session ON session_turns(session_id, local_index);
CREATE INDEX idx_session_turns_scene ON session_turns(scene_turn_id);
CREATE INDEX idx_world_turns_parent ON world_turns(parent_turn_id);
CREATE INDEX idx_world_turns_session ON world_turns(session_id);
CREATE INDEX idx_world_turns_story_time ON world_turns(timeline_id, story_time_anchor);
CREATE INDEX idx_commit_records_turn ON state_commit_records(scene_turn_id);
CREATE INDEX idx_location_parent ON location_nodes(parent_id);
CREATE INDEX idx_location_polity ON location_nodes(polity_id);
CREATE INDEX idx_location_level ON location_nodes(canonical_level);
CREATE INDEX idx_location_alias_lookup ON location_aliases(normalized_alias);
CREATE INDEX idx_location_spatial_source ON location_spatial_relations(source_location_id);
CREATE INDEX idx_location_spatial_target ON location_spatial_relations(target_location_id);
CREATE INDEX idx_location_edges_from ON location_edges(from_location_id);
CREATE INDEX idx_location_edges_to ON location_edges(to_location_id);
CREATE INDEX idx_knowledge_kind ON knowledge_entries(kind);
CREATE INDEX idx_knowledge_subject ON knowledge_entries(subject_type, subject_id);
CREATE INDEX idx_knowledge_facet ON knowledge_entries(subject_id, facet_type) WHERE kind = 'character_facet';
CREATE INDEX idx_knowledge_time ON knowledge_entries(valid_from, valid_until);
CREATE INDEX idx_knowledge_source_session ON knowledge_entries(source_session_id, source_scene_turn_id);
CREATE INDEX idx_reveal_knowledge ON knowledge_reveal_events(knowledge_id);
CREATE INDEX idx_access_known_by_character ON knowledge_access_known_by(character_id);
CREATE INDEX idx_access_scopes_lookup ON knowledge_access_scopes(scope_type, scope_value);
CREATE INDEX idx_character_scope_lookup ON character_scope_memberships(character_id, scope_type, scope_value);
CREATE INDEX idx_subjective_char ON character_subjective_snapshots(character_id, scene_turn_id);
CREATE INDEX idx_subjective_char_time ON character_subjective_snapshots(character_id, story_time_anchor, canon_status);
CREATE INDEX idx_provisional_session ON provisional_session_truth(session_id, promotion_status);
CREATE INDEX idx_conflicts_session ON conflict_reports(session_id, severity, created_at);
CREATE INDEX idx_config_snapshots_scope ON config_snapshots(scope, world_id, created_at);
CREATE INDEX idx_traces_turn ON turn_traces(scene_turn_id);
CREATE INDEX idx_traces_session ON turn_traces(session_id, canon_status);
CREATE INDEX idx_traces_config ON turn_traces(config_snapshot_id);
CREATE INDEX idx_step_traces_turn ON agent_step_traces(scene_turn_id);
CREATE INDEX idx_step_traces_trace ON agent_step_traces(trace_id);
CREATE INDEX idx_llm_logs_turn ON llm_call_logs(scene_turn_id);
CREATE INDEX idx_llm_logs_session ON llm_call_logs(session_id);
CREATE INDEX idx_llm_logs_trace ON llm_call_logs(trace_id);
CREATE INDEX idx_llm_logs_api_config ON llm_call_logs(api_config_id);
CREATE INDEX idx_llm_logs_config ON llm_call_logs(config_snapshot_id);
CREATE INDEX idx_llm_logs_created ON llm_call_logs(created_at);
CREATE INDEX idx_stream_chunks_request ON llm_stream_chunks(request_id, chunk_index);
CREATE INDEX idx_app_events_context ON app_event_logs(world_id, scene_turn_id, trace_id);
CREATE INDEX idx_app_events_created ON app_event_logs(created_at);
```

**说明**：

- `knowledge_entries.access_policy` JSON 是权威结构；`knowledge_access_known_by`、`knowledge_access_scopes` 与 `character_scope_memberships` 是可重建的查询索引，只用于候选预筛，最终访问权限仍由 `KnowledgeAccessResolver` 判定。
- `location_nodes.parent_id` 是地点层级归属权威；`location_spatial_relations` 是自然地理覆盖 / 穿过 / 重叠关系权威；`location_edges` 是路线和相邻关系权威。三者不可混用：同父级或同自然地理带只能生成提示，不能直接硬算距离。
- `location_aliases` 是地点别名的持久化权威；运行时 `LocationNode.aliases` 只是从该表 hydrate 出来的视图，不得另行双写。`location_aliases` 允许同一 alias 指向多个地点；解析器必须返回 ambiguity，除非上下文锚点足以确定唯一 `location_id`。
- `location_polity_templates` 只用于编辑器显示、导入辅助和合法父子模板校验，不替代 `parent_id`。
- `NaturalRegion` 跨越多个行政节点时，用 `location_spatial_relations` 表达，不复制节点，也不允许多重 `parent_id`。
- `subject_id + facet_type` 联合索引服务"取角色 X 的所有 facets"这一最高频查询。
- `world_mainline_cursor` 定义玩家当前主线前沿；判断过去线必须比较 `agent_sessions.period_anchor` 与 `mainline_time_anchor`，不得使用 `world_turns.created_at`。
- `agent_sessions` 保存同一 World 下的聊天入口；`session_turns` 保存会话内显示顺序。删除 / 归档会话不等于删除 canonical Truth。
- `world_turns` 是 canonical commit journal；`parent_turn_id` 表示提交依赖链，`story_time_anchor` 表示故事内时间。补玩过去线时，新的 commit 可以晚创建但 story time 更早。
- `provisional_session_truth` 保存过去线和预演会话产生的候选事实。只有 `promotion_status = promoted` 的候选能对应到 canonical `knowledge_entries` 或 `world_turns`；非正史会话不得提升。
- `conflict_reports` 记录过去线与 TruthGuidance 的硬冲突。冲突不会打断游玩，只通过 `policy_decision` 把冲突后或整条会话降为非正史。
- `character_subjective_snapshots` 的 canonical 最新快照才是角色当前心智状态；非正史或过去线会话的快照必须带 `session_id` / `canon_status`，不得覆盖 canonical 当前心智。
- 没有"memory_records"表；正史事件约束使用 `knowledge_entries.kind = 'historical_event'`，角色亲历 / 听闻 / 推断记忆使用 `knowledge_entries.kind = 'memory'`。
- 回滚 canonical turn 时，必须检查后续 canonical facts、已提升 provisional truth、其他会话和 `world_mainline_cursor` 是否依赖该回合；存在依赖时默认阻止并生成影响报告，不能只按聊天消息删除。
- Agent Trace 以 `scene_turn_id` 为主轴，解释回合如何演化；运行 Logs 以 `request_id` / `event_id` 为主轴，解释应用运行时发生了什么。两者可互相关联，但日志不得作为 Agent 判断或 LLM 输入来源。
- `config_snapshots` 记录已发布运行配置快照的 hash 和脱敏摘要，用于复盘“当时用了哪套阈值”；它不是热路径配置源，Resolver 不得从表中查询阈值。
- Agent 模式允许五类 LLM 节点绑定不同 API 配置；`agent_llm_profiles.bindings` 保存用户选择，`llm_call_logs.api_config_id` 保存每次调用实际使用的配置。
- `llm_call_logs.request_json`、`response_json`、`llm_stream_chunks.raw_chunk` 尽量还原 Provider 原貌；`readable_text` 仅用于流式响应的段落化查看，不替代原始响应。
- `app_event_logs` 同表结构可用于 `./data/logs/app_logs.sqlite`。ST 模式只写全局运行 Logs；Agent 模式与回合相关的记录写入对应 `world.sqlite`，同时可在全局异常日志中保留带 `world_id` / `scene_turn_id` 的索引事件。
- 默认清理上限为 1GB，但实际上限来自 `RuntimeConfigSnapshot.log_retention`；`log_retention_state.size_limit_bytes` 只记录本轮 retention 检查采用的值。自动清理只处理全局运行 Logs；Agent Trace 和仍被 `state_commit_records.trace_ids` 引用的记录不自动删除。30 天以上未更新且日志体积较大的 World 只产生提示事件，等待用户确认。
