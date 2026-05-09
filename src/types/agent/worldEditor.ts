// Agent World Editor Types
// Corresponds to Rust types in src-tauri/src/agent/world_editor/

import type { KnowledgeEntry } from './knowledge'
import type { CharacterRecord } from './character'

export type EditorEntityType =
  | 'world_settings'
  | 'location'
  | 'knowledge'
  | 'character'
  | 'relationship'
  | 'world_rules'

export type EditorEntityStatus = 'idle' | 'loading' | 'saving' | 'validating' | 'error'

export type ValidationSeverity = 'info' | 'warning' | 'blocker'

export interface EditorValidationItem {
  severity: ValidationSeverity
  code: string
  message: string
  field_path?: string
  entity_id?: string
}

export interface EditorImpactItem {
  kind: 'blocking' | 'warning' | 'destructive' | 'cascade' | 'info'
  target_entity_type: string
  target_entity_id: string
  description: string
  affected_count?: number
}

export interface WorldEditorSnapshot {
  world_id: string
  editor_revision: number
  world_status: 'paused' | 'running' | 'active_turn' | 'pending_llm' | 'needs_rollback'
  locations: LocationNodeSummary[]
  knowledges: KnowledgeEntrySummary[]
  characters: CharacterRecordSummary[]
  relationships: RelationshipSummary[]
  world_rules_keys: string[]
}

export interface LocationNodeSummary {
  location_id: string
  name: string
  canonical_level: string
  parent_id: string | null
  status: string
}

export interface KnowledgeEntrySummary {
  knowledge_id: string
  kind: string
  subject_type: string
  subject_id: string | null
  facet_type: string | null
  summary_text: string
  has_god_only: boolean
  has_apparent_content: boolean
  updated_at: string
}

export interface CharacterRecordSummary {
  character_id: string
  base_attributes_summary: string
  mana_expression_tendency: string
  temporary_state_summary: string
}

export interface RelationshipSummary {
  relation_id: string
  subject_character_id: string
  target_character_id: string
  relation_kind: string
  access_level: number
}

export interface WorldEditorPatch {
  world_id: string
  base_editor_revision: number
  operations: WorldEditorOperation[]
  author_note?: string
}

export type WorldEditorOperation =
  | { kind: 'UpsertLocationNode'; payload: unknown }
  | { kind: 'DeleteLocationNode'; location_id: string }
  | { kind: 'UpsertLocationAlias'; payload: unknown }
  | { kind: 'DeleteLocationAlias'; normalized_alias: string; location_id: string }
  | { kind: 'UpsertLocationSpatialRelation'; payload: unknown }
  | { kind: 'DeleteLocationSpatialRelation'; relation_id: string }
  | { kind: 'UpsertLocationEdge'; payload: unknown }
  | { kind: 'DeleteLocationEdge'; edge_id: string }
  | { kind: 'UpsertKnowledgeEntry'; payload: Partial<KnowledgeEntry> }
  | { kind: 'DeleteKnowledgeEntry'; knowledge_id: string }
  | { kind: 'UpsertCharacterRecord'; payload: Partial<CharacterRecord> }
  | { kind: 'DeleteCharacterRecord'; character_id: string }
  | { kind: 'UpsertObjectiveRelationship'; payload: unknown }
  | { kind: 'DeleteObjectiveRelationship'; relation_id: string }
  | { kind: 'UpsertTemporalStateRecord'; payload: unknown }
  | { kind: 'DeleteTemporalStateRecord'; state_record_id: string }
  | { kind: 'UpsertWorldRules'; payload: unknown }

export interface WorldEditorValidationResult {
  is_valid: boolean
  blockers: EditorValidationItem[]
  warnings: EditorValidationItem[]
  info: EditorValidationItem[]
}

export interface WorldEditorCommitResult {
  success: boolean
  commit_id?: string
  new_revision: number
  error?: string
}

export interface WorldEditorDraftState {
  entityType: EditorEntityType
  entityId: string | null
  draft: unknown
  original: unknown | null
  isDirty: boolean
  isNew: boolean
}

// ===== Trace / Runtime Debug Types =====

export interface AgentTraceEvent {
  event_id: string
  event_type: 'cognitive_pass_start' | 'cognitive_pass_end' | 'llm_request' | 'llm_response' | 'state_commit' | 'knowledge_reveal' | 'scene_init' | 'scene_extract' | 'user_input' | 'rollback'
  timestamp: string
  scene_turn_id?: string
  character_id?: string
  summary: string
  details: Record<string, unknown>
  level: 'debug' | 'info' | 'warn' | 'error'
}

export interface AgentTraceFilter {
  eventTypes?: string[]
  characterId?: string
  sceneTurnId?: string
  level?: string[]
  search?: string
}

export interface ReactionWindowEntry {
  entry_id: string
  scene_turn_id: string
  character_id: string
  reaction_type: 'emotion' | 'dialogue' | 'action' | 'thought' | 'memory_access'
  content: string
  confidence: number
  latency_ms: number
  created_at: string
}

export interface LocationTreeNode extends LocationNodeSummary {
  children: LocationTreeNode[]
  depth: number
}

export interface KnowledgeSummaryLazyLoadState {
  loadedIds: Set<string>
  loadingIds: Set<string>
}
