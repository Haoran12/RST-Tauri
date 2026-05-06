<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NAlert,
  NButton,
  NEmpty,
  NIcon,
  NInput,
  NModal,
  NScrollbar,
  NSpin,
  NTag,
  NText,
  useDialog,
  useMessage,
} from 'naive-ui'
import { ArrowBackOutline, SendOutline } from '@vicons/ionicons5'
import ChatMessageItem from '@/components/shared/ChatMessageItem.vue'
import { useAgentStore } from '@/stores/agent'
import type { AgentSession, SessionTurn } from '@/types/agent/session'
import {
  deleteAgentSessionTurn,
  getAgentSession,
  listAgentSessionTurns,
  processAgentTurn,
  updateAgentSessionTurn,
} from '@/services/agentApi'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const dialog = useDialog()
const agentStore = useAgentStore()

const worldId = computed(() => {
  const routeWorldId = route.params.worldId
  if (typeof routeWorldId === 'string' && routeWorldId.length > 0) return routeWorldId
  return agentStore.currentWorldId ?? ''
})
const sessionId = computed(() => String(route.params.sessionId || ''))

const session = ref<AgentSession | null>(null)
const turns = ref<SessionTurn[]>([])
const inputText = ref('')
const isLoading = ref(false)
const isSending = ref(false)
const error = ref<string | null>(null)
const messagesContainer = ref<HTMLElement | null>(null)
const editingTurnId = ref<string | null>(null)
const editingContent = ref('')

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

function turnRole(turn: SessionTurn): 'user' | 'assistant' | 'system' {
  if (turn.role === 'User') return 'user'
  if (turn.role === 'Assistant') return 'assistant'
  return 'system'
}

async function copyTurn(content: string) {
  try {
    await navigator.clipboard.writeText(content)
    message.success('已复制')
  } catch (e) {
    message.error(`复制失败: ${e}`)
  }
}

function startEditTurn(turn: SessionTurn) {
  editingTurnId.value = turn.session_turn_id
  editingContent.value = turnText(turn)
}

async function saveEditedTurn() {
  if (!editingTurnId.value) return
  try {
    const updated = await updateAgentSessionTurn({
      world_id: worldId.value,
      session_id: sessionId.value,
      session_turn_id: editingTurnId.value,
      content: editingContent.value,
    })
    const index = turns.value.findIndex(turn => turn.session_turn_id === updated.session_turn_id)
    if (index !== -1) turns.value[index] = updated
    editingTurnId.value = null
    editingContent.value = ''
  } catch (e) {
    message.error(String(e))
  }
}

function confirmDeleteTurn(turn: SessionTurn) {
  dialog.warning({
    title: '删除消息',
    content: '确定删除这条 Agent 会话消息？这只删除会话可见记录，不回滚已经提交的世界状态。',
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await deleteAgentSessionTurn({
          world_id: worldId.value,
          session_id: sessionId.value,
          session_turn_id: turn.session_turn_id,
        })
        turns.value = turns.value.filter(item => item.session_turn_id !== turn.session_turn_id)
      } catch (e) {
        message.error(String(e))
      }
    },
  })
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
              <ChatMessageItem
                v-for="(turn, index) in turns"
                :key="turn.session_turn_id"
                :role="turnRole(turn)"
                :name="roleLabel(turn)"
                :content="turnText(turn)"
                :created-at="turn.created_at"
                :floor="index + 1"
                @copy="copyTurn(turnText(turn))"
                @edit="startEditTurn(turn)"
                @delete="confirmDeleteTurn(turn)"
              />
            </div>
          </NScrollbar>
        </div>

        <div class="input-area">
          <div class="input-row">
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
          </div>
        </div>
      </template>
    </NSpin>

    <NModal
      :show="editingTurnId !== null"
      preset="card"
      title="修改消息"
      class="message-edit-modal"
      @update:show="value => { if (!value) editingTurnId = null }"
    >
      <NInput
        v-model:value="editingContent"
        type="textarea"
        :autosize="{ minRows: 8, maxRows: 16 }"
        placeholder="消息内容"
      />
      <div class="modal-actions">
        <NButton @click="editingTurnId = null">取消</NButton>
        <NButton type="primary" @click="saveEditedTurn">保存</NButton>
      </div>
    </NModal>
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
  max-width: 900px;
  margin: 0 auto;
}

.input-area {
  padding: 14px 16px;
  border-top: 1px solid var(--n-border-color);
  flex-shrink: 0;
  min-width: 0;
}

.input-row {
  display: flex;
  align-items: flex-end;
  gap: 8px;
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

.empty-state {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
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
