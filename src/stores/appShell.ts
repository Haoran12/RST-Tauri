import { defineStore } from 'pinia'
import { ref } from 'vue'

export type AppMode = 'st' | 'agent'

export interface ChatBubbleRoleAppearance {
  color: string
  opacity: number
}

export interface ChatBubbleAppearance {
  user: ChatBubbleRoleAppearance
  assistant: ChatBubbleRoleAppearance
  system: ChatBubbleRoleAppearance
}

export interface MarkdownTextStyle {
  color: string
  fontSize: number
  fontWeight: number
  fontStyle: 'normal' | 'italic'
}

export interface ChatMarkdownAppearance {
  paragraph: MarkdownTextStyle
  heading: MarkdownTextStyle
  italic: MarkdownTextStyle
  bold: MarkdownTextStyle
  quoted: MarkdownTextStyle
}

export const defaultChatBubbleAppearance: ChatBubbleAppearance = {
  user: { color: '#2080f0', opacity: 16 },
  assistant: { color: '#18a058', opacity: 7 },
  system: { color: '#6b7280', opacity: 9 },
}

export const defaultChatMarkdownAppearance: ChatMarkdownAppearance = {
  paragraph: { color: '#1f2937', fontSize: 14, fontWeight: 400, fontStyle: 'normal' },
  heading: { color: '#111827', fontSize: 16, fontWeight: 700, fontStyle: 'normal' },
  italic: { color: '#374151', fontSize: 14, fontWeight: 400, fontStyle: 'italic' },
  bold: { color: '#111827', fontSize: 14, fontWeight: 700, fontStyle: 'normal' },
  quoted: { color: '#7c3aed', fontSize: 14, fontWeight: 500, fontStyle: 'normal' },
}

export const useAppShellStore = defineStore('appShell', () => {
  type RecentSessionItem = { id: string; type: 'st' | 'agent'; name: string; updatedAt: string }
  type RecentResourceItem = { id: string; type: string; name: string; updatedAt: string }

  // Navigation state
  const currentMode = ref<AppMode>(loadAppMode())
  const lastStRoute = ref(loadStoredRoute('rst.lastStRoute', '/st'))
  const lastAgentRoute = ref(loadStoredRoute('rst.lastAgentRoute', '/agent'))
  const navCollapsed = ref(true)
  const contextListWidth = ref(280)
  const inspectPanelWidth = ref(340)
  const inspectPanelOpen = ref(false)

  // Theme
  const theme = ref<'system' | 'light' | 'dark'>(loadTheme())
  const chatBubbleAppearance = ref<ChatBubbleAppearance>(loadChatBubbleAppearance())
  const chatMarkdownAppearance = ref<ChatMarkdownAppearance>(loadChatMarkdownAppearance())

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

  function setCurrentMode(mode: AppMode) {
    currentMode.value = mode
    persistAppMode(mode)
  }

  function rememberModeRoute(mode: AppMode, route: string) {
    if (!route) return
    if (mode === 'st') {
      lastStRoute.value = route
      persistStoredRoute('rst.lastStRoute', route)
      return
    }
    lastAgentRoute.value = route
    persistStoredRoute('rst.lastAgentRoute', route)
  }

  function toggleInspectPanel() {
    inspectPanelOpen.value = !inspectPanelOpen.value
  }

  function setTheme(newTheme: 'system' | 'light' | 'dark') {
    theme.value = newTheme
    persistTheme(newTheme)
  }

  function setChatBubbleAppearance(next: ChatBubbleAppearance) {
    chatBubbleAppearance.value = normalizeChatBubbleAppearance(next)
    persistChatBubbleAppearance(chatBubbleAppearance.value)
  }

  function resetChatBubbleAppearance() {
    chatBubbleAppearance.value = structuredClone(defaultChatBubbleAppearance)
    persistChatBubbleAppearance(chatBubbleAppearance.value)
  }

  function setChatMarkdownAppearance(next: ChatMarkdownAppearance) {
    chatMarkdownAppearance.value = normalizeChatMarkdownAppearance(next)
    persistChatMarkdownAppearance(chatMarkdownAppearance.value)
  }

  function resetChatMarkdownAppearance() {
    chatMarkdownAppearance.value = structuredClone(defaultChatMarkdownAppearance)
    persistChatMarkdownAppearance(chatMarkdownAppearance.value)
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
    currentMode,
    lastStRoute,
    lastAgentRoute,
    navCollapsed,
    contextListWidth,
    inspectPanelWidth,
    inspectPanelOpen,
    theme,
    chatBubbleAppearance,
    chatMarkdownAppearance,
    recentSessions,
    recentResources,
    globalLoading,
    globalMessage,

    // Actions
    setCurrentMode,
    rememberModeRoute,
    toggleNav,
    toggleInspectPanel,
    setTheme,
    setChatBubbleAppearance,
    resetChatBubbleAppearance,
    setChatMarkdownAppearance,
    resetChatMarkdownAppearance,
    setRecentSessions,
    setRecentResources,
    showGlobalMessage,
    clearGlobalMessage,
  }
})

function loadAppMode(): AppMode {
  if (typeof localStorage === 'undefined') {
    return 'st'
  }

  const raw = localStorage.getItem('rst.currentMode')
  return raw === 'agent' ? 'agent' : 'st'
}

function persistAppMode(mode: AppMode) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem('rst.currentMode', mode)
}

function loadTheme(): 'system' | 'light' | 'dark' {
  if (typeof localStorage === 'undefined') {
    return 'system'
  }

  const raw = localStorage.getItem('rst.theme')
  if (raw === 'light' || raw === 'dark' || raw === 'system') {
    return raw
  }
  return 'system'
}

function persistTheme(theme: 'system' | 'light' | 'dark') {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem('rst.theme', theme)
}

function loadStoredRoute(key: string, fallback: string) {
  if (typeof localStorage === 'undefined') {
    return fallback
  }

  const raw = localStorage.getItem(key)
  return raw?.trim() ? raw : fallback
}

function persistStoredRoute(key: string, route: string) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(key, route)
}

function loadChatBubbleAppearance(): ChatBubbleAppearance {
  if (typeof localStorage === 'undefined') {
    return structuredClone(defaultChatBubbleAppearance)
  }

  try {
    const raw = localStorage.getItem('rst.chatBubbleAppearance')
    if (!raw) return structuredClone(defaultChatBubbleAppearance)
    return normalizeChatBubbleAppearance(JSON.parse(raw) as Partial<ChatBubbleAppearance>)
  } catch {
    return structuredClone(defaultChatBubbleAppearance)
  }
}

function loadChatMarkdownAppearance(): ChatMarkdownAppearance {
  if (typeof localStorage === 'undefined') {
    return structuredClone(defaultChatMarkdownAppearance)
  }

  try {
    const raw = localStorage.getItem('rst.chatMarkdownAppearance')
    if (!raw) return structuredClone(defaultChatMarkdownAppearance)
    return normalizeChatMarkdownAppearance(JSON.parse(raw) as Partial<ChatMarkdownAppearance>)
  } catch {
    return structuredClone(defaultChatMarkdownAppearance)
  }
}

function persistChatBubbleAppearance(appearance: ChatBubbleAppearance) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem('rst.chatBubbleAppearance', JSON.stringify(appearance))
}

function persistChatMarkdownAppearance(appearance: ChatMarkdownAppearance) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem('rst.chatMarkdownAppearance', JSON.stringify(appearance))
}

function normalizeChatBubbleAppearance(input: Partial<ChatBubbleAppearance>): ChatBubbleAppearance {
  return {
    user: normalizeRoleAppearance(input.user, defaultChatBubbleAppearance.user),
    assistant: normalizeRoleAppearance(input.assistant, defaultChatBubbleAppearance.assistant),
    system: normalizeRoleAppearance(input.system, defaultChatBubbleAppearance.system),
  }
}

function normalizeChatMarkdownAppearance(input: Partial<ChatMarkdownAppearance>): ChatMarkdownAppearance {
  return {
    paragraph: normalizeMarkdownStyle(input.paragraph, defaultChatMarkdownAppearance.paragraph),
    heading: normalizeMarkdownStyle(input.heading, defaultChatMarkdownAppearance.heading),
    italic: normalizeMarkdownStyle(input.italic, defaultChatMarkdownAppearance.italic),
    bold: normalizeMarkdownStyle(input.bold, defaultChatMarkdownAppearance.bold),
    quoted: normalizeMarkdownStyle(input.quoted, defaultChatMarkdownAppearance.quoted),
  }
}

function normalizeMarkdownStyle(
  input: Partial<MarkdownTextStyle> | undefined,
  fallback: MarkdownTextStyle
): MarkdownTextStyle {
  return {
    color: isHexColor(input?.color) ? input.color : fallback.color,
    fontSize: clampFontSize(input?.fontSize ?? fallback.fontSize),
    fontWeight: clampFontWeight(input?.fontWeight ?? fallback.fontWeight),
    fontStyle: input?.fontStyle === 'italic' ? 'italic' : 'normal',
  }
}

function normalizeRoleAppearance(
  input: Partial<ChatBubbleRoleAppearance> | undefined,
  fallback: ChatBubbleRoleAppearance
): ChatBubbleRoleAppearance {
  return {
    color: isHexColor(input?.color) ? input.color : fallback.color,
    opacity: clampOpacity(input?.opacity ?? fallback.opacity),
  }
}

function isHexColor(value: unknown): value is string {
  return typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value)
}

function clampOpacity(value: number) {
  if (!Number.isFinite(value)) return 0
  return Math.min(100, Math.max(0, Math.round(value)))
}

function clampFontSize(value: number) {
  if (!Number.isFinite(value)) return 14
  return Math.min(28, Math.max(10, Math.round(value)))
}

function clampFontWeight(value: number) {
  if (!Number.isFinite(value)) return 400
  return Math.min(900, Math.max(300, Math.round(value / 100) * 100))
}
