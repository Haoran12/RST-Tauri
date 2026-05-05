import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  ChatSession,
  ChatMessage,
  CharacterCard,
  ApiConfig,
  ChatAttachmentRef,
  ChatSessionMetadata,
} from '@/types/st'
import * as storage from '@/services/storage'
import { sendAssembledSTChatMessage } from '@/services/runtime'
import { useRuntimeStore } from '@/stores/runtime'

const MAX_CHAT_ATTACHMENT_BYTES = 10 * 1024 * 1024

export const useChatStore = defineStore('chat', () => {
  // Current session
  const currentSession = ref<ChatSession | null>(null)
  const sessions = ref<ChatSession[]>([])

  // Current character
  const currentCharacter = ref<CharacterCard | null>(null)

  // Messages
  const messages = ref<ChatMessage[]>([])
  const pendingMessage = ref<string>('')
  const pendingAttachments = ref<ChatAttachmentRef[]>([])
  const isGenerating = ref(false)
  const streamingContent = ref<string>('')

  // Error state
  const error = ref<string | null>(null)

  // Computed
  const hasSession = computed(() => currentSession.value !== null)
  const hasCharacter = computed(() => currentCharacter.value !== null)
  const hasPendingAttachments = computed(() => pendingAttachments.value.length > 0)

  function normalizeChatMetadata(metadata?: ChatSessionMetadata): ChatSessionMetadata {
    return {
      world_info: metadata?.world_info ?? null,
      disabled_world_info: metadata?.disabled_world_info ?? [],
      ...metadata,
    }
  }

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
      chat_metadata: normalizeChatMetadata(),
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
      session.chat_metadata = normalizeChatMetadata(session.chat_metadata)
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

  async function addPendingAttachments(files: File[]) {
    const uploaded: ChatAttachmentRef[] = []
    for (const file of files) {
      if (file.size > MAX_CHAT_ATTACHMENT_BYTES) {
        throw new Error(`附件不能超过 10 MB: ${file.name}`)
      }
      const mimeType = file.type || inferMimeType(file.name)
      if (!isSupportedAttachment(mimeType, file.name)) {
        throw new Error(`不支持的附件类型: ${file.name}`)
      }

      const buffer = await file.arrayBuffer()
      const bytes = Array.from(new Uint8Array(buffer))
      const record = await storage.saveChatAttachment(file.name, mimeType, bytes)
      uploaded.push({
        attachment_id: record.attachment_id,
        kind: record.kind,
        mime_type: record.mime_type,
        filename: record.filename,
        size_bytes: record.size_bytes,
      })
    }
    pendingAttachments.value.push(...uploaded)
  }

  function removePendingAttachment(attachmentId: string) {
    pendingAttachments.value = pendingAttachments.value.filter(
      attachment => attachment.attachment_id !== attachmentId
    )
  }

  function clearPendingAttachments() {
    pendingAttachments.value = []
  }

  // Send message (non-streaming)
  async function sendMessage(content: string, apiConfig: ApiConfig) {
    if (!currentSession.value || isGenerating.value) return

    isGenerating.value = true
    error.value = null
    const messageContent = content.trim()
    const attachments = [...pendingAttachments.value]
    if (!messageContent && attachments.length === 0) {
      isGenerating.value = false
      return
    }

    // Add user message
    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content: messageContent,
      created_at: new Date().toISOString(),
      attachments,
    }
    messages.value.push(userMessage)
    pendingAttachments.value = []

    try {
      await saveCurrentSession()
      const runtimeStore = useRuntimeStore()
      await runtimeStore.loadGlobalState()

      const response = await sendAssembledSTChatMessage({
        api_config_id: apiConfig.id,
        character_id: currentSession.value.character_id ?? null,
        session_id: currentSession.value.id,
        preset_name: runtimeStore.globalState.active_preset || 'Default',
        world_info_settings: runtimeStore.globalState.world_info_settings,
        chat_lore_id: null,
        global_lore_ids: [],
        max_context: 8192,
      })

      // Add assistant message
      const assistantMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: response.content,
        created_at: new Date().toISOString(),
        attachments: [],
      }
      messages.value.push(assistantMessage)

      // Save session
      await saveCurrentSession()
    } catch (e) {
      error.value = String(e)
      pendingAttachments.value = attachments
      // Remove the user message on error
      messages.value.pop()
      await saveCurrentSession()
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
    const messageContent = content.trim()
    const attachments = [...pendingAttachments.value]
    if (!messageContent && attachments.length === 0) {
      isGenerating.value = false
      return
    }

    // Add user message
    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content: messageContent,
      created_at: new Date().toISOString(),
      attachments,
    }
    messages.value.push(userMessage)
    pendingAttachments.value = []

    try {
      await saveCurrentSession()
      const runtimeStore = useRuntimeStore()
      await runtimeStore.loadGlobalState()

      // Add placeholder only after persisting the user message used by runtime assembly.
      const assistantMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: '',
        created_at: new Date().toISOString(),
        attachments: [],
      }
      messages.value.push(assistantMessage)

      const response = await sendAssembledSTChatMessage({
        api_config_id: apiConfig.id,
        character_id: currentSession.value.character_id ?? null,
        session_id: currentSession.value.id,
        preset_name: runtimeStore.globalState.active_preset || 'Default',
        world_info_settings: runtimeStore.globalState.world_info_settings,
        chat_lore_id: null,
        global_lore_ids: [],
        max_context: 8192,
      })

      streamingContent.value = response.content
      assistantMessage.content = response.content
      await saveCurrentSession()
    } catch (e) {
      error.value = String(e)
      pendingAttachments.value = attachments
      // Remove both user and placeholder assistant messages on error
      if (messages.value[messages.value.length - 1]?.role === 'assistant') {
        messages.value.pop()
      }
      if (messages.value[messages.value.length - 1]?.id === userMessage.id) {
        messages.value.pop()
      }
      await saveCurrentSession()
    } finally {
      isGenerating.value = false
      streamingContent.value = ''
    }
  }

  // Stop only affects the local pending indicator until provider streaming is implemented.
  function stopGeneration() {
    isGenerating.value = false
    streamingContent.value = ''
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
    currentSession.value.chat_metadata = normalizeChatMetadata(currentSession.value.chat_metadata)
    currentSession.value.updated_at = new Date().toISOString()

    try {
      await storage.saveChatSession(currentSession.value)
    } catch (e) {
      error.value = String(e)
    }
  }

  async function setWorldbookDisabled(loreId: string, disabled: boolean) {
    if (!currentSession.value) return

    const metadata = normalizeChatMetadata(currentSession.value.chat_metadata)
    const disabledSet = new Set(metadata.disabled_world_info ?? [])
    if (disabled) {
      disabledSet.add(loreId)
    } else {
      disabledSet.delete(loreId)
    }
    currentSession.value.chat_metadata = {
      ...metadata,
      disabled_world_info: Array.from(disabledSet),
    }
    await saveCurrentSession()
  }

  return {
    // State
    currentSession,
    sessions,
    currentCharacter,
    messages,
    pendingMessage,
    pendingAttachments,
    isGenerating,
    streamingContent,
    error,

    // Computed
    hasSession,
    hasCharacter,
    hasPendingAttachments,

    // Actions
    loadSessions,
    createSession,
    loadSession,
    setCharacter,
    addPendingAttachments,
    removePendingAttachment,
    clearPendingAttachments,
    sendMessage,
    sendMessageStream,
    stopGeneration,
    clearMessages,
    deleteSession,
    saveCurrentSession,
    setWorldbookDisabled,
  }
})

function isSupportedAttachment(mimeType: string, filename: string) {
  return mimeType.startsWith('image/') || mimeType === 'application/pdf' || filename.toLowerCase().endsWith('.pdf')
}

function inferMimeType(filename: string) {
  if (filename.toLowerCase().endsWith('.pdf')) {
    return 'application/pdf'
  }
  return 'application/octet-stream'
}
