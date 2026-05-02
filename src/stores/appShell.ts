import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useAppShellStore = defineStore('appShell', () => {
  // Navigation state
  const navCollapsed = ref(false)
  const contextListWidth = ref(280)
  const inspectPanelWidth = ref(340)
  const inspectPanelOpen = ref(false)

  // Theme
  const theme = ref<'system' | 'light' | 'dark'>('system')

  // Recent items
  const recentSessions = ref<Array<{ id: string; type: 'st' | 'agent'; name: string; updatedAt: string }>>([])
  const recentResources = ref<Array<{ id: string; type: string; name: string; updatedAt: string }>>([])

  // Global UI state
  const globalLoading = ref(false)
  const globalMessage = ref<{ type: 'info' | 'success' | 'warning' | 'error'; text: string } | null>(null)

  // Actions
  function toggleNav() {
    navCollapsed.value = !navCollapsed.value
  }

  function toggleInspectPanel() {
    inspectPanelOpen.value = !inspectPanelOpen.value
  }

  function setTheme(newTheme: 'system' | 'light' | 'dark') {
    theme.value = newTheme
  }

  function showGlobalMessage(type: 'info' | 'success' | 'warning' | 'error', text: string) {
    globalMessage.value = { type, text }
  }

  function clearGlobalMessage() {
    globalMessage.value = null
  }

  return {
    // State
    navCollapsed,
    contextListWidth,
    inspectPanelWidth,
    inspectPanelOpen,
    theme,
    recentSessions,
    recentResources,
    globalLoading,
    globalMessage,

    // Actions
    toggleNav,
    toggleInspectPanel,
    setTheme,
    showGlobalMessage,
    clearGlobalMessage,
  }
})
