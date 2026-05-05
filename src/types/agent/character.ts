// Agent Character Types
// Corresponds to Rust types in src-tauri/src/agent/models/character.rs

/**
 * Mana expression tendency
 */
export type ManaExpressionTendency = 'Inward' | 'Neutral' | 'Expressive'

/**
 * Injury severity
 */
export type InjurySeverity = 'None' | 'Minor' | 'Moderate' | 'Severe' | 'Critical' | 'Crippling'

/**
 * Injury state
 */
export interface InjuryState {
  injury_id: string
  body_part: string
  severity: InjurySeverity
  description: string
  received_at: string
  expected_recovery_turns?: number
}

/**
 * Environmental exposure state
 */
export interface EnvironmentalExposure {
  cold_accumulation: number
  heat_accumulation: number
  toxin_accumulation: number
  mana_drain_accumulation: number
  last_exposure_turn: string | null
}

/**
 * Temporary character state
 */
export interface TemporaryCharacterState {
  injuries: InjuryState[]
  fatigue: number
  pain_load: number
  mana_reserve: number | null
  active_conditions: string[]
  environmental_exposure: EnvironmentalExposure
  position_in_scene: unknown | null
  active_skill_cooldowns: Record<string, number>
  temporary_modifiers: unknown[]
}

/**
 * Base attributes
 */
export interface BaseAttributes {
  physical: number
  agility: number
  endurance: number
  insight: number
  mana_power: number
  soul_strength: number
}

/**
 * Baseline body profile
 */
export interface BaselineBodyProfile {
  height_cm: number
  weight_kg: number
  build: string
  distinctive_features: string[]
  sensory_baseline: {
    vision: number
    hearing: number
    smell: number
    touch: number
    proprioception: number
    mana: number
  }
}

/**
 * Character record
 */
export interface CharacterRecord {
  character_id: string
  base_attributes: BaseAttributes
  baseline_body_profile: BaselineBodyProfile
  mana_expression_tendency: ManaExpressionTendency
  mana_expression_tendency_factor_override: number | null
  mind_model_card_knowledge_id: string | null
  temporary_state: TemporaryCharacterState
  schema_version: string
  created_at: string
  updated_at: string
}

/**
 * Create a default base attributes
 */
export function createDefaultBaseAttributes(): BaseAttributes {
  return {
    physical: 10,
    agility: 10,
    endurance: 10,
    insight: 10,
    mana_power: 0,
    soul_strength: 10,
  }
}

/**
 * Create a default baseline body profile
 */
export function createDefaultBaselineBodyProfile(): BaselineBodyProfile {
  return {
    height_cm: 170,
    weight_kg: 65,
    build: 'average',
    distinctive_features: [],
    sensory_baseline: {
      vision: 1.0,
      hearing: 1.0,
      smell: 1.0,
      touch: 1.0,
      proprioception: 1.0,
      mana: 0.5,
    },
  }
}

/**
 * Create a default temporary character state
 */
export function createDefaultTemporaryState(): TemporaryCharacterState {
  return {
    injuries: [],
    fatigue: 0,
    pain_load: 0,
    mana_reserve: null,
    active_conditions: [],
    environmental_exposure: {
      cold_accumulation: 0,
      heat_accumulation: 0,
      toxin_accumulation: 0,
      mana_drain_accumulation: 0,
      last_exposure_turn: null,
    },
    position_in_scene: null,
    active_skill_cooldowns: {},
    temporary_modifiers: [],
  }
}

/**
 * Create a new character record
 */
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
    mind_model_card_knowledge_id: null,
    temporary_state: createDefaultTemporaryState(),
    schema_version: '0.1',
    created_at: now,
    updated_at: now,
    ...overrides,
  }
}
