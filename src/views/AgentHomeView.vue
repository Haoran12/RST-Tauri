<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NInput,
  NButton,
  NCard,
  NEmpty,
  NIcon,
  NList,
  NListItem,
  NModal,
  NSpace,
  NTag,
  useDialog,
  useMessage,
} from 'naive-ui'
import {
  AddOutline,
  GitBranchOutline,
  KeyOutline,
  MapOutline,
  PlayOutline,
  SettingsOutline,
  TimerOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import { useAgentStore } from '@/stores/agent'
import { useSettingsStore } from '@/stores/settings'
import { useAppShellStore } from '@/stores/appShell'
import type { AgentSession } from '@/types/agent/session'
import { modalSizeStyles } from '@/composables/useModalSize'

const router = useRouter()
const agentStore = useAgentStore()
const settingsStore = useSettingsStore()
const appShell = useAppShellStore()
const message = useMessage()
const dialog = useDialog()

const worldId = computed(() => agentStore.currentWorldId)
const currentWorld = computed(() => agentStore.currentWorld)
const showCreateWorldModal = ref(false)
const newWorldName = ref('')
const isCreatingWorld = ref(false)

const summaryItems = computed(() => [
  {
    label: 'World 数量',
    value: String(agentStore.worlds.length),
    icon: MapOutline,
  },
  {
    label: '当前 World',
    value: worldId.value ?? '未选择',
    icon: GitBranchOutline,
  },
  {
    label: '活动会话',
    value: String(currentWorld.value?.active_session_count ?? 0),
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
  await settingsStore.loadApiConfigs()
  const worlds = await agentStore.loadWorldList()
  const targetWorldId = agentStore.currentWorldId ?? worlds[0]?.world_id
  if (targetWorldId) {
    await agentStore.loadWorld(targetWorldId)
  } else {
    agentStore.clearWorld()
  }
}

async function selectWorld(nextWorldId: string) {
  if (nextWorldId === agentStore.currentWorldId) return
  await agentStore.loadWorld(nextWorldId)
}

function openCreateWorldModal() {
  newWorldName.value = ''
  showCreateWorldModal.value = true
}

async function submitCreateWorld() {
  const name = newWorldName.value.trim()
  if (!name) {
    message.warning('先填写 World 名称')
    return
  }
  isCreatingWorld.value = true
  try {
    const world = await agentStore.createWorld({ name })
    showCreateWorldModal.value = false
    message.success(`已创建 World：${world.world_id}`)
    router.push({ name: 'agent-home' })
  } catch (error) {
    message.error(`创建 World 失败: ${String(error)}`)
  } finally {
    isCreatingWorld.value = false
  }
}

function openCurrentWorld() {
  if (!worldId.value) return
  router.push({ name: 'agent-sessions' })
}

function openSession(sessionId: string, sessionWorldId: string) {
  router.push({ name: 'agent-chat', params: { worldId: sessionWorldId, sessionId } })
}

function formatTime(value: string | null | undefined) {
  if (!value) return '未记录'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

function confirmDeleteAgentSession(session: AgentSession) {
  dialog.warning({
    title: '删除会话',
    content: `确定删除会话 "${session.title}"？此操作不可恢复。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await agentStore.deleteSession(session.world_id, session.session_id)
        message.success('会话已删除')
      } catch (e) {
        message.error(`删除失败: ${String(e)}`)
      }
    },
  })
}

onMounted(async () => {
  try {
    await hydrate()
  } catch (e) {
    console.error('Failed to hydrate agent home:', e)
  }
})
</script>

<template>
  <div class="workspace-view agent-home">
    <header class="page-header">
      <div>
        <h1>Agent Workspace</h1>
        <p>这里先选 World，再进入会话、时间线和编辑器工作流。</p>
      </div>
      <NSpace>
        <NButton secondary @click="router.push({ name: 'mode-select' })">切换模式</NButton>
        <NButton secondary @click="router.push({ name: 'logs' })">查看日志</NButton>
        <NButton secondary @click="openCreateWorldModal">
          <template #icon><NIcon><AddOutline /></NIcon></template>
          新建 World
        </NButton>
        <NButton type="primary" :disabled="!worldId" @click="openCurrentWorld">打开当前 World</NButton>
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
      <NCard size="small" title="World 列表 / 入口">
        <div v-if="agentStore.isWorldListLoading" class="empty-inline">加载 World 列表中...</div>
        <NList v-else-if="agentStore.worlds.length" hoverable clickable>
          <NListItem
            v-for="world in agentStore.worlds"
            :key="world.world_id"
            @click="selectWorld(world.world_id)"
          >
            <div class="world-row">
              <div class="world-main">
                <div class="world-name-line">
                  <strong>{{ world.world_id }}</strong>
                  <NTag size="small" :type="world.world_id === worldId ? 'success' : 'default'">
                    {{ world.world_id === worldId ? '当前' : '可进入' }}
                  </NTag>
                </div>
                <div class="session-meta">
                  主线 {{ world.mainline_time_anchor?.display_text ?? '未初始化' }} ·
                  会话 {{ world.session_count }} ·
                  角色 {{ world.character_count }}
                </div>
              </div>
              <NButton
                size="small"
                secondary
                @click.stop="router.push({ name: 'agent-home' })"
              >
                进入
              </NButton>
            </div>
          </NListItem>
        </NList>
        <NEmpty v-else description="还没有 Agent World">
          <template #extra>
            <NButton type="primary" @click="openCreateWorldModal">创建第一个 World</NButton>
          </template>
        </NEmpty>
      </NCard>

      <NCard size="small" title="当前 World 摘要">
        <div v-if="currentWorld" class="info-list">
          <div class="info-row">
            <span>World ID</span>
            <strong>{{ currentWorld.world_id }}</strong>
          </div>
          <div class="info-row">
            <span>主线时间</span>
            <strong>{{ currentWorld.mainline_time_anchor?.display_text ?? '未初始化' }}</strong>
          </div>
          <div class="info-row">
            <span>角色数</span>
            <strong>{{ currentWorld.character_count }}</strong>
          </div>
          <div class="info-row">
            <span>最近更新</span>
            <strong>{{ formatTime(currentWorld.updated_at) }}</strong>
          </div>
          <div class="info-row">
            <span>运行门禁</span>
            <strong>Paused-only commit</strong>
          </div>
        </div>
        <div v-else class="empty-inline">当前未选择 World</div>
        <div class="card-actions">
          <NButton size="small" type="primary" :disabled="!worldId" @click="openCurrentWorld">进入会话</NButton>
          <NButton
            size="small"
            secondary
            :disabled="!worldId"
            @click="router.push({ name: 'agent-knowledge' })"
          >
            打开编辑器
          </NButton>
        </div>
      </NCard>
    </div>

    <div class="section-grid">
      <NCard size="small" title="最近 Agent 会话">
        <NList v-if="recentSessions.length" hoverable>
          <NListItem
            v-for="session in recentSessions"
            :key="session.session_id"
          >
            <div class="session-row">
              <div>
                <div class="session-name">{{ session.title }}</div>
                <div class="session-meta">
                  {{ session.period_anchor.display_text }} · {{ session.player_mode }} · {{ formatTime(session.updated_at) }}
                </div>
              </div>
              <NSpace>
                <NButton size="small" secondary @click="openSession(session.session_id, session.world_id)">
                  打开
                </NButton>
                <NButton size="small" type="error" secondary @click="confirmDeleteAgentSession(session)">
                  <template #icon>
                    <NIcon><TrashOutline /></NIcon>
                  </template>
                  删除
                </NButton>
              </NSpace>
            </div>
          </NListItem>
        </NList>
        <div v-else class="empty-inline">当前 World 暂无 Agent 会话</div>
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
      <NCard size="small" title="快捷入口">
        <div class="quick-list">
          <button class="action-tile" type="button" :disabled="!worldId" @click="openCurrentWorld">
            <NIcon :size="22"><PlayOutline /></NIcon>
            <strong>进入会话</strong>
            <span>打开当前 World 的会话列表并运行。</span>
          </button>
          <button class="action-tile" type="button" @click="openCreateWorldModal">
            <NIcon :size="22"><AddOutline /></NIcon>
            <strong>新建 World</strong>
            <span>创建新的 Agent 世界目录与最小初始主线。</span>
          </button>
          <button
            class="action-tile"
            type="button"
            :disabled="!worldId"
            @click="router.push({ name: 'agent-knowledge' })"
          >
            <NIcon :size="22"><GitBranchOutline /></NIcon>
            <strong>打开编辑器</strong>
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

    <NModal
      v-model:show="showCreateWorldModal"
      preset="card"
      title="新建 Agent World"
      :style="modalSizeStyles.editor"
      :mask-closable="!isCreatingWorld"
    >
      <div class="modal-form">
        <p class="modal-copy">Agent 模式以 World 为顶层隔离单元。先建 World，后续会话、主线和编辑器都挂在它下面。</p>
        <NInput
          v-model:value="newWorldName"
          placeholder="例如：北境裂谷 / Demo World"
          maxlength="80"
          @keydown.enter.prevent="submitCreateWorld"
        />
      </div>
      <div class="modal-actions">
        <NButton @click="showCreateWorldModal = false">取消</NButton>
        <NButton type="primary" :loading="isCreatingWorld" @click="submitCreateWorld">创建</NButton>
      </div>
    </NModal>
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
.card-actions,
.world-row {
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
.card-actions,
.world-row {
  justify-content: space-between;
  gap: 12px;
}

.world-main {
  min-width: 0;
}

.world-name-line {
  display: flex;
  align-items: center;
  gap: 8px;
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

.action-tile:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.modal-form {
  display: grid;
  gap: 12px;
}

.modal-copy {
  margin: 0;
  color: var(--color-text-secondary, #6b7280);
  line-height: 1.6;
}

.modal-actions {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
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
