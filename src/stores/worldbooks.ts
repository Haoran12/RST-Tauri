import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WorldInfoFile, WorldInfoEntry } from '@/types/st'
import {
  listWorldbooks,
  getWorldbook,
  createWorldbook,
  saveWorldbook,
  deleteWorldbook,
  updateWorldbookMeta,
  createWorldbookEntry,
  updateWorldbookEntry,
  deleteWorldbookEntry,
  reorderWorldbookEntries,
  importWorldbook,
  exportWorldbook,
  type WorldbookListItem,
} from '@/services/storage'

export const useWorldbooksStore = defineStore('worldbooks', () => {
  // State
  const worldbookList = ref<WorldbookListItem[]>([])
  const currentWorldbook = ref<WorldInfoFile | null>(null)
  const currentEntryUid = ref<number | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // Computed
  const worldbookCount = computed(() => worldbookList.value.length)

  const currentEntry = computed(() => {
    if (!currentWorldbook.value || currentEntryUid.value === null) return null
    return currentWorldbook.value.entries[currentEntryUid.value.toString()] ?? null
  })

  const sortedEntries = computed(() => {
    if (!currentWorldbook.value) return []
    return Object.entries(currentWorldbook.value.entries)
      .map(([uid, entry]) => ({ uid: parseInt(uid), entry }))
      .sort((a, b) => {
        const aOrder = a.entry.order ?? 100
        const bOrder = b.entry.order ?? 100
        if (aOrder !== bOrder) return aOrder - bOrder
        const aDisplay = a.entry.display_index ?? a.uid
        const bDisplay = b.entry.display_index ?? b.uid
        return aDisplay - bDisplay
      })
  })

  // Groups computed from entries
  const groups = computed(() => {
    if (!currentWorldbook.value) return []
    const groupSet = new Set<string>()
    for (const entry of Object.values(currentWorldbook.value.entries)) {
      if (entry.group && entry.group.trim()) {
        groupSet.add(entry.group)
      }
    }
    return Array.from(groupSet).sort()
  })

  // Actions
  async function loadWorldbooks() {
    isLoading.value = true
    error.value = null

    try {
      worldbookList.value = await listWorldbooks()
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function loadWorldbook(id: string) {
    isLoading.value = true
    error.value = null

    try {
      currentWorldbook.value = await getWorldbook(id)
      currentEntryUid.value = null
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function createNewWorldbook(name: string): Promise<string> {
    isLoading.value = true
    error.value = null

    try {
      const id = await createWorldbook(name)
      await loadWorldbooks()
      return id
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function saveCurrentWorldbook() {
    if (!currentWorldbook.value) return

    isLoading.value = true
    error.value = null

    try {
      // Find the ID from the list
      const item = worldbookList.value.find(
        (w) => w.name === currentWorldbook.value!.name
      )
      if (!item) throw new Error('Worldbook not found in list')

      await saveWorldbook(item.id, currentWorldbook.value)
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function deleteWorldbookById(id: string) {
    isLoading.value = true
    error.value = null

    try {
      await deleteWorldbook(id)
      worldbookList.value = await listWorldbooks()
      if (currentWorldbook.value) {
        currentWorldbook.value = null
        currentEntryUid.value = null
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function updateMeta(name: string, description: string) {
    if (!currentWorldbook.value) return

    isLoading.value = true
    error.value = null

    try {
      const item = worldbookList.value.find(
        (w) => w.name === currentWorldbook.value!.name
      )
      if (!item) throw new Error('Worldbook not found in list')

      await updateWorldbookMeta(item.id, name, description)
      currentWorldbook.value.name = name
      currentWorldbook.value.description = description

      // Update list
      const listIndex = worldbookList.value.findIndex((w) => w.id === item.id)
      if (listIndex >= 0) {
        worldbookList.value[listIndex] = {
          ...worldbookList.value[listIndex],
          name,
          description,
        }
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function createNewEntry(): Promise<number> {
    if (!currentWorldbook.value) throw new Error('No worldbook loaded')

    isLoading.value = true
    error.value = null

    try {
      const item = worldbookList.value.find(
        (w) => w.name === currentWorldbook.value!.name
      )
      if (!item) throw new Error('Worldbook not found in list')

      const uid = await createWorldbookEntry(item.id)
      // Reload to get the new entry with defaults
      currentWorldbook.value = await getWorldbook(item.id)
      currentEntryUid.value = uid
      return uid
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function updateEntry(uid: number, entry: WorldInfoEntry) {
    if (!currentWorldbook.value) return

    isLoading.value = true
    error.value = null

    try {
      const item = worldbookList.value.find(
        (w) => w.name === currentWorldbook.value!.name
      )
      if (!item) throw new Error('Worldbook not found in list')

      await updateWorldbookEntry(item.id, uid, entry)
      currentWorldbook.value.entries[uid.toString()] = entry

      // Update entry count in list
      const listIndex = worldbookList.value.findIndex((w) => w.id === item.id)
      if (listIndex >= 0) {
        worldbookList.value[listIndex].entry_count = Object.keys(
          currentWorldbook.value.entries
        ).length
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function deleteEntry(uid: number) {
    if (!currentWorldbook.value) return

    isLoading.value = true
    error.value = null

    try {
      const item = worldbookList.value.find(
        (w) => w.name === currentWorldbook.value!.name
      )
      if (!item) throw new Error('Worldbook not found in list')

      await deleteWorldbookEntry(item.id, uid)
      delete currentWorldbook.value.entries[uid.toString()]

      if (currentEntryUid.value === uid) {
        currentEntryUid.value = null
      }

      // Update entry count in list
      const listIndex = worldbookList.value.findIndex((w) => w.id === item.id)
      if (listIndex >= 0) {
        worldbookList.value[listIndex].entry_count = Object.keys(
          currentWorldbook.value.entries
        ).length
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function reorderEntries(uidOrder: number[]) {
    if (!currentWorldbook.value) return

    isLoading.value = true
    error.value = null

    try {
      const item = worldbookList.value.find(
        (w) => w.name === currentWorldbook.value!.name
      )
      if (!item) throw new Error('Worldbook not found in list')

      await reorderWorldbookEntries(item.id, uidOrder)
      // Reload to get updated display_index values
      currentWorldbook.value = await getWorldbook(item.id)
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function importFromFile(file: File): Promise<string> {
    isLoading.value = true
    error.value = null

    try {
      const arrayBuffer = await file.arrayBuffer()
      const jsonData = Array.from(new Uint8Array(arrayBuffer))

      const id = await importWorldbook(jsonData, file.name)
      await loadWorldbooks()
      return id
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function exportToFile(id: string): Promise<Blob> {
    isLoading.value = true
    error.value = null

    try {
      const jsonData = await exportWorldbook(id)
      return new Blob([new Uint8Array(jsonData)], { type: 'application/json' })
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  function selectEntry(uid: number | null) {
    currentEntryUid.value = uid
  }

  function clearCurrentWorldbook() {
    currentWorldbook.value = null
    currentEntryUid.value = null
  }

  function clearError() {
    error.value = null
  }

  return {
    // State
    worldbookList,
    currentWorldbook,
    currentEntryUid,
    isLoading,
    error,

    // Computed
    worldbookCount,
    currentEntry,
    sortedEntries,
    groups,

    // Actions
    loadWorldbooks,
    loadWorldbook,
    createNewWorldbook,
    saveCurrentWorldbook,
    deleteWorldbookById,
    updateMeta,
    createNewEntry,
    updateEntry,
    deleteEntry,
    reorderEntries,
    importFromFile,
    exportToFile,
    selectEntry,
    clearCurrentWorldbook,
    clearError,
  }
})