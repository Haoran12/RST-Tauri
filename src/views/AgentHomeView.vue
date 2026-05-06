<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NCard, NIcon, NList, NListItem, NSpace, NTag } from 'naive-ui'
import { GitBranchOutline, KeyOutline, MapOutline, PlayOutline, SettingsOutline, TimerOutline } from '@vicons/ionicons5'
import { useAgentStore } from '@/stores/agent'
import { useSettingsStore } from '@/stores/settings'
import { useAppShellStore } from '@/stores/appShell'

const router = useRouter()
const agentStore = useAgentStore()
const settingsStore = useSettingsStore()
const appShell = useAppShellStore()

const worldId = computed(() => agentStore.currentWorldId ?? 'default')

const summaryItems = computed(() => [
  {
    label: '当前 World',
    value: worldId.value,
    icon: MapOutline,
  },
  {
    label: '已加载会话',
    value: String(agentStore.sessions.length),
    icon: GitBranchOutline,
  },
  {
    label: '活动会话',
    value: String(agentStore.activeSessions.length),
    icon: PlayOutline,
  },
  {
    label: '共享 API',
    value: String(settingsStore.apiConfigs.length),
    icon: KeyOutline,
  },
])

const recentSessions = computed(() =>
  [...agentStore.sessions]
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 6),
)

async function hydrate() {
  appShell.setCurrentMode('agent')
  await Promise.allSettled([
    settingsStore.loadApiConfigs(),
    agentStore.loadWorld('default'),
  ])
}

function openDefaultWorld() {
  router.push({ name: 'agent-worlds', params: { worldId: worldId.value } })
}

function openSession(sessionId: string) {
  router.push({ name: 'agent-chat', params: { worldId: worldId.value, sessionId } })
}

onMounted(() => {
  void hydrate()
})
</script>

<template>
  <div class="workspace-view agent-home">
    <header class="page-header">
      <div>
        <h1>Agent Workspace</h1>
        <p>聚合当前 World、会话入口和运行侧工作流，不混入 ST 资源页。</p>
      </div>
      <NSpace>
        <NButton secondary @click="router.push({ name: 'mode-select' })">切换模式</NButton>
        <NButton secondary @click="router.push({ name: 'logs' })">查看日志</NButton>
        <NButton type="primary" @click="openDefaultWorld">打开当前 World</NButton>
      </NSpace>
    </header>

    <div class="section-grid four">
      <NCard v-for="item in summaryItems" :key="item.label" size="small" class="summary-card">
        <div class="summary-head">
          <NIcon :size="18"><component :is="item.icon" /></NIcon>
          <span>{{ item.label }}</span>
        </div>
        <div class="summary-value">{{ item.value }}</div>
      </NCard>
    </div>

    <div class="section-grid">
      <NCard size="small" title="当前 World 摘要">
        <div class="info-list">
          <div class="info-row">
            <span>World ID</span>
            <strong>{{ worldId }}</strong>
          </div>
          <div class="info-row">
            <span>主线时间</span>
            <strong>{{ agentStore.mainlineCursor?.mainline_time_anchor.display_text ?? '未初始化' }}</strong>
          </div>
          <div class="info-row">
            <span>角色数</span>
            <strong>{{ agentStore.characters.length }}</strong>
          </div>
          <div class="info-row">
            <span>运行门禁</span>
            <strong>Paused-only commit</strong>
          </div>
        </div>
        <div class="card-actions">
          <NButton size="small" type="primary" @click="openDefaultWorld">进入工作区</NButton>
          <NButton
            size="small"
            secondary
            @click="router.push({ name: 'agent-world-editor', params: { worldId } })"
          >
            World Editor
          </NButton>
        </div>
      </NCard>

      <NCard size="small" title="共享连接状态">
        <div class="info-list">
          <div class="info-row">
            <span>配置数量</span>
            <strong>{{ settingsStore.apiConfigs.length }}</strong>
          </div>
          <div class="info-row">
            <span>当前连接</span>
            <strong>{{ settingsStore.activeApiConfig?.name ?? '未选择' }}</strong>
          </div>
          <div class="info-row">
            <span>共享规则</span>
            <strong>API 配置跨模式共用</strong>
          </div>
          <div class="info-row">
            <span>共享页</span>
            <strong>日志 / 设置 / API 配置</strong>
          </div>
        </div>
        <div class="card-actions">
          <NButton size="small" secondary @click="router.push({ name: 'api-configs' })">
            <template #icon><NIcon><SettingsOutline /></NIcon></template>
            管理配置
          </NButton>
        </div>
      </NCard>
    </div>

    <div class="section-grid">
      <NCard size="small" title="最近 Agent 会话">
        <NList v-if="recentSessions.length" hoverable clickable>
          <NListItem v-for="session in recentSessions" :key="session.session_id" @click="openSession(session.session_id)">
            <div class="session-row">
              <div>
                <div class="session-name">{{ session.title }}</div>
                <div class="session-meta">
                  {{ session.period_anchor.display_text }} · {{ session.player_mode }}
                </div>
              </div>
              <NTag size="small" :type="session.status === 'Active' ? 'success' : 'default'">
                {{ session.session_kind }}
              </NTag>
            </div>
          </NListItem>
        </NList>
        <div v-else class="empty-inline">当前 World 暂无 Agent 会话</div>
      </NCard>

      <NCard size="small" title="快捷入口">
        <div class="quick-list">
          <button class="action-tile" type="button" @click="openDefaultWorld">
            <NIcon :size="22"><MapOutline /></NIcon>
            <strong>打开 World</strong>
            <span>进入当前 World 的会话、时间线和运行状态入口。</span>
          </button>
          <button class="action-tile" type="button" @click="router.push({ name: 'agent-world-editor', params: { worldId } })">
            <NIcon :size="22"><GitBranchOutline /></NIcon>
            <strong>World Editor</strong>
            <span>进入结构化编辑器与提交边界。</span>
          </button>
          <button class="action-tile" type="button" @click="router.push({ name: 'logs' })">
            <NIcon :size="22"><TimerOutline /></NIcon>
            <strong>查看日志</strong>
            <span>查看全局 Logs、World Logs 和 Agent Trace。</span>
          </button>
        </div>
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.workspace-view {
  height: 100%;
  min-height: 0;
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: var(--color-bg-app, #f0f2f5);
  overflow: auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
}

.page-header h1 {
  margin: 0 0 8px;
}

.page-header p {
  margin: 0;
  color: var(--color-text-secondary, #6b7280);
}

.section-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

.section-grid.four {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

.summary-card,
.action-tile {
  border-radius: 10px;
}

.summary-head,
.info-row,
.session-row,
.card-actions {
  display: flex;
  align-items: center;
}

.summary-head {
  gap: 8px;
  color: var(--color-text-secondary, #6b7280);
}

.summary-value {
  margin-top: 14px;
  font-size: 24px;
  font-weight: 700;
}

.info-list,
.quick-list {
  display: grid;
  gap: 12px;
}

.info-row,
.session-row,
.card-actions {
  justify-content: space-between;
  gap: 12px;
}

.card-actions {
  margin-top: 16px;
}

.quick-list {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.action-tile {
  padding: 14px;
  border: 1px solid var(--color-border-subtle, #e0e0e6);
  background: var(--color-bg-surface, #fff);
  display: grid;
  gap: 8px;
  text-align: left;
  cursor: pointer;
}

.session-name {
  font-weight: 600;
}

.session-meta,
.empty-inline,
.action-tile span,
.info-row span {
  color: var(--color-text-secondary, #6b7280);
}
</style>
