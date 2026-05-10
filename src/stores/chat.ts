import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  ChatSession,
  ChatMessage,
  CharacterCard,
  ApiConfig,
  ChatAttachmentRef,
  ChatSessionMetadata,
  STUserPersona,
} from '@/types/st'
import * as storage from '@/services/storage'
import { sendAssembledSTChatMessage, startSTChatStream, type StreamController } from '@/services/runtime'
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

  // Stream controller for abort
  let currentStreamController: StreamController | null = null

  // Error state
  const error = ref<string | null>(null)

  // Computed
  const hasSession = computed(() => currentSession.value !== null)
  const hasCharacter = computed(() => currentCharacter.value !== null)
  const hasPendingAttachments = computed(() => pendingAttachments.value.length > 0)

  function normalizeChatMetadata(metadata?: ChatSessionMetadata): ChatSessionMetadata {
    const enabledWorldInfo = metadata?.enabled_world_info ?? (
      metadata?.world_info ? [metadata.world_info] : []
    )
    const userPersona = normalizeUserPersona(metadata?.user_persona)
    return {
      ...metadata,
      world_info: enabledWorldInfo[0] ?? metadata?.world_info ?? null,
      enabled_world_info: enabledWorldInfo,
      disabled_world_info: metadata?.disabled_world_info ?? [],
      user_persona: userPersona,
    }
  }

  function normalizeUserPersona(persona?: STUserPersona): STUserPersona {
    return {
      name: persona?.name ?? '',
      description: persona?.description ?? '',
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
  async function createSession(
    name: string,
    characterId?: string,
    userPersona?: STUserPersona
  ) {
    const session: ChatSession = {
      id: crypto.randomUUID(),
      name,
      character_id: characterId,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      chat_metadata: normalizeChatMetadata({
        user_persona: userPersona,
      }),
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

  // Regenerate last AI response
  // - If last message is assistant: delete it, then generate new response
  // - If last message is user: keep it, generate new response based on user message
  async function regenerateLastResponse(apiConfig: ApiConfig) {
    if (!currentSession.value || isGenerating.value) return

    const lastMessage = messages.value[messages.value.length - 1]
    if (!lastMessage) return

    // If last message is assistant, delete it before regenerating
    if (lastMessage.role === 'assistant') {
      messages.value.pop()
      await saveCurrentSession()
    }

    // Generate new response (no new user message, use existing context)
    // Use sendMessageStream with empty content - it will trigger 'normal' mode
    // since last message is now user (or was user originally)
    await sendMessageStream('', apiConfig)
  }

  // Determine generation type based on input and last message
  // - 'continue': empty input + has messages + last message is assistant (append to last message)
  // - 'normal': other cases (generate new response)
  function determineGenerateType(content: string): 'normal' | 'continue' {
    const trimmedContent = content.trim()
    const hasAttachments = pendingAttachments.value.length > 0
    const lastMessage = messages.value[messages.value.length - 1]

    // Continue mode: empty input, no attachments, has messages, last message is assistant
    if (
      !trimmedContent &&
      !hasAttachments &&
      messages.value.length > 0 &&
      lastMessage?.role === 'assistant'
    ) {
      return 'continue'
    }
    return 'normal'
  }

  // Send message (non-streaming)
  async function sendMessage(content: string, apiConfig: ApiConfig) {
    if (!currentSession.value || isGenerating.value) return

    isGenerating.value = true
    error.value = null
    const messageContent = content.trim()
    const attachments = [...pendingAttachments.value]
    const generateType = determineGenerateType(content)

    // Add user message only if there's content or attachments (not in continue mode)
    if (generateType === 'normal' && (messageContent || attachments.length > 0)) {
      const userMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'user',
        content: messageContent,
        created_at: new Date().toISOString(),
        attachments,
      }
      messages.value.push(userMessage)
      pendingAttachments.value = []
    }

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

      // Handle response based on generate type
      if (response.content?.trim()) {
        if (generateType === 'continue' && messages.value.length > 0) {
          // Append to last assistant message
          const lastMessage = messages.value[messages.value.length - 1]
          if (lastMessage.role === 'assistant') {
            lastMessage.content += response.content
          }
        } else {
          // Add new assistant message
          const assistantMessage: ChatMessage = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: response.content,
            created_at: new Date().toISOString(),
            attachments: [],
          }
          messages.value.push(assistantMessage)
        }
      }

      // Save session
      await saveCurrentSession()
    } catch (e) {
      error.value = String(e)
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
    const generateType = determineGenerateType(content)

    // Add user message only if there's content or attachments (not in continue mode)
    if (generateType === 'normal' && (messageContent || attachments.length > 0)) {
      const userMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'user',
        content: messageContent,
        created_at: new Date().toISOString(),
        attachments,
      }
      messages.value.push(userMessage)
      pendingAttachments.value = []
    }

    // For continue mode, get the last assistant message
    const lastAssistantMessage = generateType === 'continue' && messages.value.length > 0
      ? messages.value[messages.value.length - 1]
      : null
    const isContinueMode = generateType === 'continue' && lastAssistantMessage?.role === 'assistant'

    // Pre-create assistant message placeholder for efficient updates
    const assistantId = isContinueMode ? lastAssistantMessage!.id : crypto.randomUUID()
    const originalContent = isContinueMode ? lastAssistantMessage!.content : ''
    let assistantAdded = !isContinueMode // In continue mode, message already exists

    try {
      await saveCurrentSession()
      const runtimeStore = useRuntimeStore()
      await runtimeStore.loadGlobalState()

      let accumulatedContent = isContinueMode ? originalContent : ''
      let resolveStream: () => void = () => {}
      let rejectStream: (error: Error) => void = () => {}
      const streamDone = new Promise<void>((resolve, reject) => {
        resolveStream = resolve
        rejectStream = reject
      })

      // Start streaming
      currentStreamController = await startSTChatStream(
        {
          api_config_id: apiConfig.id,
          character_id: currentSession.value.character_id ?? null,
          session_id: currentSession.value.id,
          preset_name: runtimeStore.globalState.active_preset || 'Default',
          world_info_settings: runtimeStore.globalState.world_info_settings,
          chat_lore_id: null,
          global_lore_ids: [],
          max_context: 8192,
        },
        {
          onStart: () => {
            // Stream started, wait for first chunk
          },
          onChunk: (event) => {
            accumulatedContent += event.delta
            streamingContent.value = accumulatedContent
            if (!isContinueMode) {
              // Normal mode: add new assistant message on first non-empty content
              if (!assistantAdded && accumulatedContent.trim()) {
                messages.value.push({
                  id: assistantId,
                  role: 'assistant',
                  content: accumulatedContent,
                  created_at: new Date().toISOString(),
                  attachments: [],
                })
                assistantAdded = true
              } else if (assistantAdded) {
                // Direct update by index - faster than findIndex + splice
                const lastIndex = messages.value.length - 1
                if (lastIndex >= 0 && messages.value[lastIndex].id === assistantId) {
                  messages.value[lastIndex] = {
                    ...messages.value[lastIndex],
                    content: accumulatedContent,
                  }
                }
              }
            } else {
              // Continue mode: update last assistant message directly
              const lastIndex = messages.value.length - 1
              if (lastIndex >= 0 && messages.value[lastIndex].id === assistantId) {
                messages.value[lastIndex] = {
                  ...messages.value[lastIndex],
                  content: accumulatedContent,
                }
              }
            }
          },
          onError: (event) => {
            error.value = event.error
            // In continue mode, restore original content on error
            if (isContinueMode && lastAssistantMessage) {
              const lastIndex = messages.value.length - 1
              if (lastIndex >= 0 && messages.value[lastIndex].id === assistantId) {
                messages.value[lastIndex] = {
                  ...messages.value[lastIndex],
                  content: originalContent,
                }
              }
            } else if (assistantAdded) {
              // Normal mode: remove the assistant message on error
              const lastIndex = messages.value.length - 1
              if (lastIndex >= 0 && messages.value[lastIndex].id === assistantId) {
                messages.value.pop()
              }
            }
            rejectStream(new Error(event.error))
          },
          onEnd: async () => {
            currentStreamController = null
            resolveStream()
          },
        }
      )

      await streamDone
      await saveCurrentSession()
    } catch (e) {
      error.value = String(e)
      // In continue mode, restore original content on error
      if (isContinueMode && lastAssistantMessage) {
        const lastIndex = messages.value.length - 1
        if (lastIndex >= 0 && messages.value[lastIndex].id === assistantId) {
          messages.value[lastIndex] = {
            ...messages.value[lastIndex],
            content: originalContent,
          }
        }
      } else if (assistantAdded) {
        // Normal mode: remove assistant message if added
        const lastIndex = messages.value.length - 1
        if (lastIndex >= 0 && messages.value[lastIndex].id === assistantId) {
          messages.value.pop()
        }
      }
      currentStreamController = null
      await saveCurrentSession()
    } finally {
      isGenerating.value = false
      streamingContent.value = ''
    }
  }

  // Stop generation
  function stopGeneration() {
    if (currentStreamController) {
      currentStreamController.abort()
      currentStreamController = null
    }
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

  async function updateMessageContent(messageId: string, content: string) {
    if (!currentSession.value) return

    const target = messages.value.find(msg => msg.id === messageId)
    if (!target) return

    target.content = content
    await saveCurrentSession()
  }

  async function deleteMessage(messageId: string) {
    if (!currentSession.value) return

    messages.value = messages.value.filter(msg => msg.id !== messageId)
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

  async function updateSessionSettings(
    id: string,
    settings: {
      name: string
      character_id: string | null
      enabled_world_info: string[]
      user_persona: STUserPersona
    }
  ) {
    try {
      const session = currentSession.value?.id === id
        ? currentSession.value
        : await storage.getChatSession(id)
      const metadata = normalizeChatMetadata(session.chat_metadata)
      const enabledWorldInfo = Array.from(new Set(settings.enabled_world_info))

      session.name = settings.name.trim() || '未命名会话'
      session.character_id = settings.character_id ?? undefined
      session.chat_metadata = {
        ...metadata,
        world_info: enabledWorldInfo[0] ?? null,
        enabled_world_info: enabledWorldInfo,
        disabled_world_info: (metadata.disabled_world_info ?? []).filter(
          loreId => !enabledWorldInfo.includes(loreId)
        ),
        user_persona: normalizeUserPersona(settings.user_persona),
      }
      session.updated_at = new Date().toISOString()

      await storage.saveChatSession(session)

      const index = sessions.value.findIndex(s => s.id === id)
      if (index !== -1) {
        sessions.value[index] = { ...session, messages: sessions.value[index].messages }
      }
      if (currentSession.value?.id === id) {
        currentSession.value = session
        currentCharacter.value = session.character_id
          ? await storage.getCharacter(session.character_id)
          : null
      }
    } catch (e) {
      error.value = String(e)
      throw e
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
    regenerateLastResponse,
    stopGeneration,
    clearMessages,
    updateMessageContent,
    deleteMessage,
    deleteSession,
    saveCurrentSession,
    updateSessionSettings,
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
