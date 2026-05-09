<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import {
  NTag,
  NIcon,
  NEmpty,
  NSpace,
  NButton,
  NSpin,
  NTabs,
  NTabPane,
  NInput,
  NSelect,
} from 'naive-ui'
import {
  RefreshOutline,
  PersonOutline,
} from '@vicons/ionicons5'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'

const editorStore = useAgentWorldEditorStore()
const route = useRoute()

const worldId = computed(() => {
  const id = route.params.worldId
  return typeof id === 'string' ? id : ''
})

const traceEventTypeOptions = [
  { label: '全部', value: '' },
  { label: '认知回合开始', value: 'cognitive_pass_start' },
  { label: '认知回合结束', value: 'cognitive_pass_end' },
  { label: 'LLM 请求', value: 'llm_request' },
  { label: 'LLM 响应', value: 'llm_response' },
  { label: '状态提交', value: 'state_commit' },
  { label: '知识揭示', value: 'knowledge_reveal' },
  { label: '场景初始化', value: 'scene_init' },
  { label: '场景提取', value: 'scene_extract' },
  { label: '用户输入', value: 'user_input' },
  { label: '回滚', value: 'rollback' },
]

const levelOptions = [
  { label: '全部', value: '' },
  { label: 'Debug', value: 'debug' },
  { label: 'Info', value: 'info' },
  { label: 'Warn', value: 'warn' },
  { label: 'Error', value: 'error' },
]

const levelTagType = (level: string) => {
  switch (level) {
    case 'debug': return 'default'
    case 'info': return 'info'
    case 'warn': return 'warning'
    case 'error': return 'error'
    default: return 'default'
  }
}

async function refreshTrace() {
  if (!worldId.value) return
  await editorStore.loadTraceEvents(worldId.value)
}

async function refreshReactions() {
  if (!worldId.value) return
  await editorStore.loadReactionEntries(worldId.value)
}

function updateTraceFilter(key: string, value: unknown) {
  editorStore.setTraceFilter({ [key]: value })
}

onMounted(() => {
  if (worldId.value) {
    void refreshTrace()
    void refreshReactions()
  }
})
</script>

<template>
  <div class="reaction-window">
    <NTabs v-model:value="editorStore.debugPanelTab" type="line" size="small">
      <!-- Trace Tab -->
      <NTabPane name="trace" tab="Trace Viewer">
        <div class="debug-panel-content">
          <div class="debug-toolbar">
            <NSpace size="small" align="center">
              <NButton size="tiny" quaternary :loading="editorStore.isLoadingTrace" @click="refreshTrace">
                <template #icon><NIcon><RefreshOutline /></NIcon></template>
                刷新
              </NButton>
              <NSelect
                :value="editorStore.traceFilter.eventTypes?.[0] ?? ''"
                size="tiny"
                :options="traceEventTypeOptions"
                style="width: 140px"
                @update:value="v => updateTraceFilter('eventTypes', v ? [v] : undefined)"
              />
              <NSelect
                :value="editorStore.traceFilter.level?.[0] ?? ''"
                size="tiny"
                :options="levelOptions"
                style="width: 100px"
                @update:value="v => updateTraceFilter('level', v ? [v] : undefined)"
              />
              <NInput
                :value="editorStore.traceFilter.search"
                size="tiny"
                placeholder="搜索摘要"
                clearable
                style="width: 140px"
                @update:value="v => updateTraceFilter('search', v || undefined)"
              />
            </NSpace>
          </div>

          <NSpin v-if="editorStore.isLoadingTrace" size="small" />

          <div v-else class="trace-list">
            <div
              v-for="evt in editorStore.filteredTraceEvents"
              :key="evt.event_id"
              class="trace-item"
            >
              <div class="trace-header">
                <NTag size="tiny" :type="levelTagType(evt.level)">{{ evt.level }}</NTag>
                <NTag size="tiny" :bordered="false">{{ evt.event_type }}</NTag>
                <span class="trace-time">{{ new Date(evt.timestamp).toLocaleTimeString() }}</span>
                <span v-if="evt.character_id" class="trace-char">
                  <NIcon :size="12"><PersonOutline /></NIcon>
                  {{ evt.character_id }}
                </span>
              </div>
              <div class="trace-summary">{{ evt.summary }}</div>
              <div v-if="evt.scene_turn_id" class="trace-meta">
                turn: {{ evt.scene_turn_id }}
              </div>
            </div>
            <NEmpty v-if="!editorStore.filteredTraceEvents.length" size="small" description="无 Trace 事件" />
          </div>
        </div>
      </NTabPane>

      <!-- Reaction Tab -->
      <NTabPane name="reaction" tab="Reaction Window">
        <div class="debug-panel-content">
          <div class="debug-toolbar">
            <NSpace size="small" align="center">
              <NButton size="tiny" quaternary :loading="editorStore.isLoadingReactions" @click="refreshReactions">
                <template #icon><NIcon><RefreshOutline /></NIcon></template>
                刷新
              </NButton>
            </NSpace>
          </div>

          <NSpin v-if="editorStore.isLoadingReactions" size="small" />

          <div v-else class="reaction-list">
            <div
              v-for="entry in editorStore.reactionEntries"
              :key="entry.entry_id"
              class="reaction-item"
            >
              <div class="reaction-header">
                <NTag size="tiny" type="info">{{ entry.reaction_type }}</NTag>
                <span class="reaction-char">{{ entry.character_id }}</span>
                <span class="reaction-confidence">置信度: {{ (entry.confidence * 100).toFixed(0) }}%</span>
                <span class="reaction-latency">{{ entry.latency_ms }}ms</span>
              </div>
              <div class="reaction-content">{{ entry.content }}</div>
              <div class="reaction-meta">
                {{ entry.scene_turn_id }} · {{ new Date(entry.created_at).toLocaleTimeString() }}
              </div>
            </div>
            <NEmpty v-if="!editorStore.reactionEntries.length" size="small" description="无 Reaction 数据" />
          </div>
        </div>
      </NTabPane>
    </NTabs>
  </div>
</template>

<style scoped>
.reaction-window {
  border-top: 1px solid var(--color-border-subtle, #e0e0e6);
  background: var(--color-bg-surface, #fff);
  flex-shrink: 0;
  max-height: 320px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.debug-panel-content {
  padding: 8px 12px 12px;
  overflow: auto;
  max-height: 260px;
}

.debug-toolbar {
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  margin-bottom: 8px;
  position: sticky;
  top: 0;
  background: var(--color-bg-surface, #fff);
  z-index: 1;
}

.trace-list,
.reaction-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.trace-item,
.reaction-item {
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--color-bg-app, #f0f2f5);
  font-size: 12px;
}

.trace-header,
.reaction-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
  flex-wrap: wrap;
}

.trace-time,
.trace-meta,
.reaction-meta {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  font-family: monospace;
}

.trace-char,
.reaction-char {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
}

.trace-summary,
.reaction-content {
  line-height: 1.5;
  color: var(--color-text-primary, #1f2937);
  word-break: break-word;
}

.reaction-confidence,
.reaction-latency {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  font-family: monospace;
}

:deep(.n-tabs-nav) {
  padding: 0 12px;
}
</style>
