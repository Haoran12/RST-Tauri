<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  NCard,
  NForm,
  NFormItem,
  NInput,
  NSelect,
  NTag,
  NIcon,
  NEmpty,
  NTooltip,
} from 'naive-ui'
import {
  LocationOutline,
  WarningOutline,
} from '@vicons/ionicons5'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import type { LocationNodeSummary } from '@/types/agent/worldEditor'

const editorStore = useAgentWorldEditorStore()

const draft = computed(() => {
  if (editorStore.selectedEntityType !== 'location') return null
  return editorStore.draft?.draft as LocationNodeSummary | undefined
})

const isNew = computed(() => editorStore.draft?.isNew ?? false)

const dragState = ref<{
  draggedId: string | null
  dragOverId: string | null
}>({ draggedId: null, dragOverId: null })

function updateField(path: string, value: unknown) {
  editorStore.updateDraftField(path, value)
}

// Drag handlers for tree reordering
function handleDragStart(id: string) {
  dragState.value.draggedId = id
}

function handleDragOver(id: string, event: DragEvent) {
  event.preventDefault()
  if (dragState.value.draggedId && dragState.value.draggedId !== id) {
    dragState.value.dragOverId = id
  }
}

function handleDragLeave() {
  dragState.value.dragOverId = null
}

async function handleDrop(targetId: string, event: DragEvent) {
  event.preventDefault()
  const draggedId = dragState.value.draggedId
  dragState.value.dragOverId = null
  dragState.value.draggedId = null

  if (!draggedId || draggedId === targetId) return

  // Check for cycles
  if (wouldCreateCycle(draggedId, targetId)) {
    return
  }

  await editorStore.updateLocationParent(draggedId, targetId)
}

function wouldCreateCycle(draggedId: string, newParentId: string): boolean {
  let current: string | null = newParentId
  while (current) {
    if (current === draggedId) return true
    const node = editorStore.locationList.find(l => l.location_id === current)
    current = node?.parent_id ?? null
  }
  return false
}

const flattenedTree = computed(() => {
  const result: Array<LocationNodeSummary & { depth: number }> = []
  function walk(nodes: typeof editorStore.locationTree) {
    for (const node of nodes) {
      result.push(node)
      if (node.children.length) walk(node.children)
    }
  }
  walk(editorStore.locationTree)
  return result
})

const parentOptions = computed(() => [
  { label: '无父级 (根)', value: '' as string },
  ...editorStore.locationList
    .filter(l => l.location_id !== draft.value?.location_id)
    .map(l => ({
      label: l.name,
      value: l.location_id,
    })),
])
</script>

<template>
  <div v-if="!draft" class="empty-editor">
    <NCard size="small" class="empty-card">
      <div class="empty-content">
        <NTag type="info">Location Editor</NTag>
        <p>请在左侧实体导航中选择一个地点，或点击新建。</p>
      </div>
    </NCard>
  </div>

  <div v-else class="location-editor">
    <!-- Header -->
    <div class="editor-header">
      <div class="header-main">
        <NTag size="small" :type="isNew ? 'success' : 'default'">
          {{ isNew ? '新建' : '编辑' }}
        </NTag>
        <span class="entity-id">{{ draft.location_id }}</span>
      </div>
    </div>

    <!-- Basic Form -->
    <NCard size="small" title="地点信息">
      <NForm label-placement="left" label-width="100">
        <NFormItem label="ID">
          <NInput
            v-model:value="draft.location_id"
            size="small"
            placeholder="location_001"
            @update:value="v => updateField('location_id', v)"
          />
        </NFormItem>
        <NFormItem label="名称">
          <NInput
            v-model:value="draft.name"
            size="small"
            placeholder="地点名称"
            @update:value="v => updateField('name', v)"
          />
        </NFormItem>
        <NFormItem label="层级">
          <NInput
            v-model:value="draft.canonical_level"
            size="small"
            placeholder="WorldRoot / Region / City / District"
            @update:value="v => updateField('canonical_level', v)"
          />
        </NFormItem>
        <NFormItem label="父级地点">
          <NSelect
            :value="draft.parent_id ?? ''"
            size="small"
            :options="parentOptions"
            clearable
            @update:value="v => updateField('parent_id', v || null)"
          />
        </NFormItem>
        <NFormItem label="状态">
          <NInput
            v-model:value="draft.status"
            size="small"
            placeholder="active / pending_confirmation / deprecated"
            @update:value="v => updateField('status', v)"
          />
        </NFormItem>
      </NForm>
    </NCard>

    <!-- Tree View with Drag -->
    <NCard size="small" title="地点树 (拖拽调整 parent)">
      <template #header-extra>
        <NTooltip>
          <template #trigger>
            <NTag size="tiny" :bordered="false">
              <template #icon><NIcon><WarningOutline /></NIcon></template>
              提示
            </NTag>
          </template>
          拖拽地点行可调整 parent_id，系统会自动检测循环依赖
        </NTooltip>
      </template>

      <div class="location-tree">
        <div
          v-for="loc in flattenedTree"
          :key="loc.location_id"
          class="location-tree-row"
          :class="{
            'is-drag-over': dragState.dragOverId === loc.location_id,
            'is-selected': draft.location_id === loc.location_id,
          }"
          :style="{ paddingLeft: `${12 + loc.depth * 20}px` }"
          draggable="true"
          @dragstart="handleDragStart(loc.location_id)"
          @dragover="(e) => handleDragOver(loc.location_id, e)"
          @dragleave="handleDragLeave"
          @drop="(e) => handleDrop(loc.location_id, e)"
        >
          <NIcon :size="14"><LocationOutline /></NIcon>
          <span class="loc-name">{{ loc.name }}</span>
          <NTag size="tiny" :bordered="false">{{ loc.canonical_level }}</NTag>
          <span v-if="loc.parent_id" class="loc-parent">← {{ loc.parent_id }}</span>
        </div>
        <NEmpty v-if="!flattenedTree.length" size="small" description="无地点数据" />
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.location-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-bottom: 24px;
}

.empty-editor {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-card {
  max-width: 420px;
}

.empty-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 24px;
  text-align: center;
  color: var(--color-text-secondary, #6b7280);
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 4px 2px;
}

.header-main {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.entity-id {
  font-family: monospace;
  font-size: 13px;
  color: var(--color-text-secondary, #6b7280);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.location-tree {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.location-tree-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: grab;
  transition: background 0.15s;
  font-size: 13px;
}

.location-tree-row:hover {
  background: var(--color-bg-hover, #f2f3f5);
}

.location-tree-row.is-drag-over {
  background: var(--color-primary-light, #e6f7ff);
  border: 1px dashed var(--color-primary, #1890ff);
}

.location-tree-row.is-selected {
  background: #f6ffed;
  font-weight: 600;
}

.location-tree-row:active {
  cursor: grabbing;
}

.loc-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.loc-parent {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  font-family: monospace;
}
</style>
