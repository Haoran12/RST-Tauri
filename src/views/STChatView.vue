<script setup lang="ts">
import { ref, computed, onMounted, nextTick, onBeforeUnmount, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NButton,
  NEmpty,
  NIcon,
  NInput,
  NScrollbar,
  NSpin,
  NText,
  NAvatar,
  useMessage,
} from 'naive-ui'
import {
  AttachOutline,
  ChevronBackOutline,
  SendOutline,
  StopOutline,
  TrashOutline,
  PersonOutline,
  SparklesOutline,
} from '@vicons/ionicons5'
import { useChatStore } from '@/stores/chat'
import { useSettingsStore } from '@/stores/settings'
import { useRuntimeStore } from '@/stores/runtime'
import { useWorldbooksStore } from '@/stores/worldbooks'
import type { CharacterCard, ChatAttachmentRef } from '@/types/st'
import { getChatAttachmentBlob } from '@/services/storage'

const route = useRoute()
const router = useRouter()
const message = useMessage()

const chatStore = useChatStore()
const settingsStore = useSettingsStore()
const runtimeStore = useRuntimeStore()
const worldbooksStore = useWorldbooksStore()

const inputText = ref('')
const messagesContainer = ref<HTMLElement | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)
const isInitialLoading = ref(false)
const previewUrls = ref<Record<string, string>>({})

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

function formatTime(dateStr: string) {
  return new Date(dateStr).toLocaleTimeString()
}

const characterAvatar = computed(() => {
  const avatarPath = chatStore.currentCharacter?.data.avatar
  if (avatarPath) {
    // 返回相对路径或base64，这里需要根据实际存储方式处理
    return null // 暂时返回null，后续可以加载头像
  }
  return null
})

const characterName = computed(() => {
  return chatStore.currentCharacter?.data.name ?? 'AI'
})

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

  const chatLoreId = chatStore.currentSession?.chat_metadata?.world_info
  if (chatLoreId) add(chatLoreId, worldbookName(chatLoreId), '聊天')

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
              <div v-for="msg in chatStore.messages" :key="msg.id" :class="['msg', msg.role]">
                <!-- Avatar -->
                <div class="msg-avatar">
                  <template v-if="msg.role === 'user'">
                    <NAvatar round size="small" color="var(--n-primary-color)">
                      <NIcon :component="PersonOutline" />
                    </NAvatar>
                  </template>
                  <template v-else>
                    <NAvatar v-if="characterAvatar" round size="small" :src="characterAvatar" />
                    <NAvatar v-else round size="small" color="var(--n-success-color)">
                      <NIcon :component="SparklesOutline" />
                    </NAvatar>
                  </template>
                </div>
                <!-- Content -->
                <div class="msg-body">
                  <div class="msg-header">
                    <span class="msg-name">{{ msg.role === 'user' ? '你' : characterName }}</span>
                    <span class="msg-time">{{ formatTime(msg.created_at) }}</span>
                  </div>
                  <div class="msg-content">
                    <div class="msg-text">{{ msg.content }}</div>
                    <div v-if="msg.attachments?.length" class="msg-attachments">
                      <img
                        v-for="att in msg.attachments"
                        :key="att.attachment_id"
                        :src="getPreviewUrl(att) ?? ''"
                        class="att-thumb"
                      >
                    </div>
                  </div>
                </div>
              </div>
              <div v-if="chatStore.isGenerating" class="msg assistant">
                <div class="msg-avatar">
                  <NAvatar v-if="characterAvatar" round size="small" :src="characterAvatar" />
                  <NAvatar v-else round size="small" color="var(--n-success-color)">
                    <NIcon :component="SparklesOutline" />
                  </NAvatar>
                </div>
                <div class="msg-body">
                  <div class="msg-header">
                    <span class="msg-name">{{ characterName }}</span>
                    <span class="msg-time">生成中...</span>
                  </div>
                  <div class="msg-content">
                    <NSpin v-if="!chatStore.streamingContent" size="small" />
                    <div v-else class="msg-text">{{ chatStore.streamingContent }}</div>
                  </div>
                </div>
              </div>
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
  </div>
</template>

<style scoped>
.st-chat {
  height: 100%;
  display: flex;
  background: var(--n-color);
}

/* Main */
.chat-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
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

.msg {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
}

.msg.user {
  flex-direction: row-reverse;
}

.msg-avatar {
  flex-shrink: 0;
  margin-top: 4px;
}

.msg-body {
  flex: 1;
  min-width: 0;
  max-width: 75%;
}

.msg.user .msg-body {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.msg-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.msg.user .msg-header {
  flex-direction: row-reverse;
}

.msg-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--n-text-color);
}

.msg-time {
  font-size: 11px;
  color: var(--n-text-color-3);
}

.msg-content {
  display: inline-block;
}

.msg-text {
  padding: 12px 16px;
  border-radius: 16px;
  background: var(--n-color);
  border: 1px solid var(--n-border-color);
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.6;
  font-size: 14px;
}

.msg.user .msg-text {
  background: var(--n-primary-color);
  color: white;
  border-color: transparent;
  border-bottom-right-radius: 4px;
}

.msg.assistant .msg-text {
  border-bottom-left-radius: 4px;
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
}

.input-row .n-input {
  flex: 1;
}
</style>
