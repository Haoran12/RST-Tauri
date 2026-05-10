<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NButton,
  NEmpty,
  NIcon,
  NInput,
  NList,
  NListItem,
  NSpin,
  NTag,
} from 'naive-ui'
import {
  SearchOutline,
  TimeOutline,
} from '@vicons/ionicons5'
import { useAgentStore } from '@/stores/agent'
import type { AgentSession } from '@/types/agent/session'

const router = useRouter()
const agentStore = useAgentStore()

const searchQuery = ref('')

// Use global currentWorldId from store
const worldId = computed(() => agentStore.currentWorldId ?? '')

// ===== Session List =====

const filteredSessions = computed(() => {
  const worldSessions = agentStore.sessions.filter(
    s => s.world_id === worldId.value
  )
  const sorted = [...worldSessions].sort(
    (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
  )
  if (!searchQuery.value) return sorted
  const query = searchQuery.value.toLowerCase()
  return sorted.filter(
    s =>
      s.title.toLowerCase().includes(query) ||
      s.session_kind.toLowerCase().includes(query) ||
      s.period_anchor.display_text.toLowerCase().includes(query)
  )
})

const sessionGroups = computed(() => {
  const groups: { kind: string; label: string; type: string; sessions: AgentSession[] }[] = [
    { kind: 'Mainline', label: '主线会话', type: 'success', sessions: [] },
    { kind: 'Retrospective', label: '过去线', type: 'warning', sessions: [] },
    { kind: 'FuturePreview', label: '未来预演', type: 'info', sessions: [] },
  ]
  for (const session of filteredSessions.value) {
    const group = groups.find(g => g.kind === session.session_kind)
    if (group) group.sessions.push(session)
  }
  return groups.filter(g => g.sessions.length > 0)
})

// ===== Navigation =====

function openSession(session: AgentSession) {
  router.push({
    name: 'agent-chat',
    params: { worldId: session.world_id, sessionId: session.session_id },
  })
}

function openCreateSession() {
  window.dispatchEvent(new CustomEvent('open-agent-session-create'))
}

function formatShortTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleString()
}

function getPlayerModeLabel(mode: string) {
  switch (mode) {
    case 'Character':
      return '扮演'
    case 'Director':
      return '导演'
    default:
      return mode
  }
}
</script>

<template>
  <div class="context-list">
    <!-- Session Search -->
    <div class="list-search">
      <NInput
        v-model:value="searchQuery"
        placeholder="搜索会话..."
        clearable
        size="small"
      >
        <template #prefix>
          <NIcon :size="16"><SearchOutline /></NIcon>
        </template>
      </NInput>
    </div>

    <!-- Session List -->
    <div class="list-content">
      <NSpin :show="agentStore.isLoading">
        <template v-if="!worldId">
          <NEmpty description="请先选择一个 World" size="small" />
        </template>
        <template v-else-if="sessionGroups.length === 0">
          <NEmpty description="该 World 暂无会话" size="small">
            <template #extra>
              <NButton size="tiny" @click="openCreateSession">创建会话</NButton>
            </template>
          </NEmpty>
        </template>
        <template v-else>
          <div v-for="group in sessionGroups" :key="group.kind" class="session-group">
            <div class="group-header">
              <NTag size="tiny" :type="group.type as any" :bordered="false">
                {{ group.label }}
              </NTag>
              <span class="group-count">{{ group.sessions.length }}</span>
            </div>
            <NList hoverable clickable>
              <NListItem
                v-for="session in group.sessions"
                :key="session.session_id"
                class="context-item"
                @click="openSession(session)"
              >
                <div class="context-item-row">
                  <div class="context-item-main">
                    <span class="context-item-name">{{ session.title }}</span>
                  </div>
                  <NTag size="tiny" :bordered="false">
                    {{ getPlayerModeLabel(session.player_mode) }}
                  </NTag>
                </div>
                <div class="context-item-meta">
                  <NIcon :size="12"><TimeOutline /></NIcon>
                  {{ session.period_anchor.display_text }}
                  <span class="meta-sep">·</span>
                  {{ formatShortTime(session.updated_at) }}
                </div>
              </NListItem>
            </NList>
          </div>
        </template>
      </NSpin>
    </div>
  </div>
</template>

<style scoped>
.context-list {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.list-search {
  padding: 10px 12px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
}

.list-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 4px;
}

.session-group {
  margin-bottom: 8px;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
}

.group-count {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
}

.context-item {
  border-radius: 6px;
  cursor: pointer;
  padding: 8px 10px;
}

.context-item:hover {
  background-color: var(--n-color-hover);
}

.context-item-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.context-item-main {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  flex: 1;
}

.context-item-name {
  font-weight: 600;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.context-item-meta {
  margin-top: 4px;
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  display: flex;
  align-items: center;
  gap: 4px;
}

.meta-sep {
  margin: 0 2px;
}
</style>
