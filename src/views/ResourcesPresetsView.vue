<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  NButton,
  NCard,
  NEmpty,
  NForm,
  NFormItem,
  NInput,
  NModal,
  NPopconfirm,
  NSpace,
  NSelect,
  NTag,
  NUpload,
  useMessage,
  type UploadCustomRequestOptions,
} from 'naive-ui'
import { usePresetsStore, type PresetSectionKey } from '@/stores/presets'
import { useRuntimeStore } from '@/stores/runtime'
import type { PromptItem } from '@/types/preset'

const store = usePresetsStore()
const runtimeStore = useRuntimeStore()
const message = useMessage()

const showCreateModal = ref(false)
const createName = ref('')
const showEditModal = ref(false)
const editIdentifier = ref('')
const editName = ref('')
const editRole = ref<'system' | 'user' | 'assistant'>('system')
const editContent = ref('')

const sectionLabels: Record<PresetSectionKey, string> = {
  sampler: 'Sampler',
  instruct: 'Instruct',
  context: 'Context',
  sysprompt: 'System Prompt',
  reasoning: 'Reasoning',
  prompt: 'Prompt',
}

const sectionDescriptions: Record<PresetSectionKey, string> = {
  sampler: '采样、惩罚、DRY、Mirostat 与 Provider 字段覆盖。',
  instruct: '用户/助手/系统消息序列、停止符和名称行为。',
  context: '角色故事串、示例分隔、上下文插入位置和裁剪选项。',
  sysprompt: '预设级系统提示词，不包含 API 连接、模型或鉴权信息。',
  reasoning: '推理参数模板和扩展字段，最终由 Provider 映射层裁剪。',
  prompt: 'Prompt 条目、排序和世界书/角色字段格式化模板。',
}

const currentTitle = computed(() => store.currentPreset?.name || '预设')
const currentSectionLabel = computed(() => sectionLabels[store.currentSection])
const currentSectionDescription = computed(() => sectionDescriptions[store.currentSection])
const isDefaultPreset = computed(() => store.currentPreset?.name === 'Default')
const isActivePreset = computed(
  () => !!store.currentPreset && runtimeStore.globalState.active_preset === store.currentPreset.name,
)

const roleOptions = [
  { label: 'System', value: 'system' },
  { label: 'User', value: 'user' },
  { label: 'Assistant', value: 'assistant' },
]

function openCreateModal() {
  showCreateModal.value = true
}

function openEditModal(item: PromptItem) {
  editIdentifier.value = item.identifier
  editName.value = item.name
  editRole.value = item.role
  editContent.value = item.content || ''
  showEditModal.value = true
}

onMounted(async () => {
  window.addEventListener('open-preset-create', openCreateModal)
  await Promise.all([store.loadPresetList(), runtimeStore.loadGlobalState()])
  if (!store.currentPreset && store.presetList[0]) {
    const activeName = runtimeStore.globalState.active_preset
    const preferred = store.presetList.find((preset) => preset.name === activeName)
    await store.loadPreset((preferred ?? store.presetList[0]).name)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('open-preset-create', openCreateModal)
})

async function createPreset() {
  if (!createName.value.trim()) {
    message.error('名称不能为空')
    return
  }

  try {
    store.createNewPreset(createName.value.trim())
    if (store.currentPreset) {
      await store.savePreset(store.currentPreset)
    }
    createName.value = ''
    showCreateModal.value = false
    message.success('预设已创建')
  } catch (e) {
    message.error(String(e))
  }
}

async function savePromptItem() {
  if (!store.currentPreset?.prompt?.prompts) return

  const prompts = store.currentPreset.prompt.prompts
  const index = prompts.findIndex((p) => p.identifier === editIdentifier.value)
  if (index >= 0) {
    prompts[index] = {
      identifier: editIdentifier.value,
      name: editName.value,
      role: editRole.value,
      content: editContent.value,
    }
    await store.savePreset(store.currentPreset)
    message.success('提示词已保存')
    showEditModal.value = false
  }
}

async function activateCurrentPreset() {
  if (!store.currentPreset) return

  try {
    await runtimeStore.setPresetName(store.currentPreset.name)
    message.success('当前预设已切换')
  } catch (e) {
    message.error(String(e))
  }
}

async function handleImport(options: UploadCustomRequestOptions) {
  const file = options.file.file
  if (!file) return

  try {
    await store.importPreset(file)
    message.success('预设已导入')
  } catch (e) {
    message.error(String(e))
  }
}

async function handleExport() {
  if (!store.currentPreset) return

  try {
    const blob = await store.exportPreset(store.currentPreset.name)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${store.currentPreset.name}.json`
    a.click()
    URL.revokeObjectURL(url)
    message.success('预设已导出')
  } catch (e) {
    message.error(String(e))
  }
}

// Listen for prompt item edit event from ContextList
function handleEditPromptItem(event: CustomEvent<PromptItem>) {
  openEditModal(event.detail)
}

onMounted(() => {
  window.addEventListener('edit-prompt-item', handleEditPromptItem as EventListener)
})

onBeforeUnmount(() => {
  window.removeEventListener('edit-prompt-item', handleEditPromptItem as EventListener)
})
</script>

<template>
  <div class="presets-view">
    <!-- Empty state when no preset selected -->
    <div v-if="!store.currentPreset" class="empty-state">
      <NCard class="empty-card">
        <div class="empty-content">
          <NEmpty description="请在左侧面板选择或创建预设" />
        </div>
      </NCard>
    </div>

    <!-- Preset Editor when preset is selected -->
    <template v-else>
      <!-- Header with actions -->
      <div class="editor-header">
        <div class="header-left">
          <h2 class="preset-title">{{ store.currentPreset.name }}</h2>
          <NSpace>
            <NTag v-if="isActivePreset" size="small" type="success">当前</NTag>
          </NSpace>
        </div>
        <NSpace class="header-actions">
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
          <NButton
            size="small"
            :disabled="isActivePreset"
            @click="activateCurrentPreset"
          >
            设为当前
          </NButton>
        </NSpace>
      </div>

      <!-- Section Tabs -->
      <div class="section-tabs">
        <NButton
          v-for="(label, key) in sectionLabels"
          :key="key"
          size="small"
          :type="store.currentSection === key ? 'primary' : 'default'"
          @click="store.selectSection(key as PresetSectionKey)"
        >
          {{ label }}
        </NButton>
      </div>

      <!-- Section Editor -->
      <NCard class="section-editor-card">
        <div class="section-header">
          <h3 class="section-title">{{ currentSectionLabel }}</h3>
          <p class="section-desc">{{ currentSectionDescription }}</p>
        </div>

        <!-- Prompt section shows prompt items -->
        <template v-if="store.currentSection === 'prompt'">
          <div class="prompt-section">
            <NEmpty description="提示词条目在左侧面板管理" />
          </div>
        </template>

        <!-- Other sections show JSON editor -->
        <template v-else>
          <div class="json-section">
            <p class="json-hint">此分区通过 JSON 编辑，请在左侧面板选择 Prompt 分区管理提示词条目。</p>
          </div>
        </template>
      </NCard>
    </template>

    <!-- Create Modal -->
    <NModal
      v-model:show="showCreateModal"
      preset="dialog"
      title="新建预设"
      positive-text="创建"
      negative-text="取消"
      @positive-click="createPreset"
    >
      <NForm>
        <NFormItem label="名称" required>
          <NInput v-model:value="createName" placeholder="预设名称" />
        </NFormItem>
      </NForm>
    </NModal>

    <!-- Edit Prompt Item Modal -->
    <NModal
      v-model:show="showEditModal"
      preset="card"
      title="编辑提示词"
      style="width: 600px"
    >
      <NForm label-placement="left" label-width="80px">
        <NFormItem label="标识符">
          <NInput :value="editIdentifier" disabled />
        </NFormItem>
        <NFormItem label="名称">
          <NInput v-model:value="editName" placeholder="提示词名称" />
        </NFormItem>
        <NFormItem label="角色">
          <NSelect
            v-model:value="editRole"
            :options="roleOptions"
            placeholder="选择角色"
          />
        </NFormItem>
        <NFormItem label="内容">
          <NInput
            v-model:value="editContent"
            type="textarea"
            placeholder="提示词内容"
            :autosize="{ minRows: 5, maxRows: 15 }"
          />
        </NFormItem>
      </NForm>
      <template #footer>
        <NSpace justify="end">
          <NButton @click="showEditModal = false">取消</NButton>
          <NButton type="primary" @click="savePromptItem">保存</NButton>
        </NSpace>
      </template>
    </NModal>
  </div>
</template>

<style scoped>
.presets-view {
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

.preset-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.header-actions {
  flex-shrink: 0;
}

.section-tabs {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.section-editor-card {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.section-editor-card :deep(.n-card__content) {
  height: 100%;
  overflow-y: auto;
}

.section-header {
  margin-bottom: 16px;
}

.section-title {
  margin: 0 0 8px;
  font-size: 16px;
  font-weight: 600;
}

.section-desc {
  margin: 0;
  font-size: 13px;
  color: var(--color-text-secondary, #6b7280);
}

.prompt-section {
  padding: 24px;
}

.json-section {
  padding: 24px;
}

.json-hint {
  margin: 0;
  font-size: 14px;
  color: var(--color-text-secondary, #6b7280);
}
</style>