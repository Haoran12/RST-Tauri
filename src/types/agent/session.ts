// Agent Session Types
// Corresponds to Rust types in src-tauri/src/agent/models/session.rs and common.rs

/**
 * Player mode for AgentSession
 * - Character: Player roleplays as a specific character in the World
 * - Director: Player acts as a world-external director
 */
export type PlayerMode = 'Character' | 'Director'

/**
 * Session kind for AgentSession
 */
export type AgentSessionKind = 'Mainline' | 'Retrospective' | 'FuturePreview'

/**
 * Canon status for AgentSession (whole session level)
 */
export type SessionCanonStatus = 'CanonCandidate' | 'PartiallyCanon' | 'NonCanon'

/**
 * Canon status for SessionTurn (chat message level)
 */
export type SessionTurnCanonStatus = 'CanonCandidate' | 'CanonPromoted' | 'ConflictWarned' | 'NonCanon'

/**
 * Canon status for WorldTurns (runtime turn level)
 */
export type RuntimeTurnCanonStatus = 'Canon' | 'ProvisionalPromoted' | 'ProvisionalOnly' | 'NonCanon' | 'FuturePreview'

/**
 * Conflict policy decision for past timeline sessions
 */
export type ConflictPolicyDecision = 'NonCanonAfterConflict' | 'WholeSessionNonCanon'

/**
 * Session status
 */
export type SessionStatus = 'Active' | 'Archived' | 'Deleted'

/**
 * Time precision level
 */
export type TimePrecision = 'Exact' | 'Day' | 'Period' | 'Era'

/**
 * Time anchor for story timeline
 * Must be programmatically comparable within the same World
 */
export interface TimeAnchor {
  /** Calendar system identifier */
  calendar_id: string
  /** Sortable time ordinal within the World */
  ordinal: number
  /** Time precision level */
  precision: TimePrecision
  /** Human-readable display text (for LLM/UI, not for sorting) */
  display_text: string
}

/**
 * Agent session - a chat/play session within a World
 */
export interface AgentSession {
  session_id: string
  world_id: string
  title: string
  session_kind: AgentSessionKind
  /** Time anchor for this session's period */
  period_anchor: TimeAnchor
  /** Player mode: Character or Director */
  player_mode: PlayerMode
  /** Player-controlled character ID (required for Character mode, must be null for Director mode) */
  player_character_id: string | null
  canon_status: SessionCanonStatus
  conflict_policy: ConflictPolicyDecision | null
  status: SessionStatus
  created_at: string
  updated_at: string
}

/**
 * Session turn - message within a session
 */
export interface SessionTurn {
  session_turn_id: string
  session_id: string
  /** Reference to world turn (if canonical) */
  scene_turn_id: string | null
  /** Local index within session */
  local_index: number
  role: 'User' | 'Assistant' | 'System'
  /** Message content (JSON) */
  message_json: unknown
  canon_status: SessionTurnCanonStatus
  created_at: string
}

/**
 * World mainline cursor - tracks current mainline frontier
 */
export interface WorldMainlineCursor {
  world_id: string
  /** Timeline ID (first version fixed to "main") */
  timeline_id: string
  /** Current mainline head turn ID */
  mainline_head_turn_id: string | null
  /** Current mainline time anchor */
  mainline_time_anchor: TimeAnchor
  updated_at: string
}

// ===== Helper Functions =====

/**
 * Create a default TimeAnchor
 */
export function createTimeAnchor(
  ordinal: number,
  displayText: string,
  precision: TimePrecision = 'Exact',
  calendarId: string = 'default'
): TimeAnchor {
  return {
    calendar_id: calendarId,
    ordinal,
    precision,
    display_text: displayText,
  }
}

/**
 * Create a new AgentSession with explicit player mode
 */
export function createAgentSession(
  worldId: string,
  title: string,
  sessionKind: AgentSessionKind,
  periodAnchor: TimeAnchor,
  playerMode: PlayerMode,
  playerCharacterId: string | null
): AgentSession | null {
  // Validate player_mode and player_character_id consistency
  if (playerMode === 'Character' && !playerCharacterId) {
    console.error('Character mode requires player_character_id')
    return null
  }
  if (playerMode === 'Director' && playerCharacterId) {
    console.error('Director mode must not have player_character_id')
    return null
  }

  const now = new Date().toISOString()
  return {
    session_id: `session_${Date.now().toString(16)}_${Math.random().toString(16).slice(2, 10)}`,
    world_id: worldId,
    title,
    session_kind: sessionKind,
    period_anchor: periodAnchor,
    player_mode: playerMode,
    player_character_id: playerCharacterId,
    canon_status: 'CanonCandidate',
    conflict_policy: null,
    status: 'Active',
    created_at: now,
    updated_at: now,
  }
}

/**
 * Determine session kind from period anchor and mainline cursor
 */
export function determineSessionKind(
  periodAnchor: TimeAnchor,
  mainlineTimeAnchor: TimeAnchor
): AgentSessionKind {
  if (periodAnchor.ordinal < mainlineTimeAnchor.ordinal) {
    return 'Retrospective'
  } else if (periodAnchor.ordinal > mainlineTimeAnchor.ordinal) {
    return 'FuturePreview'
  }
  return 'Mainline'
}

/**
 * Check if a session is in Character mode
 */
export function isCharacterMode(session: AgentSession): boolean {
  return session.player_mode === 'Character'
}

/**
 * Check if a session is in Director mode
 */
export function isDirectorMode(session: AgentSession): boolean {
  return session.player_mode === 'Director'
}

/**
 * Validate session player_mode and player_character_id consistency
 */
export function validateSession(session: AgentSession): string | null {
  if (session.player_mode === 'Character' && !session.player_character_id) {
    return 'Character mode requires player_character_id'
  }
  if (session.player_mode === 'Director' && session.player_character_id) {
    return 'Director mode must not have player_character_id'
  }
  return null
}

// ===== Past Timeline Types =====

/**
 * Truth constraint kind
 */
export type TruthConstraintKind = 'RequiredOutcome' | 'ForbiddenOutcome' | 'KnownAfterEffect'

/**
 * Detail kind for open detail slots
 */
export type DetailKind = 'Motive' | 'Dialogue' | 'Witness' | 'Route' | 'LocalCause'

/**
 * Promotion policy for detail slots
 */
export type PromotionPolicy = 'PromoteIfConsistent' | 'TraceOnly'

/**
 * Truth constraint
 */
export interface TruthConstraint {
  constraint_id: string
  source_knowledge_id: string
  constraint_kind: TruthConstraintKind
  applies_to_refs: string[]
  structured_payload: unknown
}

/**
 * Open detail slot - can be filled in past timeline sessions
 */
export interface OpenDetailSlot {
  slot_id: string
  source_event_id: string
  detail_kind: DetailKind
  promotion_policy: PromotionPolicy
}

/**
 * Truth guidance for past timeline sessions
 */
export interface TruthGuidance {
  session_id: string
  period_anchor: TimeAnchor
  related_event_ids: string[]
  hard_constraints: TruthConstraint[]
  soft_context: string[]
  open_detail_slots: OpenDetailSlot[]
  future_knowledge_warnings: string[]
}

/**
 * Provisional candidate kind
 */
export type ProvisionalCandidateKind = 'KnowledgeEntry' | 'EventDetail' | 'RelationDetail' | 'LocationDetail'

/**
 * Promotion status for provisional truth
 */
export type PromotionStatus = 'Pending' | 'Promoted' | 'BlockedConflict' | 'NonCanon' | 'TraceOnly'

/**
 * Provisional session truth - candidate facts from past timeline
 */
export interface ProvisionalSessionTruth {
  provisional_id: string
  session_id: string
  source_session_turn_id: string
  source_scene_turn_id: string | null
  story_time_anchor: TimeAnchor
  derived_from_event_id: string | null
  candidate_kind: ProvisionalCandidateKind
  candidate_payload: unknown
  promotion_status: PromotionStatus
  promoted_knowledge_id: string | null
  promoted_scene_turn_id: string | null
  created_at: string
  updated_at: string
}

/**
 * Slot conflict kind
 */
export type SlotConflictKind = 'HardConstraint' | 'TimelineConflict' | 'CharacterConflict' | 'LocationConflict'

/**
 * Slot conflict
 */
export interface SlotConflict {
  conflict_kind: SlotConflictKind
  constraint_id: string
  description: string
}

/**
 * Slot validation result
 */
export interface SlotValidationResult {
  is_valid: boolean
  conflicts: SlotConflict[]
  warnings: string[]
}

/**
 * Detail slot fill result
 */
export interface DetailSlotFillResult {
  provisional_id: string
  slot_id: string
  event_id: string
  validation_result: SlotValidationResult
  can_promote: boolean
}

// ===== Canon Status Types =====

/**
 * Promotable candidate
 */
export interface PromotableCandidate {
  provisional_id: string
  candidate_kind: ProvisionalCandidateKind
  confidence: number
  warnings: string[]
}

/**
 * Blocked candidate
 */
export interface BlockedCandidate {
  provisional_id: string
  violations: string[]
}

/**
 * Promotion evaluation result
 */
export interface PromotionEvaluationResult {
  session_id: string
  promotable_count: number
  blocked_count: number
  promotable: PromotableCandidate[]
  blocked: BlockedCandidate[]
  warnings: string[]
  evaluated_at: string
}

/**
 * Promotion result
 */
export interface PromotionResult {
  session_id: string
  promoted_count: number
  failed_count: number
  promoted_ids: string[]
  failed_ids: string[]
  new_session_status: SessionCanonStatus
  promoted_at: string
}

/**
 * Conflict report
 */
export interface ConflictReport {
  conflict_id: string
  session_id: string
  session_turn_id: string
  scene_turn_id: string | null
  severity: 'Soft' | 'Hard'
  source_constraint_ids: string[]
  affected_provisional_ids: string[]
  policy_decision: ConflictPolicyDecision | null
  summary: unknown
  created_at: string
  resolved_at: string | null
}
