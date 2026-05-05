// Director Hint Types
// Corresponds to Rust types in src-tauri/src/agent/director_hint.rs

/**
 * Outcome direction hint
 */
export type OutcomeDirection =
  | 'Positive'
  | 'Negative'
  | 'Twist'
  | 'Bittersweet'
  | 'Ambiguous'

/**
 * Pacing hint
 */
export type PacingHint = 'Slow' | 'Normal' | 'Fast'

/**
 * Suggested event
 */
export interface SuggestedEvent {
  /** Event description (for LLM to interpret) */
  description: string
  /** Target characters */
  target_characters: string[]
  /** Importance (0.0-1.0) */
  importance: number
  /** Whether this is mandatory */
  mandatory: boolean
}

/**
 * Outcome bias - influence the result planning
 */
export interface OutcomeBias {
  /** Suggested outcome direction */
  direction?: OutcomeDirection
  /** Characters to favor */
  favor_characters: string[]
  /** Characters to disfavor */
  disfavor_characters: string[]
  /** Suggested events to occur */
  suggested_events: SuggestedEvent[]
  /** Events to avoid */
  avoid_events: string[]
  /** Desired tension level (0.0 = relaxed, 1.0 = maximum tension) */
  tension_level?: number
  /** Desired pacing */
  pacing?: PacingHint
}

/**
 * Tone hint
 */
export type ToneHint =
  | 'Serious'
  | 'Lighthearted'
  | 'Dramatic'
  | 'Mysterious'
  | 'Romantic'
  | 'Tense'
  | 'Melancholic'
  | 'Humorous'

/**
 * Perspective hint
 */
export type PerspectiveHint =
  | 'FirstPerson'
  | 'ThirdPersonLimited'
  | 'ThirdPersonOmniscient'
  | 'Cinematic'

/**
 * Detail level
 */
export type DetailLevel = 'Minimal' | 'Normal' | 'Detailed' | 'Exhaustive'

/**
 * Focus area for narrative
 */
export interface FocusArea {
  /** What to focus on */
  focus: string
  /** Importance weight */
  weight: number
}

/**
 * Formatting hint
 */
export type FormattingHint =
  | 'MoreDialogue'
  | 'LessDialogue'
  | 'MoreDescription'
  | 'LessDescription'
  | 'MoreAction'
  | 'LessAction'
  | 'ShorterSentences'
  | 'LongerSentences'
  | 'MoreInternalThought'
  | 'LessInternalThought'

/**
 * Style override - influence narrative presentation
 */
export interface StyleOverride {
  /** Tone hints */
  tone?: ToneHint
  /** Perspective hints */
  perspective?: PerspectiveHint
  /** Detail level */
  detail_level?: DetailLevel
  /** Focus areas */
  focus_areas: FocusArea[]
  /** Things to de-emphasize */
  de_emphasize: string[]
  /** Narrative voice hints */
  voice_hints?: string
  /** Formatting hints */
  formatting_hints: FormattingHint[]
}

/**
 * Turn scope for a hint
 */
export type TurnScope =
  | { kind: 'ThisTurn' }
  | { kind: 'NextTurns'; turns: number }
  | { kind: 'UntilCancelled' }
  | { kind: 'UntilCondition'; condition: string }

/**
 * Target scope for a director hint
 */
export interface HintTargetScope {
  /** Target characters (empty = all) */
  target_characters: string[]
  /** Target locations (empty = all) */
  target_locations: string[]
  /** Target events (empty = all) */
  target_events: string[]
  /** Turn scope */
  turn_scope: TurnScope
}

/**
 * Director hint input from player
 */
export interface DirectorHint {
  /** Unique hint ID */
  hint_id: string
  /** Outcome bias suggestions */
  outcome_bias?: OutcomeBias
  /** Style override suggestions */
  style_override?: StyleOverride
  /** Target scope for this hint */
  target_scope: HintTargetScope
  /** Priority level (higher = more important) */
  priority: number
  /** Whether this hint is mandatory */
  mandatory: boolean
  /** Notes from the director */
  notes?: string
}

/**
 * Director hint collection for a turn
 */
export interface DirectorHintCollection {
  /** All active hints */
  hints: DirectorHint[]
  /** Compiled outcome bias (merged from all hints) */
  compiled_outcome_bias?: OutcomeBias
  /** Compiled style override (merged from all hints) */
  compiled_style_override?: StyleOverride
}

// ===== Helper Functions =====

/**
 * Generate a hint ID
 */
export function generateHintId(): string {
  return `hint_${Date.now()}`
}

/**
 * Create a simple outcome bias hint
 */
export function createOutcomeBiasHint(
  direction: OutcomeDirection,
  options?: {
    favorCharacters?: string[]
    tensionLevel?: number
    pacing?: PacingHint
  }
): DirectorHint {
  return {
    hint_id: generateHintId(),
    outcome_bias: {
      direction,
      favor_characters: options?.favorCharacters || [],
      disfavor_characters: [],
      suggested_events: [],
      avoid_events: [],
      tension_level: options?.tensionLevel,
      pacing: options?.pacing,
    },
    style_override: undefined,
    target_scope: {
      target_characters: [],
      target_locations: [],
      target_events: [],
      turn_scope: { kind: 'ThisTurn' },
    },
    priority: 1,
    mandatory: false,
  }
}

/**
 * Create a simple style override hint
 */
export function createStyleOverrideHint(
  tone: ToneHint,
  options?: {
    perspective?: PerspectiveHint
    detailLevel?: DetailLevel
    voiceHints?: string
  }
): DirectorHint {
  return {
    hint_id: generateHintId(),
    outcome_bias: undefined,
    style_override: {
      tone,
      perspective: options?.perspective,
      detail_level: options?.detailLevel,
      focus_areas: [],
      de_emphasize: [],
      voice_hints: options?.voiceHints,
      formatting_hints: [],
    },
    target_scope: {
      target_characters: [],
      target_locations: [],
      target_events: [],
      turn_scope: { kind: 'ThisTurn' },
    },
    priority: 1,
    mandatory: false,
  }
}

/**
 * Create an empty director hint collection
 */
export function createDirectorHintCollection(): DirectorHintCollection {
  return {
    hints: [],
  }
}

/**
 * Add a hint to a collection
 */
export function addHintToCollection(
  collection: DirectorHintCollection,
  hint: DirectorHint
): DirectorHintCollection {
  return {
    hints: [...collection.hints, hint],
    // Note: compilation happens on backend
  }
}

/**
 * Remove a hint from a collection
 */
export function removeHintFromCollection(
  collection: DirectorHintCollection,
  hintId: string
): DirectorHintCollection {
  return {
    hints: collection.hints.filter((h) => h.hint_id !== hintId),
  }
}
