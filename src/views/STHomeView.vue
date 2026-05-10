<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NCard, NDivider, NForm, NFormItem, NGrid, NGi, NIcon, NInput, NModal, NPagination, NSelect, NSpace, NTag, useDialog, useMessage } from 'naive-ui'
import { AddOutline, AlertCircleOutline, BookOutline, ChatbubbleOutline, KeyOutline, PersonOutline, SettingsOutline, TrashOutline } from '@vicons/ionicons5'
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
const dialog = useDialog()
const message = useMessage()

const recentSessions = computed(() =>
  [...chatStore.sessions]
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()),
)

// Pagination & character filter for sessions
const page = ref(1)
const pageSize = ref(12)
const selectedCharacterFilter = ref('')

const characterFilterOptions = computed(() => [
  { label: '全部角色', value: '' },
  ...charactersStore.characters.map((item) => ({
    label: item.character.data.name || '未命名角色',
    value: item.id,
  })),
])

const filteredSessions = computed(() => {
  if (!selectedCharacterFilter.value) return recentSessions.value
  return recentSessions.value.filter((s) => s.character_id === selectedCharacterFilter.value)
})

const paginatedSessions = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return filteredSessions.value.slice(start, start + pageSize.value)
})

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
    route: 'st-chat',
  },
  {
    title: '角色卡',
    value: String(charactersStore.characterCount),
    detail: 'PNG / JSON 兼容角色卡共用同一资源池。',
    icon: PersonOutline,
    route: 'resources-characters',
  },
  {
    title: '世界书',
    value: String(worldbooksStore.worldbookCount),
    detail: '运行时注入使用稳定 lore id，而不是文件名分组。',
    icon: BookOutline,
    route: 'resources-worldbooks',
  },
  {
    title: '预设',
    value: String(presetsStore.presetList.length),
    detail: `当前激活：${activePreset.value}`,
    icon: SettingsOutline,
    route: 'resources-presets',
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

function confirmDeleteStSession(session: { id: string; name: string }) {
  dialog.warning({
    title: '删除会话',
    content: `确定删除会话 "${session.name}"？此操作不可恢复。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await chatStore.deleteSession(session.id)
        message.success('会话已删除')
        // Adjust page if current page becomes empty
        const totalPages = Math.ceil(filteredSessions.value.length / pageSize.value)
        if (page.value > totalPages && page.value > 1) {
          page.value = totalPages || 1
        }
      } catch (e) {
        message.error(`删除失败: ${String(e)}`)
      }
    },
  })
}

// Create session modal
const showCreateModal = ref(false)
const newSessionName = ref('')
const newSessionCharacter = ref<string | null>(null)
const newSessionWorldbooks = ref<string[]>([])

const worldbookOptions = computed(() =>
  worldbooksStore.worldbookList.map((w) => ({
    label: w.name,
    value: w.id,
  }))
)

function openCreateModal() {
  newSessionName.value = ''
  newSessionCharacter.value = null
  newSessionWorldbooks.value = []
  showCreateModal.value = true
}

async function handleCreateSession() {
  const name = newSessionName.value.trim()
  if (!name) {
    message.warning('请输入会话名')
    return
  }
  try {
    await chatStore.createSession(
      name,
      newSessionCharacter.value || undefined
    )
    const session = chatStore.currentSession
    if (session && newSessionWorldbooks.value.length > 0) {
      await chatStore.updateSessionSettings(session.id, {
        name: session.name,
        character_id: session.character_id ?? null,
        enabled_world_info: newSessionWorldbooks.value,
        user_persona: { name: '', description: '' },
      })
    }
    message.success('会话创建成功')
    showCreateModal.value = false
    await hydrate()
    if (session) {
      router.push({ name: 'st-chat', params: { sessionId: session.id } })
    }
  } catch (e) {
    message.error(`创建失败: ${String(e)}`)
  }
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
          <NCard size="small" class="stat-card" hoverable @click="router.push({ name: card.route })">
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

      <NCard size="small" title="ST 会话">
        <template #header-extra>
          <NSpace>
            <NButton size="small" type="success" @click="openCreateModal">
              <template #icon>
                <NIcon><AddOutline /></NIcon>
              </template>
              新建会话
            </NButton>
            <NSelect
              v-model:value="selectedCharacterFilter"
              :options="characterFilterOptions"
              placeholder="按角色卡筛选"
              size="small"
              style="width: 180px"
              @update:value="page = 1"
            />
          </NSpace>
        </template>

        <div v-if="paginatedSessions.length" class="session-grid">
          <NCard
            v-for="session in paginatedSessions"
            :key="session.id"
            size="small"
            class="session-card"
            hoverable
          >
            <div class="session-card-content">
              <div>
                <div class="session-name">{{ session.name }}</div>
                <div class="session-meta">{{ formatTime(session.updated_at) }}</div>
              </div>
              <NSpace>
                <NButton size="small" secondary @click="router.push({ name: 'st-chat', params: { sessionId: session.id } })">
                  打开
                </NButton>
                <NButton size="small" type="error" secondary @click="confirmDeleteStSession(session)">
                  <template #icon>
                    <NIcon><TrashOutline /></NIcon>
                  </template>
                  删除
                </NButton>
              </NSpace>
            </div>
          </NCard>
        </div>
        <div v-else class="empty-inline">暂无 ST 会话</div>

        <div v-if="filteredSessions.length > pageSize" class="session-pagination">
          <NPagination
            v-model:page="page"
            :page-size="pageSize"
            :item-count="filteredSessions.length"
            size="small"
          />
        </div>
      </NCard>
    </div>

    <!-- 新建 ST 会话弹窗 -->
    <NModal
      v-model:show="showCreateModal"
      preset="card"
      title="新建 ST 会话"
      style="width: min(480px, 90vw)"
      :mask-closable="false"
    >
      <NForm label-placement="left" label-width="80">
        <NFormItem label="会话名" required>
          <NInput
            v-model:value="newSessionName"
            placeholder="输入会话名称"
            :maxlength="100"
            show-count
            clearable
          />
        </NFormItem>

        <NDivider style="margin: 12px 0" />

        <NFormItem label="角色卡">
          <NSelect
            v-model:value="newSessionCharacter"
            :options="characterFilterOptions.filter(opt => opt.value !== '')"
            placeholder="选择关联的角色卡（可选）"
            clearable
          />
        </NFormItem>

        <NFormItem label="世界书">
          <NSelect
            v-model:value="newSessionWorldbooks"
            :options="worldbookOptions"
            placeholder="选择要启用的世界书（可选，可多选）"
            multiple
            clearable
          />
        </NFormItem>
      </NForm>

      <template #footer>
        <NSpace justify="end">
          <NButton @click="showCreateModal = false">取消</NButton>
          <NButton
            type="primary"
            :disabled="!newSessionName.trim()"
            @click="handleCreateSession"
          >
            创建会话
          </NButton>
        </NSpace>
      </template>
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

.page-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.stat-card,
.status-card {
  border-radius: 10px;
}

.stat-card {
  height: 100%;
  cursor: pointer;
  transition: box-shadow 0.2s, transform 0.2s;
}

.stat-card:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  transform: translateY(-2px);
}

.stat-card :deep(.n-card__content) {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.stat-card p {
  flex: 1;
  margin: 0;
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

.session-meta,
.empty-inline {
  color: var(--color-text-secondary, #6b7280);
}

.session-name {
  font-weight: 600;
}

.session-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 12px;
}

.session-card {
  cursor: default;
}

.session-card-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
}

.session-pagination {
  margin-top: 14px;
  display: flex;
  justify-content: flex-end;
}
</style>
