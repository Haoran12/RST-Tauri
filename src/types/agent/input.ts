// Input Preparser Types
// Corresponds to Rust types in src-tauri/src/agent/input_preparser.rs

/**
 * Preparsed segment type
 */
export type SegmentKind = 'Plain' | 'Quoted' | 'InnerThought' | 'DirectorBlock' | 'Command'

/**
 * A preparsed segment
 */
export interface PreparsedSegment {
  kind: SegmentKind
  text: string
  /** Start position in original input */
  start: number
  /** End position in original input */
  end: number
}

/**
 * Parsed meta command
 */
export interface ParsedCommand {
  command: string
  args: string[]
  raw: string
}

/**
 * Preparsed user input
 */
export interface PreparsedUserInput {
  original_text: string
  segments: PreparsedSegment[]
  /** Whether the input starts with / (command mode) */
  is_command_mode: boolean
  /** Parsed command if is_command_mode */
  command: ParsedCommand | null
  /** Warnings generated during parsing */
  warnings: string[]
}

/**
 * Authority class for user input
 */
export type InputAuthorityClass =
  | 'CharacterRoleplay'
  | 'SceneNarration'
  | 'MetaCommand'
  | 'DirectorHint'
  | 'Ambiguous'
  | 'Rejected'

/**
 * User input delta from SceneStateExtractor
 */
export interface UserInputDelta {
  /** Authority class for this input */
  authority_class: InputAuthorityClass
  /** Notes about authority resolution */
  authority_notes: string[]
  /** The preparsed input */
  preparsed: PreparsedUserInput
  /** Interpreted content (from SceneStateExtractor) */
  interpreted_content: unknown | null
}

// ===== Helper Functions =====

/**
 * Simple client-side input preparser
 * For full parsing, use the backend InputPreparser
 */
export function preprocessInput(text: string): PreparsedUserInput {
  const trimmed = text.trimStart()

  // Check for command mode
  if (trimmed.startsWith('/')) {
    const parts = trimmed.split(/\s+/)
    const commandRaw = parts[0] || ''
    const command = commandRaw.slice(1)
    const args = parts.slice(1)

    return {
      original_text: text,
      segments: [
        {
          kind: 'Command',
          text: trimmed,
          start: 0,
          end: text.length,
        },
      ],
      is_command_mode: true,
      command: {
        command,
        args,
        raw: trimmed,
      },
      warnings: [],
    }
  }

  // Simple client-side parsing
  const segments: PreparsedSegment[] = []
  const warnings: string[] = []

  // Regex patterns
  const patterns = [
    // Director block [[...]]
    { regex: /\[\[([^\]]*)\]\]/g, kind: 'DirectorBlock' as SegmentKind },
    // Inner thought *...*
    { regex: /\*([^*]+)\*/g, kind: 'InnerThought' as SegmentKind },
    // Quoted text "..."
    { regex: /"([^"]*)"/g, kind: 'Quoted' as SegmentKind },
    // Quoted text 「...」
    { regex: /「([^」]*)」/g, kind: 'Quoted' as SegmentKind },
  ]

  // Find all special segments
  const specialSegments: { start: number; end: number; kind: SegmentKind; text: string }[] = []

  for (const { regex, kind } of patterns) {
    let match
    const re = new RegExp(regex.source, regex.flags)
    while ((match = re.exec(text)) !== null) {
      specialSegments.push({
        start: match.index,
        end: match.index + match[0].length,
        kind,
        text: match[1] || match[0],
      })
    }
  }

  // Sort by position
  specialSegments.sort((a, b) => a.start - b.start)

  // Build segments
  let lastEnd = 0
  for (const seg of specialSegments) {
    // Add plain text before this segment
    if (seg.start > lastEnd) {
      const plainText = text.slice(lastEnd, seg.start).trim()
      if (plainText) {
        segments.push({
          kind: 'Plain',
          text: plainText,
          start: lastEnd,
          end: seg.start,
        })
      }
    }
    // Add the special segment
    segments.push({
      kind: seg.kind,
      text: seg.text,
      start: seg.start,
      end: seg.end,
    })
    lastEnd = seg.end
  }

  // Add remaining plain text
  if (lastEnd < text.length) {
    const plainText = text.slice(lastEnd).trim()
    if (plainText) {
      segments.push({
        kind: 'Plain',
        text: plainText,
        start: lastEnd,
        end: text.length,
      })
    }
  }

  // If no segments found, treat all as plain
  if (segments.length === 0 && text.trim()) {
    segments.push({
      kind: 'Plain',
      text: text.trim(),
      start: 0,
      end: text.length,
    })
  }

  return {
    original_text: text,
    segments,
    is_command_mode: false,
    command: null,
    warnings,
  }
}

/**
 * Check if input is a meta command
 */
export function isMetaCommand(text: string): boolean {
  return text.trimStart().startsWith('/')
}

/**
 * Get command name from input
 */
export function getCommandName(text: string): string | null {
  const trimmed = text.trimStart()
  if (!trimmed.startsWith('/')) {
    return null
  }
  const parts = trimmed.split(/\s+/)
  return parts[0]?.slice(1) || null
}

/**
 * Known meta commands
 */
export const META_COMMANDS = {
  scene: {
    description: '切换场景',
    usage: '/scene <location_id>',
  },
  back: {
    description: '回退会话到历史回合',
    usage: '/back <turn_id>',
  },
  fork: {
    description: '复制当前 World 副本并进入',
    usage: '/fork [new_world_name]',
  },
} as const

export type MetaCommandName = keyof typeof META_COMMANDS
