<script setup lang="ts">
import { computed, ref, onBeforeUnmount, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NButton,
  NCard,
  NGrid,
  NGi,
  NIcon,
  NProgress,
  NSpace,
  NTag,
  NCollapse,
  NCollapseItem,
  NEmpty,
  NModal,
  NList,
  NListItem,
  NThing,
  NAlert,
  useMessage,
} from 'naive-ui'
import {
  CreateOutline,
  GitBranchOutline,
  ShieldCheckmarkOutline,
  TimeOutline,
  SettingsOutline,
  PersonOutline,
  VideocamOutline,
  AddOutline,
  PlayOutline,
} from '@vicons/ionicons5'
import AgentLlmConfigEditor from '@/components/agent/AgentLlmConfigEditor.vue'
import SessionCreateDialog from '@/components/agent/SessionCreateDialog.vue'
import { useAgentStore } from '@/stores/agent'
import type { AgentLlmProfile } from '@/types/agent/llm-config'
import { createDefaultAgentLlmProfile } from '@/types/agent/llm-config'
import type { AgentSession, TimeAnchor } from '@/types/agent/session'
import { createTimeAnchor } from '@/types/agent/session'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const agentStore = useAgentStore()

const worldId = computed(() => {
  const routeWorldId = route.params.worldId
  if (typeof routeWorldId === 'string' && routeWorldId.length > 0) return routeWorldId
  return agentStore.currentWorldId ?? ''
})

// Agent LLM Profile state
const llmProfile = ref<AgentLlmProfile | null>(null)

// Session creation modal
const showSessionModal = ref(false)

// Computed
const mainlineCursor = computed(() => agentStore.mainlineCursor)
const mainlineSessions = computed(() => agentStore.mainlineSessions)
const retrospectiveSessions = computed(() => agentStore.retrospectiveSessions)
const futurePreviewSessions = computed(() => agentStore.futurePreviewSessions)
const characterOptions = computed(() => agentStore.characterOptions)

// Default time anchors for selection
const availableTimeAnchors = computed((): TimeAnchor[] => {
  const anchors: TimeAnchor[] = []
  if (mainlineCursor.value) {
    anchors.push(mainlineCursor.value.mainline_time_anchor)
    // Add some example past anchors
    const currentOrdinal = mainlineCursor.value.mainline_time_anchor.ordinal
    if (currentOrdinal > 100) {
      anchors.push(createTimeAnchor(currentOrdinal - 100, '一周前'))
    }
    if (currentOrdinal > 500) {
      anchors.push(createTimeAnchor(currentOrdinal - 500, '一个月前'))
    }
    if (currentOrdinal > 1000) {
      anchors.push(createTimeAnchor(currentOrdinal - 1000, '三个月前'))
    }
  }
  return anchors
})

const statusCards = computed(() => [
  {
    title: '主线光标',
    value: mainlineCursor.value?.mainline_time_anchor.display_text || '未初始化',
    detail: `当前主线时间点，用于判断过去线和未来预演。`,
    tone: 'success',
  },
  {
    title: '运行门禁',
    value: 'Paused-only',
    detail: 'World Editor 只允许在无 active turn 与 pending LLM call 时提交。',
    tone: 'info',
  },
  {
    title: '会话数量',
    value: `${agentStore.sessions.length} 个`,
    detail: `主线 ${mainlineSessions.value.length} / 过去线 ${retrospectiveSessions.value.length} / 未来 ${futurePreviewSessions.value.length}`,
    tone: 'warning',
  },
])

const domains = [
  { label: 'LocationGraph', progress: 72 },
  { label: 'KnowledgeEntry', progress: 66 },
  { label: 'CharacterRecord', progress: 62 },
  { label: 'Validation', progress: 58 },
]

const agentRunStatus = computed(() => ({
  type: 'info' as const,
  title: 'Agent 回合运行链路已开放',
  description: '当前入口使用后端 AgentRuntime 提交回合并写入 session_turns / world_turns / Trace；部分认知与技能规则仍在补强中。',
}))

// Methods
function loadLlmProfile() {
  // TODO: Load from backend
  llmProfile.value = createDefaultAgentLlmProfile(worldId.value)
}

function handleProfileUpdate(profile: AgentLlmProfile) {
  llmProfile.value = profile
}

async function handleProfileSave() {
  // TODO: Save to backend
  console.log('Save LLM profile:', llmProfile.value)
}

function openSessionModal() {
  showSessionModal.value = true
}

async function handleSessionCreate(session: AgentSession) {
  try {
    await agentStore.createSession({
      world_id: session.world_id,
      title: session.title,
      player_mode: session.player_mode,
      player_character_id: session.player_character_id,
      period_anchor: session.period_anchor,
    })
    message.success(`会话 "${session.title}" 创建成功`)
    showSessionModal.value = false
  } catch (e) {
    message.error(`创建会话失败: ${e}`)
  }
}

function handleSessionCancel() {
  showSessionModal.value = false
}

function enterSession(session: AgentSession) {
  router.push({
    name: 'agent-chat',
    params: {
      worldId: session.world_id,
      sessionId: session.session_id,
    },
  })
}

function getPlayerModeLabel(mode: string) {
  switch (mode) {
    case 'Character':
      return { text: '扮演角色', icon: PersonOutline }
    case 'Director':
      return { text: '导演模式', icon: VideocamOutline }
    default:
      return { text: mode, icon: PersonOutline }
  }
}

// Watch worldId changes
watch(worldId, (newWorldId) => {
  if (newWorldId) {
    agentStore.loadWorld(newWorldId)
  }
}, { immediate: true })

onMounted(() => {
  window.addEventListener('open-agent-session-create', openSessionModal)
  loadLlmProfile()
})

onBeforeUnmount(() => {
  window.removeEventListener('open-agent-session-create', openSessionModal)
})
</script>

<template>
  <div class="agent-world-view">
    <header class="page-header">
      <div>
        <h1>Agent 工作区</h1>
        <div class="world-id">{{ worldId }}</div>
      </div>
      <NSpace>
        <NButton type="primary" @click="openSessionModal">
          <template #icon>
            <NIcon><AddOutline /></NIcon>
          </template>
          创建会话
        </NButton>
        <NButton secondary @click="router.push({ name: 'agent-world-editor', params: { worldId } })">
          <template #icon>
            <NIcon><CreateOutline /></NIcon>
          </template>
          World Editor
        </NButton>
      </NSpace>
    </header>

    <section class="content">
      <!-- Status Cards -->
      <NGrid :cols="3" :x-gap="12" :y-gap="12" responsive="screen">
        <NGi v-for="card in statusCards" :key="card.title">
          <NCard size="small" class="status-card">
            <div class="card-head">
              <span>{{ card.title }}</span>
              <NTag size="small" :type="card.tone as any">{{ card.value }}</NTag>
            </div>
            <p>{{ card.detail }}</p>
          </NCard>
        </NGi>
      </NGrid>

      <!-- Session List Section -->
      <NCard size="small" title="会话列表" style="margin-top: 12px">
        <template #header-extra>
          <NButton size="small" @click="openSessionModal">
            <template #icon>
              <NIcon><AddOutline /></NIcon>
            </template>
            新建
          </NButton>
        </template>

        <div v-if="agentStore.isLoading" class="loading-state">
          <NProgress type="line" :percentage="50" :show-indicator="false" status="info" />
          <span>加载中...</span>
        </div>

        <div v-else-if="agentStore.sessions.length === 0">
          <NEmpty description="暂无会话，点击上方按钮创建">
            <template #extra>
              <NButton size="small" @click="openSessionModal">创建第一个会话</NButton>
            </template>
          </NEmpty>
        </div>

        <div v-else class="session-groups">
          <!-- Mainline Sessions -->
          <div v-if="mainlineSessions.length > 0" class="session-group">
            <div class="group-header">
              <NTag type="success" size="small">主线</NTag>
              <span class="group-count">{{ mainlineSessions.length }} 个会话</span>
            </div>
            <NList>
              <NListItem v-for="session in mainlineSessions" :key="session.session_id">
                <NThing :title="session.title">
                  <template #header-extra>
                    <NButton size="tiny" secondary @click="enterSession(session)">
                      <template #icon>
                        <NIcon><PlayOutline /></NIcon>
                      </template>
                      进入
                    </NButton>
                  </template>
                  <template #description>
                    <NSpace :size="8">
                      <NTag size="tiny">
                        <template #icon>
                          <NIcon :size="12">
                            <component :is="getPlayerModeLabel(session.player_mode).icon" />
                          </NIcon>
                        </template>
                        {{ getPlayerModeLabel(session.player_mode).text }}
                      </NTag>
                      <span class="time-text">{{ session.period_anchor.display_text }}</span>
                    </NSpace>
                  </template>
                </NThing>
              </NListItem>
            </NList>
          </div>

          <!-- Retrospective Sessions -->
          <div v-if="retrospectiveSessions.length > 0" class="session-group">
            <div class="group-header">
              <NTag type="warning" size="small">过去线</NTag>
              <span class="group-count">{{ retrospectiveSessions.length }} 个会话</span>
            </div>
            <NList>
              <NListItem v-for="session in retrospectiveSessions" :key="session.session_id">
                <NThing :title="session.title">
                  <template #header-extra>
                    <NButton size="tiny" secondary @click="enterSession(session)">
                      <template #icon>
                        <NIcon><PlayOutline /></NIcon>
                      </template>
                      进入
                    </NButton>
                  </template>
                  <template #description>
                    <NSpace :size="8">
                      <NTag size="tiny">
                        <template #icon>
                          <NIcon :size="12">
                            <component :is="getPlayerModeLabel(session.player_mode).icon" />
                          </NIcon>
                        </template>
                        {{ getPlayerModeLabel(session.player_mode).text }}
                      </NTag>
                      <span class="time-text">{{ session.period_anchor.display_text }}</span>
                      <NTag v-if="session.canon_status !== 'CanonCandidate'" size="tiny" type="warning">
                        {{ session.canon_status }}
                      </NTag>
                    </NSpace>
                  </template>
                </NThing>
              </NListItem>
            </NList>
          </div>

          <!-- Future Preview Sessions -->
          <div v-if="futurePreviewSessions.length > 0" class="session-group">
            <div class="group-header">
              <NTag type="info" size="small">未来预演</NTag>
              <span class="group-count">{{ futurePreviewSessions.length }} 个会话</span>
            </div>
            <NList>
              <NListItem v-for="session in futurePreviewSessions" :key="session.session_id">
                <NThing :title="session.title">
                  <template #header-extra>
                    <NButton size="tiny" secondary @click="enterSession(session)">
                      <template #icon>
                        <NIcon><PlayOutline /></NIcon>
                      </template>
                      进入
                    </NButton>
                  </template>
                  <template #description>
                    <NSpace :size="8">
                      <NTag size="tiny">
                        <template #icon>
                          <NIcon :size="12">
                            <component :is="getPlayerModeLabel(session.player_mode).icon" />
                          </NIcon>
                        </template>
                        {{ getPlayerModeLabel(session.player_mode).text }}
                      </NTag>
                      <span class="time-text">{{ session.period_anchor.display_text }}</span>
                      <NTag size="tiny" type="default">非正史</NTag>
                    </NSpace>
                  </template>
                </NThing>
              </NListItem>
            </NList>
          </div>
        </div>
      </NCard>

      <NAlert
        style="margin-top: 12px"
        :type="agentRunStatus.type"
        :title="agentRunStatus.title"
      >
        {{ agentRunStatus.description }}
      </NAlert>

      <!-- Domain Progress and Boundaries -->
      <div class="panel-grid">
        <NCard size="small" title="核心数据域">
          <div class="domain-list">
            <div v-for="domain in domains" :key="domain.label" class="domain-row">
              <span>{{ domain.label }}</span>
              <NProgress
                type="line"
                :percentage="domain.progress"
                :height="6"
                :show-indicator="false"
              />
            </div>
          </div>
        </NCard>

        <NCard size="small" title="运行边界">
          <div class="boundary-list">
            <div class="boundary-item">
              <NIcon><ShieldCheckmarkOutline /></NIcon>
              <span>LLM 节点只产出结构化建议，确定性状态写入由程序校验后执行。</span>
            </div>
            <div class="boundary-item">
              <NIcon><GitBranchOutline /></NIcon>
              <span>主线时间、回顾线和未来预览必须通过 TimeAnchor ordinal 比较。</span>
            </div>
            <div class="boundary-item">
              <NIcon><TimeOutline /></NIcon>
              <span>提交、回滚、日志写入应保持事务一致性。</span>
            </div>
          </div>
        </NCard>
      </div>

      <!-- LLM Config Section -->
      <div class="config-section">
        <NCollapse>
          <NCollapseItem name="llm-config">
            <template #header>
              <div class="collapse-header">
                <NIcon :size="18"><SettingsOutline /></NIcon>
                <span>Agent LLM 节点配置</span>
              </div>
            </template>
            <AgentLlmConfigEditor
              :world-id="worldId"
              :profile="llmProfile"
              @update:profile="handleProfileUpdate"
              @save="handleProfileSave"
            />
          </NCollapseItem>
        </NCollapse>
      </div>
    </section>

    <!-- Session Creation Modal -->
    <NModal
      v-model:show="showSessionModal"
      preset="card"
      style="width: 600px; max-width: 90vw"
      :mask-closable="true"
    >
      <SessionCreateDialog
        :world-id="worldId"
        :characters="characterOptions"
        :mainline-time-anchor="mainlineCursor?.mainline_time_anchor || createTimeAnchor(0, '故事开始')"
        :available-time-anchors="availableTimeAnchors"
        @create="handleSessionCreate"
        @cancel="handleSessionCancel"
      />
    </NModal>
  </div>
</template>

<style scoped>
.agent-world-view {
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-app, #f0f2f5);
}

.page-header {
  padding: 18px 24px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
  background: var(--color-bg-surface, #fff);
}

h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.world-id {
  margin-top: 4px;
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
}

.content {
  flex: 1;
  min-height: 0;
  min-width: 0;
  overflow: auto;
  padding: 18px 24px;
  scrollbar-width: thin;
  scrollbar-gutter: stable;
}

.content::-webkit-scrollbar {
  width: 8px;
}

.content::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.05);
  border-radius: 4px;
}

.content::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.5);
  border-radius: 4px;
  min-height: 30px;
}

.content::-webkit-scrollbar-thumb:hover {
  background: rgba(128, 128, 128, 0.7);
}

.status-card {
  min-height: 116px;
}

.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-wrap: wrap;
  font-weight: 600;
}

.status-card p {
  margin: 12px 0 0;
  color: var(--color-text-secondary, #6b7280);
  line-height: 1.5;
}

.loading-state {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: center;
  padding: 24px;
}

.session-groups {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.session-group {
  border-left: 3px solid var(--n-border-color);
  padding-left: 12px;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.group-count {
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
}

.time-text {
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
}

.panel-grid {
  display: grid;
  grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.15fr);
  gap: 12px;
  margin-top: 12px;
}

.domain-list {
  display: grid;
  gap: 14px;
}

.domain-row {
  display: grid;
  grid-template-columns: minmax(96px, 130px) minmax(0, 1fr);
  align-items: center;
  gap: 12px;
  font-size: 13px;
}

.boundary-list {
  display: grid;
  gap: 12px;
}

.boundary-item {
  display: grid;
  grid-template-columns: 20px 1fr;
  gap: 8px;
  align-items: start;
  color: var(--color-text-primary, #344054);
  line-height: 1.55;
}

@media (max-width: 900px) {
  .panel-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 640px) {
  .page-header,
  .content {
    padding-left: 14px;
    padding-right: 14px;
  }
}

.config-section {
  margin-top: 16px;
}

.collapse-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
}
</style>
