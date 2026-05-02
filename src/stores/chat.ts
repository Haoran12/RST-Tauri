import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { ChatSession, ChatMessage, CharacterCard, ApiConfig } from '@/types/st'
import type { ChatRequestMessage } from '@/services/api'
import * as storage from '@/services/storage'
import { sendChatMessage, streamChatMessage } from '@/services/api'

export const useChatStore = defineStore('chat', () => {
  // Current session
  const currentSession = ref<ChatSession | null>(null)
  const sessions = ref<ChatSession[]>([])

  // Current character
  const currentCharacter = ref<CharacterCard | null>(null)

  // Messages
  const messages = ref<ChatMessage[]>([])
  const pendingMessage = ref<string>('')
  const isGenerating = ref(false)
  const streamingContent = ref<string>('')

  // Error state
  const error = ref<string | null>(null)

  // Computed
  const hasSession = computed(() => currentSession.value !== null)
  const hasCharacter = computed(() => currentCharacter.value !== null)

  // Load sessions
  async function loadSessions() {
    try {
      sessions.value = await storage.listChatSessions()
    } catch (e) {
      error.value = String(e)
    }
  }

  // Create new session
  async function createSession(name: string, characterId?: string) {
    const session: ChatSession = {
      id: crypto.randomUUID(),
      name,
      character_id: characterId,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      messages: [],
    }

    try {
      await storage.saveChatSession(session)
      sessions.value.push(session)
      currentSession.value = session
      messages.value = []
    } catch (e) {
      error.value = String(e)
    }
  }

  // Load session
  async function loadSession(id: string) {
    try {
      const session = await storage.getChatSession(id)
      currentSession.value = session
      messages.value = session.messages

      // Load character if associated
      if (session.character_id) {
        const character = await storage.getCharacter(session.character_id)
        currentCharacter.value = character
      }
    } catch (e) {
      error.value = String(e)
    }
  }

  // Set character
  async function setCharacter(character: CharacterCard | null) {
    currentCharacter.value = character
  }

  // Send message (non-streaming)
  async function sendMessage(content: string, apiConfig: ApiConfig) {
    if (!currentSession.value || isGenerating.value) return

    isGenerating.value = true
    error.value = null

    // Add user message
    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      created_at: new Date().toISOString(),
    }
    messages.value.push(userMessage)

    try {
      // Build request
      const systemPrompt = buildSystemPrompt()
      const requestMessages = buildRequestMessages()

      const response = await sendChatMessage(apiConfig, systemPrompt, requestMessages)

      // Add assistant message
      const assistantMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: response.content,
        created_at: new Date().toISOString(),
      }
      messages.value.push(assistantMessage)

      // Save session
      await saveCurrentSession()
    } catch (e) {
      error.value = String(e)
      // Remove the user message on error
      messages.value.pop()
    } finally {
      isGenerating.value = false
    }
  }

  // Send message (streaming)
  async function sendMessageStream(content: string, apiConfig: ApiConfig) {
    if (!currentSession.value || isGenerating.value) return

    isGenerating.value = true
    streamingContent.value = ''
    error.value = null

    // Add user message
    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      created_at: new Date().toISOString(),
    }
    messages.value.push(userMessage)

    // Add placeholder assistant message for streaming
    const assistantMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'assistant',
      content: '',
      created_at: new Date().toISOString(),
    }
    messages.value.push(assistantMessage)

    try {
      const systemPrompt = buildSystemPrompt()
      const requestMessages = buildRequestMessages()

      await streamChatMessage(
        apiConfig,
        systemPrompt,
        requestMessages,
        (chunk: string) => {
          streamingContent.value += chunk
          // Update the assistant message content
          const lastMsg = messages.value[messages.value.length - 1]
          if (lastMsg && lastMsg.role === 'assistant') {
            lastMsg.content = streamingContent.value
          }
        },
        () => {
          // Stream complete
          isGenerating.value = false
          streamingContent.value = ''
          saveCurrentSession()
        }
      )
    } catch (e) {
      error.value = String(e)
      // Remove both user and placeholder assistant messages on error
      messages.value.pop()
      messages.value.pop()
      isGenerating.value = false
      streamingContent.value = ''
    }
  }

  // Stop generation
  function stopGeneration() {
    isGenerating.value = false
    streamingContent.value = ''
    // TODO: Implement actual abort
  }

  // Clear messages
  async function clearMessages() {
    if (!currentSession.value) return

    messages.value = []
    currentSession.value.messages = []
    await saveCurrentSession()
  }

  // Delete session
  async function deleteSession(id: string) {
    try {
      await storage.deleteChatSession(id)
      const index = sessions.value.findIndex(s => s.id === id)
      if (index !== -1) {
        sessions.value.splice(index, 1)
      }
      if (currentSession.value?.id === id) {
        currentSession.value = null
        messages.value = []
        currentCharacter.value = null
      }
    } catch (e) {
      error.value = String(e)
    }
  }

  // Save current session
  async function saveCurrentSession() {
    if (!currentSession.value) return

    currentSession.value.messages = messages.value
    currentSession.value.updated_at = new Date().toISOString()

    try {
      await storage.saveChatSession(currentSession.value)
    } catch (e) {
      error.value = String(e)
    }
  }

  // Build system prompt from character
  function buildSystemPrompt(): string {
    if (!currentCharacter.value) return ''

    const data = currentCharacter.value.data
    const parts: string[] = []

    if (data.system_prompt) {
      parts.push(data.system_prompt)
    }

    if (data.description) {
      parts.push(`Description: ${data.description}`)
    }

    if (data.personality) {
      parts.push(`Personality: ${data.personality}`)
    }

    if (data.scenario) {
      parts.push(`Scenario: ${data.scenario}`)
    }

    return parts.join('\n\n')
  }

  // Build request messages from chat history
  function buildRequestMessages(): ChatRequestMessage[] {
    // Exclude the last assistant message if streaming
    const msgs = isGenerating.value
      ? messages.value.slice(0, -1)
      : messages.value

    return msgs.map(m => ({
      role: m.role as 'system' | 'user' | 'assistant',
      content: m.content,
    }))
  }

  return {
    // State
    currentSession,
    sessions,
    currentCharacter,
    messages,
    pendingMessage,
    isGenerating,
    streamingContent,
    error,

    // Computed
    hasSession,
    hasCharacter,

    // Actions
    loadSessions,
    createSession,
    loadSession,
    setCharacter,
    sendMessage,
    sendMessageStream,
    stopGeneration,
    clearMessages,
    deleteSession,
    saveCurrentSession,
  }
})