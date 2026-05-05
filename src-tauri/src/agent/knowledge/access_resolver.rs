//! Knowledge access resolver
//!
//! Unified access permission determination for KnowledgeEntry.
//! NEVER calls LLM - purely programmatic.

use sqlx::SqlitePool;

use crate::agent::models::{
    AccessCondition, AccessExpression, AccessPolicy, AccessScope, AccessSource, AccessibleEntry,
    KnowledgeEntry, KnowledgeKind, KnowledgeSubject,
};

/// Knowledge access resolver - determines who can access what
///
/// Key invariant: This resolver NEVER calls LLM.
/// All decisions are purely programmatic based on access_policy.
pub struct KnowledgeAccessResolver {
    pool: Option<SqlitePool>,
}

impl KnowledgeAccessResolver {
    /// Create a new resolver without database access (for pure logic checks)
    pub fn new() -> Self {
        Self { pool: None }
    }

    /// Create a new resolver with database access (for condition checks)
    pub fn with_pool(pool: SqlitePool) -> Self {
        Self { pool: Some(pool) }
    }

    /// Check if a character can access a knowledge entry
    pub fn can_access(
        entry: &KnowledgeEntry,
        character_id: &str,
        character_scopes: &[CharacterScopeMembership],
        scene_context: Option<&AccessSceneContext>,
    ) -> bool {
        let policy = &entry.access_policy;

        // GodOnly is hard deny - check first
        if policy
            .scope
            .iter()
            .any(|s| matches!(s, AccessScope::GodOnly))
        {
            return false;
        }

        // Check known_by (name-list)
        if policy.known_by.contains(&character_id.to_string()) {
            return true;
        }

        // Check scope-based access
        for scope in &policy.scope {
            if Self::matches_scope(scope, character_id, character_scopes) {
                return true;
            }
        }

        // Check condition-based access
        for condition in &policy.conditions {
            if Self::check_condition(condition, character_id, scene_context) {
                return true;
            }
        }

        false
    }

    /// Check access with database-backed condition evaluation.
    pub async fn can_access_async(
        entry: &KnowledgeEntry,
        character_id: &str,
        character_scopes: &[CharacterScopeMembership],
        scene_context: Option<&AccessSceneContext>,
        pool: &SqlitePool,
    ) -> Result<bool, String> {
        let policy = &entry.access_policy;

        if policy
            .scope
            .iter()
            .any(|s| matches!(s, AccessScope::GodOnly))
        {
            return Ok(false);
        }

        if policy.known_by.iter().any(|id| id == character_id) {
            return Ok(true);
        }

        for scope in &policy.scope {
            if Self::matches_scope(scope, character_id, character_scopes) {
                return Ok(true);
            }
        }

        for condition in &policy.conditions {
            if Self::check_condition_async(condition, character_id, scene_context, pool).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if scope matches character's memberships
    fn matches_scope(
        scope: &AccessScope,
        character_id: &str,
        character_scopes: &[CharacterScopeMembership],
    ) -> bool {
        match scope {
            AccessScope::Public => true,
            AccessScope::GodOnly => false, // Already handled above
            AccessScope::Region(region_id) => character_scopes
                .iter()
                .any(|m| m.scope_type == "region" && m.scope_value == *region_id),
            AccessScope::Faction(faction_id) => character_scopes
                .iter()
                .any(|m| m.scope_type == "faction" && m.scope_value == *faction_id),
            AccessScope::Realm(realm_id) => character_scopes
                .iter()
                .any(|m| m.scope_type == "realm" && m.scope_value == *realm_id),
            AccessScope::Role(role_id) => character_scopes
                .iter()
                .any(|m| m.scope_type == "role" && m.scope_value == *role_id),
            AccessScope::Bloodline(bloodline_id) => character_scopes
                .iter()
                .any(|m| m.scope_type == "bloodline" && m.scope_value == *bloodline_id),
        }
    }

    /// Check condition-based access (static checks without database)
    fn check_condition(
        condition: &AccessCondition,
        character_id: &str,
        scene_context: Option<&AccessSceneContext>,
    ) -> bool {
        match condition {
            AccessCondition::InSameSceneObservable => scene_context
                .map(|ctx| ctx.observable_characters.iter().any(|c| c == character_id))
                .unwrap_or(false),
            AccessCondition::SocialAccessAtLeast { target, threshold } => {
                // This requires database lookup - return false for static check
                // Full implementation needs async database query
                let _ = (target, threshold);
                false
            }
            AccessCondition::HasSkill(skill_id) => {
                // This requires database lookup - return false for static check
                let _ = skill_id;
                false
            }
            AccessCondition::CultivationAtLeast(level) => {
                // This requires database lookup - return false for static check
                let _ = level;
                false
            }
            AccessCondition::CustomPredicate(expr) => {
                // Custom predicates require evaluation context
                let _ = expr;
                false
            }
        }
    }

    /// Check condition-based access with database support (async)
    pub async fn check_condition_async(
        condition: &AccessCondition,
        character_id: &str,
        scene_context: Option<&AccessSceneContext>,
        pool: &SqlitePool,
    ) -> Result<bool, String> {
        match condition {
            AccessCondition::InSameSceneObservable => Ok(scene_context
                .map(|ctx| ctx.observable_characters.iter().any(|c| c == character_id))
                .unwrap_or(false)),
            AccessCondition::SocialAccessAtLeast { target, threshold } => {
                Self::check_social_access(pool, character_id, target, *threshold).await
            }
            AccessCondition::HasSkill(skill_id) => {
                Self::check_has_skill(pool, character_id, skill_id).await
            }
            AccessCondition::CultivationAtLeast(level) => {
                Self::check_cultivation_at_least(pool, character_id, level).await
            }
            AccessCondition::CustomPredicate(expr) => {
                Self::evaluate_expression(pool, character_id, expr).await
            }
        }
    }

    /// Check social access level from objective_relationships
    async fn check_social_access(
        pool: &SqlitePool,
        subject_character_id: &str,
        target_character_id: &str,
        threshold: f64,
    ) -> Result<bool, String> {
        let row = sqlx::query_as::<_, AccessLevelRow>(
            r#"
            SELECT access_level
            FROM objective_relationships
            WHERE subject_character_id = ? AND target_character_id = ?
            LIMIT 1
            "#,
        )
        .bind(subject_character_id)
        .bind(target_character_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to query objective_relationships: {}", e))?;

        Ok(row.map(|r| r.access_level >= threshold).unwrap_or(false))
    }

    /// Check if character has a specific skill
    async fn check_has_skill(
        pool: &SqlitePool,
        character_id: &str,
        skill_id: &str,
    ) -> Result<bool, String> {
        // Check character's known abilities from knowledge_entries
        let row = sqlx::query_as::<_, CountRow>(
            r#"
            SELECT COUNT(*) as count
            FROM knowledge_entries
            WHERE subject_type = 'character'
              AND subject_id = ?
              AND facet_type IN ('known_ability', 'hidden_ability')
              AND json_extract(content, '$.ability_id') = ?
            "#,
        )
        .bind(character_id)
        .bind(skill_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to query skill: {}", e))?;

        Ok(row.count > 0)
    }

    /// Check if character's cultivation is at least the given level
    async fn check_cultivation_at_least(
        pool: &SqlitePool,
        character_id: &str,
        level: &str,
    ) -> Result<bool, String> {
        // Get character's cultivation realm from knowledge_entries
        let row = sqlx::query_as::<_, RealmRow>(
            r#"
            SELECT json_extract(content, '$.realm') as realm
            FROM knowledge_entries
            WHERE subject_type = 'character'
              AND subject_id = ?
              AND facet_type = 'cultivation_realm'
            LIMIT 1
            "#,
        )
        .bind(character_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to query cultivation realm: {}", e))?;

        if let Some(realm_row) = row {
            // Compare realm levels using a predefined hierarchy
            let realm_hierarchy = [
                "mundane",
                "awakened",
                "foundation",
                "golden_core",
                "nascent_soul",
                "spirit_severing",
                "dao Seeking",
                "immortal",
            ];
            let character_realm = realm_row.realm.as_deref().unwrap_or("mundane");
            let required_realm = level;

            let character_idx = realm_hierarchy.iter().position(|r| *r == character_realm);
            let required_idx = realm_hierarchy.iter().position(|r| *r == required_realm);

            if let (Some(ci), Some(ri)) = (character_idx, required_idx) {
                return Ok(ci >= ri);
            }
        }

        Ok(false)
    }

    /// Evaluate a custom access expression
    async fn evaluate_expression(
        pool: &SqlitePool,
        character_id: &str,
        expr: &AccessExpression,
    ) -> Result<bool, String> {
        match expr {
            AccessExpression::All(sub_exprs) => {
                for sub in sub_exprs {
                    if !Box::pin(Self::evaluate_expression(pool, character_id, sub)).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            AccessExpression::Any(sub_exprs) => {
                for sub in sub_exprs {
                    if Box::pin(Self::evaluate_expression(pool, character_id, sub)).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            AccessExpression::Not(sub_expr) => {
                Ok(!Box::pin(Self::evaluate_expression(pool, character_id, sub_expr)).await?)
            }
            AccessExpression::HasTag { subject_id, tag } => {
                // Check if character has this tag in their scope memberships
                let _ = subject_id;
                let row = sqlx::query_as::<_, CountRow>(
                    r#"
                    SELECT COUNT(*) as count
                    FROM character_scope_memberships
                    WHERE character_id = ? AND scope_value = ?
                    "#,
                )
                .bind(character_id)
                .bind(tag)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Failed to query tag: {}", e))?;

                Ok(row.count > 0)
            }
            AccessExpression::NumericAtLeast { path, value } => {
                // Extract numeric value from character's temporary_state or attributes
                let row = sqlx::query_as::<_, NumericValueRow>(
                    r#"
                    SELECT json_extract(temporary_state, ?) as value
                    FROM character_records
                    WHERE character_id = ?
                    LIMIT 1
                    "#,
                )
                .bind(path)
                .bind(character_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Failed to query numeric value: {}", e))?;

                Ok(row.map(|r| r.value >= *value).unwrap_or(false))
            }
            AccessExpression::BooleanFlag { path, expected } => {
                let row = sqlx::query_as::<_, BooleanValueRow>(
                    r#"
                    SELECT json_extract(temporary_state, ?) as value
                    FROM character_records
                    WHERE character_id = ?
                    LIMIT 1
                    "#,
                )
                .bind(path)
                .bind(character_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Failed to query boolean flag: {}", e))?;

                Ok(row.map(|r| r.value == *expected).unwrap_or(false))
            }
        }
    }

    /// Determine which content version to return for a character
    pub fn resolve_content(
        entry: &KnowledgeEntry,
        character_id: &str,
    ) -> (serde_json::Value, AccessSource) {
        // Check if this is a CharacterFacet and character is the subject
        if let KnowledgeSubject::Character { id, facet: _ } = &entry.subject {
            if id == character_id {
                // Subject accessing their own facet
                match &entry.subject_awareness {
                    crate::agent::models::SubjectAwareness::Aware => {
                        (entry.content.clone(), AccessSource::SelfFacetAware)
                    }
                    crate::agent::models::SubjectAwareness::Unaware { self_belief } => {
                        (self_belief.clone(), AccessSource::SelfFacetBelief)
                    }
                }
            } else {
                // Non-subject accessing character facet
                if let Some(apparent) = &entry.apparent_content {
                    (apparent.clone(), AccessSource::ApparentFromObservation)
                } else {
                    (entry.content.clone(), AccessSource::InKnownBy)
                }
            }
        } else {
            // Non-character knowledge
            if let Some(apparent) = &entry.apparent_content {
                (apparent.clone(), AccessSource::ApparentFromObservation)
            } else {
                (entry.content.clone(), AccessSource::InKnownBy)
            }
        }
    }
}

impl Default for KnowledgeAccessResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Character scope membership for access checking
#[derive(Debug, Clone)]
pub struct CharacterScopeMembership {
    pub character_id: String,
    pub scope_type: String,
    pub scope_value: String,
}

/// Scene context for access checking
#[derive(Debug, Clone)]
pub struct AccessSceneContext {
    pub scene_id: String,
    pub observable_characters: Vec<String>,
}

// ===== Database row types =====

#[derive(sqlx::FromRow)]
struct AccessLevelRow {
    access_level: f64,
}

#[derive(sqlx::FromRow)]
struct CountRow {
    count: i64,
}

#[derive(sqlx::FromRow)]
struct RealmRow {
    realm: Option<String>,
}

#[derive(sqlx::FromRow)]
struct NumericValueRow {
    value: f64,
}

#[derive(sqlx::FromRow)]
struct BooleanValueRow {
    value: bool,
}
