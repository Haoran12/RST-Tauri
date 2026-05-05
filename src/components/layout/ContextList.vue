<script setup lang="ts">
import {
  NList,
  NListItem,
  NEmpty,
  NSpin,
  NInput,
  NButton,
  NIcon,
  NSelect,
  NSwitch,
  NPopconfirm,
  NText,
  NTag,
} from 'naive-ui'
import { computed, ref, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { SearchOutline, AddOutline, TrashOutline, SettingsOutline } from '@vicons/ionicons5'
import { useWorldbooksStore } from '@/stores/worldbooks'
import type { WorldInfoEntry } from '@/types/st'
import { WorldInfoPosition } from '@/types/st'

const route = useRoute()
const worldbooksStore = useWorldbooksStore()
const searchQuery = ref('')
const loading = ref(false)

// Computed page type
const isWorldbooksPage = computed(() => route.name === 'resources-worldbooks')

// Worldbook file options for selector
const worldbookOptions = computed(() => {
  return worldbooksStore.worldbookList.map((wb) => ({
    label: wb.name || '未命名世界书',
    value: wb.id,
  }))
})

// Filtered entries for worldbook
const filteredEntries = computed(() => {
  if (!worldbooksStore.sortedEntries) return []
  if (!searchQuery.value) return worldbooksStore.sortedEntries
  const query = searchQuery.value.toLowerCase()
  return worldbooksStore.sortedEntries.filter(({ entry }) => {
    const name = entry.comment || entry.key?.join(', ') || `条目 ${entry.uid}`
    return name.toLowerCase().includes(query) ||
      (entry.content?.toLowerCase().includes(query) ?? false)
  })
})

// Page title
const pageTitle = computed(() => {
  const titles: Record<string, string> = {
    'library': '最近',
    'st-chat': '会话',
    'agent-worlds': 'Worlds',
    'resources-characters': '角色卡',
    'resources-worldbooks': '世界书',
    'resources-presets': '预设',
    'resources-regex': 'Regex',
    'api-configs': 'API 配置',
    'logs': '日志',
  }
  return titles[route.name as string] || '列表'
})

// Placeholder data for non-worldbook pages
const items = ref<Array<{ id: string; name: string; type: string }>>([])

const filteredItems = computed(() => {
  if (!searchQuery.value) return items.value
  const query = searchQuery.value.toLowerCase()
  return items.value.filter(item =>
    item.name.toLowerCase().includes(query)
  )
})

// Handle worldbook selection
async function handleWorldbookSelect(id: string | null) {
  if (id) {
    await worldbooksStore.loadWorldbook(id)
  } else {
    worldbooksStore.clearCurrentWorldbook()
  }
}

// Handle entry selection
function selectEntry(uid: number) {
  worldbooksStore.selectEntry(uid)
}

// Handle entry enable/disable toggle
async function toggleEntryEnabled(uid: number, entry: WorldInfoEntry, enabled: boolean) {
  const updatedEntry = { ...entry, disable: !enabled }
  await worldbooksStore.updateEntry(uid, updatedEntry)
}

// Handle entry deletion
async function deleteEntry(uid: number) {
  await worldbooksStore.deleteEntry(uid)
}

// Create new entry
async function createEntry() {
  await worldbooksStore.createNewEntry()
}

// Create new worldbook
function createWorldbook() {
  // Emit event or call store - the view will handle showing the modal
  window.dispatchEvent(new CustomEvent('create-worldbook'))
}

// Show global settings in right panel
function showGlobalSettings() {
  window.dispatchEvent(new CustomEvent('show-worldbook-global-settings'))
}

// Delete current worldbook
async function deleteCurrentWorldbook() {
  if (!worldbooksStore.currentWorldbookId) return
  await worldbooksStore.deleteWorldbookById(worldbooksStore.currentWorldbookId)
}

// Get entry display name
function getEntryName(entry: WorldInfoEntry): string {
  if (entry.comment && entry.comment.trim()) {
    return entry.comment
  }
  if (entry.key && entry.key.length > 0) {
    return entry.key.slice(0, 2).join(', ') + (entry.key.length > 2 ? '...' : '')
  }
  return `条目 ${entry.uid}`
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

// Get activation mode label
function getActivationModeLabel(entry: WorldInfoEntry): string {
  if (entry.constant) return '常驻'
  if (entry.vectorized) return '向量化'
  return '关键词'
}

// Get activation mode tag type
function getActivationModeType(entry: WorldInfoEntry): 'default' | 'success' | 'info' | 'warning' | 'error' {
  if (entry.constant) return 'success'
  if (entry.vectorized) return 'info'
  return 'default'
}

// Load worldbooks when entering the page
watch(() => route.name, async (newName) => {
  if (newName === 'resources-worldbooks') {
    await worldbooksStore.loadWorldbooks()
  }
}, { immediate: true })

onMounted(async () => {
  if (isWorldbooksPage.value) {
    await worldbooksStore.loadWorldbooks()
  }
})
</script>

<template>
  <div class="context-list">
    <!-- Worldbook-specific layout -->
    <template v-if="isWorldbooksPage">
      <!-- File selector header -->
      <div class="list-header">
        <span class="list-title-worldbook">世界书</span>
        <NButton quaternary size="small" @click="showGlobalSettings">
          <template #icon>
            <NIcon><SettingsOutline /></NIcon>
          </template>
        </NButton>
      </div>

      <!-- Worldbook file selector -->
      <div class="file-selector">
        <NSelect
          :value="worldbooksStore.currentWorldbookId"
          :options="worldbookOptions"
          placeholder="选择世界书..."
          clearable
          size="small"
          @update:value="handleWorldbookSelect"
        />
      </div>

      <!-- File action buttons -->
      <div class="file-actions">
        <NButton size="small" type="primary" @click="createWorldbook">
          <template #icon>
            <NIcon><AddOutline /></NIcon>
          </template>
          新建
        </NButton>
        <NPopconfirm
          v-if="worldbooksStore.currentWorldbookId"
          @positive-click="deleteCurrentWorldbook"
        >
          <template #trigger>
            <NButton size="small" type="error">
              <template #icon>
                <NIcon><TrashOutline /></NIcon>
              </template>
              删除
            </NButton>
          </template>
          确定删除此世界书吗？
        </NPopconfirm>
      </div>

      <!-- Entry list when worldbook is selected -->
      <template v-if="worldbooksStore.currentWorldbook">
        <!-- Entry actions -->
        <div class="entry-actions">
          <NText depth="3" class="entry-count">
            条目: {{ worldbooksStore.sortedEntries.length }}
          </NText>
          <NButton size="small" type="primary" @click="createEntry">
            <template #icon>
              <NIcon><AddOutline /></NIcon>
            </template>
            新建
          </NButton>
        </div>

        <!-- Search -->
        <div class="list-search">
          <NInput
            v-model:value="searchQuery"
            placeholder="搜索条目..."
            clearable
            size="small"
          >
            <template #prefix>
              <NIcon :size="16"><SearchOutline /></NIcon>
            </template>
          </NInput>
        </div>

        <!-- Entry list -->
        <div class="list-content">
          <NSpin :show="worldbooksStore.isLoading">
            <div v-if="filteredEntries.length > 0" class="entry-list">
              <div
                v-for="{ uid, entry } in filteredEntries"
                :key="uid"
                class="entry-item"
                :class="{ 'entry-selected': worldbooksStore.currentEntryUid === uid }"
              >
                <!-- Enable switch -->
                <div class="entry-switch">
                  <NSwitch
                    :value="!entry.disable"
                    size="small"
                    @update:value="(v) => toggleEntryEnabled(uid, entry, v)"
                  />
                </div>

                <!-- Entry info -->
                <div class="entry-info" @click="selectEntry(uid)">
                  <div class="entry-header">
                    <span class="entry-name">{{ getEntryName(entry) }}</span>
                    <NTag
                      size="tiny"
                      :type="getActivationModeType(entry)"
                      :bordered="false"
                    >
                      {{ getActivationModeLabel(entry) }}
                    </NTag>
                    <NTag
                      v-if="entry.group"
                      size="tiny"
                      type="info"
                      :bordered="false"
                    >
                      {{ entry.group }}
                    </NTag>
                  </div>
                  <div class="entry-meta">
                    <NText depth="3" class="entry-position">
                      {{ getPositionLabel(entry.position) }}
                    </NText>
                  </div>
                </div>

                <!-- Delete button -->
                <NPopconfirm @positive-click="deleteEntry(uid)">
                  <template #trigger>
                    <NButton quaternary circle size="tiny" type="error" class="delete-btn">
                      <template #icon>
                        <NIcon><TrashOutline /></NIcon>
                      </template>
                    </NButton>
                  </template>
                  确定删除此条目吗？
                </NPopconfirm>
              </div>
            </div>
            <NEmpty v-else description="暂无条目" />
          </NSpin>
        </div>
      </template>

      <!-- Empty state when no worldbook selected -->
      <div v-else class="empty-state">
        <NEmpty description="请选择或创建世界书" />
      </div>
    </template>

    <!-- Default layout for other pages -->
    <template v-else>
      <div class="list-header">
        <span class="list-title">{{ pageTitle }}</span>
        <NButton quaternary size="small">
          <template #icon>
            <NIcon><AddOutline /></NIcon>
          </template>
        </NButton>
      </div>

      <div class="list-search">
        <NInput
          v-model:value="searchQuery"
          placeholder="搜索..."
          clearable
          size="small"
        >
          <template #prefix>
            <NIcon :size="16"><SearchOutline /></NIcon>
          </template>
        </NInput>
      </div>

      <div class="list-content">
        <NSpin :show="loading">
          <NList v-if="filteredItems.length > 0" hoverable clickable>
            <NListItem v-for="item in filteredItems" :key="item.id">
              {{ item.name }}
            </NListItem>
          </NList>
          <NEmpty v-else description="暂无数据" />
        </NSpin>
      </div>
    </template>
  </div>
</template>

<style scoped>
.context-list {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.list-header {
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.list-title {
  font-weight: 500;
  font-size: 14px;
}

.list-title-worldbook {
  font-weight: 600;
  font-size: 18px;
}

.file-selector {
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.file-actions {
  padding: 8px 12px;
  display: flex;
  gap: 8px;
  justify-content: flex-start;
  flex-shrink: 0;
}

.entry-actions {
  padding: 8px 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.entry-count {
  font-size: 12px;
}

.list-search {
  padding: 8px 12px;
  flex-shrink: 0;
}

.list-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 4px;
}

.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

/* Entry list styles */
.entry-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.entry-item {
  display: flex;
  align-items: center;
  padding: 8px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: background-color 0.2s;
  gap: 8px;
}

.entry-item:hover {
  background-color: rgba(0, 0, 0, 0.04);
}

.entry-selected {
  background-color: rgba(24, 160, 88, 0.1);
}

.entry-switch {
  flex-shrink: 0;
}

.entry-info {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.entry-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 2px;
}

.entry-name {
  font-size: 13px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.entry-meta {
  display: flex;
  align-items: center;
  gap: 4px;
}

.entry-position {
  font-size: 11px;
}

.delete-btn {
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.2s;
}

.entry-item:hover .delete-btn {
  opacity: 1;
}
</style>
