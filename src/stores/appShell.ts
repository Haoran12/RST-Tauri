import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useAppShellStore = defineStore('appShell', () => {
  type RecentSessionItem = { id: string; type: 'st' | 'agent'; name: string; updatedAt: string }
  type RecentResourceItem = { id: string; type: string; name: string; updatedAt: string }

  // Navigation state
  const navCollapsed = ref(false)
  const contextListWidth = ref(280)
  const inspectPanelWidth = ref(340)
  const inspectPanelOpen = ref(false)

  // Theme
  const theme = ref<'system' | 'light' | 'dark'>('system')

  // Recent items
  const recentSessions = ref<RecentSessionItem[]>([])
  const recentResources = ref<RecentResourceItem[]>([])

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

  function setRecentSessions(items: RecentSessionItem[]) {
    recentSessions.value = items
  }

  function setRecentResources(items: RecentResourceItem[]) {
    recentResources.value = items
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
    setRecentSessions,
    setRecentResources,
    showGlobalMessage,
    clearGlobalMessage,
  }
})
