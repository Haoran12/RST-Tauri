<script setup lang="ts">
import { ref } from 'vue'
import { NSelect, NButton, NIcon, NInput, NPopconfirm, NTooltip } from 'naive-ui'
import { AddOutline, CreateOutline, TrashOutline, DownloadOutline, CloudUploadOutline } from '@vicons/ionicons5'

export type SelectorOption = { label: string; value: string }

const props = withDefaults(
  defineProps<{
    options: SelectorOption[]
    selectedName: string | null
    placeholder?: string
    canRename?: boolean
    canDelete?: boolean
    canExport?: boolean
    canImport?: boolean
  }>(),
  {
    placeholder: '选择配置...',
    canRename: true,
    canDelete: true,
    canExport: true,
    canImport: true,
  }
)

const emit = defineEmits<{
  (e: 'select', name: string): void
  (e: 'create', name: string): void
  (e: 'rename', newName: string): void
  (e: 'delete'): void
  (e: 'export'): void
  (e: 'import', file: File): void
}>()

const newName = ref('')
const renameValue = ref('')
const isCreatingMode = ref(false)
const isRenamingMode = ref(false)

const startCreate = () => {
  newName.value = ''
  isCreatingMode.value = true
  isRenamingMode.value = false
}

const handleCreate = () => {
  if (newName.value.trim()) {
    emit('create', newName.value.trim())
    isCreatingMode.value = false
  }
}

const startRename = () => {
  if (props.selectedName) {
    renameValue.value = props.selectedName
    isRenamingMode.value = true
    isCreatingMode.value = false
  }
}

const handleRename = () => {
  if (renameValue.value.trim() && renameValue.value !== props.selectedName) {
    emit('rename', renameValue.value.trim())
  }
  isRenamingMode.value = false
}

const cancel = () => {
  isCreatingMode.value = false
  isRenamingMode.value = false
}

const fileInput = ref<HTMLInputElement | null>(null)

const triggerImport = () => {
  fileInput.value?.click()
}

const handleFileChange = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files && target.files[0]) {
    emit('import', target.files[0])
    target.value = ''
  }
}
</script>

<template>
  <div class="config-manager">
    <!-- Creating Mode -->
    <div v-if="isCreatingMode" class="create-mode">
      <div class="mode-header">
        <span class="mode-label">新建配置</span>
        <NButton quaternary size="tiny" @click="cancel">✕</NButton>
      </div>
      <div class="mode-input-row">
        <NInput
          v-model:value="newName"
          :placeholder="`输入名称...`"
          size="small"
          @keyup.enter="handleCreate"
        />
        <NButton type="primary" size="small" @click="handleCreate">创建</NButton>
      </div>
    </div>

    <!-- Renaming Mode -->
    <div v-else-if="isRenamingMode" class="rename-mode">
      <div class="mode-header">
        <span class="mode-label">重命名</span>
        <NButton quaternary size="tiny" @click="cancel">✕</NButton>
      </div>
      <div class="mode-input-row">
        <NInput
          v-model:value="renameValue"
          size="small"
          @keyup.enter="handleRename"
        />
        <NButton type="primary" size="small" @click="handleRename">保存</NButton>
      </div>
    </div>

    <!-- Selection Mode -->
    <div v-else class="select-mode">
      <NSelect
        :value="selectedName"
        :options="options"
        :placeholder="placeholder"
        size="small"
        class="config-select"
        @update:value="(v) => v && emit('select', v)"
      />

      <div class="action-buttons">
        <NTooltip trigger="hover" placement="bottom">
          <template #trigger>
            <NButton quaternary size="tiny" @click="startCreate">
              <template #icon>
                <NIcon :size="16"><AddOutline /></NIcon>
              </template>
            </NButton>
          </template>
          新建
        </NTooltip>

        <NTooltip v-if="canRename" trigger="hover" placement="bottom">
          <template #trigger>
            <NButton quaternary size="tiny" :disabled="!selectedName" @click="startRename">
              <template #icon>
                <NIcon :size="16"><CreateOutline /></NIcon>
              </template>
            </NButton>
          </template>
          重命名
        </NTooltip>

        <NPopconfirm v-if="canDelete" @positive-click="emit('delete')">
          <template #trigger>
            <NTooltip trigger="hover" placement="bottom">
              <template #trigger>
                <NButton quaternary size="tiny" :disabled="!selectedName" type="error">
                  <template #icon>
                    <NIcon :size="16"><TrashOutline /></NIcon>
                  </template>
                </NButton>
              </template>
              删除
            </NTooltip>
          </template>
          确定删除此配置？
        </NPopconfirm>

        <div class="button-divider" />

        <NTooltip v-if="canExport" trigger="hover" placement="bottom">
          <template #trigger>
            <NButton quaternary size="tiny" :disabled="!selectedName" @click="emit('export')">
              <template #icon>
                <NIcon :size="16"><DownloadOutline /></NIcon>
              </template>
            </NButton>
          </template>
          导出
        </NTooltip>

        <NTooltip v-if="canImport" trigger="hover" placement="bottom">
          <template #trigger>
            <NButton quaternary size="tiny" @click="triggerImport">
              <template #icon>
                <NIcon :size="16"><CloudUploadOutline /></NIcon>
              </template>
            </NButton>
          </template>
          导入
        </NTooltip>

        <input
          type="file"
          ref="fileInput"
          class="hidden-input"
          accept=".json"
          @change="handleFileChange"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-manager {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.create-mode,
.rename-mode {
  padding: 8px;
  border-radius: 6px;
  background: var(--color-bg-subtle, #f5f5f5);
}

.create-mode {
  border: 1px solid var(--color-success, #18a058);
}

.rename-mode {
  border: 1px solid var(--color-primary, #2080f0);
}

.mode-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.mode-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary, #666);
}

.mode-input-row {
  display: flex;
  gap: 8px;
}

.mode-input-row .n-input {
  flex: 1;
}

.select-mode {
  display: flex;
  align-items: center;
  gap: 8px;
}

.config-select {
  flex: 1;
  min-width: 0;
}

.action-buttons {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.button-divider {
  width: 1px;
  height: 16px;
  background: var(--color-border, #e0e0e6);
  margin: 0 4px;
}

.hidden-input {
  display: none;
}
</style>
