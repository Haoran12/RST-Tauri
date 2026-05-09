<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  NButton,
  NCard,
  NSpace,
  NTag,
  NIcon,
  NSpin,
  NTooltip,
  useMessage,
} from 'naive-ui'
import {
  SaveOutline,
  CheckmarkCircleOutline,
  WarningOutline,
  CodeWorkingOutline,
  BulbOutline,
} from '@vicons/ionicons5'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import WorldEditorEntityNav from './WorldEditorEntityNav.vue'
import KnowledgeEntryEditor from './KnowledgeEntryEditor.vue'
import CharacterRecordEditor from './CharacterRecordEditor.vue'
import LocationGraphEditor from './LocationGraphEditor.vue'
import WorldRulesEditor from './WorldRulesEditor.vue'
import ValidationPanel from './ValidationPanel.vue'
import ImpactSummaryPanel from './ImpactSummaryPanel.vue'
import ReactionWindow from './ReactionWindow.vue'

const route = useRoute()
const message = useMessage()
const editorStore = useAgentWorldEditorStore()

const worldId = computed(() => {
  const id = route.params.worldId
  return typeof id === 'string' ? id : ''
})

const showRightPanel = ref(true)

const statusTagType = computed(() => {
  switch (editorStore.worldStatus) {
    case 'paused': return 'default'
    case 'running': return 'success'
    case 'active_turn': return 'warning'
    case 'pending_llm': return 'info'
    case 'needs_rollback': return 'error'
    default: return 'default'
  }
})

const currentEditorComponent = computed(() => {
  switch (editorStore.selectedEntityType) {
    case 'knowledge': return KnowledgeEntryEditor
    case 'character': return CharacterRecordEditor
    case 'location': return LocationGraphEditor
    case 'world_rules': return WorldRulesEditor
    default: return null
  }
})

async function handleValidate() {
  if (!worldId.value) return
  try {
    const result = await editorStore.validateDraft(worldId.value)
    if (result.blockers.length === 0) {
      message.success('校验通过')
    } else {
      message.error(`校验发现 ${result.blockers.length} 个阻断问题`)
    }
  } catch (e) {
    message.error(`校验失败: ${String(e)}`)
  }
}

async function handleCommit() {
  if (!worldId.value) return
  try {
    const result = await editorStore.commitDraft(worldId.value)
    if (result.success) {
      message.success('提交成功')
      await editorStore.loadSnapshot(worldId.value)
    } else {
      message.error(`提交失败: ${result.error ?? '未知错误'}`)
    }
  } catch (e) {
    message.error(`提交失败: ${String(e)}`)
  }
}

function toggleDebugPanel() {
  editorStore.toggleDebugPanel()
}

watch(() => editorStore.selectedEntityType, () => {
  showRightPanel.value = true
})
</script>

<template>
  <div class="world-editor-shell">
    <!-- Top Bar -->
    <div class="shell-header">
      <div class="header-left">
        <NTag size="small" :type="statusTagType">
          {{ editorStore.worldStatus }}
        </NTag>
        <span class="revision-text">Revision {{ editorStore.editorRevision }}</span>
        <NTooltip v-if="editorStore.draft?.isDirty">
          <template #trigger>
            <NTag size="small" type="warning">
              <template #icon><NIcon><WarningOutline /></NIcon></template>
              未保存
            </NTag>
          </template>
          当前有未提交的草稿
        </NTooltip>
      </div>
      <NSpace size="small">
        <NButton size="small" secondary @click="toggleDebugPanel">
          <template #icon><NIcon><CodeWorkingOutline /></NIcon></template>
          调试面板
        </NButton>
        <NButton
          size="small"
          secondary
          :disabled="!editorStore.canValidate"
          :loading="editorStore.isValidating"
          @click="handleValidate"
        >
          <template #icon><NIcon><CheckmarkCircleOutline /></NIcon></template>
          校验
        </NButton>
        <NButton
          size="small"
          type="primary"
          :disabled="!editorStore.canCommit"
          :loading="editorStore.isSaving"
          @click="handleCommit"
        >
          <template #icon><NIcon><SaveOutline /></NIcon></template>
          提交
        </NButton>
      </NSpace>
    </div>

    <!-- Main Layout -->
    <div class="shell-body">
      <!-- Left: Entity Navigation -->
      <div class="panel-left">
        <WorldEditorEntityNav />
      </div>

      <!-- Center: Editor Area -->
      <div class="panel-center">
        <NSpin v-if="editorStore.isLoading" class="center-spinner" />
        <component
          :is="currentEditorComponent"
          v-else-if="currentEditorComponent"
        />
        <NCard v-else size="small" class="empty-editor-card">
          <div class="empty-editor-content">
            <NIcon :size="32"><BulbOutline /></NIcon>
            <p>请从左侧导航选择一个实体进行编辑</p>
          </div>
        </NCard>
      </div>

      <!-- Right: Validation + Impact + Debug -->
      <div v-show="showRightPanel" class="panel-right">
        <ValidationPanel />
        <ImpactSummaryPanel />
      </div>
    </div>

    <!-- Bottom: Debug Panel -->
    <ReactionWindow v-if="editorStore.showDebugPanel" />
  </div>
</template>

<style scoped>
.world-editor-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--color-bg-app, #f0f2f5);
}

.shell-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  background: var(--color-bg-surface, #fff);
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.revision-text {
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
  font-family: monospace;
}

.shell-body {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.panel-left {
  width: 260px;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border-subtle, #e0e0e6);
  background: var(--color-bg-surface, #fff);
  overflow: auto;
}

.panel-center {
  flex: 1;
  min-width: 0;
  overflow: auto;
  padding: 12px;
}

.panel-right {
  width: 300px;
  flex-shrink: 0;
  border-left: 1px solid var(--color-border-subtle, #e0e0e6);
  background: var(--color-bg-surface, #fff);
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px;
}

.center-spinner {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}

.empty-editor-card {
  max-width: 480px;
  margin: 40px auto;
}

.empty-editor-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 32px;
  text-align: center;
  color: var(--color-text-secondary, #6b7280);
}
</style>
