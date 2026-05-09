<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import {
  NButton,
  NEmpty,
  NIcon,
  NInput,
  NList,
  NListItem,
  NSelect,
  NSpin,
  NTag,
  useMessage,
} from 'naive-ui'
import {
  AddOutline,
  CheckmarkCircleOutline,
  SaveOutline,
  BookOutline,
} from '@vicons/ionicons5'
import { useAgentStore } from '@/stores/agent'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import { KNOWLEDGE_KIND_LABELS } from '@/types/agent/knowledge'
import KnowledgeEntryEditor from '@/components/agent/world-editor/KnowledgeEntryEditor.vue'

const message = useMessage()
const agentStore = useAgentStore()
const editorStore = useAgentWorldEditorStore()

const worldId = computed(() => agentStore.currentWorldId ?? '')

const knowledgeKindFilterOptions = [
  { label: '全部', value: '' },
  ...Object.entries(KNOWLEDGE_KIND_LABELS).map(([value, label]) => ({ label, value })),
]

async function loadData() {
  if (!worldId.value) return
  try {
    await editorStore.loadSnapshot(worldId.value)
    editorStore.selectEntity('knowledge', null)
  } catch (e) {
    message.error(`加载 Knowledge 数据失败: ${String(e)}`)
  }
}

async function handleKnowledgeSelect(id: string) {
  editorStore.selectEntity('knowledge', id)
  const knowledge = await editorStore.loadKnowledgeDetail(worldId.value, id)
  if (knowledge) {
    editorStore.initDraft('knowledge', id, { ...knowledge }, false)
  }
}

function createNewKnowledge() {
  const id = `knowledge_${Date.now()}`
  editorStore.selectEntity('knowledge', id)
  editorStore.initDraft('knowledge', id, {
    knowledge_id: id,
    kind: 'world_fact',
    subject_type: 'world',
    subject_id: null,
    facet_type: null,
    content: { summary_text: '' },
    apparent_content: null,
    access_policy: { known_by: [], scope: [{ type: 'Public' }], conditions: [] },
    subject_awareness: { kind: 'Aware' },
    metadata: { created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
    valid_from: null,
    valid_until: null,
    source_session_id: null,
    source_scene_turn_id: null,
    derived_from_event_id: null,
    schema_version: '0.1',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  } as any, true)
}

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

watch(worldId, (newId, oldId) => {
  if (newId && newId !== oldId) {
    editorStore.clearDraft()
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
    <div v-else class="module-layout">
      <!-- Left: Entity List -->
      <div class="module-list">
        <div class="list-header">
          <span class="list-title">
            <NIcon :size="16"><BookOutline /></NIcon>
            Knowledge
          </span>
          <NButton size="tiny" quaternary @click="createNewKnowledge">
            <template #icon><NIcon><AddOutline /></NIcon></template>
          </NButton>
        </div>
        <div class="list-filters">
          <NInput
            v-model:value="editorStore.knowledgeFilterSearch"
            size="tiny"
            placeholder="搜索 ID / 摘要"
            clearable
          />
          <NSelect
            v-model:value="editorStore.knowledgeFilterKind"
            size="tiny"
            :options="knowledgeKindFilterOptions"
            placeholder="类型筛选"
            clearable
          />
        </div>
        <div class="list-content">
          <NSpin :show="editorStore.isLoading">
            <NList hoverable clickable style="background: transparent">
              <NListItem
                v-for="item in editorStore.filteredKnowledgeList"
                :key="item.knowledge_id"
                :class="{ active: editorStore.selectedEntityId === item.knowledge_id }"
                @click="handleKnowledgeSelect(item.knowledge_id)"
              >
                <div class="knowledge-list-item">
                  <div class="knowledge-title">
                    <NTag size="tiny" :type="item.access_scope_summary === 'GodOnly' ? 'error' : 'default'">
                      {{ item.kind.slice(0, 4) }}
                    </NTag>
                    <span class="knowledge-id" :title="item.knowledge_id">{{ item.knowledge_id }}</span>
                  </div>
                  <div class="knowledge-summary">{{ item.summary_text || '(无摘要)' }}</div>
                </div>
              </NListItem>
              <NEmpty v-if="!editorStore.filteredKnowledgeList.length" size="small" description="无数据" />
            </NList>
          </NSpin>
        </div>
      </div>

      <!-- Right: Editor -->
      <div class="module-editor">
        <!-- Toolbar -->
        <div class="editor-toolbar">
          <div class="toolbar-left">
            <NTag size="small" :type="editorStore.draft?.isDirty ? 'warning' : 'default'">
              {{ editorStore.draft?.isDirty ? '未保存' : '已同步' }}
            </NTag>
          </div>
          <div class="toolbar-right">
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
          </div>
        </div>

        <!-- Editor Content -->
        <div class="editor-body">
          <KnowledgeEntryEditor />
        </div>
      </div>
    </div>
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

.module-layout {
  display: flex;
  height: 100%;
  min-height: 0;
}

.module-list {
  width: 280px;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border-subtle, #e0e0e6);
  background: var(--color-bg-surface, #fff);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.list-title {
  font-weight: 600;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.list-filters {
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex-shrink: 0;
}

.list-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 4px;
}

.module-editor {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  background: var(--color-bg-surface, #fff);
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.editor-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 12px;
}

.knowledge-list-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 2px 0;
}

.knowledge-title {
  display: flex;
  align-items: center;
  gap: 6px;
}

.knowledge-id {
  font-family: monospace;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.knowledge-summary {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

:deep(.n-list-item.active) {
  background: var(--color-primary-light, #e6f7ff);
}
</style>
