import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  EditorEntityType,
  WorldEditorSnapshot,
  WorldEditorDraftState,
  WorldEditorValidationResult,
  WorldEditorCommitResult,
  WorldEditorPatch,
  EditorValidationItem,
  EditorImpactItem,
  AgentTraceEvent,
  AgentTraceFilter,
  ReactionWindowEntry,
  LocationTreeNode,
  LocationNodeSummary,
  LocationNodeDetailDto,
  CharacterRecordSummary,
} from '@/types/agent/worldEditor'
import type { KnowledgeEntry, KnowledgeListItem } from '@/types/agent/knowledge'
import type { CharacterRecord } from '@/types/agent/character'
import { listWorldCharacters } from '@/services/agentApi'

export const useAgentWorldEditorStore = defineStore('agentWorldEditor', () => {
  // ===== State =====
  const snapshot = ref<WorldEditorSnapshot | null>(null)
  const isLoading = ref(false)
  const isSaving = ref(false)
  const isValidating = ref(false)
  const isAnalyzingImpact = ref(false)
  const worldStatus = ref<string>('paused')
  const editorRevision = ref(0)

  const selectedEntityType = ref<EditorEntityType>('none')
  const selectedEntityId = ref<string | null>(null)

  const knowledgeList = ref<KnowledgeListItem[]>([])
  const characterList = ref<CharacterRecordSummary[]>([])
  const locationList = ref<LocationNodeSummary[]>([])

  const draft = ref<WorldEditorDraftState | null>(null)
  const validationResult = ref<WorldEditorValidationResult | null>(null)
  const impactSummary = ref<EditorImpactItem[]>([])
  const lastCommitResult = ref<WorldEditorCommitResult | null>(null)

  const knowledgeFilterKind = ref<string>('')
  const knowledgeFilterSearch = ref('')
  const knowledgeFilterSubjectId = ref('')

  // Knowledge lazy-loading state
  const knowledgeLoadedIds = ref<Set<string>>(new Set())
  const knowledgeLoadingIds = ref<Set<string>>(new Set())

  // Trace / Reaction debug state
  const traceEvents = ref<AgentTraceEvent[]>([])
  const traceFilter = ref<AgentTraceFilter>({})
  const isLoadingTrace = ref(false)
  const reactionEntries = ref<ReactionWindowEntry[]>([])
  const isLoadingReactions = ref(false)
  const showDebugPanel = ref(false)
  const debugPanelTab = ref<'trace' | 'reaction'>('trace')

  // ===== Computed =====
  const canCommit = computed(() => {
    if (!draft.value?.isDirty) return false
    if (worldStatus.value !== 'paused') return false
    if (isSaving.value || isValidating.value || isAnalyzingImpact.value) return false
    if (!validationResult.value) return false
    return validationResult.value.blockers.length === 0
  })

  const canValidate = computed(() => {
    return draft.value?.isDirty === true && !isSaving.value && !isValidating.value
  })

  const hasBlockers = computed(() => {
    return (validationResult.value?.blockers.length ?? 0) > 0
  })

  const filteredKnowledgeList = computed(() => {
    let list = knowledgeList.value
    if (knowledgeFilterKind.value) {
      list = list.filter(k => k.kind === knowledgeFilterKind.value)
    }
    if (knowledgeFilterSubjectId.value) {
      list = list.filter(k => k.subject_id === knowledgeFilterSubjectId.value)
    }
    if (knowledgeFilterSearch.value) {
      const q = knowledgeFilterSearch.value.toLowerCase()
      list = list.filter(
        k =>
          k.knowledge_id.toLowerCase().includes(q) ||
          k.summary_text.toLowerCase().includes(q)
      )
    }
    return list
  })

  const selectedKnowledge = computed<KnowledgeEntry | null>(() => {
    if (selectedEntityType.value !== 'knowledge' || !selectedEntityId.value) return null
    return draft.value?.entityId === selectedEntityId.value
      ? (draft.value.draft as KnowledgeEntry)
      : null
  })

  const blockers = computed<EditorValidationItem[]>(() => validationResult.value?.blockers ?? [])
  const warnings = computed<EditorValidationItem[]>(() => validationResult.value?.warnings ?? [])
  const infos = computed<EditorValidationItem[]>(() => validationResult.value?.info ?? [])

  const locationTree = computed<LocationTreeNode[]>(() => {
    const nodes = locationList.value
    const map = new Map<string, LocationTreeNode>()
    nodes.forEach(n => {
      map.set(n.location_id, { ...n, children: [], depth: 0 })
    })
    const roots: LocationTreeNode[] = []
    nodes.forEach(n => {
      const node = map.get(n.location_id)!
      if (n.parent_id && map.has(n.parent_id)) {
        const parent = map.get(n.parent_id)!
        node.depth = parent.depth + 1
        parent.children.push(node)
      } else {
        node.depth = 0
        roots.push(node)
      }
    })
    return roots
  })

  const filteredTraceEvents = computed(() => {
    let list = traceEvents.value
    if (traceFilter.value.eventTypes?.length) {
      list = list.filter(e => traceFilter.value.eventTypes!.includes(e.event_type))
    }
    if (traceFilter.value.characterId) {
      list = list.filter(e => e.character_id === traceFilter.value.characterId)
    }
    if (traceFilter.value.sceneTurnId) {
      list = list.filter(e => e.scene_turn_id === traceFilter.value.sceneTurnId)
    }
    if (traceFilter.value.level?.length) {
      list = list.filter(e => traceFilter.value.level!.includes(e.level))
    }
    if (traceFilter.value.search) {
      const q = traceFilter.value.search.toLowerCase()
      list = list.filter(e => e.summary.toLowerCase().includes(q))
    }
    return list
  })

  // ===== Actions =====

  async function loadSnapshot(worldId: string): Promise<void> {
    isLoading.value = true
    try {
      const result = await invoke<WorldEditorSnapshot>('get_world_editor_snapshot', {
        worldId,
      })
      snapshot.value = result
      worldStatus.value = result.world_status
      editorRevision.value = result.editor_revision

      // Hydrate lists from snapshot
      knowledgeList.value = result.knowledges.map(k => ({
        knowledge_id: k.knowledge_id,
        kind: k.kind as any,
        subject_type: k.subject_type as any,
        subject_id: k.subject_id,
        facet_type: k.facet_type as any,
        summary_text: k.summary_text,
        access_scope_summary: k.has_god_only ? 'GodOnly' : 'Mixed',
        updated_at: k.updated_at,
      }))
      characterList.value = result.characters
      locationList.value = result.locations

      // Reset lazy-load state
      knowledgeLoadedIds.value = new Set()
      knowledgeLoadingIds.value = new Set()
      selectedEntityType.value = 'none'
      selectedEntityId.value = null
      clearDraft()
    } catch (e) {
      console.error('Failed to load world editor snapshot:', e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  function selectEntity(type: EditorEntityType, id: string | null) {
    selectedEntityType.value = type
    selectedEntityId.value = id
    // Discard unsaved draft if switching to different entity
    if (draft.value && draft.value.entityId !== id) {
      draft.value = null
      validationResult.value = null
      impactSummary.value = []
    }
  }

  function initDraft<T>(type: EditorEntityType, id: string | null, data: T, isNew = false) {
    draft.value = {
      entityType: type,
      entityId: id,
      draft: data,
      original: isNew ? null : JSON.parse(JSON.stringify(data)),
      isDirty: true,
      isNew,
    }
    validationResult.value = null
    impactSummary.value = []
  }

  function updateDraftField(path: string, value: unknown) {
    if (!draft.value) return
    const keys = path.split('.')
    let target: Record<string, unknown> = draft.value.draft as Record<string, unknown>
    for (let i = 0; i < keys.length - 1; i++) {
      target = target[keys[i]] as Record<string, unknown>
      if (!target) return
    }
    target[keys[keys.length - 1]] = value
    draft.value.isDirty = true
  }

  async function validateDraft(worldId: string): Promise<WorldEditorValidationResult> {
    if (!draft.value) {
      return { is_valid: false, blockers: [], warnings: [], info: [] }
    }
    isValidating.value = true
    try {
      const patch = buildPatchFromDraft()

      const result = await invoke<WorldEditorValidationResult>('validate_world_editor_patch', {
        worldId,
        patch,
      })
      validationResult.value = result
      return result
    } catch (e) {
      console.error('Validation failed:', e)
      const fallback: WorldEditorValidationResult = {
        is_valid: false,
        blockers: [{ severity: 'blocker', code: 'validation_error', message: String(e) }],
        warnings: [],
        info: [],
      }
      validationResult.value = fallback
      return fallback
    } finally {
      isValidating.value = false
    }
  }

  async function analyzeImpact(worldId: string, entityType: EditorEntityType, entityId: string): Promise<EditorImpactItem[]> {
    isAnalyzingImpact.value = true
    try {
      const impacts = await invoke<EditorImpactItem[]>('analyze_world_editor_impact', {
        worldId,
        entityType,
        entityId,
      })
      impactSummary.value = impacts
      return impacts
    } catch (e) {
      console.error('Impact analysis failed:', e)
      impactSummary.value = []
      return []
    } finally {
      isAnalyzingImpact.value = false
    }
  }

  async function commitDraft(worldId: string): Promise<WorldEditorCommitResult> {
    if (!canCommit.value || !draft.value) {
      return { success: false, error: '当前状态不允许提交', new_revision: editorRevision.value }
    }
    isSaving.value = true
    try {
      const patch = buildPatchFromDraft()

      const result = await invoke<WorldEditorCommitResult>('commit_world_editor_patch', {
        worldId,
        patch,
      })
      lastCommitResult.value = result
      if (result.success) {
        draft.value.isDirty = false
        draft.value.original = JSON.parse(JSON.stringify(draft.value.draft))
        editorRevision.value = result.new_revision
      }
      return result
    } catch (e) {
      console.error('Commit failed:', e)
      const fallback: WorldEditorCommitResult = {
        success: false,
        error: String(e),
        new_revision: editorRevision.value,
      }
      lastCommitResult.value = fallback
      return fallback
    } finally {
      isSaving.value = false
    }
  }

  function buildPatchFromDraft(): WorldEditorPatch {
    const patch: WorldEditorPatch = {
      world_id: snapshot.value?.world_id ?? '',
      base_editor_revision: editorRevision.value,
      operations: [],
    }
    if (!draft.value) return patch

    if (draft.value.entityType === 'knowledge' && draft.value.draft) {
      patch.operations.push({
        kind: 'UpsertKnowledgeEntry',
        payload: draft.value.draft as Partial<KnowledgeEntry>,
      })
    } else if (draft.value.entityType === 'character' && draft.value.draft) {
      patch.operations.push({
        kind: 'UpsertCharacterRecord',
        payload: draft.value.draft as Partial<CharacterRecord>,
      })
    } else if (draft.value.entityType === 'location' && draft.value.draft) {
      patch.operations.push({
        kind: 'UpsertLocationNode',
        payload: draft.value.draft,
      })
    } else if (draft.value.entityType === 'world_rules' && draft.value.draft) {
      patch.operations.push({
        kind: 'UpsertWorldRules',
        payload: draft.value.draft,
      })
    }
    return patch
  }

  function clearDraft() {
    draft.value = null
    validationResult.value = null
    impactSummary.value = []
    lastCommitResult.value = null
  }

  function clearValidationResult() {
    validationResult.value = null
  }

  function clearImpactSummary() {
    impactSummary.value = []
  }

  // ===== Knowledge Lazy Loading =====

  async function loadKnowledgeDetail(worldId: string, knowledgeId: string): Promise<KnowledgeEntry | null> {
    knowledgeLoadingIds.value.add(knowledgeId)
    try {
      const entry = await invoke<KnowledgeEntry>('get_knowledge_entry_detail', {
        worldId,
        knowledgeId,
      })
      knowledgeLoadedIds.value.add(knowledgeId)
      return entry
    } catch (e) {
      console.error('Failed to load knowledge detail:', e)
      return null
    } finally {
      knowledgeLoadingIds.value.delete(knowledgeId)
    }
  }

  async function loadCharacterDetail(worldId: string, characterId: string): Promise<CharacterRecord | null> {
    try {
      const characters = await listWorldCharacters(worldId)
      return characters.find((character) => character.character_id === characterId) ?? null
    } catch (e) {
      console.error('Failed to load character detail:', e)
      return null
    }
  }

  async function loadLocationDetail(worldId: string, locationId: string): Promise<LocationNodeDetailDto | null> {
    try {
      return await invoke<LocationNodeDetailDto>('get_location_node_detail', {
        worldId,
        locationId,
      })
    } catch (e) {
      console.error('Failed to load location detail:', e)
      return null
    }
  }

  // ===== Location Tree Drag =====

  async function updateLocationParent(locationId: string, newParentId: string | null): Promise<void> {
    const loc = locationList.value.find(l => l.location_id === locationId)
    if (!loc) return
    const updated = { ...loc, parent_id: newParentId }
    const index = locationList.value.findIndex(l => l.location_id === locationId)
    if (index >= 0) {
      locationList.value[index] = updated as LocationNodeSummary
    }
    // Also update draft if this location is being edited
    if (draft.value?.entityType === 'location' && draft.value.entityId === locationId) {
      updateDraftField('parent_id', newParentId)
    }
  }

  // ===== Trace / Reaction =====

  async function loadTraceEvents(worldId: string, limit = 200): Promise<void> {
    isLoadingTrace.value = true
    try {
      const events = await invoke<AgentTraceEvent[]>('get_agent_trace_events', {
        worldId,
        limit,
      })
      traceEvents.value = events
    } catch (e) {
      console.error('Failed to load trace events:', e)
      traceEvents.value = []
    } finally {
      isLoadingTrace.value = false
    }
  }

  async function loadReactionEntries(worldId: string, sessionId?: string): Promise<void> {
    isLoadingReactions.value = true
    try {
      const entries = await invoke<ReactionWindowEntry[]>('get_reaction_window_entries', {
        worldId,
        sessionId: sessionId ?? null,
      })
      reactionEntries.value = entries
    } catch (e) {
      console.error('Failed to load reaction entries:', e)
      reactionEntries.value = []
    } finally {
      isLoadingReactions.value = false
    }
  }

  function setTraceFilter(filter: AgentTraceFilter) {
    traceFilter.value = { ...traceFilter.value, ...filter }
  }

  function toggleDebugPanel(show?: boolean) {
    showDebugPanel.value = show ?? !showDebugPanel.value
  }

  return {
    // State
    snapshot,
    isLoading,
    isSaving,
    isValidating,
    isAnalyzingImpact,
    worldStatus,
    editorRevision,
    selectedEntityType,
    selectedEntityId,
    knowledgeList,
    characterList,
    locationList,
    draft,
    validationResult,
    impactSummary,
    lastCommitResult,
    knowledgeFilterKind,
    knowledgeFilterSearch,
    knowledgeFilterSubjectId,
    knowledgeLoadedIds,
    knowledgeLoadingIds,
    traceEvents,
    traceFilter,
    isLoadingTrace,
    reactionEntries,
    isLoadingReactions,
    showDebugPanel,
    debugPanelTab,
    // Computed
    canCommit,
    canValidate,
    hasBlockers,
    filteredKnowledgeList,
    selectedKnowledge,
    blockers,
    warnings,
    infos,
    locationTree,
    filteredTraceEvents,
    // Actions
    loadSnapshot,
    selectEntity,
    initDraft,
    updateDraftField,
    validateDraft,
    analyzeImpact,
    commitDraft,
    clearDraft,
    clearValidationResult,
    clearImpactSummary,
    loadKnowledgeDetail,
    loadLocationDetail,
    loadCharacterDetail,
    updateLocationParent,
    loadTraceEvents,
    loadReactionEntries,
    setTraceFilter,
    toggleDebugPanel,
  }
})
