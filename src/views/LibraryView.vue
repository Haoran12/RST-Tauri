<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import {
  NButton,
  NCard,
  NEmpty,
  NGrid,
  NGi,
  NIcon,
  NList,
  NListItem,
  NSpace,
  NTag,
} from 'naive-ui'
import {
  AlertCircleOutline,
  BookOutline,
  ChatbubbleOutline,
  KeyOutline,
  LayersOutline,
  MapOutline,
  PersonOutline,
  SparklesOutline,
} from '@vicons/ionicons5'
import { useSettingsStore } from '@/stores/settings'
import { useRuntimeStore } from '@/stores/runtime'
import { useChatStore } from '@/stores/chat'
import { useCharactersStore } from '@/stores/characters'
import { useWorldbooksStore } from '@/stores/worldbooks'
import { useAppShellStore } from '@/stores/appShell'

const router = useRouter()
const settingsStore = useSettingsStore()
const runtimeStore = useRuntimeStore()
const chatStore = useChatStore()
const charactersStore = useCharactersStore()
const worldbooksStore = useWorldbooksStore()
const appShell = useAppShellStore()

const recentSessions = computed(() =>
  [...chatStore.sessions]
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 6),
)

const resourceHighlights = computed(() => {
  const characters = charactersStore.characters.slice(0, 3).map((item, index) => ({
    id: `character-${index}`,
    label: item.character.data.name,
    type: '角色卡',
    action: () => router.push({ name: 'resources-characters' }),
  }))
  const worldbooks = worldbooksStore.worldbookList.slice(0, 3).map((item) => ({
    id: item.id,
    label: item.name,
    type: '世界书',
    action: () => router.push({ name: 'resources-worldbooks' }),
  }))
  return [...characters, ...worldbooks].slice(0, 6)
})

const activeConfig = computed(() => settingsStore.activeApiConfig)

const apiStatus = computed(() => {
  if (settingsStore.apiConfigs.length === 0) {
    return {
      tone: 'warning' as const,
      title: '未配置 API',
      detail: '请先创建至少一个 API 配置，ST 发送与 Agent 节点绑定都依赖它。',
      cta: '添加配置',
      icon: AlertCircleOutline,
    }
  }
  if (!activeConfig.value) {
    return {
      tone: 'warning' as const,
      title: '未选择当前配置',
      detail: '配置池已有内容，但当前 `active_api_config_id` 为空，后续请求无法直接发送。',
      cta: '选择当前配置',
      icon: KeyOutline,
    }
  }
  if (!activeConfig.value.enabled) {
    return {
      tone: 'warning' as const,
      title: '当前配置已禁用',
      detail: `${activeConfig.value.name} 已存在，但标记为 disabled，建议切换到可用连接。`,
      cta: '管理配置',
      icon: AlertCircleOutline,
    }
  }
  if (!activeConfig.value.api_key) {
    return {
      tone: 'warning' as const,
      title: '当前配置缺少 Key',
      detail: `${activeConfig.value.name} 已设为当前连接，但尚未填写 API Key。`,
      cta: '补充配置',
      icon: AlertCircleOutline,
    }
  }
  return {
    tone: 'success' as const,
    title: activeConfig.value.name,
    detail: `${activeConfig.value.provider} · ${activeConfig.value.model}`,
    cta: '查看配置',
    icon: SparklesOutline,
  }
})

const statCards = computed(() => [
  {
    title: 'ST 会话',
    value: String(chatStore.sessions.length),
    detail: '最近聊天记录与附件引用保存在本地数据目录。',
    icon: ChatbubbleOutline,
  },
  {
    title: '角色卡',
    value: String(charactersStore.characterCount),
    detail: 'PNG / JSON 兼容角色卡共用同一资源池。',
    icon: PersonOutline,
  },
  {
    title: '世界书',
    value: String(worldbooksStore.worldbookCount),
    detail: '运行时注入使用稳定 lore id，而不是文件名分组。',
    icon: BookOutline,
  },
  {
    title: 'API 配置',
    value: String(settingsStore.apiConfigs.length),
    detail: activeConfig.value ? `当前：${activeConfig.value.name}` : '尚未选择当前配置',
    icon: KeyOutline,
  },
])

function formatTime(value: string) {
  return new Date(value).toLocaleString()
}

async function hydrate() {
  await Promise.all([
    settingsStore.loadApiConfigs(),
    runtimeStore.loadGlobalState(),
    chatStore.loadSessions(),
    charactersStore.loadCharacters(),
    worldbooksStore.loadWorldbooks(),
  ])
  settingsStore.setActiveApiConfig(runtimeStore.activeApiConfigId)
  appShell.setRecentSessions(
    recentSessions.value.map((session) => ({
      id: session.id,
      type: 'st',
      name: session.name,
      updatedAt: session.updated_at,
    })),
  )
  appShell.setRecentResources(
    resourceHighlights.value.map((resource) => ({
      id: resource.id,
      type: resource.type,
      name: resource.label,
      updatedAt: '',
    })),
  )
}

onMounted(() => {
  void hydrate()
})
</script>

<template>
  <div class="library-view">
    <header class="page-header">
      <div>
        <h1>资源工作台</h1>
        <p>默认首页聚合当前连接状态、最近 ST 会话和常用资源入口；不在这里直接做破坏性提交。</p>
      </div>
      <NSpace>
        <NButton secondary @click="router.push({ name: 'api-configs' })">管理 API</NButton>
        <NButton type="primary" @click="router.push({ name: 'st-chat' })">进入 ST 聊天</NButton>
      </NSpace>
    </header>

    <div class="page-content">
      <NGrid :cols="4" :x-gap="14" :y-gap="14" responsive="screen">
        <NGi v-for="card in statCards" :key="card.title">
          <NCard size="small" class="stat-card">
            <div class="stat-head">
              <div class="stat-icon">
                <NIcon :size="18">
                  <component :is="card.icon" />
                </NIcon>
              </div>
              <div>
                <div class="stat-title">{{ card.title }}</div>
                <div class="stat-value">{{ card.value }}</div>
              </div>
            </div>
            <p>{{ card.detail }}</p>
          </NCard>
        </NGi>
      </NGrid>

      <div class="section-grid">
        <NCard size="small" class="status-card">
          <template #header>
            <div class="section-header">
              <div class="header-left">
                <NIcon :size="18">
                  <component :is="apiStatus.icon" />
                </NIcon>
                <span>当前 API 配置</span>
              </div>
              <NTag :type="apiStatus.tone">{{ apiStatus.title }}</NTag>
            </div>
          </template>
          <div class="status-detail">{{ apiStatus.detail }}</div>
          <div class="status-actions">
            <NButton size="small" type="primary" @click="router.push({ name: 'api-configs' })">
              {{ apiStatus.cta }}
            </NButton>
          </div>
        </NCard>

        <NCard size="small" class="status-card">
          <template #header>
            <div class="section-header">
              <div class="header-left">
                <NIcon :size="18"><MapOutline /></NIcon>
                <span>Agent 入口</span>
              </div>
              <NTag type="info">工作区</NTag>
            </div>
          </template>
          <div class="status-detail">
            当前前端已经有 Agent 工作区、Session Launcher 和 World Editor 入口；跨 World 容量摘要后续继续接入日志 / Trace 视图。
          </div>
          <div class="status-actions">
            <NButton size="small" secondary @click="router.push({ name: 'agent-worlds' })">
              打开 Agent 工作区
            </NButton>
          </div>
        </NCard>
      </div>

      <div class="section-grid">
        <NCard size="small" title="快捷入口">
          <div class="quick-actions">
            <button class="action-tile" type="button" @click="router.push({ name: 'st-chat' })">
              <NIcon :size="22"><ChatbubbleOutline /></NIcon>
              <strong>新建 ST 会话</strong>
              <span>保持当前资源与预设选择，直接进入聊天工作流。</span>
            </button>
            <button class="action-tile" type="button" @click="router.push({ name: 'agent-worlds' })">
              <NIcon :size="22"><LayersOutline /></NIcon>
              <strong>打开 Agent World</strong>
              <span>进入多时期会话与 World Editor 工作区。</span>
            </button>
            <button class="action-tile" type="button" @click="router.push({ name: 'resources-characters' })">
              <NIcon :size="22"><PersonOutline /></NIcon>
              <strong>管理角色卡</strong>
              <span>导入 PNG / JSON，维护角色与世界书绑定。</span>
            </button>
            <button class="action-tile" type="button" @click="router.push({ name: 'resources-worldbooks' })">
              <NIcon :size="22"><BookOutline /></NIcon>
              <strong>管理世界书</strong>
              <span>编辑词条、结构化正文与注入来源。</span>
            </button>
          </div>
        </NCard>

        <NCard size="small" title="最近 ST 会话">
          <NEmpty v-if="recentSessions.length === 0" description="还没有 ST 会话" />
          <NList v-else hoverable clickable>
            <NListItem
              v-for="session in recentSessions"
              :key="session.id"
              class="recent-row"
              @click="router.push({ name: 'st-chat', params: { sessionId: session.id } })"
            >
              <div>
                <div class="recent-name">{{ session.name }}</div>
                <div class="recent-meta">{{ formatTime(session.updated_at) }}</div>
              </div>
              <NTag size="small" type="default">{{ session.messages.length }} 条消息</NTag>
            </NListItem>
          </NList>
        </NCard>
      </div>

      <NCard size="small" title="资源摘要">
        <NEmpty v-if="resourceHighlights.length === 0" description="角色卡和世界书资源还为空" />
        <div v-else class="resource-grid">
          <button
            v-for="resource in resourceHighlights"
            :key="resource.id"
            class="resource-chip"
            type="button"
            @click="resource.action()"
          >
            <span class="resource-type">{{ resource.type }}</span>
            <strong>{{ resource.label }}</strong>
          </button>
        </div>
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.library-view {
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background:
    radial-gradient(circle at top right, rgba(24, 160, 88, 0.08), transparent 28%),
    var(--color-bg-app, #f0f2f5);
}

.page-header {
  padding: 22px 24px 18px;
  border-bottom: 1px solid var(--color-border-subtle, rgba(15, 23, 42, 0.08));
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 18px;
  flex-wrap: wrap;
  background: var(--color-bg-surface, rgba(255, 255, 255, 0.82));
  backdrop-filter: blur(18px);
}

.page-header h1 {
  margin: 0;
  font-size: 24px;
}

.page-header p {
  margin: 6px 0 0;
  max-width: 760px;
  color: var(--color-text-secondary, #526071);
}

.page-content {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: auto;
  padding: 18px 20px 20px;
  display: grid;
  align-content: start;
  gap: 16px;
  scrollbar-width: thin;
}

.page-content::-webkit-scrollbar {
  width: 8px;
}

.page-content::-webkit-scrollbar-track {
  background: transparent;
}

.page-content::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 4px;
}

.page-content::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

.stat-card {
  min-height: 132px;
  border-radius: 8px;
  box-shadow: 0 18px 44px rgba(15, 23, 42, 0.08);
}

.stat-head {
  display: flex;
  gap: 12px;
  align-items: center;
}

.stat-icon {
  width: 40px;
  height: 40px;
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(32, 128, 240, 0.08);
  color: var(--color-status-info, #2080f0);
}

.stat-title {
  font-size: 12px;
  color: var(--color-text-secondary, #667085);
}

.stat-value {
  margin-top: 2px;
  font-size: 22px;
  font-weight: 700;
  color: var(--color-text-primary, #1f2937);
}

.stat-card p {
  margin-top: 14px;
  color: var(--color-text-secondary, #526071);
}

.section-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.status-card {
  border-radius: 8px;
  box-shadow: 0 18px 44px rgba(15, 23, 42, 0.08);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--color-text-primary, #1f2937);
}

.header-left span {
  color: var(--color-text-primary, #1f2937);
}

.status-detail {
  color: var(--color-text-secondary, #526071);
  line-height: 1.6;
}

.status-actions {
  margin-top: 14px;
}

.quick-actions {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.action-tile {
  border: 1px solid var(--color-border-subtle, rgba(15, 23, 42, 0.08));
  border-radius: 8px;
  background: var(--color-bg-surface, #fff);
  padding: 18px;
  text-align: left;
  display: grid;
  gap: 8px;
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
  color: var(--color-text-primary, #1f2937);
}

.action-tile:hover {
  transform: translateY(-1px);
  box-shadow: 0 14px 28px rgba(15, 23, 42, 0.1);
}

.action-tile span {
  color: var(--color-text-secondary, #526071);
}

.action-tile strong {
  color: var(--color-text-primary, #1f2937);
}

.recent-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
}

.recent-name {
  font-weight: 600;
  color: var(--color-text-primary, #1f2937);
}

.recent-meta {
  margin-top: 4px;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.resource-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.resource-chip {
  border: 1px solid var(--color-border-subtle, rgba(15, 23, 42, 0.08));
  border-radius: 8px;
  background: var(--color-bg-subtle, rgba(255, 255, 255, 0.88));
  padding: 14px;
  display: grid;
  gap: 6px;
  text-align: left;
  cursor: pointer;
}

.resource-chip strong {
  color: var(--color-text-primary, #1f2937);
}

.resource-type {
  font-size: 12px;
  color: var(--color-text-secondary);
}

@media (max-width: 980px) {
  .section-grid,
  .quick-actions,
  .resource-grid {
    grid-template-columns: 1fr;
  }

  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
