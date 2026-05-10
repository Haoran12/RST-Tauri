// Agent Knowledge Types
// Corresponds to Rust types in src-tauri/src/agent/models/knowledge.rs

export type KnowledgeKind =
  | 'world_fact'
  | 'region_fact'
  | 'faction_fact'
  | 'character_facet'
  | 'historical_event'
  | 'memory'

export type KnowledgeSubjectType = 'world' | 'region' | 'faction' | 'character' | 'event'

export type CharacterFacetType =
  | 'Appearance'
  | 'Identity'
  | 'TrueName'
  | 'Species'
  | 'Bloodline'
  | 'CultivationRealm'
  | 'KnownAbility'
  | 'HiddenAbility'
  | 'Personality'
  | 'Background'
  | 'Motivation'
  | 'Trauma'
  | 'MindModelCard'

export type RiskTolerance = 'VeryLow' | 'Low' | 'Moderate' | 'High' | 'VeryHigh'

export type AccessScopeType =
  | 'Public'
  | 'GodOnly'
  | 'Region'
  | 'Faction'
  | 'Realm'
  | 'Role'
  | 'Bloodline'

export interface AccessScope {
  type: AccessScopeType
  value?: string
}

export interface AccessCondition {
  kind: string
  payload?: Record<string, unknown>
}

export interface AccessPolicy {
  known_by: string[]
  scope: AccessScope[]
  conditions: AccessCondition[]
}

export type SubjectAwareness =
  | { kind: 'Aware' }
  | { kind: 'Unaware'; self_belief: unknown }

export interface KnowledgeMetadata {
  created_at: string
  updated_at: string
  valid_from?: unknown
  valid_until?: unknown
  source_session_id?: string
  source_scene_turn_id?: string
  derived_from_event_id?: string
  emotional_weight?: number
  last_accessed_at?: string
  source?: string
}

export interface KnowledgeEntry {
  knowledge_id: string
  kind: KnowledgeKind
  subject_type: KnowledgeSubjectType
  subject_id: string | null
  facet_type: CharacterFacetType | null
  content: unknown
  apparent_content: unknown | null
  access_policy: AccessPolicy
  subject_awareness: SubjectAwareness
  metadata: KnowledgeMetadata
  valid_from: unknown | null
  valid_until: unknown | null
  source_session_id: string | null
  source_scene_turn_id: string | null
  derived_from_event_id: string | null
  schema_version: string
  created_at: string
  updated_at: string
}

export interface MindModelCardContent {
  summary_text: string
  attention_biases: string[]
  risk_tolerance: RiskTolerance
  default_social_strategy: string
  value_priorities: string[]
  cognitive_patterns: string[]
  extensions: Record<string, unknown>
}

export interface KnowledgeListItem {
  knowledge_id: string
  kind: KnowledgeKind
  subject_type: KnowledgeSubjectType
  subject_id: string | null
  facet_type: CharacterFacetType | null
  summary_text: string
  access_scope_summary: string
  updated_at: string
}

export interface KnowledgeRevealEvent {
  event_id: string
  knowledge_id: string
  newly_known_by: string[]
  trigger: string
  scope_change?: unknown
  scene_turn_id: string
}

export interface AccessibleEntry {
  knowledge_id: string
  kind: KnowledgeKind
  subject_type: KnowledgeSubjectType
  subject_id: string | null
  accessible_content: unknown
  source_hint: string
}

export interface AccessibleKnowledge {
  character_id: string
  scene_turn_id: string
  entries: AccessibleEntry[]
}

export const KNOWLEDGE_KIND_LABELS: Record<KnowledgeKind, string> = {
  world_fact: '世界事实',
  region_fact: '地区事实',
  faction_fact: '势力事实',
  character_facet: '角色分面',
  historical_event: '历史事件',
  memory: '记忆',
}

export const CHARACTER_FACET_LABELS: Record<CharacterFacetType, string> = {
  Appearance: '外貌',
  Identity: '身份',
  TrueName: '真名',
  Species: '种族',
  Bloodline: '血脉',
  CultivationRealm: '修为境界',
  KnownAbility: '已知能力',
  HiddenAbility: '隐藏能力',
  Personality: '性格',
  Background: '出身背景',
  Motivation: '动机',
  Trauma: '创伤',
  MindModelCard: '认知基线卡',
}

export function createDefaultMindModelCardContent(
  overrides?: Partial<MindModelCardContent>
): MindModelCardContent {
  return {
    summary_text: '',
    attention_biases: [],
    risk_tolerance: 'Moderate',
    default_social_strategy: '',
    value_priorities: [],
    cognitive_patterns: [],
    extensions: {},
    ...overrides,
  }
}

export function normalizeMindModelCardContent(value: unknown): MindModelCardContent {
  const source =
    value && typeof value === 'object' && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : {}

  return createDefaultMindModelCardContent({
    summary_text: typeof source.summary_text === 'string' ? source.summary_text : '',
    attention_biases: Array.isArray(source.attention_biases)
      ? source.attention_biases.filter((item): item is string => typeof item === 'string')
      : [],
    risk_tolerance: isRiskTolerance(source.risk_tolerance)
      ? source.risk_tolerance
      : 'Moderate',
    default_social_strategy:
      typeof source.default_social_strategy === 'string' ? source.default_social_strategy : '',
    value_priorities: Array.isArray(source.value_priorities)
      ? source.value_priorities.filter((item): item is string => typeof item === 'string')
      : [],
    cognitive_patterns: Array.isArray(source.cognitive_patterns)
      ? source.cognitive_patterns.filter((item): item is string => typeof item === 'string')
      : [],
    extensions:
      source.extensions && typeof source.extensions === 'object' && !Array.isArray(source.extensions)
        ? (source.extensions as Record<string, unknown>)
        : {},
  })
}

export function normalizeKnowledgeEntry(entry: KnowledgeEntry): KnowledgeEntry {
  if (entry.facet_type !== 'MindModelCard') {
    return entry
  }

  return {
    ...entry,
    content: normalizeMindModelCardContent(entry.content),
  }
}

function isRiskTolerance(value: unknown): value is RiskTolerance {
  return (
    value === 'VeryLow' ||
    value === 'Low' ||
    value === 'Moderate' ||
    value === 'High' ||
    value === 'VeryHigh'
  )
}

export function createMindModelCardKnowledgeEntry(
  knowledgeId: string,
  characterId: string,
  overrides?: Partial<KnowledgeEntry>
): KnowledgeEntry {
  const now = new Date().toISOString()
  return normalizeKnowledgeEntry({
    knowledge_id: knowledgeId,
    kind: 'character_facet',
    subject_type: 'character',
    subject_id: characterId,
    facet_type: 'MindModelCard',
    content: createDefaultMindModelCardContent(),
    apparent_content: null,
    access_policy: { known_by: [], scope: [{ type: 'Public' }], conditions: [] },
    subject_awareness: { kind: 'Aware' },
    metadata: { created_at: now, updated_at: now },
    valid_from: null,
    valid_until: null,
    source_session_id: null,
    source_scene_turn_id: null,
    derived_from_event_id: null,
    schema_version: '0.1',
    created_at: now,
    updated_at: now,
    ...overrides,
  })
}
