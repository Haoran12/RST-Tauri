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
  valid_from: unknown | null
  valid_until: unknown | null
  source_session_id: string | null
  source_scene_turn_id: string | null
  derived_from_event_id: string | null
  emotional_weight: number | null
  last_accessed_at: string | null
  source: string | null
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

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isAccessScopeType(value: unknown): value is AccessScopeType {
  return (
    value === 'Public' ||
    value === 'GodOnly' ||
    value === 'Region' ||
    value === 'Faction' ||
    value === 'Realm' ||
    value === 'Role' ||
    value === 'Bloodline'
  )
}

export function createAccessScope(
  type: AccessScopeType,
  value: string | null = null
): AccessScope {
  return type === 'Public' || type === 'GodOnly' ? { type } : { type, value: value ?? '' }
}

export function createDefaultAccessPolicy(): AccessPolicy {
  return {
    known_by: [],
    scope: [createAccessScope('Public')],
    conditions: [],
  }
}

function createBlankAccessPolicy(): AccessPolicy {
  return {
    known_by: [],
    scope: [],
    conditions: [],
  }
}

export function createDefaultKnowledgeMetadata(
  createdAt = new Date().toISOString(),
  updatedAt = createdAt
): KnowledgeMetadata {
  return {
    created_at: createdAt,
    updated_at: updatedAt,
    valid_from: null,
    valid_until: null,
    source_session_id: null,
    source_scene_turn_id: null,
    derived_from_event_id: null,
    emotional_weight: null,
    last_accessed_at: null,
    source: null,
  }
}

export function createDefaultSubjectAwareness(): SubjectAwareness {
  return { kind: 'Aware' }
}

export function createDefaultKnowledgeEntry(
  knowledgeId: string,
  overrides?: Partial<KnowledgeEntry>
): KnowledgeEntry {
  const now = new Date().toISOString()
  return normalizeKnowledgeEntry({
    knowledge_id: knowledgeId,
    kind: 'world_fact',
    subject_type: 'world',
    subject_id: null,
    facet_type: null,
    content: { summary_text: '' },
    apparent_content: null,
    access_policy: createDefaultAccessPolicy(),
    subject_awareness: createDefaultSubjectAwareness(),
    metadata: createDefaultKnowledgeMetadata(now, now),
    valid_from: null,
    valid_until: null,
    source_session_id: null,
    source_scene_turn_id: null,
    derived_from_event_id: null,
    schema_version: '0.1',
    created_at: now,
    updated_at: now,
    ...overrides,
  } as KnowledgeEntry)
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

function normalizeAccessScope(value: unknown): AccessScope | null {
  if (typeof value === 'string') {
    return isAccessScopeType(value) ? { type: value } : null
  }
  if (!isPlainObject(value)) {
    return null
  }

  const source = value as Record<string, unknown>
  if (isAccessScopeType(source.type)) {
    if (source.type === 'Public' || source.type === 'GodOnly') {
      return { type: source.type }
    }
    const rawValue =
      typeof source.value === 'string'
        ? source.value
        : typeof source[source.type] === 'string'
          ? String(source[source.type])
          : ''
    return { type: source.type, value: rawValue }
  }

  for (const type of ['Public', 'GodOnly', 'Region', 'Faction', 'Realm', 'Role', 'Bloodline'] as const) {
    if (!(type in source)) continue
    const rawValue = source[type]
    if (type === 'Public' || type === 'GodOnly') {
      return { type }
    }
    return {
      type,
      value: typeof rawValue === 'string' ? rawValue : rawValue == null ? '' : String(rawValue),
    }
  }

  return null
}

function normalizeAccessCondition(value: unknown): AccessCondition | null {
  if (typeof value === 'string') {
    return { kind: value }
  }
  if (!isPlainObject(value)) {
    return null
  }

  const source = value as Record<string, unknown>
  if (typeof source.kind === 'string') {
    const condition: AccessCondition = { kind: source.kind }
    if (isPlainObject(source.payload)) {
      condition.payload = source.payload
    } else {
      const payload: Record<string, unknown> = {}
      for (const [key, item] of Object.entries(source)) {
        if (key === 'kind' || key === 'payload') continue
        payload[key] = item
      }
      if (Object.keys(payload).length > 0) {
        condition.payload = payload
      }
    }
    return condition
  }

  const variant = Object.keys(source).find(key =>
    ['InSameSceneObservable', 'SocialAccessAtLeast', 'HasSkill', 'CultivationAtLeast', 'CustomPredicate'].includes(key)
  )
  if (!variant) {
    return null
  }

  const raw = source[variant]
  if (raw === null || raw === undefined) {
    return { kind: variant }
  }
  if (isPlainObject(raw)) {
    return { kind: variant, payload: raw }
  }
  return { kind: variant, payload: { value: raw } }
}

function normalizeKnowledgeMetadata(
  value: unknown,
  fallbackCreatedAt?: string,
  fallbackUpdatedAt?: string
): KnowledgeMetadata {
  const source = isPlainObject(value) ? value : {}
  const metadata = createDefaultKnowledgeMetadata(
    typeof fallbackCreatedAt === 'string' ? fallbackCreatedAt : new Date().toISOString(),
    typeof fallbackUpdatedAt === 'string'
      ? fallbackUpdatedAt
      : typeof fallbackCreatedAt === 'string'
        ? fallbackCreatedAt
        : new Date().toISOString()
  )

  if (typeof source.created_at === 'string') metadata.created_at = source.created_at
  if (typeof source.updated_at === 'string') metadata.updated_at = source.updated_at
  if ('valid_from' in source) metadata.valid_from = source.valid_from ?? null
  if ('valid_until' in source) metadata.valid_until = source.valid_until ?? null
  if ('source_session_id' in source) {
    metadata.source_session_id =
      typeof source.source_session_id === 'string' ? source.source_session_id : null
  }
  if ('source_scene_turn_id' in source) {
    metadata.source_scene_turn_id =
      typeof source.source_scene_turn_id === 'string' ? source.source_scene_turn_id : null
  }
  if ('derived_from_event_id' in source) {
    metadata.derived_from_event_id =
      typeof source.derived_from_event_id === 'string' ? source.derived_from_event_id : null
  }
  if ('emotional_weight' in source) {
    metadata.emotional_weight =
      typeof source.emotional_weight === 'number' ? source.emotional_weight : null
  }
  if ('last_accessed_at' in source) {
    metadata.last_accessed_at =
      typeof source.last_accessed_at === 'string' ? source.last_accessed_at : null
  }
  if ('source' in source) {
    metadata.source = typeof source.source === 'string' ? source.source : null
  }

  return metadata
}

function normalizeAccessPolicy(value: unknown): AccessPolicy {
  const source = isPlainObject(value) ? value : {}
  const policy = createBlankAccessPolicy()

  if (Array.isArray(source.known_by)) {
    policy.known_by = source.known_by.filter((item): item is string => typeof item === 'string')
  }

  if (Array.isArray(source.scope)) {
    policy.scope = source.scope
      .map(normalizeAccessScope)
      .filter((item): item is AccessScope => item !== null)
  }

  if (Array.isArray(source.conditions)) {
    policy.conditions = source.conditions
      .map(normalizeAccessCondition)
      .filter((item): item is AccessCondition => item !== null)
  }

  return policy
}

function normalizeSubjectAwareness(value: unknown): SubjectAwareness {
  if (typeof value === 'string') {
    return value === 'Unaware'
      ? { kind: 'Unaware', self_belief: { summary_text: '' } }
      : { kind: 'Aware' }
  }
  if (!isPlainObject(value)) {
    return { kind: 'Aware' }
  }

  const source = value as Record<string, unknown>
  if (source.kind === 'Unaware') {
    return {
      kind: 'Unaware',
      self_belief: source.self_belief ?? { summary_text: '' },
    }
  }

  if (source.kind === 'Aware' || source.Aware !== undefined) {
    return { kind: 'Aware' }
  }

  if ('Unaware' in source && isPlainObject(source.Unaware)) {
    const unaware = source.Unaware as Record<string, unknown>
    return {
      kind: 'Unaware',
      self_belief: unaware.self_belief ?? { summary_text: '' },
    }
  }

  return { kind: 'Aware' }
}

function normalizeContent(entry: KnowledgeEntry): unknown {
  if (
    entry.kind === 'character_facet' &&
    entry.facet_type === 'MindModelCard'
  ) {
    return normalizeMindModelCardContent(entry.content)
  }
  return entry.content
}

export function normalizeKnowledgeEntry(entry: KnowledgeEntry): KnowledgeEntry {
  return {
    ...entry,
    content: normalizeContent(entry),
    access_policy: normalizeAccessPolicy(entry.access_policy),
    subject_awareness: normalizeSubjectAwareness(entry.subject_awareness),
    metadata: normalizeKnowledgeMetadata(
      entry.metadata,
      entry.created_at,
      entry.updated_at
    ),
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
  return createDefaultKnowledgeEntry(knowledgeId, {
    kind: 'character_facet',
    subject_type: 'character',
    subject_id: characterId,
    facet_type: 'MindModelCard',
    content: createDefaultMindModelCardContent(),
    ...overrides,
  })
}
