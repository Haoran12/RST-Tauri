<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { NButton, NEmpty, NIcon, NInput, NList, NListItem, NSpin, NTag } from 'naive-ui'
import { AddOutline, SearchOutline } from '@vicons/ionicons5'
import { useAgentStore } from '@/stores/agent'

const route = useRoute()
const router = useRouter()
const agentStore = useAgentStore()
const searchQuery = ref('')

const currentWorldId = computed(() => {
  const routeWorldId = route.params.worldId
  if (typeof routeWorldId === 'string' && routeWorldId.length > 0) return routeWorldId
  return agentStore.currentWorldId
})

const pageTitle = computed(() => {
  switch (route.name) {
    case 'agent-home':
      return 'Agent'
    case 'agent-worlds':
      return 'Worlds'
    default:
      return 'Agent'
  }
})

const sessions = computed(() => {
  const sorted = agentStore.sessions
    .slice()
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())

  if (!searchQuery.value) return sorted
  const query = searchQuery.value.toLowerCase()
  return sorted.filter(session =>
    session.title.toLowerCase().includes(query) ||
    session.session_kind.toLowerCase().includes(query) ||
    session.period_anchor.display_text.toLowerCase().includes(query)
  )
})

function formatShortTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleString()
}

function openSession(sessionId: string, worldId: string) {
  router.push({
    name: 'agent-chat',
    params: {
      worldId,
      sessionId,
    },
  })
}

function openCreateSession() {
  window.dispatchEvent(new CustomEvent('open-agent-session-create'))
}

watch(currentWorldId, async (worldId) => {
  if (!worldId) {
    agentStore.clearWorld()
    return
  }
  await agentStore.loadWorld(worldId)
}, { immediate: true })
</script>

<template>
  <div class="context-list">
    <div class="list-header">
      <span class="list-title">{{ pageTitle }}</span>
      <NButton quaternary size="small" @click="openCreateSession">
        <template #icon>
          <NIcon><AddOutline /></NIcon>
        </template>
      </NButton>
    </div>

    <div class="list-search">
      <NInput
        v-model:value="searchQuery"
        placeholder="搜索 Agent 会话..."
        clearable
        size="small"
      >
        <template #prefix>
          <NIcon :size="16"><SearchOutline /></NIcon>
        </template>
      </NInput>
    </div>

    <div class="world-summary">
      <div class="summary-row">
        <span>当前 World</span>
        <strong>{{ currentWorldId }}</strong>
      </div>
      <div class="summary-row">
        <span>主线时间</span>
        <strong>{{ agentStore.mainlineCursor?.mainline_time_anchor.display_text ?? '未加载' }}</strong>
      </div>
    </div>

    <div class="list-content">
      <NSpin :show="agentStore.isLoading">
        <NList v-if="sessions.length > 0" hoverable clickable>
          <NListItem
            v-for="session in sessions"
            :key="session.session_id"
            class="context-item"
            @click="openSession(session.session_id, session.world_id)"
          >
            <div class="context-item-row">
              <div class="context-item-main">
                <span class="context-item-name">{{ session.title }}</span>
                <NTag size="tiny" :bordered="false">{{ session.session_kind }}</NTag>
              </div>
            </div>
            <div class="context-item-meta">
              {{ session.period_anchor.display_text }} · {{ formatShortTime(session.updated_at) }}
            </div>
          </NListItem>
        </NList>
        <NEmpty v-else description="暂无 Agent 会话" />
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

.list-header {
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.list-title {
  font-weight: 600;
  font-size: 15px;
}

.list-search {
  padding: 8px 12px;
  flex-shrink: 0;
}

.world-summary {
  padding: 0 12px 10px;
  display: grid;
  gap: 8px;
  flex-shrink: 0;
}

.summary-row {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--color-bg-subtle, #f5f7fa);
}

.summary-row span {
  color: var(--color-text-secondary, #6b7280);
}

.list-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 4px;
}

.context-item {
  border-radius: 6px;
  cursor: pointer;
}

.context-item:hover {
  background-color: rgba(0, 0, 0, 0.04);
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
}

.context-item-name {
  font-weight: 600;
}

.context-item-meta {
  margin-top: 4px;
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
}
</style>
