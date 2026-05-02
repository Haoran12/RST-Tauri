<script setup lang="ts">
import {
  NButton,
  NTag,
  NBadge,
  NText,
  NIcon,
} from 'naive-ui'
import {
  EyeOffOutline,
  PinOutline,
  FlashOutline,
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
}>()

// Get position label
function getPositionLabel(position: number | undefined): string {
  switch (position) {
    case WorldInfoPosition.BEFORE_CHAR: return 'Before'
    case WorldInfoPosition.AFTER_CHAR: return 'After'
    case WorldInfoPosition.AN_TOP: return 'AN Top'
    case WorldInfoPosition.AN_BOTTOM: return 'AN Bottom'
    case WorldInfoPosition.AT_DEPTH: return 'At Depth'
    case WorldInfoPosition.EM_TOP: return 'EM Top'
    case WorldInfoPosition.EM_BOTTOM: return 'EM Bottom'
    case WorldInfoPosition.OUTLET: return 'Outlet'
    default: return 'Before'
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
  return `Entry ${entry.uid}`
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
</script>

<template>
  <div class="entry-list">
    <div class="entry-list-header">
      <NText depth="3">Entries: {{ entries.length }}</NText>
      <NButton size="small" type="primary" @click="createEntry">
        + New Entry
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
            <NIcon :size="16" color="#999">
              <EyeOffOutline />
            </NIcon>
          </NBadge>
          <NBadge v-else-if="entry.constant" :value="''" type="success">
            <NIcon :size="16" color="#63e2b7">
              <PinOutline />
            </NIcon>
          </NBadge>
          <NBadge v-else-if="(entry.probability ?? 100) < 100" :value="''" type="warning">
            <NIcon :size="16" color="#f2c97d">
              <FlashOutline />
            </NIcon>
          </NBadge>
        </div>

        <div class="entry-content">
          <div class="entry-header">
            <NText class="entry-name">{{ getEntryName(entry) }}</NText>
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
      </div>
    </div>

    <div v-if="entries.length === 0" class="entry-list-empty">
      <NText depth="3">No entries yet. Click "New Entry" to create one.</NText>
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
  overflow-y: auto;
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
  background-color: rgba(255, 255, 255, 0.05);
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
</style>