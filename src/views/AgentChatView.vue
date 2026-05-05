<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NAlert,
  NButton,
  NEmpty,
  NIcon,
  NInput,
  NInputGroup,
  NScrollbar,
  NSpin,
  NTag,
  NText,
  useMessage,
} from 'naive-ui'
import { ArrowBackOutline, SendOutline } from '@vicons/ionicons5'
import type { AgentSession, SessionTurn } from '@/types/agent/session'
import {
  getAgentSession,
  listAgentSessionTurns,
  processAgentTurn,
} from '@/services/agentApi'

const route = useRoute()
const router = useRouter()
const message = useMessage()

const worldId = computed(() => String(route.params.worldId || 'default'))
const sessionId = computed(() => String(route.params.sessionId || ''))

const session = ref<AgentSession | null>(null)
const turns = ref<SessionTurn[]>([])
const inputText = ref('')
const isLoading = ref(false)
const isSending = ref(false)
const error = ref<string | null>(null)
const messagesContainer = ref<HTMLElement | null>(null)

const canSend = computed(() => inputText.value.trim().length > 0 && !isSending.value && !!session.value)

async function loadSession() {
  if (!sessionId.value) return
  isLoading.value = true
  error.value = null
  try {
    const loaded = await getAgentSession(worldId.value, sessionId.value)
    if (!loaded) {
      error.value = '会话不存在'
      session.value = null
      turns.value = []
      return
    }
    session.value = loaded
    turns.value = await listAgentSessionTurns(worldId.value, sessionId.value)
    scrollToBottom()
  } catch (e) {
    error.value = String(e)
  } finally {
    isLoading.value = false
  }
}

async function handleSend() {
  if (!canSend.value) return
  const content = inputText.value.trim()
  inputText.value = ''
  isSending.value = true
  error.value = null
  try {
    const output = await processAgentTurn({
      world_id: worldId.value,
      session_id: sessionId.value,
      content,
    })
    turns.value.push(output.user_turn, output.assistant_turn)
    scrollToBottom()
  } catch (e) {
    inputText.value = content
    message.error(String(e))
    error.value = String(e)
  } finally {
    isSending.value = false
  }
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSend()
  }
}

function turnText(turn: SessionTurn) {
  if (typeof turn.message_json === 'string') return turn.message_json
  if (turn.message_json && typeof turn.message_json === 'object') {
    const value = turn.message_json as Record<string, unknown>
    if (typeof value.content === 'string') return value.content
    if (typeof value.text === 'string') return value.text
  }
  return JSON.stringify(turn.message_json, null, 2)
}

function roleLabel(turn: SessionTurn) {
  if (turn.role === 'User') return '你'
  if (turn.role === 'Assistant') return 'Agent'
  return '系统'
}

function formatTime(dateStr: string) {
  return new Date(dateStr).toLocaleTimeString()
}

function scrollToBottom() {
  nextTick(() => {
    const scrollbar = messagesContainer.value?.querySelector('.n-scrollbar-container') as HTMLElement | null
    if (scrollbar) {
      scrollbar.scrollTop = scrollbar.scrollHeight
    }
  })
}

watch(() => [worldId.value, sessionId.value], loadSession)
onMounted(loadSession)
</script>

<template>
  <div class="agent-chat-view">
    <header class="chat-header">
      <div class="chat-title">
        <NButton quaternary circle @click="router.push({ name: 'agent-worlds', params: { worldId } })">
          <template #icon>
            <NIcon :component="ArrowBackOutline" />
          </template>
        </NButton>
        <div>
          <h1>{{ session?.title ?? 'Agent 会话' }}</h1>
          <div class="chat-meta">
            <NTag size="small" :bordered="false">{{ session?.player_mode ?? 'Unknown' }}</NTag>
            <NText depth="3">{{ session?.period_anchor.display_text ?? worldId }}</NText>
          </div>
        </div>
      </div>
      <NTag v-if="session" size="small" :bordered="false">
        {{ session.session_kind }}
      </NTag>
    </header>

    <NAlert v-if="error" type="error" class="top-alert">
      {{ error }}
    </NAlert>

    <NSpin :show="isLoading" class="chat-body">
      <div v-if="!session && !isLoading" class="empty-state">
        <NEmpty description="会话不可用" />
      </div>

      <template v-else>
        <div ref="messagesContainer" class="messages-container">
          <NScrollbar>
            <div v-if="turns.length === 0" class="empty-state">
              <NEmpty description="还没有回合记录" />
            </div>
            <div v-else class="messages-list">
              <div
                v-for="turn in turns"
                :key="turn.session_turn_id"
                :class="['message-row', turn.role.toLowerCase()]"
              >
                <div class="message-meta">
                  <strong>{{ roleLabel(turn) }}</strong>
                  <span>{{ formatTime(turn.created_at) }}</span>
                  <NTag size="small" :bordered="false">{{ turn.canon_status }}</NTag>
                </div>
                <div class="message-text">{{ turnText(turn) }}</div>
              </div>
            </div>
          </NScrollbar>
        </div>

        <div class="input-area">
          <NInputGroup>
            <NInput
              v-model:value="inputText"
              type="textarea"
              placeholder="输入角色言行、场景叙述或导演提示..."
              :autosize="{ minRows: 1, maxRows: 5 }"
              :disabled="isSending"
              @keydown="handleKeyDown"
            />
            <NButton type="primary" :disabled="!canSend" :loading="isSending" @click="handleSend">
              <template #icon>
                <NIcon :component="SendOutline" />
              </template>
            </NButton>
          </NInputGroup>
        </div>
      </template>
    </NSpin>
  </div>
</template>

<style scoped>
.agent-chat-view {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--n-color);
  overflow: hidden;
}

.chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--n-border-color);
  flex-shrink: 0;
}

.chat-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.chat-title h1 {
  margin: 0;
  font-size: 18px;
  line-height: 1.25;
}

.chat-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
}

.top-alert {
  margin: 12px 16px 0;
  flex-shrink: 0;
}

.chat-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.chat-body :deep(.n-spin-container) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.messages-container {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.messages-list {
  padding: 18px;
}

.message-row {
  max-width: min(760px, 88%);
  margin-bottom: 16px;
}

.message-row.user {
  margin-left: auto;
}

.message-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
  color: var(--n-text-color-3);
  font-size: 12px;
}

.message-row.user .message-meta {
  justify-content: flex-end;
}

.message-text {
  white-space: pre-wrap;
  word-break: break-word;
  padding: 10px 12px;
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  background: var(--n-color);
}

.message-row.user .message-text {
  background: color-mix(in srgb, var(--n-primary-color) 14%, var(--n-color));
}

.input-area {
  padding: 14px 16px;
  border-top: 1px solid var(--n-border-color);
  flex-shrink: 0;
}

.empty-state {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
