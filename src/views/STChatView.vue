<script setup lang="ts">
import { ref, computed, onMounted, nextTick, onBeforeUnmount, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NButton,
  NEmpty,
  NIcon,
  NInput,
  NModal,
  NScrollbar,
  NSpin,
  NText,
  useDialog,
  useMessage,
} from 'naive-ui'
import {
  AttachOutline,
  ChevronBackOutline,
  SendOutline,
  StopOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import ChatMessageItem from '@/components/shared/ChatMessageItem.vue'
import { useChatStore } from '@/stores/chat'
import { useSettingsStore } from '@/stores/settings'
import { useRuntimeStore } from '@/stores/runtime'
import { useWorldbooksStore } from '@/stores/worldbooks'
import type { CharacterCard, ChatAttachmentRef, ChatMessage } from '@/types/st'
import { getChatAttachmentBlob } from '@/services/storage'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const dialog = useDialog()

const chatStore = useChatStore()
const settingsStore = useSettingsStore()
const runtimeStore = useRuntimeStore()
const worldbooksStore = useWorldbooksStore()

const inputText = ref('')
const messagesContainer = ref<HTMLElement | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)
const isInitialLoading = ref(false)
const previewUrls = ref<Record<string, string>>({})
const editingMessageId = ref<string | null>(null)
const editingContent = ref('')

const hasActiveApiConfig = computed(() => settingsStore.activeApiConfig !== null)
const canSend = computed(() => {
  return (inputText.value.trim() || chatStore.hasPendingAttachments) && !chatStore.isGenerating && hasActiveApiConfig.value
})

async function handleSend() {
  if (!canSend.value) return
  const content = inputText.value.trim()
  inputText.value = ''
  if (!settingsStore.activeApiConfig) {
    message.error('请先选择 API 配置')
    return
  }
  await chatStore.sendMessageStream(content, settingsStore.activeApiConfig)
  if (chatStore.error) {
    message.error(chatStore.error)
  }
  scrollToBottom()
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSend()
  }
}

function scrollToBottom() {
  nextTick(() => {
    const el = messagesContainer.value?.querySelector('.n-scrollbar-container') as HTMLElement | null
    if (el) el.scrollTop = el.scrollHeight
  })
}

function openFilePicker() {
  fileInput.value?.click()
}

async function onFileChange(e: Event) {
  const target = e.target as HTMLInputElement
  const files = Array.from(target.files ?? [])
  if (!files.length) return
  try {
    await chatStore.addPendingAttachments(files)
  } catch (err) {
    message.error(String(err))
  } finally {
    target.value = ''
  }
}

const characterName = computed(() => {
  return chatStore.currentCharacter?.data.name ?? 'AI'
})

const shouldShowGenerating = computed(() => {
  return chatStore.isGenerating && chatStore.messages[chatStore.messages.length - 1]?.role !== 'assistant'
})

function messageRole(msg: ChatMessage): 'user' | 'assistant' | 'system' {
  if (msg.role === 'user') return 'user'
  if (msg.role === 'assistant') return 'assistant'
  return 'system'
}

function messageName(msg: ChatMessage) {
  if (msg.role === 'user') return '你'
  if (msg.role === 'assistant') return characterName.value
  return '系统'
}

async function copyMessage(content: string) {
  try {
    await navigator.clipboard.writeText(content)
    message.success('已复制')
  } catch (err) {
    message.error(`复制失败: ${err}`)
  }
}

function startEditMessage(msg: ChatMessage) {
  editingMessageId.value = msg.id
  editingContent.value = msg.content
}

async function saveEditedMessage() {
  if (!editingMessageId.value) return
  await chatStore.updateMessageContent(editingMessageId.value, editingContent.value)
  editingMessageId.value = null
  editingContent.value = ''
}

function confirmDeleteMessage(msg: ChatMessage) {
  dialog.warning({
    title: '删除消息',
    content: '确定删除这条消息？此操作会立即保存到当前 ST 会话。',
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await chatStore.deleteMessage(msg.id)
    },
  })
}

function getPreviewUrl(attachment: ChatAttachmentRef): string | null {
  return previewUrls.value[attachment.attachment_id] ?? null
}

async function loadPreviewUrl(attachment: ChatAttachmentRef) {
  if (previewUrls.value[attachment.attachment_id]) return
  try {
    const bytes = await getChatAttachmentBlob(attachment.attachment_id)
    const blob = new Blob([new Uint8Array(bytes)], { type: attachment.mime_type })
    previewUrls.value = { ...previewUrls.value, [attachment.attachment_id]: URL.createObjectURL(blob) }
  } catch {
    // ignore
  }
}

function releaseUnusedPreviews() {
  const activeIds = new Set<string>()
  chatStore.pendingAttachments.forEach(a => activeIds.add(a.attachment_id))
  chatStore.messages.forEach(m => m.attachments?.forEach(a => activeIds.add(a.attachment_id)))
  const next: Record<string, string> = {}
  for (const [id, url] of Object.entries(previewUrls.value)) {
    if (activeIds.has(id)) next[id] = url
    else URL.revokeObjectURL(url)
  }
  previewUrls.value = next
}

function worldbookName(loreId: string) {
  return worldbooksStore.worldbookList.find(w => w.id === loreId)?.name ?? loreId
}

function getCharacterWorldLoreId(character: CharacterCard | null) {
  const stableLoreId = character?.data.extensions?.rst_world_lore_id
  if (typeof stableLoreId === 'string') return stableLoreId
  const worldName = character?.data.extensions?.world
  if (typeof worldName !== 'string') return null
  return worldbooksStore.worldbookList.find(w => w.name === worldName)?.id ?? null
}

const sessionWorldbooks = computed(() => {
  const result: Array<{ loreId: string; label: string; source: string; enabled: boolean }> = []
  const seen = new Set<string>()
  const disabled = new Set(chatStore.currentSession?.chat_metadata?.disabled_world_info ?? [])

  const add = (loreId: string, label: string, source: string) => {
    if (!loreId || seen.has(loreId)) return
    seen.add(loreId)
    result.push({ loreId, label, source, enabled: !disabled.has(loreId) })
  }

  const chatLoreIds = chatStore.currentSession?.chat_metadata?.enabled_world_info ?? (
    chatStore.currentSession?.chat_metadata?.world_info
      ? [chatStore.currentSession.chat_metadata.world_info]
      : []
  )
  for (const chatLoreId of chatLoreIds) {
    add(chatLoreId, worldbookName(chatLoreId), '会话')
  }

  for (const loreId of runtimeStore.globalState.world_info_settings.global_select) {
    add(loreId, worldbookName(loreId), '全局')
  }

  const charLoreId = getCharacterWorldLoreId(chatStore.currentCharacter)
  if (charLoreId) add(charLoreId, worldbookName(charLoreId), '角色')

  const charName = chatStore.currentCharacter?.data.name
  if (charName) {
    const binding = runtimeStore.globalState.world_info_settings.char_lore.find(b => b.name === charName)
    for (const loreId of binding?.extra_books ?? []) {
      add(loreId, worldbookName(loreId), '附加')
    }
  }

  return result
})

async function toggleWorldbook(loreId: string, enabled: boolean) {
  try {
    await chatStore.setWorldbookDisabled(loreId, !enabled)
  } catch (err) {
    message.error(String(err))
  }
}

function routeSessionId() {
  const v = route.params.sessionId
  return Array.isArray(v) ? v[0] : v
}

async function syncRouteSession() {
  const id = routeSessionId()
  if (!id || chatStore.currentSession?.id === id) return
  await chatStore.loadSession(id)
  scrollToBottom()
}

onMounted(async () => {
  isInitialLoading.value = true
  try {
    await Promise.all([
      chatStore.loadSessions(),
      settingsStore.loadApiConfigs(),
      runtimeStore.loadGlobalState(),
      worldbooksStore.loadWorldbooks(),
    ])
    settingsStore.setActiveApiConfig(runtimeStore.activeApiConfigId)
    await syncRouteSession()
  } finally {
    isInitialLoading.value = false
  }
})

watch(() => route.params.sessionId, syncRouteSession)
watch(() => chatStore.currentSession?.id, scrollToBottom)

watch(
  () => [
    chatStore.pendingAttachments.map(a => a.attachment_id).join(','),
    chatStore.messages.map(m => (m.attachments ?? []).map(a => a.attachment_id).join(',')).join('|'),
  ],
  async () => {
    const all = [...chatStore.pendingAttachments, ...chatStore.messages.flatMap(m => m.attachments ?? [])]
    for (const a of all) await loadPreviewUrl(a)
    releaseUnusedPreviews()
  },
  { immediate: true }
)

onBeforeUnmount(() => {
  Object.values(previewUrls.value).forEach(URL.revokeObjectURL)
})
</script>

<template>
  <div class="st-chat">
    <!-- Main: Chat -->
    <main class="chat-main">
      <template v-if="!chatStore.hasSession">
        <div class="chat-empty">
          <NEmpty description="选择或创建会话开始聊天" />
        </div>
      </template>

      <template v-else>
        <!-- Header -->
        <header class="chat-header">
          <NButton quaternary circle @click="router.push({ name: 'library' })">
            <template #icon><NIcon :component="ChevronBackOutline" /></template>
          </NButton>
          <div class="chat-title">
            <h1>{{ chatStore.currentSession?.name ?? '聊天' }}</h1>
          </div>
          <div v-if="!hasActiveApiConfig" class="api-warning">未选择 API</div>
        </header>

        <!-- Worldbooks -->
        <div v-if="sessionWorldbooks.length" class="worldbooks-bar">
          <div v-for="wb in sessionWorldbooks" :key="wb.loreId" class="wb-tag" :class="{ disabled: !wb.enabled }" @click="toggleWorldbook(wb.loreId, !wb.enabled)">
            {{ wb.label }}
          </div>
        </div>

        <!-- Messages -->
        <div ref="messagesContainer" class="messages">
          <NScrollbar>
            <div v-if="chatStore.messages.length === 0" class="messages-empty">
              <NText depth="3">开始对话吧</NText>
            </div>
            <div v-else class="message-list">
              <ChatMessageItem
                v-for="(msg, index) in chatStore.messages"
                :key="msg.id"
                :role="messageRole(msg)"
                :name="messageName(msg)"
                :content="msg.content"
                :created-at="msg.created_at"
                :floor="index + 1"
                @copy="copyMessage(msg.content)"
                @edit="startEditMessage(msg)"
                @delete="confirmDeleteMessage(msg)"
              >
                <template v-if="msg.attachments?.length" #attachments>
                  <div class="msg-attachments">
                    <img
                      v-for="att in msg.attachments"
                      :key="att.attachment_id"
                      :src="getPreviewUrl(att) ?? ''"
                      class="att-thumb"
                    >
                  </div>
                </template>
              </ChatMessageItem>
              <ChatMessageItem
                v-if="shouldShowGenerating"
                role="assistant"
                :name="characterName"
                :content="chatStore.streamingContent || '...'"
                :floor="chatStore.messages.length + 1"
                pending
                :editable="false"
                :deletable="false"
                @copy="copyMessage(chatStore.streamingContent)"
              >
                <template v-if="!chatStore.streamingContent" #attachments>
                  <NSpin size="small" />
                </template>
              </ChatMessageItem>
            </div>
          </NScrollbar>
        </div>

        <!-- Input -->
        <footer class="input-bar">
          <input ref="fileInput" type="file" multiple accept="image/*,.pdf" hidden @change="onFileChange">

          <div v-if="chatStore.pendingAttachments.length" class="pending-atts">
            <div v-for="att in chatStore.pendingAttachments" :key="att.attachment_id" class="pending-att">
              <img v-if="att.kind === 'image'" :src="getPreviewUrl(att) ?? ''" class="pending-thumb">
              <span v-else class="pending-file">{{ att.filename }}</span>
              <button class="pending-remove" @click="chatStore.removePendingAttachment(att.attachment_id)">
                <NIcon :component="TrashOutline" size="12" />
              </button>
            </div>
          </div>

          <div class="input-row">
            <NButton quaternary :disabled="chatStore.isGenerating" @click="openFilePicker">
              <template #icon><NIcon :component="AttachOutline" /></template>
            </NButton>
            <NInput
              v-model:value="inputText"
              type="textarea"
              placeholder="输入消息..."
              :autosize="{ minRows: 1, maxRows: 4 }"
              :disabled="chatStore.isGenerating"
              @keydown="handleKeyDown"
            />
            <NButton v-if="!chatStore.isGenerating" type="primary" :disabled="!canSend" @click="handleSend">
              <template #icon><NIcon :component="SendOutline" /></template>
            </NButton>
            <NButton v-else type="error" @click="chatStore.stopGeneration">
              <template #icon><NIcon :component="StopOutline" /></template>
            </NButton>
          </div>
        </footer>
      </template>
    </main>

    <NModal
      :show="editingMessageId !== null"
      preset="card"
      title="修改消息"
      class="message-edit-modal"
      @update:show="value => { if (!value) editingMessageId = null }"
    >
      <NInput
        v-model:value="editingContent"
        type="textarea"
        :autosize="{ minRows: 8, maxRows: 16 }"
        placeholder="消息内容"
      />
      <div class="modal-actions">
        <NButton @click="editingMessageId = null">取消</NButton>
        <NButton type="primary" @click="saveEditedMessage">保存</NButton>
      </div>
    </NModal>
  </div>
</template>

<style scoped>
.st-chat {
  height: 100%;
  min-height: 0;
  min-width: 0;
  display: flex;
  background: var(--n-color);
  overflow: hidden;
}

/* Main */
.chat-main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.chat-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.chat-header {
  padding: 12px 16px;
  display: flex;
  align-items: center;
  gap: 10px;
  border-bottom: 1px solid var(--n-border-color);
  flex-shrink: 0;
}

.chat-title {
  flex: 1;
  min-width: 0;
}

.chat-title h1 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.api-warning {
  font-size: 12px;
  color: var(--n-warning-color);
}

/* Worldbooks */
.worldbooks-bar {
  padding: 8px 14px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  border-bottom: 1px solid var(--n-border-color);
  flex-shrink: 0;
}

.wb-tag {
  padding: 4px 10px;
  font-size: 12px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--n-primary-color) 10%, var(--n-color));
  cursor: pointer;
  transition: all 0.15s;
}

.wb-tag:hover {
  background: color-mix(in srgb, var(--n-primary-color) 18%, var(--n-color));
}

.wb-tag.disabled {
  opacity: 0.5;
  text-decoration: line-through;
}

/* Messages */
.messages {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.messages-empty {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.message-list {
  padding: 16px 20px;
  max-width: 900px;
  margin: 0 auto;
}

.msg-attachments {
  margin-top: 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.att-thumb {
  width: 160px;
  max-height: 200px;
  object-fit: cover;
  border-radius: 12px;
  border: 1px solid var(--n-border-color);
}

/* Input */
.input-bar {
  padding: 12px 14px;
  border-top: 1px solid var(--n-border-color);
  flex-shrink: 0;
  min-width: 0;
}

.pending-atts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 10px;
}

.pending-att {
  position: relative;
}

.pending-thumb {
  width: 60px;
  height: 60px;
  object-fit: cover;
  border-radius: 6px;
  border: 1px solid var(--n-border-color);
}

.pending-file {
  display: inline-flex;
  align-items: center;
  padding: 6px 10px;
  font-size: 12px;
  border-radius: 6px;
  background: var(--n-color-hover);
}

.pending-remove {
  position: absolute;
  top: -6px;
  right: -6px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: none;
  background: var(--n-error-color);
  color: white;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.input-row {
  display: flex;
  gap: 8px;
  align-items: flex-end;
  min-width: 0;
  overflow: hidden;
}

.input-row :deep(.n-input) {
  flex: 1;
  min-width: 0;
}

.input-row :deep(.n-button) {
  flex: 0 0 auto;
}

.input-row :deep(.n-input-wrapper) {
  min-width: 0;
}

.message-edit-modal {
  width: min(720px, calc(100vw - 32px));
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}
</style>
