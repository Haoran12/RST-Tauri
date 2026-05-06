<script setup lang="ts">
import {
  NButton,
  NTag,
  NBadge,
  NText,
  NIcon,
  useDialog,
} from 'naive-ui'
import {
  EyeOffOutline,
  PinOutline,
  FlashOutline,
  SearchOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import type { WorldInfoEntry } from '@/types/st'
import { WorldInfoPosition } from '@/types/st'

const props = defineProps<{
  entries: Array<{ uid: number; entry: WorldInfoEntry }>
  selectedUid: number | null
}>()

const emit = defineEmits<{
  (e: 'select', uid: number): void
  (e: 'create'): void
  (e: 'reorder', uidOrder: number[]): void
  (e: 'delete', uid: number): void
}>()

const dialog = useDialog()

// Get activation mode label
function getActivationModeLabel(entry: WorldInfoEntry): string {
  if (entry.constant) return '常驻'
  if (entry.vectorized) return '向量化'
  return '关键词'
}

// Get activation mode color
function getActivationModeColor(entry: WorldInfoEntry): 'default' | 'success' | 'info' | 'warning' | 'error' {
  if (entry.constant) return 'success'
  if (entry.vectorized) return 'info'
  return 'default'
}

// Get position label
function getPositionLabel(position: number | undefined): string {
  switch (position) {
    case WorldInfoPosition.BEFORE_CHAR: return '角色前'
    case WorldInfoPosition.AFTER_CHAR: return '角色后'
    case WorldInfoPosition.AN_TOP: return 'AN顶部'
    case WorldInfoPosition.AN_BOTTOM: return 'AN底部'
    case WorldInfoPosition.AT_DEPTH: return '指定深度'
    case WorldInfoPosition.EM_TOP: return 'EM顶部'
    case WorldInfoPosition.EM_BOTTOM: return 'EM底部'
    case WorldInfoPosition.OUTLET: return '出口'
    default: return '角色前'
  }
}

// Get position color
function getPositionColor(position: number | undefined): 'default' | 'success' | 'warning' | 'error' | 'info' {
  switch (position) {
    case WorldInfoPosition.BEFORE_CHAR: return 'default'
    case WorldInfoPosition.AFTER_CHAR: return 'info'
    case WorldInfoPosition.AN_TOP:
    case WorldInfoPosition.AN_BOTTOM: return 'warning'
    case WorldInfoPosition.AT_DEPTH: return 'success'
    case WorldInfoPosition.EM_TOP:
    case WorldInfoPosition.EM_BOTTOM: return 'error'
    case WorldInfoPosition.OUTLET: return 'default'
    default: return 'default'
  }
}

// Get entry display name
function getEntryName(entry: WorldInfoEntry): string {
  if (entry.comment && entry.comment.trim()) {
    return entry.comment
  }
  if (entry.key && entry.key.length > 0) {
    return entry.key.slice(0, 3).join(', ') + (entry.key.length > 3 ? '...' : '')
  }
  return `条目 ${entry.uid}`
}

// Get entry preview
function getEntryPreview(entry: WorldInfoEntry): string {
  if (!entry.content) return ''
  const preview = entry.content.slice(0, 100)
  return preview + (entry.content.length > 100 ? '...' : '')
}

// Handle entry click
function selectEntry(uid: number) {
  emit('select', uid)
}

// Create new entry
function createEntry() {
  emit('create')
}

// Delete entry with confirmation
function deleteEntry(uid: number, event: Event) {
  event.stopPropagation()
  dialog.warning({
    title: '删除条目',
    content: '确定要删除此条目吗？此操作不可撤销。',
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: () => {
      emit('delete', uid)
    },
  })
}
</script>

<template>
  <div class="entry-list">
    <div class="entry-list-header">
      <NText depth="3">条目数: {{ entries.length }}</NText>
      <NButton size="small" type="primary" @click="createEntry">
        + 添加条目
      </NButton>
    </div>

    <div class="entry-list-content">
      <div
        v-for="{ uid, entry } in entries"
        :key="uid"
        class="entry-item"
        :class="{ 'entry-selected': selectedUid === uid }"
        @click="selectEntry(uid)"
      >
        <div class="entry-badges">
          <NBadge v-if="entry.disable" :value="''" type="error">
            <NIcon :size="16" class="icon-disabled">
              <EyeOffOutline />
            </NIcon>
          </NBadge>
          <NBadge v-else-if="entry.constant" :value="''" type="success">
            <NIcon :size="16" class="icon-success">
              <PinOutline />
            </NIcon>
          </NBadge>
          <NBadge v-else-if="entry.vectorized" :value="''" type="info">
            <NIcon :size="16" class="icon-info">
              <SearchOutline />
            </NIcon>
          </NBadge>
          <NBadge v-else-if="(entry.probability ?? 100) < 100" :value="''" type="warning">
            <NIcon :size="16" class="icon-warning">
              <FlashOutline />
            </NIcon>
          </NBadge>
        </div>

        <div class="entry-content">
          <div class="entry-header">
            <NText class="entry-name">{{ getEntryName(entry) }}</NText>
            <NTag
              size="small"
              :type="getActivationModeColor(entry)"
              :bordered="false"
            >
              {{ getActivationModeLabel(entry) }}
            </NTag>
            <NTag
              size="small"
              :type="getPositionColor(entry.position)"
              :bordered="false"
            >
              {{ getPositionLabel(entry.position) }}
            </NTag>
            <NTag v-if="entry.group" size="small" type="info" :bordered="false">
              {{ entry.group }}
            </NTag>
          </div>
          <NText depth="3" class="entry-preview">
            {{ getEntryPreview(entry) }}
          </NText>
        </div>

        <div class="entry-actions">
          <NButton
            quaternary
            circle
            size="small"
            @click="(e: Event) => deleteEntry(uid, e)"
          >
            <template #icon>
              <NIcon :size="16" class="icon-error">
                <TrashOutline />
              </NIcon>
            </template>
          </NButton>
        </div>
      </div>
    </div>

    <div v-if="entries.length === 0" class="entry-list-empty">
      <NText depth="3">暂无条目，点击"新建条目"创建。</NText>
    </div>
  </div>
</template>

<style scoped>
.entry-list {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.entry-list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid var(--n-border-color);
}

.entry-list-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  scrollbar-width: thin;
}

.entry-list-content::-webkit-scrollbar {
  width: 6px;
}

.entry-list-content::-webkit-scrollbar-track {
  background: transparent;
}

.entry-list-content::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 3px;
}

.entry-list-content::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

.entry-list-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.entry-item {
  display: flex;
  align-items: flex-start;
  padding: 8px 12px;
  cursor: pointer;
  border-bottom: 1px solid var(--n-border-color);
  transition: background-color 0.2s;
}

.entry-item:hover {
  background-color: var(--n-color-hover);
}

.entry-selected {
  background-color: rgba(24, 160, 88, 0.1);
}

.entry-badges {
  width: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 8px;
  flex-shrink: 0;
}

.entry-content {
  flex: 1;
  min-width: 0;
}

.entry-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.entry-name {
  font-weight: 500;
}

.entry-preview {
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 250px;
  display: inline-block;
}

.entry-actions {
  display: flex;
  align-items: center;
  margin-left: 8px;
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.2s;
}

.entry-item:hover .entry-actions {
  opacity: 1;
}

.icon-disabled {
  color: var(--n-text-color-disabled);
}

.icon-success {
  color: var(--n-success-color);
}

.icon-info {
  color: var(--n-info-color);
}

.icon-warning {
  color: var(--n-warning-color);
}

.icon-error {
  color: var(--n-error-color);
}
</style>
