import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { ApiConfig } from '@/types/st'
import * as storage from '@/services/storage'

export const useSettingsStore = defineStore('settings', () => {
  // API Configs
  const apiConfigs = ref<ApiConfig[]>([])
  const activeApiConfigId = ref<string | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Default configs
  const defaultStApiConfigId = ref<string | null>(null)
  const defaultAgentApiConfigId = ref<string | null>(null)

  // Computed
  const activeApiConfig = computed(() => {
    if (!activeApiConfigId.value) return null
    return apiConfigs.value.find(c => c.id === activeApiConfigId.value) || null
  })

  // Actions
  async function loadApiConfigs() {
    loading.value = true
    error.value = null
    try {
      apiConfigs.value = await storage.listApiConfigs()
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function addApiConfig(config: ApiConfig) {
    try {
      await storage.saveApiConfig(config)
      apiConfigs.value.push(config)
    } catch (e) {
      error.value = String(e)
    }
  }

  async function updateApiConfig(id: string, updates: Partial<ApiConfig>) {
    const index = apiConfigs.value.findIndex(c => c.id === id)
    if (index !== -1) {
      const updated = { ...apiConfigs.value[index], ...updates, updated_at: new Date().toISOString() }
      try {
        await storage.saveApiConfig(updated)
        apiConfigs.value[index] = updated
      } catch (e) {
        error.value = String(e)
      }
    }
  }

  async function removeApiConfig(id: string) {
    try {
      await storage.deleteApiConfig(id)
      const index = apiConfigs.value.findIndex(c => c.id === id)
      if (index !== -1) {
        apiConfigs.value.splice(index, 1)
      }
    } catch (e) {
      error.value = String(e)
    }
  }

  function setActiveApiConfig(id: string | null) {
    activeApiConfigId.value = id
  }

  return {
    // State
    apiConfigs,
    activeApiConfigId,
    defaultStApiConfigId,
    defaultAgentApiConfigId,
    loading,
    error,

    // Computed
    activeApiConfig,

    // Actions
    loadApiConfigs,
    addApiConfig,
    updateApiConfig,
    removeApiConfig,
    setActiveApiConfig,
  }
})