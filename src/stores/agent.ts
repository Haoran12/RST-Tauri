import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  AgentSession,
  WorldMainlineCursor,
  TimeAnchor,
  PlayerMode,
} from '@/types/agent/session'
import type { CharacterRecord } from '@/types/agent/character'
import type { AgentWorldListItem, CreateAgentWorldInput } from '@/types/agent/world'
import { createAgentWorld, listAgentWorlds, deleteAgentWorld, deleteAgentSession } from '@/services/agentApi'

// ===== Input Types for Tauri Commands =====

export interface CreateSessionInput {
  world_id: string
  title: string
  player_mode: PlayerMode
  player_character_id: string | null
  period_anchor: TimeAnchor
}

export interface CreateTimeAnchorInput {
  ordinal: number
  display_text: string
  precision?: 'Exact' | 'Day' | 'Period' | 'Era'
  calendar_id?: string
}

// ===== Store Definition =====

export const useAgentStore = defineStore('agent', () => {
  // State
  const worlds = ref<AgentWorldListItem[]>([])
  const currentWorldId = ref<string | null>(null)
  const mainlineCursor = ref<WorldMainlineCursor | null>(null)
  const sessions = ref<AgentSession[]>([])
  const characters = ref<CharacterRecord[]>([])
  const isLoading = ref(false)
  const isWorldListLoading = ref(false)
  const error = ref<string | null>(null)

  // Computed
  const mainlineSessions = computed(() =>
    sessions.value.filter((s) => s.session_kind === 'Mainline')
  )

  const retrospectiveSessions = computed(() =>
    sessions.value.filter((s) => s.session_kind === 'Retrospective')
  )

  const futurePreviewSessions = computed(() =>
    sessions.value.filter((s) => s.session_kind === 'FuturePreview')
  )

  const activeSessions = computed(() =>
    sessions.value.filter((s) => s.status === 'Active')
  )

  const currentWorld = computed(() =>
    worlds.value.find((world) => world.world_id === currentWorldId.value) ?? null
  )

  const characterOptions = computed(() =>
    characters.value.map((c) => ({
      id: c.character_id,
      name: c.character_id, // TODO: get name from knowledge or character record
      description: '',
    }))
  )

  // Actions

  async function loadWorldList(): Promise<AgentWorldListItem[]> {
    isWorldListLoading.value = true
    error.value = null

    try {
      const list = await listAgentWorlds()
      worlds.value = list
      if (!currentWorldId.value && list.length > 0) {
        currentWorldId.value = list[0].world_id
      }
      return list
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isWorldListLoading.value = false
    }
  }

  async function createWorld(input: CreateAgentWorldInput): Promise<AgentWorldListItem> {
    error.value = null
    const world = await createAgentWorld(input)
    worlds.value = [world, ...worlds.value.filter((item) => item.world_id !== world.world_id)]
    currentWorldId.value = world.world_id
    await loadWorld(world.world_id)
    return world
  }

  /**
   * Load world data: cursor, sessions, characters
   */
  async function loadWorld(worldId: string): Promise<void> {
    currentWorldId.value = worldId
    isLoading.value = true
    error.value = null

    try {
      await Promise.all([
        loadMainlineCursor(worldId),
        loadSessions(worldId),
        loadCharacters(worldId),
      ])
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  /**
   * Load mainline cursor for a world
   */
  async function loadMainlineCursor(worldId: string): Promise<void> {
    try {
      mainlineCursor.value = await invoke<WorldMainlineCursor>(
        'get_world_mainline_cursor',
        { worldId }
      )
    } catch (e) {
      console.error('Failed to load mainline cursor:', e)
      throw e
    }
  }

  /**
   * Load sessions for a world
   */
  async function loadSessions(worldId: string): Promise<void> {
    try {
      sessions.value = await invoke<AgentSession[]>('list_agent_sessions', {
        worldId,
      })
    } catch (e) {
      console.error('Failed to load sessions:', e)
      throw e
    }
  }

  /**
   * Load characters for a world
   */
  async function loadCharacters(worldId: string): Promise<void> {
    try {
      characters.value = await invoke<CharacterRecord[]>('list_world_characters', {
        worldId,
      })
    } catch (e) {
      console.error('Failed to load characters:', e)
      throw e
    }
  }

  /**
   * Create a new session
   */
  async function createSession(input: CreateSessionInput): Promise<AgentSession> {
    const session = await invoke<AgentSession>('create_agent_session', { input })
    sessions.value.push(session)
    return session
  }

  /**
   * Get a single session
   */
  async function getSession(worldId: string, sessionId: string): Promise<AgentSession | null> {
    return await invoke<AgentSession | null>('get_agent_session', {
      worldId,
      sessionId,
    })
  }

  /**
   * Update session player mode
   */
  async function updateSessionPlayerMode(
    worldId: string,
    sessionId: string,
    playerMode: PlayerMode,
    playerCharacterId: string | null
  ): Promise<AgentSession> {
    const session = await invoke<AgentSession>('update_session_player_mode', {
      input: {
        world_id: worldId,
        session_id: sessionId,
        player_mode: playerMode,
        player_character_id: playerCharacterId,
      },
    })

    const index = sessions.value.findIndex((s) => s.session_id === sessionId)
    if (index >= 0) {
      sessions.value[index] = session
    }

    return session
  }

  /**
   * Create a time anchor
   */
  async function createTimeAnchor(input: CreateTimeAnchorInput): Promise<TimeAnchor> {
    return await invoke<TimeAnchor>('create_time_anchor', { input })
  }

  /**
   * Delete a world
   */
  async function deleteWorld(worldId: string): Promise<void> {
    await deleteAgentWorld(worldId)
    worlds.value = worlds.value.filter((w) => w.world_id !== worldId)
    if (currentWorldId.value === worldId) {
      clearWorld()
    }
  }

  /**
   * Delete a session
   */
  async function deleteSession(worldId: string, sessionId: string): Promise<void> {
    await deleteAgentSession({ world_id: worldId, session_id: sessionId })
    sessions.value = sessions.value.filter((s) => s.session_id !== sessionId)
  }

  /**
   * Clear current world data
   */
  function clearWorld(): void {
    currentWorldId.value = null
    mainlineCursor.value = null
    sessions.value = []
    characters.value = []
    error.value = null
  }

  return {
    // State
    worlds,
    currentWorldId,
    mainlineCursor,
    sessions,
    characters,
    isLoading,
    isWorldListLoading,
    error,

    // Computed
    currentWorld,
    mainlineSessions,
    retrospectiveSessions,
    futurePreviewSessions,
    activeSessions,
    characterOptions,

    // Actions
    loadWorldList,
    createWorld,
    deleteWorld,
    loadWorld,
    loadMainlineCursor,
    loadSessions,
    loadCharacters,
    createSession,
    getSession,
    updateSessionPlayerMode,
    deleteSession,
    createTimeAnchor,
    clearWorld,
  }
})
