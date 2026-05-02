<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import {
  NLayout,
  NLayoutSider,
  NLayoutContent,
  NButton,
  NInput,
  NInputGroup,
  NEmpty,
  NSpin,
  NList,
  NListItem,
  NThing,
  NAvatar,
  NText,
  NScrollbar,
  NIcon,
  useMessage,
} from 'naive-ui'
import {
  SendOutline,
  AddOutline,
  TrashOutline,
  StopOutline,
} from '@vicons/ionicons5'
import { useChatStore } from '@/stores/chat'
import { useSettingsStore } from '@/stores/settings'

const chatStore = useChatStore()
const settingsStore = useSettingsStore()
const message = useMessage()

const inputText = ref('')
const messagesContainer = ref<HTMLElement | null>(null)

// Computed
const hasActiveApiConfig = computed(() => settingsStore.activeApiConfig !== null)
const canSend = computed(() => {
  return inputText.value.trim() && !chatStore.isGenerating && hasActiveApiConfig.value
})

// Methods
async function handleSend() {
  if (!canSend.value) return

  const content = inputText.value.trim()
  inputText.value = ''

  if (!settingsStore.activeApiConfig) {
    message.error('请先选择一个 API 配置')
    return
  }

  await chatStore.sendMessageStream(content, settingsStore.activeApiConfig)
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
    if (messagesContainer.value) {
      const scrollbar = messagesContainer.value.querySelector('.n-scrollbar-container')
      if (scrollbar) {
        scrollbar.scrollTop = scrollbar.scrollHeight
      }
    }
  })
}

async function createNewSession() {
  const name = `聊天 ${new Date().toLocaleString()}`
  await chatStore.createSession(name)
}

async function selectSession(id: string) {
  await chatStore.loadSession(id)
  scrollToBottom()
}

async function deleteSession(id: string, e: Event) {
  e.stopPropagation()
  await chatStore.deleteSession(id)
}

function formatTime(dateStr: string) {
  return new Date(dateStr).toLocaleTimeString()
}

// Load sessions on mount
onMounted(async () => {
  await chatStore.loadSessions()
  await settingsStore.loadApiConfigs()
})
</script>

<template>
  <div class="chat-view">
    <NLayout has-sider>
      <!-- Session List -->
      <NLayoutSider
        bordered
        :width="240"
        :native-scrollbar="false"
        content-style="padding: 12px;"
      >
        <div class="session-header">
          <NText strong>会话列表</NText>
          <NButton quaternary circle size="small" @click="createNewSession">
            <template #icon>
              <NIcon :component="AddOutline" />
            </template>
          </NButton>
        </div>

        <NSpin :show="chatStore.sessions.length === 0">
          <NList hoverable clickable>
            <NListItem
              v-for="session in chatStore.sessions"
              :key="session.id"
              :class="{ active: chatStore.currentSession?.id === session.id }"
              @click="selectSession(session.id)"
            >
              <NThing :title="session.name" :description="session.messages.length + ' 条消息'" />
              <template #suffix>
                <NButton
                  quaternary
                  circle
                  size="tiny"
                  @click="(e: Event) => deleteSession(session.id, e)"
                >
                  <template #icon>
                    <NIcon :component="TrashOutline" />
                  </template>
                </NButton>
              </template>
            </NListItem>
          </NList>
        </NSpin>
      </NLayoutSider>

      <!-- Chat Area -->
      <NLayoutContent>
        <div v-if="!chatStore.hasSession" class="empty-chat">
          <NEmpty description="选择或创建一个会话开始聊天" />
        </div>

        <template v-else>
          <!-- Messages -->
          <div ref="messagesContainer" class="messages-container">
            <NScrollbar>
              <div class="messages-list">
                <div
                  v-for="msg in chatStore.messages"
                  :key="msg.id"
                  :class="['message', msg.role]"
                >
                  <div class="message-avatar">
                    <NAvatar round size="small">
                      {{ msg.role === 'user' ? 'U' : 'A' }}
                    </NAvatar>
                  </div>
                  <div class="message-content">
                    <div class="message-header">
                      <NText depth="3" style="font-size: 12px">
                        {{ formatTime(msg.created_at) }}
                      </NText>
                    </div>
                    <div class="message-text">
                      {{ msg.content }}
                    </div>
                  </div>
                </div>

                <!-- Generating indicator -->
                <div v-if="chatStore.isGenerating" class="message assistant">
                  <div class="message-avatar">
                    <NAvatar round size="small">A</NAvatar>
                  </div>
                  <div class="message-content">
                    <NSpin size="small" />
                    <span v-if="chatStore.streamingContent" class="message-text">
                      {{ chatStore.streamingContent }}
                    </span>
                  </div>
                </div>
              </div>
            </NScrollbar>
          </div>

          <!-- Input Area -->
          <div class="input-area">
            <NInputGroup>
              <NInput
                v-model:value="inputText"
                type="textarea"
                placeholder="输入消息... (Enter 发送, Shift+Enter 换行)"
                :autosize="{ minRows: 1, maxRows: 4 }"
                :disabled="chatStore.isGenerating"
                @keydown="handleKeyDown"
              />
              <NButton
                v-if="!chatStore.isGenerating"
                type="primary"
                :disabled="!canSend"
                @click="handleSend"
              >
                <template #icon>
                  <NIcon :component="SendOutline" />
                </template>
              </NButton>
              <NButton
                v-else
                type="error"
                @click="chatStore.stopGeneration"
              >
                <template #icon>
                  <NIcon :component="StopOutline" />
                </template>
              </NButton>
            </NInputGroup>

            <div v-if="!hasActiveApiConfig" class="api-warning">
              <NText type="warning">请先在设置中选择一个 API 配置</NText>
            </div>
          </div>
        </template>
      </NLayoutContent>
    </NLayout>
  </div>
</template>

<style scoped>
.chat-view {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.session-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.empty-chat {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.messages-container {
  flex: 1;
  overflow: hidden;
}

.messages-list {
  padding: 16px;
}

.message {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}

.message.user {
  flex-direction: row-reverse;
}

.message.user .message-content {
  align-items: flex-end;
}

.message-avatar {
  flex-shrink: 0;
}

.message-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-width: 70%;
}

.message-text {
  background: var(--n-color);
  padding: 8px 12px;
  border-radius: 8px;
  white-space: pre-wrap;
  word-break: break-word;
}

.message.user .message-text {
  background: var(--n-color-target);
}

.input-area {
  padding: 16px;
  border-top: 1px solid var(--n-border-color);
}

.api-warning {
  margin-top: 8px;
  text-align: center;
}

.n-list-item.active {
  background: var(--n-color-hover);
}
</style>