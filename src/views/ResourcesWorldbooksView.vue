<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import {
  NCard,
  NSpace,
  NButton,
  NInput,
  NModal,
  NForm,
  NFormItem,
  NInputNumber,
  NSwitch,
  NUpload,
  NDivider,
  NText,
  useMessage,
  type UploadCustomRequestOptions,
} from 'naive-ui'
import { useWorldbooksStore } from '@/stores/worldbooks'
import WorldbookEntryEditor from '@/components/st/worldbook/WorldbookEntryEditor.vue'
import type { WorldInfoEntry } from '@/types/st'

const store = useWorldbooksStore()
const message = useMessage()

// Modal state
const showCreateModal = ref(false)
const createName = ref('')
const showEditMetaModal = ref(false)
const editName = ref('')
const editDescription = ref('')
const editScanDepth = ref<number | null>(null)
const editTokenBudget = ref<number | null>(null)
const editRecursiveScanning = ref(false)

// Load worldbooks on mount
onMounted(() => {
  store.loadWorldbooks()
  // Listen for create event from ContextList
  window.addEventListener('create-worldbook', handleCreateWorldbookEvent)
})

onUnmounted(() => {
  window.removeEventListener('create-worldbook', handleCreateWorldbookEvent)
})

function handleCreateWorldbookEvent() {
  showCreateModal.value = true
}

// Create new worldbook
async function createWorldbook() {
  if (!createName.value.trim()) {
    message.error('请输入名称')
    return
  }

  try {
    const id = await store.createNewWorldbook(createName.value.trim())
    message.success('世界书已创建')
    showCreateModal.value = false
    createName.value = ''
    await store.loadWorldbook(id)
  } catch (e) {
    message.error(String(e))
  }
}

// Open edit meta modal
function openEditMetaModal() {
  if (!store.currentWorldbook) return
  editName.value = store.currentWorldbook.name || ''
  editDescription.value = store.currentWorldbook.description || ''
  editScanDepth.value = store.currentWorldbook.scan_depth ?? null
  editTokenBudget.value = store.currentWorldbook.token_budget ?? null
  editRecursiveScanning.value = store.currentWorldbook.recursive_scanning ?? false
  showEditMetaModal.value = true
}

// Save meta changes
async function saveMeta() {
  if (!editName.value.trim()) {
    message.error('请输入名称')
    return
  }

  try {
    // Update local state
    if (store.currentWorldbook) {
      store.currentWorldbook.name = editName.value.trim()
      store.currentWorldbook.description = editDescription.value.trim()
      store.currentWorldbook.scan_depth = editScanDepth.value
      store.currentWorldbook.token_budget = editTokenBudget.value
      store.currentWorldbook.recursive_scanning = editRecursiveScanning.value
    }

    // Save to file
    await store.saveCurrentWorldbook()

    // Update list
    await store.loadWorldbooks()

    message.success('已保存')
    showEditMetaModal.value = false
  } catch (e) {
    message.error(String(e))
  }
}

// Update entry
async function updateEntry(uid: number, entry: WorldInfoEntry) {
  try {
    await store.updateEntry(uid, entry)
  } catch (e) {
    message.error(String(e))
  }
}

// Delete entry
async function deleteEntry(uid: number) {
  try {
    await store.deleteEntry(uid)
    message.success('条目已删除')
  } catch (e) {
    message.error(String(e))
  }
}

// Handle import
async function handleImport(options: UploadCustomRequestOptions) {
  const file = options.file.file
  if (!file) return

  try {
    const id = await store.importFromFile(file)
    message.success('世界书已导入')
    await store.loadWorldbook(id)
  } catch (e) {
    message.error(String(e))
  }
}

// Handle export
async function handleExport() {
  if (!store.currentWorldbook || !store.currentWorldbookId) return

  try {
    const blob = await store.exportToFile(store.currentWorldbookId)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${store.currentWorldbook.name || 'worldbook'}.json`
    a.click()
    URL.revokeObjectURL(url)
    message.success('世界书已导出')
  } catch (e) {
    message.error(String(e))
  }
}
</script>

<template>
  <div class="worldbooks-view">
    <!-- Empty state when no worldbook selected -->
    <div v-if="!store.currentWorldbook" class="empty-state">
      <NCard class="empty-card">
        <div class="empty-content">
          <NText depth="3" class="empty-text">
            请在左侧面板选择或创建世界书
          </NText>
          <NSpace>
            <NButton type="primary" @click="showCreateModal = true">
              创建世界书
            </NButton>
            <NUpload
              :show-file-list="false"
              accept=".json"
              :custom-request="handleImport"
            >
              <NButton>导入世界书</NButton>
            </NUpload>
          </NSpace>
        </div>
      </NCard>
    </div>

    <!-- Worldbook Editor when worldbook is selected -->
    <template v-else>
      <!-- Header with actions -->
      <div class="editor-header">
        <div class="header-left">
          <h2 class="worldbook-title">{{ store.currentWorldbook.name || '未命名世界书' }}</h2>
          <NText v-if="store.currentWorldbook.description" depth="3" class="worldbook-desc">
            {{ store.currentWorldbook.description }}
          </NText>
        </div>
        <NSpace class="header-actions">
          <NButton size="small" @click="openEditMetaModal">
            编辑信息
          </NButton>
          <NUpload
            :show-file-list="false"
            accept=".json"
            :custom-request="handleImport"
          >
            <NButton size="small">导入</NButton>
          </NUpload>
          <NButton size="small" @click="handleExport">
            导出
          </NButton>
        </NSpace>
      </div>

      <!-- Global settings summary -->
      <NCard class="global-settings-card" size="small">
        <div class="global-settings-summary">
          <NSpace :size="24">
            <div class="setting-item">
              <NText depth="3" class="setting-label">扫描深度</NText>
              <NText class="setting-value">
                {{ store.currentWorldbook.scan_depth ?? '默认' }}
              </NText>
            </div>
            <div class="setting-item">
              <NText depth="3" class="setting-label">Token 预算</NText>
              <NText class="setting-value">
                {{ store.currentWorldbook.token_budget ?? '默认' }}
              </NText>
            </div>
            <div class="setting-item">
              <NText depth="3" class="setting-label">递归扫描</NText>
              <NText class="setting-value">
                {{ store.currentWorldbook.recursive_scanning ? '开启' : '关闭' }}
              </NText>
            </div>
            <div class="setting-item">
              <NText depth="3" class="setting-label">条目数</NText>
              <NText class="setting-value">
                {{ store.sortedEntries.length }}
              </NText>
            </div>
          </NSpace>
        </div>
      </NCard>

      <!-- Entry Editor -->
      <NCard class="entry-editor-card">
        <template v-if="store.currentEntry">
          <WorldbookEntryEditor
            :entry="store.currentEntry"
            :groups="store.groups"
            @update="(entry) => updateEntry(store.currentEntryUid!, entry)"
            @delete="deleteEntry(store.currentEntryUid!)"
          />
        </template>
        <template v-else>
          <div class="no-entry-selected">
            <NText depth="3">
              请在左侧面板选择条目进行编辑
            </NText>
          </div>
        </template>
      </NCard>
    </template>

    <!-- Create Modal -->
    <NModal
      v-model:show="showCreateModal"
      preset="dialog"
      title="创建世界书"
      positive-text="创建"
      negative-text="取消"
      @positive-click="createWorldbook"
    >
      <NForm>
        <NFormItem label="名称" required>
          <NInput
            v-model:value="createName"
            placeholder="输入世界书名称"
          />
        </NFormItem>
      </NForm>
    </NModal>

    <!-- Edit Meta Modal -->
    <NModal
      v-model:show="showEditMetaModal"
      preset="dialog"
      title="编辑世界书信息"
      positive-text="保存"
      negative-text="取消"
      @positive-click="saveMeta"
    >
      <NForm label-placement="left" label-width="100px">
        <NFormItem label="名称" required>
          <NInput
            v-model:value="editName"
            placeholder="世界书名称"
          />
        </NFormItem>
        <NFormItem label="描述">
          <NInput
            v-model:value="editDescription"
            type="textarea"
            placeholder="世界书描述"
          />
        </NFormItem>
        <NDivider>全局设置</NDivider>
        <NFormItem label="扫描深度">
          <NInputNumber
            v-model:value="editScanDepth"
            :min="1"
            :max="999"
            placeholder="默认"
            clearable
          />
        </NFormItem>
        <NFormItem label="Token 预算">
          <NInputNumber
            v-model:value="editTokenBudget"
            :min="1"
            placeholder="默认"
            clearable
          />
        </NFormItem>
        <NFormItem label="递归扫描">
          <NSwitch v-model:value="editRecursiveScanning" />
        </NFormItem>
      </NForm>
    </NModal>
  </div>
</template>

<style scoped>
.worldbooks-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 16px;
  gap: 16px;
  overflow: hidden;
}

.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-card {
  max-width: 400px;
}

.empty-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 24px;
}

.empty-text {
  font-size: 14px;
}

.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.worldbook-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.worldbook-desc {
  font-size: 13px;
}

.header-actions {
  flex-shrink: 0;
}

.global-settings-card {
  flex-shrink: 0;
}

.global-settings-summary {
  display: flex;
  align-items: center;
}

.setting-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.setting-label {
  font-size: 11px;
}

.setting-value {
  font-size: 14px;
  font-weight: 500;
}

.entry-editor-card {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.entry-editor-card :deep(.n-card__content) {
  height: 100%;
  overflow-y: auto;
}

.no-entry-selected {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
