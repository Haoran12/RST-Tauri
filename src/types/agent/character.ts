export type ManaAttribute = 'Metal' | 'Wood' | 'Water' | 'Fire' | 'Earth' | 'Wind'

export type ManaExpressionTendency = 'Inward' | 'Neutral' | 'Expressive'

export type ManaExpressionMode = 'Sealed' | 'Suppressed' | 'Natural' | 'Released' | 'Dominating'

export type ManaPresenceRadiusTier = 'SelfOnly' | 'Touch' | 'Close' | 'Room' | 'Area' | 'Scene'

export type ManaExpressionIntentionality = 'Intentional' | 'Unintentional' | 'Forced'

export type InjurySeverity = 'Bruise' | 'Light' | 'Moderate' | 'Severe' | 'Critical'

export type SizeClass = 'Tiny' | 'Small' | 'Humanoid' | 'Large' | 'Huge' | 'Kaiju'

export type StateDomain =
  | 'Body'
  | 'Resource'
  | 'Position'
  | 'Perception'
  | 'Mind'
  | 'Soul'
  | 'Scene'
  | 'KnowledgeReveal'

export interface BaseAttributes {
  physical: number
  agility: number
  endurance: number
  insight: number
  mana_power: number
  soul_strength: number
}

export interface ManaSenseBaseline {
  acuity: number
  overload_threshold: number
  attribute_bias: ManaAttribute | null
}

export interface BaselineBodyProfile {
  species: string
  comfort_temperature_range: [number, number]
  mana_sense_baseline: ManaSenseBaseline
  mana_attribute_affinity: ManaAttribute[]
  size_class: SizeClass
}

export interface InjuryState {
  injury_id: string
  body_region: string
  severity: InjurySeverity
  effect_tags: string[]
  source_event_id: string | null
}

export interface ManaExpressionState {
  mode: ManaExpressionMode
  display_ratio: number
  pressure_ratio: number
  radius_tier: ManaPresenceRadiusTier
  intentionality: ManaExpressionIntentionality
  source_id: string | null
  expires_at_turn: string | null
}

export interface ManaSuppressionState {
  source_id: string
  multiplier: number
  expires_at_turn: string | null
}

export interface EnvironmentalExposureState {
  cold_strain: number
  heat_strain: number
  respiration_strain: number
  last_updated_turn: string | null
}

export interface ConditionState {
  condition_id: string
  domain: StateDomain
  condition_kind: string
  intensity: number
  source_id: string | null
}

export interface CooldownState {
  ability_id: string
  remaining_turns: number
}

export interface TemporaryCharacterState {
  injuries: InjuryState[]
  fatigue: number
  pain_load: number
  mana_reserve_current: number | null
  mana_expression: ManaExpressionState
  mana_suppression: ManaSuppressionState[]
  environmental_exposure: EnvironmentalExposureState
  active_conditions: ConditionState[]
  cooldowns: CooldownState[]
  transient_signals: string[]
  schema_version: string
}

export interface CharacterRecord {
  character_id: string
  base_attributes: BaseAttributes
  baseline_body_profile: BaselineBodyProfile
  mana_expression_tendency: ManaExpressionTendency
  mana_expression_tendency_factor_override: number | null
  mind_model_card_knowledge_id: string
  temporary_state: TemporaryCharacterState
  schema_version: string
  created_at: string
  updated_at: string
}

export function createDefaultBaseAttributes(): BaseAttributes {
  return {
    physical: 100,
    agility: 100,
    endurance: 100,
    insight: 100,
    mana_power: 0,
    soul_strength: 100,
  }
}

export function createDefaultBaselineBodyProfile(): BaselineBodyProfile {
  return {
    species: '人类',
    comfort_temperature_range: [18, 26],
    mana_sense_baseline: {
      acuity: 0.5,
      overload_threshold: 1.0,
      attribute_bias: null,
    },
    mana_attribute_affinity: [],
    size_class: 'Humanoid',
  }
}

export function createDefaultTemporaryState(): TemporaryCharacterState {
  return {
    injuries: [],
    fatigue: 0,
    pain_load: 0,
    mana_reserve_current: null,
    mana_expression: {
      mode: 'Natural',
      display_ratio: 1,
      pressure_ratio: 1,
      radius_tier: 'Close',
      intentionality: 'Intentional',
      source_id: null,
      expires_at_turn: null,
    },
    mana_suppression: [],
    environmental_exposure: {
      cold_strain: 0,
      heat_strain: 0,
      respiration_strain: 0,
      last_updated_turn: null,
    },
    active_conditions: [],
    cooldowns: [],
    transient_signals: [],
    schema_version: '0.1',
  }
}

export function createCharacterRecord(
  characterId: string,
  overrides?: Partial<CharacterRecord>
): CharacterRecord {
  const now = new Date().toISOString()
  return {
    character_id: characterId,
    base_attributes: createDefaultBaseAttributes(),
    baseline_body_profile: createDefaultBaselineBodyProfile(),
    mana_expression_tendency: 'Neutral',
    mana_expression_tendency_factor_override: null,
    mind_model_card_knowledge_id: '',
    temporary_state: createDefaultTemporaryState(),
    schema_version: '0.1',
    created_at: now,
    updated_at: now,
    ...overrides,
  }
}
