//! Agent store - SQLite persistence for Agent mode

use chrono::Utc;
use serde_json;
use sqlx::SqlitePool;

use super::schema::AgentSchema;
use crate::agent::models::*;

/// Agent store for world-specific data
pub struct AgentStore {
    pool: SqlitePool,
    world_id: String,
}

impl AgentStore {
    /// Create a new Agent store for a specific world
    pub async fn new(pool: SqlitePool, world_id: String) -> Result<Self, String> {
        let store = Self { pool, world_id };
        store.init_schema().await?;
        Ok(store)
    }

    /// Initialize Agent schema
    pub async fn init_schema(&self) -> Result<(), String> {
        AgentSchema::init(&self.pool).await
    }

    /// Get the world ID
    pub fn world_id(&self) -> &str {
        &self.world_id
    }

    /// Get the database pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// ===== WorldMainlineCursor operations =====

impl AgentStore {
    /// Get or create the mainline cursor for this world
    pub async fn get_mainline_cursor(&self) -> Result<WorldMainlineCursor, String> {
        let row = sqlx::query_as::<_, (String, Option<String>, String, String)>(
            r#"
            SELECT world_id, mainline_head_turn_id, mainline_time_anchor, updated_at
            FROM world_mainline_cursor
            WHERE world_id = ? AND timeline_id = 'main'
            "#,
        )
        .bind(&self.world_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get mainline cursor: {}", e))?;

        match row {
            Some((world_id, head_turn_id, time_anchor_json, updated_at)) => {
                let time_anchor: TimeAnchor = serde_json::from_str(&time_anchor_json)
                    .map_err(|e| format!("Failed to parse time anchor: {}", e))?;
                let updated_at: chrono::DateTime<Utc> = updated_at
                    .parse()
                    .map_err(|e| format!("Failed to parse updated_at: {}", e))?;
                Ok(WorldMainlineCursor {
                    world_id,
                    timeline_id: "main".to_string(),
                    mainline_head_turn_id: head_turn_id,
                    mainline_time_anchor: time_anchor,
                    updated_at,
                })
            }
            None => {
                // Create default cursor
                let cursor = WorldMainlineCursor::new(
                    self.world_id.clone(),
                    TimeAnchor {
                        calendar_id: "default".to_string(),
                        ordinal: 0,
                        precision: TimePrecision::Era,
                        display_text: "故事开始".to_string(),
                    },
                );
                self.save_mainline_cursor(&cursor).await?;
                Ok(cursor)
            }
        }
    }

    /// Save the mainline cursor
    pub async fn save_mainline_cursor(&self, cursor: &WorldMainlineCursor) -> Result<(), String> {
        let time_anchor_json = serde_json::to_string(&cursor.mainline_time_anchor)
            .map_err(|e| format!("Failed to serialize time anchor: {}", e))?;
        let updated_at = cursor.updated_at.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO world_mainline_cursor (world_id, timeline_id, mainline_head_turn_id, mainline_time_anchor, updated_at)
            VALUES (?, 'main', ?, ?, ?)
            ON CONFLICT(world_id, timeline_id) DO UPDATE SET
                mainline_head_turn_id = excluded.mainline_head_turn_id,
                mainline_time_anchor = excluded.mainline_time_anchor,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&cursor.world_id)
        .bind(&cursor.mainline_head_turn_id)
        .bind(&time_anchor_json)
        .bind(&updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to save mainline cursor: {}", e))?;

        Ok(())
    }

    /// Advance the mainline cursor to a new turn
    pub async fn advance_mainline(
        &self,
        turn_id: &str,
        new_time_anchor: &TimeAnchor,
    ) -> Result<(), String> {
        let now = Utc::now();
        let time_anchor_json = serde_json::to_string(new_time_anchor)
            .map_err(|e| format!("Failed to serialize time anchor: {}", e))?;

        sqlx::query(
            r#"
            UPDATE world_mainline_cursor
            SET mainline_head_turn_id = ?,
                mainline_time_anchor = ?,
                updated_at = ?
            WHERE world_id = ? AND timeline_id = 'main'
            "#,
        )
        .bind(turn_id)
        .bind(&time_anchor_json)
        .bind(now.to_rfc3339())
        .bind(&self.world_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to advance mainline: {}", e))?;

        Ok(())
    }
}

// ===== AgentSession operations =====

impl AgentStore {
    /// Create a new session
    pub async fn create_session(&self, session: &AgentSession) -> Result<(), String> {
        let period_anchor_json = serde_json::to_string(&session.period_anchor)
            .map_err(|e| format!("Failed to serialize period anchor: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO agent_sessions (
                session_id, world_id, title, session_kind, period_anchor,
                player_mode, player_character_id, canon_status, conflict_policy, status,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.session_id)
        .bind(&session.world_id)
        .bind(&session.title)
        .bind(session_kind_to_str(&session.session_kind))
        .bind(&period_anchor_json)
        .bind(player_mode_to_str(&session.player_mode))
        .bind(&session.player_character_id)
        .bind(canon_status_to_str(&session.canon_status))
        .bind(session.conflict_policy.as_ref().map(conflict_policy_to_str))
        .bind(session_status_to_str(&session.status))
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create session: {}", e))?;

        Ok(())
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<AgentSession>, String> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
                String,
                Option<String>,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT session_id, world_id, title, session_kind, period_anchor, player_mode,
                   player_character_id, canon_status, conflict_policy, status,
                   created_at, updated_at
            FROM agent_sessions
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get session: {}", e))?;

        match row {
            Some((
                session_id,
                world_id,
                title,
                kind_str,
                period_anchor_json,
                player_mode_str,
                player_character_id,
                canon_status_str,
                conflict_policy_str,
                status_str,
                created_at,
                updated_at,
            )) => {
                let period_anchor: TimeAnchor = serde_json::from_str(&period_anchor_json)
                    .map_err(|e| format!("Failed to parse period anchor: {}", e))?;
                let created_at: chrono::DateTime<Utc> = created_at
                    .parse()
                    .map_err(|e| format!("Failed to parse created_at: {}", e))?;
                let updated_at: chrono::DateTime<Utc> = updated_at
                    .parse()
                    .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

                Ok(Some(AgentSession {
                    session_id,
                    world_id,
                    title,
                    session_kind: str_to_session_kind(&kind_str)?,
                    period_anchor,
                    player_mode: str_to_player_mode(&player_mode_str)?,
                    player_character_id,
                    canon_status: str_to_canon_status(&canon_status_str)?,
                    conflict_policy: conflict_policy_str
                        .map(|s| str_to_conflict_policy(&s))
                        .transpose()?,
                    status: str_to_session_status(&status_str)?,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// List sessions for this world
    pub async fn list_sessions(&self) -> Result<Vec<AgentSession>, String> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
                String,
                Option<String>,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT session_id, world_id, title, session_kind, period_anchor, player_mode,
                   player_character_id, canon_status, conflict_policy, status,
                   created_at, updated_at
            FROM agent_sessions
            WHERE world_id = ? AND status != 'deleted'
            ORDER BY updated_at DESC
            "#,
        )
        .bind(&self.world_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to list sessions: {}", e))?;

        let mut sessions = Vec::new();
        for (
            session_id,
            world_id,
            title,
            kind_str,
            period_anchor_json,
            player_mode_str,
            player_character_id,
            canon_status_str,
            conflict_policy_str,
            status_str,
            created_at,
            updated_at,
        ) in rows
        {
            let period_anchor: TimeAnchor = serde_json::from_str(&period_anchor_json)
                .map_err(|e| format!("Failed to parse period anchor: {}", e))?;
            let created_at: chrono::DateTime<Utc> = created_at
                .parse()
                .map_err(|e| format!("Failed to parse created_at: {}", e))?;
            let updated_at: chrono::DateTime<Utc> = updated_at
                .parse()
                .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

            sessions.push(AgentSession {
                session_id,
                world_id,
                title,
                session_kind: str_to_session_kind(&kind_str)?,
                period_anchor,
                player_mode: str_to_player_mode(&player_mode_str)?,
                player_character_id,
                canon_status: str_to_canon_status(&canon_status_str)?,
                conflict_policy: conflict_policy_str
                    .map(|s| str_to_conflict_policy(&s))
                    .transpose()?,
                status: str_to_session_status(&status_str)?,
                created_at,
                updated_at,
            });
        }

        Ok(sessions)
    }

    /// Update an existing session
    pub async fn update_session(&self, session: &AgentSession) -> Result<(), String> {
        let period_anchor_json = serde_json::to_string(&session.period_anchor)
            .map_err(|e| format!("Failed to serialize period anchor: {}", e))?;

        sqlx::query(
            r#"
            UPDATE agent_sessions SET
                title = ?,
                player_mode = ?,
                player_character_id = ?,
                canon_status = ?,
                conflict_policy = ?,
                status = ?,
                updated_at = ?
            WHERE session_id = ?
            "#,
        )
        .bind(&session.title)
        .bind(player_mode_to_str(&session.player_mode))
        .bind(&session.player_character_id)
        .bind(canon_status_to_str(&session.canon_status))
        .bind(session.conflict_policy.as_ref().map(conflict_policy_to_str))
        .bind(session_status_to_str(&session.status))
        .bind(session.updated_at.to_rfc3339())
        .bind(&session.session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update session: {}", e))?;

        Ok(())
    }
}

// ===== SessionTurn operations =====

impl AgentStore {
    /// Append a chat-visible turn to an Agent session.
    pub async fn append_session_turn(
        &self,
        session_id: &str,
        scene_turn_id: Option<&str>,
        role: TurnRole,
        message_json: serde_json::Value,
        canon_status: SessionTurnCanonStatus,
    ) -> Result<SessionTurn, String> {
        let local_index: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(local_index), -1) + 1
            FROM session_turns
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to allocate session turn index: {}", e))?;

        let turn = SessionTurn {
            session_turn_id: crate::agent::models::common::generate_id("session_turn"),
            session_id: session_id.to_string(),
            scene_turn_id: scene_turn_id.map(ToOwned::to_owned),
            local_index: local_index as u32,
            role,
            message_json,
            canon_status,
            created_at: Utc::now(),
        };

        let message_json = serde_json::to_string(&turn.message_json)
            .map_err(|e| format!("Failed to serialize session turn message: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO session_turns (
                session_turn_id, session_id, scene_turn_id, local_index,
                role, message_json, canon_status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&turn.session_turn_id)
        .bind(&turn.session_id)
        .bind(&turn.scene_turn_id)
        .bind(turn.local_index as i64)
        .bind(turn_role_to_str(&turn.role))
        .bind(&message_json)
        .bind(session_turn_status_to_str(&turn.canon_status))
        .bind(turn.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to append session turn: {}", e))?;

        sqlx::query("UPDATE agent_sessions SET updated_at = ? WHERE session_id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to update session timestamp: {}", e))?;

        Ok(turn)
    }

    /// List chat-visible turns for one Agent session.
    pub async fn list_session_turns(&self, session_id: &str) -> Result<Vec<SessionTurn>, String> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                i64,
                String,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT session_turn_id, session_id, scene_turn_id, local_index,
                   role, message_json, canon_status, created_at
            FROM session_turns
            WHERE session_id = ?
            ORDER BY local_index ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to list session turns: {}", e))?;

        rows.into_iter()
            .map(
                |(
                    session_turn_id,
                    session_id,
                    scene_turn_id,
                    local_index,
                    role,
                    message_json,
                    canon_status,
                    created_at,
                )| {
                    Ok(SessionTurn {
                        session_turn_id,
                        session_id,
                        scene_turn_id,
                        local_index: u32::try_from(local_index)
                            .map_err(|e| format!("Invalid local_index: {}", e))?,
                        role: str_to_turn_role(&role)?,
                        message_json: serde_json::from_str(&message_json)
                            .map_err(|e| format!("Failed to parse session turn message: {}", e))?,
                        canon_status: str_to_session_turn_status(&canon_status)?,
                        created_at: created_at.parse().map_err(|e| {
                            format!("Failed to parse session turn created_at: {}", e)
                        })?,
                    })
                },
            )
            .collect()
    }
}

// ===== CharacterRecord operations =====

impl AgentStore {
    /// Save a character record
    pub async fn save_character(&self, character: &CharacterRecord) -> Result<(), String> {
        let base_attributes_json = serde_json::to_string(&character.base_attributes)
            .map_err(|e| format!("Failed to serialize base attributes: {}", e))?;
        let body_profile_json = serde_json::to_string(&character.baseline_body_profile)
            .map_err(|e| format!("Failed to serialize body profile: {}", e))?;
        let temp_state_json = serde_json::to_string(&character.temporary_state)
            .map_err(|e| format!("Failed to serialize temporary state: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO character_records (
                character_id, base_attributes, baseline_body_profile,
                mana_expression_tendency, mana_expression_tendency_factor_override,
                mind_model_card_knowledge_id, temporary_state, schema_version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(character_id) DO UPDATE SET
                base_attributes = excluded.base_attributes,
                baseline_body_profile = excluded.baseline_body_profile,
                mana_expression_tendency = excluded.mana_expression_tendency,
                mana_expression_tendency_factor_override = excluded.mana_expression_tendency_factor_override,
                mind_model_card_knowledge_id = excluded.mind_model_card_knowledge_id,
                temporary_state = excluded.temporary_state,
                schema_version = excluded.schema_version,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&character.character_id)
        .bind(&base_attributes_json)
        .bind(&body_profile_json)
        .bind(tendency_to_str(&character.mana_expression_tendency))
        .bind(character.mana_expression_tendency_factor_override)
        .bind(&character.mind_model_card_knowledge_id)
        .bind(&temp_state_json)
        .bind(&character.schema_version)
        .bind(character.created_at.to_rfc3339())
        .bind(character.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to save character: {}", e))?;

        Ok(())
    }

    /// Get a character by ID
    pub async fn get_character(
        &self,
        character_id: &str,
    ) -> Result<Option<CharacterRecord>, String> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                Option<f64>,
                String,
                String,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT character_id, base_attributes, baseline_body_profile,
                   mana_expression_tendency, mana_expression_tendency_factor_override,
                   mind_model_card_knowledge_id, temporary_state, schema_version,
                   created_at, updated_at
            FROM character_records
            WHERE character_id = ?
            "#,
        )
        .bind(character_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get character: {}", e))?;

        match row {
            Some((
                character_id,
                base_attrs_json,
                body_profile_json,
                tendency_str,
                tendency_override,
                mind_model_id,
                temp_state_json,
                schema_version,
                created_at,
                updated_at,
            )) => {
                let base_attributes: BaseAttributes = serde_json::from_str(&base_attrs_json)
                    .map_err(|e| format!("Failed to parse base attributes: {}", e))?;
                let baseline_body_profile: BaselineBodyProfile =
                    serde_json::from_str(&body_profile_json)
                        .map_err(|e| format!("Failed to parse body profile: {}", e))?;
                let temporary_state: TemporaryCharacterState =
                    serde_json::from_str(&temp_state_json)
                        .map_err(|e| format!("Failed to parse temporary state: {}", e))?;
                let created_at: chrono::DateTime<Utc> = created_at
                    .parse()
                    .map_err(|e| format!("Failed to parse created_at: {}", e))?;
                let updated_at: chrono::DateTime<Utc> = updated_at
                    .parse()
                    .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

                Ok(Some(CharacterRecord {
                    character_id,
                    base_attributes,
                    baseline_body_profile,
                    mana_expression_tendency: str_to_tendency(&tendency_str)?,
                    mana_expression_tendency_factor_override: tendency_override,
                    mind_model_card_knowledge_id: mind_model_id,
                    temporary_state,
                    schema_version,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// List all characters in this world
    pub async fn list_characters(&self) -> Result<Vec<CharacterRecord>, String> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                Option<f64>,
                String,
                String,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT character_id, base_attributes, baseline_body_profile,
                   mana_expression_tendency, mana_expression_tendency_factor_override,
                   mind_model_card_knowledge_id, temporary_state, schema_version,
                   created_at, updated_at
            FROM character_records
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to list characters: {}", e))?;

        let mut characters = Vec::new();
        for (
            character_id,
            base_attrs_json,
            body_profile_json,
            tendency_str,
            tendency_override,
            mind_model_id,
            temp_state_json,
            schema_version,
            created_at,
            updated_at,
        ) in rows
        {
            let base_attributes: BaseAttributes = serde_json::from_str(&base_attrs_json)
                .map_err(|e| format!("Failed to parse base attributes: {}", e))?;
            let baseline_body_profile: BaselineBodyProfile =
                serde_json::from_str(&body_profile_json)
                    .map_err(|e| format!("Failed to parse body profile: {}", e))?;
            let temporary_state: TemporaryCharacterState =
                serde_json::from_str(&temp_state_json)
                    .map_err(|e| format!("Failed to parse temporary state: {}", e))?;
            let created_at: chrono::DateTime<Utc> = created_at
                .parse()
                .map_err(|e| format!("Failed to parse created_at: {}", e))?;
            let updated_at: chrono::DateTime<Utc> = updated_at
                .parse()
                .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

            characters.push(CharacterRecord {
                character_id,
                base_attributes,
                baseline_body_profile,
                mana_expression_tendency: str_to_tendency(&tendency_str)?,
                mana_expression_tendency_factor_override: tendency_override,
                mind_model_card_knowledge_id: mind_model_id,
                temporary_state,
                schema_version,
                created_at,
                updated_at,
            });
        }

        Ok(characters)
    }

    /// List objective relationships for the current world.
    pub async fn list_objective_relationships(&self) -> Result<Vec<ObjectiveRelationship>, String> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                f64,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT relation_id, subject_character_id, target_character_id, relation_kind,
                   access_level, authorization_tags, valid_from, valid_until,
                   source_knowledge_id, source_scene_turn_id, schema_version,
                   created_at, updated_at
            FROM objective_relationships
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to list objective relationships: {}", e))?;

        let mut relationships = Vec::new();
        for (
            relation_id,
            subject_character_id,
            target_character_id,
            relation_kind_str,
            access_level,
            authorization_tags_json,
            valid_from_json,
            valid_until_json,
            source_knowledge_id,
            source_scene_turn_id,
            schema_version,
            created_at,
            updated_at,
        ) in rows
        {
            let authorization_tags: Vec<String> = serde_json::from_str(&authorization_tags_json)
                .map_err(|e| format!("Failed to parse authorization_tags: {}", e))?;
            let valid_from: TimeAnchor = serde_json::from_str(&valid_from_json)
                .map_err(|e| format!("Failed to parse valid_from: {}", e))?;
            let valid_until: Option<TimeAnchor> = valid_until_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|e| format!("Failed to parse valid_until: {}", e))?;
            let created_at: chrono::DateTime<Utc> = created_at
                .parse()
                .map_err(|e| format!("Failed to parse created_at: {}", e))?;
            let updated_at: chrono::DateTime<Utc> = updated_at
                .parse()
                .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

            relationships.push(ObjectiveRelationship {
                relation_id,
                subject_character_id,
                target_character_id,
                relation_kind: str_to_objective_relation_kind(&relation_kind_str)?,
                access_level,
                authorization_tags,
                valid_from,
                valid_until,
                source_knowledge_id,
                source_scene_turn_id,
                schema_version,
                created_at,
                updated_at,
            });
        }

        Ok(relationships)
    }
}

// ===== Helper functions for enum serialization =====

fn session_kind_to_str(kind: &AgentSessionKind) -> &'static str {
    match kind {
        AgentSessionKind::Mainline => "mainline",
        AgentSessionKind::Retrospective => "retrospective",
        AgentSessionKind::FuturePreview => "future_preview",
    }
}

fn str_to_session_kind(s: &str) -> Result<AgentSessionKind, String> {
    match s {
        "mainline" => Ok(AgentSessionKind::Mainline),
        "retrospective" => Ok(AgentSessionKind::Retrospective),
        "future_preview" => Ok(AgentSessionKind::FuturePreview),
        _ => Err(format!("Invalid session kind: {}", s)),
    }
}

fn canon_status_to_str(status: &SessionCanonStatus) -> &'static str {
    match status {
        SessionCanonStatus::CanonCandidate => "canon_candidate",
        SessionCanonStatus::PartiallyCanon => "partially_canon",
        SessionCanonStatus::NonCanon => "noncanon",
    }
}

fn str_to_canon_status(s: &str) -> Result<SessionCanonStatus, String> {
    match s {
        "canon_candidate" => Ok(SessionCanonStatus::CanonCandidate),
        "partially_canon" => Ok(SessionCanonStatus::PartiallyCanon),
        "noncanon" => Ok(SessionCanonStatus::NonCanon),
        _ => Err(format!("Invalid canon status: {}", s)),
    }
}

fn conflict_policy_to_str(policy: &ConflictPolicyDecision) -> &'static str {
    match policy {
        ConflictPolicyDecision::NonCanonAfterConflict => "noncanon_after_conflict",
        ConflictPolicyDecision::WholeSessionNonCanon => "whole_session_noncanon",
    }
}

fn str_to_conflict_policy(s: &str) -> Result<ConflictPolicyDecision, String> {
    match s {
        "noncanon_after_conflict" => Ok(ConflictPolicyDecision::NonCanonAfterConflict),
        "whole_session_noncanon" => Ok(ConflictPolicyDecision::WholeSessionNonCanon),
        _ => Err(format!("Invalid conflict policy: {}", s)),
    }
}

fn session_status_to_str(status: &SessionStatus) -> &'static str {
    match status {
        SessionStatus::Active => "active",
        SessionStatus::Archived => "archived",
        SessionStatus::Deleted => "deleted",
    }
}

fn str_to_session_status(s: &str) -> Result<SessionStatus, String> {
    match s {
        "active" => Ok(SessionStatus::Active),
        "archived" => Ok(SessionStatus::Archived),
        "deleted" => Ok(SessionStatus::Deleted),
        _ => Err(format!("Invalid session status: {}", s)),
    }
}

fn tendency_to_str(tendency: &ManaExpressionTendency) -> &'static str {
    match tendency {
        ManaExpressionTendency::Inward => "Inward",
        ManaExpressionTendency::Neutral => "Neutral",
        ManaExpressionTendency::Expressive => "Expressive",
    }
}

fn str_to_tendency(s: &str) -> Result<ManaExpressionTendency, String> {
    match s {
        "Inward" => Ok(ManaExpressionTendency::Inward),
        "Neutral" => Ok(ManaExpressionTendency::Neutral),
        "Expressive" => Ok(ManaExpressionTendency::Expressive),
        _ => Err(format!("Invalid mana expression tendency: {}", s)),
    }
}

fn player_mode_to_str(mode: &PlayerMode) -> &'static str {
    match mode {
        PlayerMode::Character => "Character",
        PlayerMode::Director => "Director",
    }
}

fn str_to_objective_relation_kind(value: &str) -> Result<ObjectiveRelationKind, String> {
    match value {
        "ally" => Ok(ObjectiveRelationKind::Ally),
        "family" => Ok(ObjectiveRelationKind::Family),
        "faction_rank" => Ok(ObjectiveRelationKind::FactionRank),
        "employer" => Ok(ObjectiveRelationKind::Employer),
        "oath" => Ok(ObjectiveRelationKind::Oath),
        "access_grant" => Ok(ObjectiveRelationKind::AccessGrant),
        "hostility" => Ok(ObjectiveRelationKind::Hostility),
        "rival" => Ok(ObjectiveRelationKind::Rival),
        "master_disciple" => Ok(ObjectiveRelationKind::MasterDisciple),
        "trade_partner" => Ok(ObjectiveRelationKind::TradePartner),
        "custom" => Ok(ObjectiveRelationKind::Custom),
        _ => Err(format!("Invalid objective relation kind: {}", value)),
    }
}

fn str_to_player_mode(s: &str) -> Result<PlayerMode, String> {
    match s {
        "Character" => Ok(PlayerMode::Character),
        "Director" => Ok(PlayerMode::Director),
        _ => Err(format!("Invalid player mode: {}", s)),
    }
}

fn turn_role_to_str(role: &TurnRole) -> &'static str {
    match role {
        TurnRole::User => "user",
        TurnRole::Assistant => "assistant",
        TurnRole::System => "system",
    }
}

fn str_to_turn_role(s: &str) -> Result<TurnRole, String> {
    match s {
        "user" => Ok(TurnRole::User),
        "assistant" => Ok(TurnRole::Assistant),
        "system" => Ok(TurnRole::System),
        _ => Err(format!("Invalid turn role: {}", s)),
    }
}

fn session_turn_status_to_str(status: &SessionTurnCanonStatus) -> &'static str {
    match status {
        SessionTurnCanonStatus::CanonCandidate => "canon_candidate",
        SessionTurnCanonStatus::CanonPromoted => "canon_promoted",
        SessionTurnCanonStatus::ConflictWarned => "conflict_warned",
        SessionTurnCanonStatus::NonCanon => "noncanon",
    }
}

fn str_to_session_turn_status(s: &str) -> Result<SessionTurnCanonStatus, String> {
    match s {
        "canon_candidate" => Ok(SessionTurnCanonStatus::CanonCandidate),
        "canon_promoted" => Ok(SessionTurnCanonStatus::CanonPromoted),
        "conflict_warned" => Ok(SessionTurnCanonStatus::ConflictWarned),
        "noncanon" => Ok(SessionTurnCanonStatus::NonCanon),
        _ => Err(format!("Invalid session turn canon status: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_turns_append_and_list_in_order() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("memory db");
        let store = AgentStore::new(pool, "world_session_turns".to_string())
            .await
            .expect("agent store");
        let session = AgentSession::new_with_mode(
            "world_session_turns".to_string(),
            "测试会话".to_string(),
            AgentSessionKind::Mainline,
            TimeAnchor {
                calendar_id: "default".to_string(),
                ordinal: 0,
                precision: TimePrecision::Era,
                display_text: "故事开始".to_string(),
            },
            PlayerMode::Director,
            None,
        )
        .expect("session");
        store
            .create_session(&session)
            .await
            .expect("create session");

        store
            .append_session_turn(
                &session.session_id,
                None,
                TurnRole::User,
                serde_json::json!({ "content": "开始" }),
                SessionTurnCanonStatus::CanonCandidate,
            )
            .await
            .expect("append user");
        store
            .append_session_turn(
                &session.session_id,
                None,
                TurnRole::Assistant,
                serde_json::json!({ "content": "已开始" }),
                SessionTurnCanonStatus::CanonCandidate,
            )
            .await
            .expect("append assistant");

        let turns = store
            .list_session_turns(&session.session_id)
            .await
            .expect("list turns");
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].local_index, 0);
        assert_eq!(turns[1].local_index, 1);
        assert_eq!(turns[0].role, TurnRole::User);
        assert_eq!(turns[1].message_json["content"], "已开始");
    }
}
