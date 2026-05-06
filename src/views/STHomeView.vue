<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NCard, NGrid, NGi, NIcon, NList, NListItem, NSpace, NTag } from 'naive-ui'
import { AlertCircleOutline, BookOutline, ChatbubbleOutline, KeyOutline, PersonOutline, SettingsOutline } from '@vicons/ionicons5'
import { useSettingsStore } from '@/stores/settings'
import { useRuntimeStore } from '@/stores/runtime'
import { useChatStore } from '@/stores/chat'
import { useCharactersStore } from '@/stores/characters'
import { useWorldbooksStore } from '@/stores/worldbooks'
import { usePresetsStore } from '@/stores/presets'
import { useAppShellStore } from '@/stores/appShell'

const router = useRouter()
const settingsStore = useSettingsStore()
const runtimeStore = useRuntimeStore()
const chatStore = useChatStore()
const charactersStore = useCharactersStore()
const worldbooksStore = useWorldbooksStore()
const presetsStore = usePresetsStore()
const appShell = useAppShellStore()

const recentSessions = computed(() =>
  [...chatStore.sessions]
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 6),
)

const activeConfig = computed(() => settingsStore.activeApiConfig)
const activePreset = computed(() => runtimeStore.globalState.active_preset || 'Default')
const resourceHighlights = computed(() => {
  const characters = charactersStore.characters.slice(0, 3).map((item, index) => ({
    id: `character-${index}`,
    label: item.character.data.name,
    type: '角色卡',
  }))
  const worldbooks = worldbooksStore.worldbookList.slice(0, 3).map((item) => ({
    id: item.id,
    label: item.name,
    type: '世界书',
  }))
  return [...characters, ...worldbooks].slice(0, 6)
})

const apiStatus = computed(() => {
  if (settingsStore.apiConfigs.length === 0) {
    return {
      tone: 'warning' as const,
      title: '未配置 API',
      detail: '请先创建至少一个 API 配置，ST 发送依赖当前连接。',
      icon: AlertCircleOutline,
    }
  }
  if (!activeConfig.value) {
    return {
      tone: 'warning' as const,
      title: '未选择当前配置',
      detail: '配置池已有内容，但当前连接未设置。',
      icon: KeyOutline,
    }
  }
  return {
    tone: activeConfig.value.enabled && activeConfig.value.api_key ? 'success' as const : 'warning' as const,
    title: activeConfig.value.name,
    detail: `${activeConfig.value.provider} · ${activeConfig.value.model}`,
    icon: KeyOutline,
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
    title: '预设',
    value: String(presetsStore.presetList.length),
    detail: `当前激活：${activePreset.value}`,
    icon: SettingsOutline,
  },
])

async function hydrate() {
  appShell.setCurrentMode('st')
  await Promise.all([
    settingsStore.loadApiConfigs(),
    runtimeStore.loadGlobalState(),
    chatStore.loadSessions(),
    charactersStore.loadCharacters(),
    worldbooksStore.loadWorldbooks(),
    presetsStore.loadPresetList(),
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

function formatTime(value: string) {
  return new Date(value).toLocaleString()
}

onMounted(async () => {
  try {
    await hydrate()
  } catch (e) {
    console.error('Failed to hydrate ST home:', e)
  }
})
</script>

<template>
  <div class="workspace-view">
    <header class="page-header">
      <div>
        <h1>ST Workspace</h1>
        <p>聚合当前连接、最近 ST 会话以及角色卡 / 世界书 / 预设入口。</p>
      </div>
      <NSpace>
        <NButton secondary @click="router.push({ name: 'mode-select' })">切换模式</NButton>
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
                <NIcon :size="18"><component :is="card.icon" /></NIcon>
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
                <NIcon :size="18"><component :is="apiStatus.icon" /></NIcon>
                <span>当前 API 配置</span>
              </div>
              <NTag :type="apiStatus.tone">{{ apiStatus.title }}</NTag>
            </div>
          </template>
          <div class="status-detail">{{ apiStatus.detail }}</div>
          <div class="status-actions">
            <NButton size="small" type="primary" @click="router.push({ name: 'api-configs' })">查看配置</NButton>
          </div>
        </NCard>

        <NCard size="small" class="status-card">
          <template #header>
            <div class="section-header">
              <div class="header-left">
                <NIcon :size="18"><SettingsOutline /></NIcon>
                <span>当前预设</span>
              </div>
              <NTag type="info">{{ activePreset }}</NTag>
            </div>
          </template>
          <div class="status-detail">
            ST 聊天发送将默认使用当前激活预设；资源页可继续编辑 sampler、prompt 与 Regex 相关配置。
          </div>
          <div class="status-actions">
            <NButton size="small" secondary @click="router.push({ name: 'resources-presets' })">打开预设</NButton>
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
            <button class="action-tile" type="button" @click="router.push({ name: 'resources-characters' })">
              <NIcon :size="22"><PersonOutline /></NIcon>
              <strong>管理角色卡</strong>
              <span>导入、编辑或绑定当前聊天角色。</span>
            </button>
            <button class="action-tile" type="button" @click="router.push({ name: 'resources-worldbooks' })">
              <NIcon :size="22"><BookOutline /></NIcon>
              <strong>管理世界书</strong>
              <span>查看 lore 资源和注入相关条目。</span>
            </button>
            <button class="action-tile" type="button" @click="router.push({ name: 'resources-presets' })">
              <NIcon :size="22"><SettingsOutline /></NIcon>
              <strong>管理预设</strong>
              <span>编辑 sampler、prompt 与系统模板。</span>
            </button>
          </div>
        </NCard>

        <NCard size="small" title="最近 ST 会话">
          <NList v-if="recentSessions.length" hoverable clickable>
            <NListItem
              v-for="session in recentSessions"
              :key="session.id"
              @click="router.push({ name: 'st-chat', params: { sessionId: session.id } })"
            >
              <div class="session-row">
                <div>
                  <div class="session-name">{{ session.name }}</div>
                  <div class="session-meta">{{ formatTime(session.updated_at) }}</div>
                </div>
                <NTag size="small" :bordered="false">ST</NTag>
              </div>
            </NListItem>
          </NList>
          <div v-else class="empty-inline">暂无 ST 会话</div>
        </NCard>
      </div>
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

.page-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.stat-card,
.status-card {
  border-radius: 10px;
}

.stat-head {
  display: flex;
  gap: 12px;
  align-items: center;
}

.stat-icon {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--n-primary-color, #2080f0) 12%, #fff);
}

.stat-title {
  font-size: 13px;
  color: var(--color-text-secondary, #6b7280);
}

.stat-value {
  font-size: 22px;
  font-weight: 700;
}

.section-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

.section-header,
.header-left,
.status-actions,
.session-row {
  display: flex;
  align-items: center;
}

.section-header,
.session-row {
  justify-content: space-between;
  gap: 10px;
}

.header-left {
  gap: 8px;
}

.status-detail {
  color: var(--color-text-secondary, #6b7280);
  line-height: 1.6;
}

.status-actions {
  margin-top: 14px;
  justify-content: flex-end;
}

.quick-actions {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.action-tile {
  padding: 16px;
  border: 1px solid var(--color-border-subtle, #e0e0e6);
  border-radius: 12px;
  background: var(--color-bg-surface, #fff);
  display: grid;
  gap: 8px;
  text-align: left;
  cursor: pointer;
}

.action-tile span,
.session-meta,
.empty-inline {
  color: var(--color-text-secondary, #6b7280);
}

.session-name {
  font-weight: 600;
}
</style>
