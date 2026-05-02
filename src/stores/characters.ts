import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { TavernCardV3 } from '@/types/st'
import {
  listCharacters,
  getCharacter,
  saveCharacter,
  deleteCharacter,
  importCharacterFromPng,
  importCharacterFromJson,
  exportCharacterAsPng,
  exportCharacterAsJson,
  importEmbeddedWorldbook,
  updateCharacterAvatar,
  getCharacterAvatar,
  type CharacterImportResult,
} from '@/services/storage'

export const useCharactersStore = defineStore('characters', () => {
  // State
  const characters = ref<TavernCardV3[]>([])
  const currentCharacter = ref<TavernCardV3 | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // Computed
  const characterCount = computed(() => characters.value.length)

  const charactersWithEmbeddedWorldbook = computed(() =>
    characters.value.filter((c) => c.data.character_book != null)
  )

  // Actions
  async function loadCharacters() {
    isLoading.value = true
    error.value = null

    try {
      characters.value = await listCharacters()
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function loadCharacter(id: string) {
    isLoading.value = true
    error.value = null

    try {
      currentCharacter.value = await getCharacter(id)
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function saveCurrentCharacter(id: string) {
    if (!currentCharacter.value) return

    isLoading.value = true
    error.value = null

    try {
      await saveCharacter(id, currentCharacter.value)
      // Update list
      const index = characters.value.findIndex(
        (c) => c.data.name === currentCharacter.value!.data.name
      )
      if (index >= 0) {
        characters.value[index] = currentCharacter.value
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function deleteCharacterById(id: string) {
    isLoading.value = true
    error.value = null

    try {
      await deleteCharacter(id)
      // Reload characters list after deletion
      characters.value = await listCharacters()
      if (currentCharacter.value) {
        currentCharacter.value = null
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function importFromPng(
    file: File
  ): Promise<CharacterImportResult> {
    isLoading.value = true
    error.value = null

    try {
      const arrayBuffer = await file.arrayBuffer()
      const pngData = Array.from(new Uint8Array(arrayBuffer))

      const result = await importCharacterFromPng(pngData, file.name)

      // Add to list
      characters.value.push(result.character)

      return result
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function importFromJson(
    file: File,
    avatarFile?: File
  ): Promise<CharacterImportResult> {
    isLoading.value = true
    error.value = null

    try {
      const arrayBuffer = await file.arrayBuffer()
      const jsonData = Array.from(new Uint8Array(arrayBuffer))

      let avatarPng: number[] | null = null
      if (avatarFile) {
        const avatarBuffer = await avatarFile.arrayBuffer()
        avatarPng = Array.from(new Uint8Array(avatarBuffer))
      }

      const result = await importCharacterFromJson(
        jsonData,
        avatarPng,
        file.name
      )

      // Add to list
      characters.value.push(result.character)

      return result
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function exportToPng(id: string): Promise<Blob> {
    isLoading.value = true
    error.value = null

    try {
      const pngData = await exportCharacterAsPng(id)
      return new Blob([new Uint8Array(pngData)], { type: 'image/png' })
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function exportToJson(id: string): Promise<Blob> {
    isLoading.value = true
    error.value = null

    try {
      const jsonData = await exportCharacterAsJson(id)
      return new Blob([new Uint8Array(jsonData)], { type: 'application/json' })
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function importWorldbook(characterId: string): Promise<string> {
    isLoading.value = true
    error.value = null

    try {
      const loreId = await importEmbeddedWorldbook(characterId)

      // Update character in list to reflect worldbook binding
      const character = await getCharacter(characterId)
      const index = characters.value.findIndex(
        (c) => c.data.name === character.data.name
      )
      if (index >= 0) {
        characters.value[index] = character
      }
      if (currentCharacter.value?.data.name === character.data.name) {
        currentCharacter.value = character
      }

      return loreId
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function updateAvatar(id: string, file: File): Promise<void> {
    isLoading.value = true
    error.value = null

    try {
      const arrayBuffer = await file.arrayBuffer()
      const pngData = Array.from(new Uint8Array(arrayBuffer))

      await updateCharacterAvatar(id, pngData)
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function getAvatarUrl(id: string): Promise<string | null> {
    try {
      const pngData = await getCharacterAvatar(id)
      const blob = new Blob([new Uint8Array(pngData)], { type: 'image/png' })
      return URL.createObjectURL(blob)
    } catch {
      return null
    }
  }

  function clearCurrentCharacter() {
    currentCharacter.value = null
  }

  function clearError() {
    error.value = null
  }

  return {
    // State
    characters,
    currentCharacter,
    isLoading,
    error,

    // Computed
    characterCount,
    charactersWithEmbeddedWorldbook,

    // Actions
    loadCharacters,
    loadCharacter,
    saveCurrentCharacter,
    deleteCharacterById,
    importFromPng,
    importFromJson,
    exportToPng,
    exportToJson,
    importWorldbook,
    updateAvatar,
    getAvatarUrl,
    clearCurrentCharacter,
    clearError,
  }
})