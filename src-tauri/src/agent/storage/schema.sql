-- Agent SQLite Schema
-- Based on docs/14_agent_persistence.md

-- ===== Session / Timeline / Turn Commit =====

-- World mainline cursor
CREATE TABLE IF NOT EXISTS world_mainline_cursor (
    world_id TEXT NOT NULL,
    timeline_id TEXT NOT NULL DEFAULT 'main',
    mainline_head_turn_id TEXT,
    mainline_time_anchor TEXT NOT NULL,       -- JSON: TimeAnchor
    updated_at TEXT NOT NULL,
    PRIMARY KEY (world_id, timeline_id)
);

-- Agent sessions
CREATE TABLE IF NOT EXISTS agent_sessions (
    session_id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    title TEXT NOT NULL,
    session_kind TEXT NOT NULL,               -- mainline / retrospective / future_preview
    period_anchor TEXT NOT NULL,              -- JSON: TimeAnchor
    player_mode TEXT NOT NULL DEFAULT 'Character', -- Character / Director
    player_character_id TEXT,
    canon_status TEXT NOT NULL,               -- canon_candidate / partially_canon / noncanon
    conflict_policy TEXT,                     -- noncanon_after_conflict / whole_session_noncanon
    status TEXT NOT NULL DEFAULT 'active',    -- active / archived / deleted
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Session turns
CREATE TABLE IF NOT EXISTS session_turns (
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

-- World turns (runtime turn journal)
CREATE TABLE IF NOT EXISTS world_turns (
    scene_turn_id TEXT PRIMARY KEY,
    parent_turn_id TEXT,
    session_id TEXT,
    timeline_id TEXT NOT NULL DEFAULT 'main',
    story_time_anchor TEXT NOT NULL,          -- JSON: TimeAnchor
    user_message TEXT NOT NULL,               -- JSON: 用户输入/扮演输入
    rendered_output TEXT,                     -- SurfaceRealizerOutput.narrative_text
    runtime_turn_status TEXT NOT NULL DEFAULT 'canon', -- canon / provisional_promoted / provisional_only / noncanon / future_preview
    status TEXT NOT NULL DEFAULT 'active',    -- active / rolled_back
    created_at TEXT NOT NULL,
    rolled_back_at TEXT,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id),
    FOREIGN KEY (parent_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- Provisional session truth
CREATE TABLE IF NOT EXISTS provisional_session_truth (
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

-- Conflict reports
CREATE TABLE IF NOT EXISTS conflict_reports (
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

-- State commit records
CREATE TABLE IF NOT EXISTS state_commit_records (
    commit_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    changed_scene_snapshot_ids TEXT NOT NULL,       -- JSON array
    changed_location_ids TEXT NOT NULL,             -- JSON array
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

-- World editor commits
CREATE TABLE IF NOT EXISTS world_editor_commits (
    editor_commit_id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    base_editor_revision INTEGER NOT NULL,
    resulting_editor_revision INTEGER NOT NULL,
    changed_location_ids TEXT NOT NULL,            -- JSON array
    changed_knowledge_ids TEXT NOT NULL,           -- JSON array
    changed_character_ids TEXT NOT NULL,           -- JSON array
    changed_relationship_ids TEXT NOT NULL,        -- JSON array
    changed_temporal_state_ids TEXT NOT NULL,      -- JSON array
    changed_config_keys TEXT NOT NULL,             -- JSON array
    rollback_patch TEXT NOT NULL,                  -- JSON
    validation_summary TEXT NOT NULL,              -- JSON
    author_note TEXT,
    created_at TEXT NOT NULL
);

-- ===== Layer 1: Truth Store =====

-- Scene snapshots
CREATE TABLE IF NOT EXISTS scene_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    scene_id TEXT NOT NULL,
    scene_turn_id TEXT NOT NULL,
    scene_model TEXT NOT NULL,             -- JSON: SceneModel
    created_at TEXT NOT NULL
);

-- Location nodes
CREATE TABLE IF NOT EXISTS location_nodes (
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

-- Location aliases
CREATE TABLE IF NOT EXISTS location_aliases (
    alias TEXT NOT NULL,
    location_id TEXT NOT NULL,
    locale TEXT,
    normalized_alias TEXT NOT NULL,
    PRIMARY KEY (normalized_alias, location_id),
    FOREIGN KEY (location_id) REFERENCES location_nodes(location_id)
);

-- Location spatial relations
CREATE TABLE IF NOT EXISTS location_spatial_relations (
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

-- Location edges
CREATE TABLE IF NOT EXISTS location_edges (
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

-- Location polity templates
CREATE TABLE IF NOT EXISTS location_polity_templates (
    polity_id TEXT PRIMARY KEY,
    level_labels TEXT NOT NULL,              -- JSON object
    allowed_parent_child TEXT NOT NULL,      -- JSON array
    override_rules TEXT NOT NULL,            -- JSON
    updated_at TEXT NOT NULL,
    FOREIGN KEY (polity_id) REFERENCES location_nodes(location_id)
);

-- Knowledge entries
CREATE TABLE IF NOT EXISTS knowledge_entries (
    knowledge_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,                    -- world_fact / region_fact / faction_fact / character_facet / historical_event / memory
    subject_type TEXT NOT NULL,            -- world / region / faction / character / event
    subject_id TEXT,                       -- region/faction/character/event 的具体 ID
    facet_type TEXT,                       -- 仅 character_facet 有值
    content TEXT NOT NULL,                 -- JSON: 客观真相
    apparent_content TEXT,                 -- JSON: 表象
    access_policy TEXT NOT NULL,           -- JSON: AccessPolicy
    subject_awareness TEXT NOT NULL,       -- JSON: SubjectAwareness
    metadata TEXT NOT NULL,                -- JSON: KnowledgeMetadata
    valid_from TEXT,                       -- JSON: TimeAnchor
    valid_until TEXT,                      -- JSON: TimeAnchor
    source_session_id TEXT,
    source_scene_turn_id TEXT,
    derived_from_event_id TEXT,
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Knowledge reveal events
CREATE TABLE IF NOT EXISTS knowledge_reveal_events (
    event_id TEXT PRIMARY KEY,
    knowledge_id TEXT NOT NULL,
    newly_known_by TEXT NOT NULL,          -- JSON array
    trigger TEXT NOT NULL,                 -- JSON: RevealTrigger
    scope_change TEXT,                     -- JSON: AccessScopeChange
    scene_turn_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (knowledge_id) REFERENCES knowledge_entries(knowledge_id)
);

-- Knowledge access known_by index
CREATE TABLE IF NOT EXISTS knowledge_access_known_by (
    knowledge_id TEXT NOT NULL,
    character_id TEXT NOT NULL,
    PRIMARY KEY (knowledge_id, character_id),
    FOREIGN KEY (knowledge_id) REFERENCES knowledge_entries(knowledge_id)
);

-- Knowledge access scopes index
CREATE TABLE IF NOT EXISTS knowledge_access_scopes (
    knowledge_id TEXT NOT NULL,
    scope_type TEXT NOT NULL,              -- public / god_only / region / faction / realm / role / bloodline / ...
    scope_value TEXT NOT NULL DEFAULT '',  -- 无值 scope 使用空字符串
    PRIMARY KEY (knowledge_id, scope_type, scope_value),
    FOREIGN KEY (knowledge_id) REFERENCES knowledge_entries(knowledge_id)
);

-- Character scope memberships
CREATE TABLE IF NOT EXISTS character_scope_memberships (
    character_id TEXT NOT NULL,
    scope_type TEXT NOT NULL,              -- region / faction / realm / role / bloodline / ...
    scope_value TEXT NOT NULL,
    source_knowledge_id TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (character_id, scope_type, scope_value)
);

-- Character records
CREATE TABLE IF NOT EXISTS character_records (
    character_id TEXT PRIMARY KEY,
    base_attributes TEXT NOT NULL,         -- JSON: BaseAttributes
    baseline_body_profile TEXT NOT NULL,   -- JSON
    mana_expression_tendency TEXT NOT NULL, -- enum: Inward / Neutral / Expressive
    mana_expression_tendency_factor_override REAL,
    mind_model_card_knowledge_id TEXT NOT NULL,
    temporary_state TEXT NOT NULL,         -- JSON: TemporaryCharacterState
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Temporal state records
CREATE TABLE IF NOT EXISTS temporal_state_records (
    state_record_id TEXT PRIMARY KEY,
    subject_type TEXT NOT NULL,              -- character / location / scene / object / relationship / resource
    subject_id TEXT NOT NULL,
    state_kind TEXT NOT NULL,                -- position / temporary_state / location_status / item_state / objective_relation / authorization / ...
    valid_from TEXT NOT NULL,                -- JSON: TimeAnchor
    valid_until TEXT,                        -- JSON: TimeAnchor
    payload TEXT NOT NULL,                   -- JSON
    source_scene_turn_id TEXT,
    source_session_id TEXT,
    canon_status TEXT NOT NULL DEFAULT 'canon', -- canon / provisional_promoted
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_scene_turn_id) REFERENCES world_turns(scene_turn_id),
    FOREIGN KEY (source_session_id) REFERENCES agent_sessions(session_id)
);

-- Objective relationships
CREATE TABLE IF NOT EXISTS objective_relationships (
    relation_id TEXT PRIMARY KEY,
    subject_character_id TEXT NOT NULL,
    target_character_id TEXT NOT NULL,
    relation_kind TEXT NOT NULL,             -- ally / family / faction_rank / employer / oath / access_grant / hostility ...
    access_level REAL NOT NULL DEFAULT 0.0,
    authorization_tags TEXT NOT NULL,         -- JSON array
    valid_from TEXT NOT NULL,                -- JSON: TimeAnchor
    valid_until TEXT,                        -- JSON: TimeAnchor
    source_knowledge_id TEXT,
    source_scene_turn_id TEXT,
    schema_version TEXT NOT NULL DEFAULT '0.1',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_knowledge_id) REFERENCES knowledge_entries(knowledge_id),
    FOREIGN KEY (source_scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- ===== Layer 3: Subjective State =====

-- Character subjective snapshots
CREATE TABLE IF NOT EXISTS character_subjective_snapshots (
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

-- ===== Trace / Logs =====

-- Config snapshots
CREATE TABLE IF NOT EXISTS config_snapshots (
    config_snapshot_id TEXT PRIMARY KEY,
    snapshot_kind TEXT NOT NULL,           -- runtime_config / world_rules
    scope TEXT NOT NULL,                   -- global / world
    world_id TEXT,
    schema_version INTEGER NOT NULL,
    config_hash TEXT NOT NULL,
    source_paths TEXT NOT NULL,            -- JSON array
    compiled_summary TEXT NOT NULL,        -- JSON
    created_at TEXT NOT NULL
);

-- Turn traces
CREATE TABLE IF NOT EXISTS turn_traces (
    trace_id TEXT PRIMARY KEY,
    scene_turn_id TEXT NOT NULL,
    session_id TEXT,
    story_time_anchor TEXT,                -- JSON: TimeAnchor
    runtime_turn_status TEXT NOT NULL DEFAULT 'canon',
    trace_kind TEXT NOT NULL,              -- turn / character / presentation / rollback
    character_id TEXT,
    runtime_config_snapshot_id TEXT NOT NULL,
    world_rules_snapshot_id TEXT,
    summary TEXT NOT NULL,                 -- JSON
    linked_request_ids TEXT NOT NULL,       -- JSON array
    linked_event_ids TEXT NOT NULL,         -- JSON array
    created_at TEXT NOT NULL,
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id),
    FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id),
    FOREIGN KEY (runtime_config_snapshot_id) REFERENCES config_snapshots(config_snapshot_id),
    FOREIGN KEY (world_rules_snapshot_id) REFERENCES config_snapshots(config_snapshot_id)
);

-- Agent step traces
CREATE TABLE IF NOT EXISTS agent_step_traces (
    step_trace_id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    scene_turn_id TEXT NOT NULL,
    character_id TEXT,
    step_name TEXT NOT NULL,               -- active_set / dirty_flags / scene_filter / cognitive_pass / validation / outcome_planning / effect_validation / state_commit
    step_status TEXT NOT NULL,             -- started / skipped / succeeded / failed / fallback_used
    input_summary TEXT,                    -- JSON
    output_summary TEXT,                   -- JSON
    decision_json TEXT,                    -- JSON
    linked_request_id TEXT,
    error_event_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (trace_id) REFERENCES turn_traces(trace_id),
    FOREIGN KEY (scene_turn_id) REFERENCES world_turns(scene_turn_id)
);

-- Agent LLM profiles
CREATE TABLE IF NOT EXISTS agent_llm_profiles (
    profile_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    default_api_config_id TEXT NOT NULL,
    bindings TEXT NOT NULL,                -- JSON: Vec<AgentLlmConfigBinding>
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- World agent settings
CREATE TABLE IF NOT EXISTS world_agent_settings (
    world_id TEXT PRIMARY KEY,
    agent_llm_profile_id TEXT NOT NULL,
    profile_overrides TEXT,                -- JSON
    updated_at TEXT NOT NULL,
    FOREIGN KEY (agent_llm_profile_id) REFERENCES agent_llm_profiles(profile_id)
);

-- LLM call logs (Agent mode, per-world)
CREATE TABLE IF NOT EXISTS llm_call_logs (
    request_id TEXT PRIMARY KEY,
    mode TEXT NOT NULL,                    -- st / agent
    world_id TEXT,
    session_id TEXT,
    scene_turn_id TEXT,
    trace_id TEXT,
    character_id TEXT,
    llm_node TEXT NOT NULL,                -- STChat / SceneInitializer / SceneStateExtractor / CharacterCognitivePass / OutcomePlanner / SurfaceRealizer
    api_config_id TEXT NOT NULL,           -- 调用时实际使用的 API 配置
    runtime_config_snapshot_id TEXT,
    world_rules_snapshot_id TEXT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    call_type TEXT NOT NULL,               -- chat / chat_structured / chat_stream
    request_json TEXT NOT NULL,            -- JSON: 写入前必须脱敏
    schema_json TEXT,                      -- JSON Schema，仅 structured 调用有值
    response_json TEXT,                    -- JSON: 完整响应（已脱敏）
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
    FOREIGN KEY (runtime_config_snapshot_id) REFERENCES config_snapshots(config_snapshot_id),
    FOREIGN KEY (world_rules_snapshot_id) REFERENCES config_snapshots(config_snapshot_id)
);

-- LLM stream chunks
CREATE TABLE IF NOT EXISTS llm_stream_chunks (
    chunk_id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    raw_chunk TEXT NOT NULL,
    received_at TEXT NOT NULL,
    FOREIGN KEY (request_id) REFERENCES llm_call_logs(request_id)
);

-- App event logs (Agent mode, per-world)
CREATE TABLE IF NOT EXISTS app_event_logs (
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
    runtime_config_snapshot_id TEXT,
    world_rules_snapshot_id TEXT,
    detail_json TEXT,                      -- JSON: 异常上下文，写入前必须脱敏
    created_at TEXT NOT NULL,
    FOREIGN KEY (runtime_config_snapshot_id) REFERENCES config_snapshots(config_snapshot_id),
    FOREIGN KEY (world_rules_snapshot_id) REFERENCES config_snapshots(config_snapshot_id)
);

-- Log retention state
CREATE TABLE IF NOT EXISTS log_retention_state (
    retention_id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,                   -- global / world
    world_id TEXT,
    runtime_config_snapshot_id TEXT,
    size_limit_bytes INTEGER NOT NULL,       -- 本轮 retention 检查采用的快照值
    current_size_bytes INTEGER,
    last_checked_at TEXT,
    last_cleanup_at TEXT,
    cleanup_needed INTEGER NOT NULL DEFAULT 0,
    user_prompt_required INTEGER NOT NULL DEFAULT 0,
    detail_json TEXT,
    FOREIGN KEY (runtime_config_snapshot_id) REFERENCES config_snapshots(config_snapshot_id)
);

-- ===== Indexes =====

CREATE INDEX IF NOT EXISTS idx_scene_snapshots_scene ON scene_snapshots(scene_id);
CREATE INDEX IF NOT EXISTS idx_sessions_world ON agent_sessions(world_id, session_kind, canon_status);
CREATE INDEX IF NOT EXISTS idx_sessions_period ON agent_sessions(world_id, period_anchor);
CREATE INDEX IF NOT EXISTS idx_session_turns_session ON session_turns(session_id, local_index);
CREATE INDEX IF NOT EXISTS idx_session_turns_scene ON session_turns(scene_turn_id);
CREATE INDEX IF NOT EXISTS idx_world_turns_parent ON world_turns(parent_turn_id);
CREATE INDEX IF NOT EXISTS idx_world_turns_session ON world_turns(session_id);
CREATE INDEX IF NOT EXISTS idx_world_turns_story_time ON world_turns(timeline_id, story_time_anchor);
CREATE INDEX IF NOT EXISTS idx_commit_records_turn ON state_commit_records(scene_turn_id);
CREATE INDEX IF NOT EXISTS idx_world_editor_commits_revision ON world_editor_commits(world_id, resulting_editor_revision);
CREATE INDEX IF NOT EXISTS idx_world_editor_commits_created ON world_editor_commits(world_id, created_at);
CREATE INDEX IF NOT EXISTS idx_location_parent ON location_nodes(parent_id);
CREATE INDEX IF NOT EXISTS idx_location_polity ON location_nodes(polity_id);
CREATE INDEX IF NOT EXISTS idx_location_level ON location_nodes(canonical_level);
CREATE INDEX IF NOT EXISTS idx_location_alias_lookup ON location_aliases(normalized_alias);
CREATE INDEX IF NOT EXISTS idx_location_spatial_source ON location_spatial_relations(source_location_id);
CREATE INDEX IF NOT EXISTS idx_location_spatial_target ON location_spatial_relations(target_location_id);
CREATE INDEX IF NOT EXISTS idx_location_edges_from ON location_edges(from_location_id);
CREATE INDEX IF NOT EXISTS idx_location_edges_to ON location_edges(to_location_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_kind ON knowledge_entries(kind);
CREATE INDEX IF NOT EXISTS idx_knowledge_subject ON knowledge_entries(subject_type, subject_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_facet ON knowledge_entries(subject_id, facet_type) WHERE kind = 'character_facet';
CREATE INDEX IF NOT EXISTS idx_knowledge_time ON knowledge_entries(valid_from, valid_until);
CREATE INDEX IF NOT EXISTS idx_knowledge_source_session ON knowledge_entries(source_session_id, source_scene_turn_id);
CREATE INDEX IF NOT EXISTS idx_reveal_knowledge ON knowledge_reveal_events(knowledge_id);
CREATE INDEX IF NOT EXISTS idx_access_known_by_character ON knowledge_access_known_by(character_id);
CREATE INDEX IF NOT EXISTS idx_access_scopes_lookup ON knowledge_access_scopes(scope_type, scope_value);
CREATE INDEX IF NOT EXISTS idx_character_scope_lookup ON character_scope_memberships(character_id, scope_type, scope_value);
CREATE INDEX IF NOT EXISTS idx_subjective_char ON character_subjective_snapshots(character_id, scene_turn_id);
CREATE INDEX IF NOT EXISTS idx_subjective_char_time ON character_subjective_snapshots(character_id, story_time_anchor, canon_status);
CREATE INDEX IF NOT EXISTS idx_temporal_state_subject ON temporal_state_records(subject_type, subject_id, state_kind);
CREATE INDEX IF NOT EXISTS idx_temporal_state_time ON temporal_state_records(valid_from, valid_until, canon_status);
CREATE INDEX IF NOT EXISTS idx_objective_relationship_pair ON objective_relationships(subject_character_id, target_character_id, relation_kind);
CREATE INDEX IF NOT EXISTS idx_objective_relationship_time ON objective_relationships(valid_from, valid_until);
CREATE INDEX IF NOT EXISTS idx_provisional_session ON provisional_session_truth(session_id, promotion_status);
CREATE INDEX IF NOT EXISTS idx_conflicts_session ON conflict_reports(session_id, severity, created_at);
CREATE INDEX IF NOT EXISTS idx_config_snapshots_scope ON config_snapshots(snapshot_kind, scope, world_id, created_at);
CREATE INDEX IF NOT EXISTS idx_traces_turn ON turn_traces(scene_turn_id);
CREATE INDEX IF NOT EXISTS idx_traces_session ON turn_traces(session_id, runtime_turn_status);
CREATE INDEX IF NOT EXISTS idx_traces_runtime_config ON turn_traces(runtime_config_snapshot_id);
CREATE INDEX IF NOT EXISTS idx_traces_world_rules ON turn_traces(world_rules_snapshot_id);
CREATE INDEX IF NOT EXISTS idx_step_traces_turn ON agent_step_traces(scene_turn_id);
CREATE INDEX IF NOT EXISTS idx_step_traces_trace ON agent_step_traces(trace_id);
CREATE INDEX IF NOT EXISTS idx_llm_logs_turn ON llm_call_logs(scene_turn_id);
CREATE INDEX IF NOT EXISTS idx_llm_logs_session ON llm_call_logs(session_id);
CREATE INDEX IF NOT EXISTS idx_llm_logs_trace ON llm_call_logs(trace_id);
CREATE INDEX IF NOT EXISTS idx_llm_logs_api_config ON llm_call_logs(api_config_id);
CREATE INDEX IF NOT EXISTS idx_llm_logs_runtime_config ON llm_call_logs(runtime_config_snapshot_id);
CREATE INDEX IF NOT EXISTS idx_llm_logs_world_rules ON llm_call_logs(world_rules_snapshot_id);
CREATE INDEX IF NOT EXISTS idx_llm_logs_created ON llm_call_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_stream_chunks_request ON llm_stream_chunks(request_id, chunk_index);
CREATE INDEX IF NOT EXISTS idx_app_events_context ON app_event_logs(world_id, scene_turn_id, trace_id);
CREATE INDEX IF NOT EXISTS idx_app_events_created ON app_event_logs(created_at);