<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { NButton, NCard, NEmpty, NIcon, NTag } from 'naive-ui'
import { CloseOutline, KeyOutline, LayersOutline, SparklesOutline } from '@vicons/ionicons5'
import { useAppShellStore } from '@/stores/appShell'
import { useChatStore } from '@/stores/chat'
import { useAgentStore } from '@/stores/agent'
import { useSettingsStore } from '@/stores/settings'
import { useWorldbooksStore } from '@/stores/worldbooks'

const appShell = useAppShellStore()
const route = useRoute()
const chatStore = useChatStore()
const agentStore = useAgentStore()
const settingsStore = useSettingsStore()
const worldbooksStore = useWorldbooksStore()

const stSummaryItems = computed(() => {
  const worldbookCount = chatStore.currentSession?.chat_metadata?.disabled_world_info?.length ?? 0
  return [
    {
      label: '当前会话',
      value: chatStore.currentSession?.name ?? '未选择',
    },
    {
      label: '消息数量',
      value: String(chatStore.messages.length),
    },
    {
      label: '当前连接',
      value: settingsStore.activeApiConfig?.name ?? '未选择',
    },
    {
      label: '关闭的世界书',
      value: `${worldbookCount} 个`,
    },
  ]
})

const agentSummaryItems = computed(() => [
  {
    label: '当前 World',
    value: agentStore.currentWorldId ?? '未选择',
  },
  {
    label: '主线时间',
    value: agentStore.mainlineCursor?.mainline_time_anchor.display_text ?? '未加载',
  },
  {
    label: '会话数量',
    value: String(agentStore.sessions.length),
  },
  {
    label: '角色数量',
    value: String(agentStore.characters.length),
  },
])

const apiSummaryItems = computed(() => [
  {
    label: '激活配置',
    value: settingsStore.activeApiConfig?.name ?? '未选择',
  },
  {
    label: 'Provider',
    value: settingsStore.activeApiConfig?.provider ?? '-',
  },
  {
    label: '模型',
    value: settingsStore.activeApiConfig?.model ?? '-',
  },
  {
    label: '配置数量',
    value: String(settingsStore.apiConfigs.length),
  },
])

const panelTitle = computed(() => {
  switch (route.name) {
    case 'st-chat':
      return 'ST 摘要'
    case 'agent-worlds':
      return 'Agent 摘要'
    case 'agent-world-editor':
      return 'Editor 摘要'
    case 'logs':
      return '日志规划'
    default:
      return '检查面板'
  }
})

const currentSummary = computed(() => {
  switch (route.name) {
    case 'st-chat':
      return {
        icon: SparklesOutline,
        tone: 'info' as const,
        tip: '切换当前 API 配置只影响下一次请求的连接与 Provider 映射，不会改写会话世界书绑定。',
        items: stSummaryItems.value,
      }
    case 'agent-worlds':
      return {
        icon: LayersOutline,
        tone: 'info' as const,
        tip: 'Agent 工作区只展示入口和只读摘要；结构化 Truth 编辑仍需进入 World Editor。',
        items: agentSummaryItems.value,
      }
    case 'agent-world-editor':
      return {
        icon: LayersOutline,
        tone: 'warning' as const,
        tip: 'World Editor 提交遵守 paused-only 边界；右侧只读摘要不参与 validation 或提交。',
        items: agentSummaryItems.value,
      }
    case 'logs':
      return {
        icon: KeyOutline,
        tone: 'default' as const,
        tip: '日志页后续会拆分全局 Logs、World Logs 与 Agent Trace，并提供 request / trace 双向跳转。',
        items: apiSummaryItems.value,
      }
    default:
      return null
  }
})
</script>

<template>
  <div class="inspect-panel">
    <div class="panel-header">
      <span class="panel-title">{{ panelTitle }}</span>
      <NButton quaternary size="small" @click="appShell.toggleInspectPanel">
        <template #icon>
          <NIcon><CloseOutline /></NIcon>
        </template>
      </NButton>
    </div>

    <div class="panel-content">
      <NEmpty v-if="!currentSummary" description="当前页面没有可展示的只读摘要" />

      <template v-else>
        <NCard size="small" class="summary-card">
          <div class="card-header">
            <div class="header-main">
              <NIcon :size="18">
                <component :is="currentSummary.icon" />
              </NIcon>
              <span>{{ panelTitle }}</span>
            </div>
            <NTag :type="currentSummary.tone">只读</NTag>
          </div>
          <p class="tip-text">{{ currentSummary.tip }}</p>
          <div class="summary-list">
            <div
              v-for="item in currentSummary.items"
              :key="item.label"
              class="summary-row"
            >
              <span>{{ item.label }}</span>
              <strong>{{ item.value }}</strong>
            </div>
          </div>
        </NCard>

        <NCard
          v-if="route.name === 'agent-world-editor'"
          size="small"
          title="结构边界"
          class="summary-card"
        >
          <div class="summary-list">
            <div class="summary-row">
              <span>Truth 编辑</span>
              <strong>World Editor</strong>
            </div>
            <div class="summary-row">
              <span>运行时提交</span>
              <strong>StateCommitter</strong>
            </div>
            <div class="summary-row">
              <span>只读摘要</span>
              <strong>InspectPanel</strong>
            </div>
          </div>
        </NCard>

        <NCard
          v-if="route.name === 'st-chat'"
          size="small"
          title="绑定检查"
          class="summary-card"
        >
          <div class="summary-list">
            <div class="summary-row">
              <span>当前角色</span>
              <strong>{{ chatStore.currentCharacter?.data.name ?? '未绑定' }}</strong>
            </div>
            <div class="summary-row">
              <span>世界书索引</span>
              <strong>{{ worldbooksStore.worldbookCount }} 本</strong>
            </div>
            <div class="summary-row">
              <span>待发送附件</span>
              <strong>{{ chatStore.pendingAttachments.length }} 个</strong>
            </div>
          </div>
        </NCard>
      </template>
    </div>
  </div>
</template>

<style scoped>
.inspect-panel {
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-surface, #fff);
}

.panel-header {
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
}

.panel-title {
  font-weight: 600;
  font-size: 14px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.panel-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 12px;
  display: grid;
  align-content: start;
  gap: 12px;
  scrollbar-width: thin;
  scrollbar-gutter: stable;
}

.panel-content::-webkit-scrollbar {
  width: 6px;
}

.panel-content::-webkit-scrollbar-track {
  background: transparent;
}

.panel-content::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 3px;
}

.panel-content::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

.summary-card {
  border-radius: 8px;
  min-width: 0;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.header-main {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
}

.tip-text {
  margin: 12px 0 0;
  color: var(--color-text-secondary, #6b7280);
  line-height: 1.6;
}

.summary-list {
  display: grid;
  gap: 10px;
  margin-top: 14px;
}

.summary-row {
  display: grid;
  grid-template-columns: minmax(72px, 96px) minmax(0, 1fr);
  gap: 10px;
  align-items: start;
}

.summary-row span {
  color: var(--color-text-secondary, #6b7280);
}

.summary-row strong {
  word-break: break-word;
}
</style>
