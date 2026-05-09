<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import {
  NButton,
  NCard,
  NEmpty,
  NIcon,
  NList,
  NListItem,
  NModal,
  NSpace,
  NSpin,
  NTag,
  NThing,
  useMessage,
} from 'naive-ui'
import {
  AddOutline,
  PlayOutline,
  PersonOutline,
  VideocamOutline,
} from '@vicons/ionicons5'
import SessionCreateDialog from '@/components/agent/SessionCreateDialog.vue'
import { useAgentStore } from '@/stores/agent'
import type { AgentSession, TimeAnchor } from '@/types/agent/session'
import { createTimeAnchor } from '@/types/agent/session'
import { modalSizeStyles } from '@/composables/useModalSize'

const router = useRouter()
const message = useMessage()
const agentStore = useAgentStore()

const worldId = computed(() => agentStore.currentWorldId ?? '')

const showSessionModal = ref(false)

const mainlineSessions = computed(() => agentStore.mainlineSessions)
const retrospectiveSessions = computed(() => agentStore.retrospectiveSessions)
const futurePreviewSessions = computed(() => agentStore.futurePreviewSessions)

const availableTimeAnchors = computed((): TimeAnchor[] => {
  const anchors: TimeAnchor[] = []
  const cursor = agentStore.mainlineCursor
  if (cursor) {
    anchors.push(cursor.mainline_time_anchor)
    const currentOrdinal = cursor.mainline_time_anchor.ordinal
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

async function loadData() {
  if (!worldId.value) return
  try {
    await agentStore.loadWorld(worldId.value)
  } catch (e) {
    message.error(`加载会话数据失败: ${String(e)}`)
  }
}

function openSessionModal() {
  if (!worldId.value) {
    message.warning('先选择一个 World')
    return
  }
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

watch(worldId, (newId, oldId) => {
  if (newId && newId !== oldId) {
    loadData()
  }
})

onMounted(() => {
  loadData()
})
</script>

<template>
  <div class="agent-module-view">
    <div v-if="!worldId" class="empty-world">
      <NEmpty description="请先选择一个 World" size="large">
        <template #extra>
          <p>使用顶部 World 选择器切换</p>
        </template>
      </NEmpty>
    </div>
    <div v-else class="sessions-layout">
      <NCard size="small" title="会话列表">
        <template #header-extra>
          <NButton size="small" @click="openSessionModal">
            <template #icon>
              <NIcon><AddOutline /></NIcon>
            </template>
            新建会话
          </NButton>
        </template>

        <NSpin :show="agentStore.isLoading">
          <div v-if="!agentStore.sessions.length" class="empty-inline">
            <NEmpty description="暂无会话">
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
        </NSpin>
      </NCard>
    </div>

    <!-- Session Creation Modal -->
    <NModal
      v-model:show="showSessionModal"
      preset="card"
      :style="modalSizeStyles.editor"
      :mask-closable="true"
    >
      <SessionCreateDialog
        :world-id="worldId"
        :characters="agentStore.characterOptions"
        :mainline-time-anchor="agentStore.mainlineCursor?.mainline_time_anchor || createTimeAnchor(0, '故事开始')"
        :available-time-anchors="availableTimeAnchors"
        @create="handleSessionCreate"
        @cancel="showSessionModal = false"
      />
    </NModal>
  </div>
</template>

<style scoped>
.agent-module-view {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.empty-world {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.sessions-layout {
  height: 100%;
  min-height: 0;
  overflow: auto;
  padding: 16px;
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

.empty-inline {
  padding: 24px;
}
</style>
