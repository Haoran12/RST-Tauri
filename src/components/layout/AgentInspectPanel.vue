<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { NButton, NCard, NEmpty, NIcon, NTag } from 'naive-ui'
import { CloseOutline, LayersOutline } from '@vicons/ionicons5'
import { useAppShellStore } from '@/stores/appShell'
import { useAgentStore } from '@/stores/agent'
import { useSettingsStore } from '@/stores/settings'

const appShell = useAppShellStore()
const route = useRoute()
const agentStore = useAgentStore()
const settingsStore = useSettingsStore()

const panelTitle = computed(() => {
  switch (route.name) {
    case 'agent-worlds':
      return 'Agent 摘要'
    case 'agent-world-editor':
      return 'Editor 摘要'
    default:
      return 'Agent 面板'
  }
})

const summaryItems = computed(() => [
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

const tipText = computed(() => {
  if (route.name === 'agent-world-editor') {
    return 'World Editor 提交遵守 paused-only 边界；右侧只读摘要不参与 validation 或提交。'
  }
  return 'Agent 工作区只展示入口和只读摘要；结构化 Truth 编辑仍需进入 World Editor。'
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
      <NEmpty
        v-if="route.name !== 'agent-worlds' && route.name !== 'agent-world-editor'"
        description="当前页面没有可展示的 Agent 摘要"
      />

      <template v-else>
        <NCard size="small" class="summary-card">
          <div class="card-header">
            <div class="header-main">
              <NIcon :size="18"><LayersOutline /></NIcon>
              <span>{{ panelTitle }}</span>
            </div>
            <NTag :type="route.name === 'agent-world-editor' ? 'warning' : 'info'">只读</NTag>
          </div>
          <p class="tip-text">{{ tipText }}</p>
          <div class="summary-list">
            <div v-for="item in summaryItems" :key="item.label" class="summary-row">
              <span>{{ item.label }}</span>
              <strong>{{ item.value }}</strong>
            </div>
          </div>
        </NCard>

        <NCard size="small" title="共享连接" class="summary-card">
          <div class="summary-list">
            <div v-for="item in apiSummaryItems" :key="item.label" class="summary-row">
              <span>{{ item.label }}</span>
              <strong>{{ item.value }}</strong>
            </div>
          </div>
        </NCard>

        <NCard v-if="route.name === 'agent-world-editor'" size="small" title="结构边界" class="summary-card">
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
              <strong>AgentInspectPanel</strong>
            </div>
          </div>
        </NCard>

        <NCard size="small" title="工作流提示" class="summary-card">
          <div class="summary-list">
            <div class="summary-row">
              <span>会话创建</span>
              <strong>Session Launcher</strong>
            </div>
            <div class="summary-row">
              <span>Trace 入口</span>
              <strong>日志页 / World 日志</strong>
            </div>
            <div class="summary-row">
              <span>模式边界</span>
              <strong>不混入 ST 资源页</strong>
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
}

.panel-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 12px;
  display: grid;
  align-content: start;
  gap: 12px;
}

.summary-card {
  border-radius: 8px;
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
