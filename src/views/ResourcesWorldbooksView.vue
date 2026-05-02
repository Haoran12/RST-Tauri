<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import {
  NCard,
  NSpace,
  NButton,
  NInput,
  NModal,
  NForm,
  NFormItem,
  NDataTable,
  NPopconfirm,
  NUpload,
  useMessage,
  type DataTableColumns,
  type UploadCustomRequestOptions,
} from 'naive-ui'
import { useWorldbooksStore } from '@/stores/worldbooks'
import WorldbookEntryList from '@/components/st/worldbook/WorldbookEntryList.vue'
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

// Layout state
const showEditor = ref(true)

// Load worldbooks on mount
onMounted(() => {
  store.loadWorldbooks()
})

// Table columns for worldbook list
const columns: DataTableColumns<{ id: string; name: string; description: string; entry_count: number }> = [
  {
    title: 'Name',
    key: 'name',
    ellipsis: { tooltip: true },
  },
  {
    title: 'Description',
    key: 'description',
    ellipsis: { tooltip: true },
  },
  {
    title: 'Entries',
    key: 'entry_count',
    width: 80,
  },
  {
    title: 'Actions',
    key: 'actions',
    width: 180,
    render(row) {
      return h(NSpace, { size: 'small' }, () => [
        h(NButton, { size: 'small', onClick: () => openWorldbook(row.id) }, () => 'Open'),
        h(NButton, { size: 'small', onClick: () => openEditMeta(row) }, () => 'Edit'),
        h(
          NPopconfirm,
          { onPositiveClick: () => deleteWorldbook(row.id) },
          {
            trigger: () => h(NButton, { size: 'small', type: 'error' }, () => 'Delete'),
            default: () => 'Delete this worldbook?',
          }
        ),
      ])
    },
  },
]

// Open worldbook for editing
async function openWorldbook(id: string) {
  await store.loadWorldbook(id)
  showEditor.value = true
}

// Create new worldbook
async function createWorldbook() {
  if (!createName.value.trim()) {
    message.error('Name is required')
    return
  }

  try {
    const id = await store.createNewWorldbook(createName.value.trim())
    message.success('Worldbook created')
    showCreateModal.value = false
    createName.value = ''
    await openWorldbook(id)
  } catch (e) {
    message.error(String(e))
  }
}

// Delete worldbook
async function deleteWorldbook(id: string) {
  try {
    await store.deleteWorldbookById(id)
    message.success('Worldbook deleted')
  } catch (e) {
    message.error(String(e))
  }
}

// Open edit meta modal
function openEditMeta(row: { id: string; name: string; description: string; entry_count: number }) {
  editName.value = row.name
  editDescription.value = row.description
  showEditMetaModal.value = true
}

// Save meta changes
async function saveMeta() {
  if (!editName.value.trim()) {
    message.error('Name is required')
    return
  }

  try {
    await store.updateMeta(editName.value.trim(), editDescription.value.trim())
    message.success('Worldbook updated')
    showEditMetaModal.value = false
  } catch (e) {
    message.error(String(e))
  }
}

// Create new entry
async function createEntry() {
  try {
    await store.createNewEntry()
    message.success('Entry created')
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
    message.success('Entry deleted')
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
    message.success('Worldbook imported')
    await openWorldbook(id)
  } catch (e) {
    message.error(String(e))
  }
}

// Handle export
async function handleExport() {
  if (!store.currentWorldbook) return

  const item = store.worldbookList.find(
    (w) => w.name === store.currentWorldbook!.name
  )
  if (!item) return

  try {
    const blob = await store.exportToFile(item.id)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${store.currentWorldbook.name}.json`
    a.click()
    URL.revokeObjectURL(url)
    message.success('Worldbook exported')
  } catch (e) {
    message.error(String(e))
  }
}

// Close editor
function closeEditor() {
  store.clearCurrentWorldbook()
  showEditor.value = false
}
</script>

<template>
  <div class="worldbooks-view">
    <!-- Worldbook List Panel -->
    <NCard class="worldbook-list-panel" title="Worldbooks">
      <template #header-extra>
        <NSpace>
          <NButton size="small" type="primary" @click="showCreateModal = true">
            + New
          </NButton>
          <NUpload
            :show-file-list="false"
            accept=".json"
            :custom-request="handleImport"
          >
            <NButton size="small">Import</NButton>
          </NUpload>
        </NSpace>
      </template>

      <NDataTable
        :columns="columns"
        :data="store.worldbookList"
        :loading="store.isLoading"
        :bordered="false"
        size="small"
        :max-height="400"
      />
    </NCard>

    <!-- Worldbook Editor Panel -->
    <NCard
      v-if="showEditor && store.currentWorldbook"
      class="worldbook-editor-panel"
      :title="store.currentWorldbook.name || 'Worldbook Editor'"
    >
      <template #header-extra>
        <NSpace>
          <NButton size="small" @click="showEditMetaModal = true">
            Edit Meta
          </NButton>
          <NButton size="small" @click="handleExport">
            Export
          </NButton>
          <NButton size="small" @click="closeEditor">
            Close
          </NButton>
        </NSpace>
      </template>

      <div class="editor-layout">
        <!-- Entry List -->
        <div class="entry-list-column">
          <WorldbookEntryList
            :entries="store.sortedEntries"
            :selected-uid="store.currentEntryUid"
            @select="(uid) => store.selectEntry(uid)"
            @create="createEntry"
          />
        </div>

        <!-- Entry Editor -->
        <div class="entry-editor-column">
          <WorldbookEntryEditor
            :entry="store.currentEntry"
            :groups="store.groups"
            @update="(entry) => updateEntry(store.currentEntryUid!, entry)"
            @delete="deleteEntry(store.currentEntryUid!)"
          />
        </div>
      </div>
    </NCard>

    <!-- Create Modal -->
    <NModal
      v-model:show="showCreateModal"
      preset="dialog"
      title="Create New Worldbook"
      positive-text="Create"
      negative-text="Cancel"
      @positive-click="createWorldbook"
    >
      <NForm>
        <NFormItem label="Name" required>
          <NInput
            v-model:value="createName"
            placeholder="Worldbook name"
          />
        </NFormItem>
      </NForm>
    </NModal>

    <!-- Edit Meta Modal -->
    <NModal
      v-model:show="showEditMetaModal"
      preset="dialog"
      title="Edit Worldbook Metadata"
      positive-text="Save"
      negative-text="Cancel"
      @positive-click="saveMeta"
    >
      <NForm>
        <NFormItem label="Name" required>
          <NInput
            v-model:value="editName"
            placeholder="Worldbook name"
          />
        </NFormItem>
        <NFormItem label="Description">
          <NInput
            v-model:value="editDescription"
            type="textarea"
            placeholder="Worldbook description"
          />
        </NFormItem>
      </NForm>
    </NModal>
  </div>
</template>

<style scoped>
.worldbooks-view {
  padding: 24px;
  display: flex;
  gap: 24px;
  min-height: calc(100vh - 120px);
}

.worldbook-list-panel {
  width: 400px;
  flex-shrink: 0;
}

.worldbook-editor-panel {
  flex: 1;
  min-width: 600px;
}

.editor-layout {
  display: flex;
  gap: 16px;
  height: calc(100vh - 200px);
}

.entry-list-column {
  width: 300px;
  flex-shrink: 0;
  border-right: 1px solid var(--n-border-color);
}

.entry-editor-column {
  flex: 1;
  overflow-y: auto;
}
</style>