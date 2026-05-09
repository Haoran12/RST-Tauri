<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NButton,
  NEmpty,
  NIcon,
  NInput,
  NList,
  NListItem,
  NSelect,
  NSpin,
  NTag,
} from 'naive-ui'
import {
  AddOutline,
  SearchOutline,
  GlobeOutline,
  GitBranchOutline,
  TimeOutline,
} from '@vicons/ionicons5'
import { useAgentStore } from '@/stores/agent'
import type { AgentSession } from '@/types/agent/session'

const route = useRoute()
const router = useRouter()
const agentStore = useAgentStore()

const searchQuery = ref('')
const selectedWorldId = ref<string>('')

// ===== World Selection =====

const worldOptions = computed(() => {
  return agentStore.worlds.map(w => ({
    label: `${w.world_id} (${w.session_count} 会话)`,
    value: w.world_id,
  }))
})

const currentWorld = computed(() =>
  agentStore.worlds.find(w => w.world_id === selectedWorldId.value)
)

// Initialize selectedWorldId from route or store
function resolveWorldId(): string {
  const routeWorldId = route.params.worldId
  if (typeof routeWorldId === 'string' && routeWorldId.length > 0) return routeWorldId
  return agentStore.currentWorldId ?? ''
}

selectedWorldId.value = resolveWorldId()

// ===== Session List =====

const filteredSessions = computed(() => {
  const worldSessions = agentStore.sessions.filter(
    s => s.world_id === selectedWorldId.value
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

function switchWorld(worldId: string) {
  if (!worldId || worldId === selectedWorldId.value) return
  selectedWorldId.value = worldId
  agentStore.loadWorld(worldId)
  // If currently on a world-specific route, redirect to the new world
  if (route.params.worldId) {
    router.push({ name: 'agent-worlds', params: { worldId } })
  }
}

function openSession(session: AgentSession) {
  router.push({
    name: 'agent-chat',
    params: { worldId: session.world_id, sessionId: session.session_id },
  })
}

function openWorldWorkspace() {
  if (!selectedWorldId.value) return
  router.push({ name: 'agent-worlds', params: { worldId: selectedWorldId.value } })
}

function openWorldEditor() {
  if (!selectedWorldId.value) return
  router.push({ name: 'agent-world-editor', params: { worldId: selectedWorldId.value } })
}

function openCreateSession() {
  if (!selectedWorldId.value) {
    // Emit to parent or use a global event; here we push to world view which has the modal
    router.push({ name: 'agent-worlds', params: { worldId: selectedWorldId.value || 'new' } })
    return
  }
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

// ===== Watchers =====

watch(
  () => route.params.worldId,
  (routeWorldId) => {
    if (typeof routeWorldId === 'string' && routeWorldId.length > 0) {
      selectedWorldId.value = routeWorldId
      agentStore.loadWorld(routeWorldId)
    }
  },
  { immediate: true }
)

watch(selectedWorldId, async (worldId) => {
  if (!worldId) return
  await agentStore.loadWorld(worldId)
})
</script>

<template>
  <div class="context-list">
    <!-- World Switcher -->
    <div class="world-switcher">
      <div class="switcher-header">
        <NIcon :size="16"><GlobeOutline /></NIcon>
        <span class="switcher-label">World</span>
        <NButton quaternary size="tiny" @click="openCreateSession">
          <template #icon><NIcon><AddOutline /></NIcon></template>
        </NButton>
      </div>
      <NSelect
        v-model:value="selectedWorldId"
        :options="worldOptions"
        size="small"
        placeholder="选择 World..."
        @update:value="switchWorld"
      />
      <div v-if="currentWorld" class="world-meta">
        <span>主线 {{ currentWorld.mainline_time_anchor?.display_text ?? '未初始化' }}</span>
        <span>·</span>
        <span>{{ currentWorld.character_count }} 角色</span>
      </div>
    </div>

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

    <!-- Quick Actions -->
    <div class="quick-actions">
      <NButton size="tiny" secondary @click="openWorldWorkspace">
        <template #icon><NIcon :size="14"><GitBranchOutline /></NIcon></template>
        工作区
      </NButton>
      <NButton size="tiny" secondary @click="openWorldEditor">
        <template #icon><NIcon :size="14"><GlobeOutline /></NIcon></template>
        编辑器
      </NButton>
    </div>

    <!-- Session List -->
    <div class="list-content">
      <NSpin :show="agentStore.isLoading">
        <template v-if="!selectedWorldId">
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

.world-switcher {
  padding: 10px 12px 8px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.switcher-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.switcher-label {
  font-weight: 600;
  font-size: 13px;
  flex: 1;
}

.world-meta {
  margin-top: 6px;
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  display: flex;
  gap: 6px;
  align-items: center;
}

.list-search {
  padding: 8px 12px;
  flex-shrink: 0;
}

.quick-actions {
  padding: 0 12px 8px;
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.list-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 4px 8px;
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
