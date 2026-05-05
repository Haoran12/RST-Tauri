<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
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
  NStatistic,
  NTag,
  NUpload,
  useMessage,
  type UploadCustomRequestOptions,
} from 'naive-ui'
import { usePresetsStore, type PresetSectionKey } from '@/stores/presets'
import { useRuntimeStore } from '@/stores/runtime'
import type { PresetFile } from '@/types/preset'

const store = usePresetsStore()
const runtimeStore = useRuntimeStore()
const message = useMessage()

const showCreateModal = ref(false)
const createName = ref('')
const showMetaModal = ref(false)
const metaName = ref('')
const editorText = ref('')

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
const configuredSectionCount = computed(() => {
  if (!store.currentPreset) return 0
  return (Object.keys(sectionLabels) as PresetSectionKey[]).filter((key) =>
    Boolean(store.currentPreset?.[key]),
  ).length
})

function getSectionValue() {
  if (!store.currentPreset) return {}
  return store.currentPreset[store.currentSection] ?? {}
}

function syncEditorText() {
  editorText.value = JSON.stringify(getSectionValue(), null, 2)
}

function openMetaModal() {
  metaName.value = store.currentPreset?.name ?? ''
  showMetaModal.value = true
}

onMounted(async () => {
  await Promise.all([store.loadPresetList(), runtimeStore.loadGlobalState()])
  if (!store.currentPreset && store.presetList[0]) {
    const activeName = runtimeStore.globalState.active_preset
    const preferred = store.presetList.find((preset) => preset.name === activeName)
    await store.loadPreset((preferred ?? store.presetList[0]).name)
  }
  syncEditorText()
})

watch(
  () => [store.currentPreset?.name, store.currentSection],
  () => syncEditorText(),
)

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
    syncEditorText()
    message.success('预设已创建')
  } catch (e) {
    message.error(String(e))
  }
}

async function saveMeta() {
  if (!store.currentPreset || !metaName.value.trim()) {
    message.error('名称不能为空')
    return
  }

  const oldName = store.currentPreset.name
  const newName = metaName.value.trim()
  try {
    if (oldName !== newName) {
      if (oldName === 'Default') {
        message.error('默认预设不能重命名')
        return
      }
      await store.renamePreset(oldName, newName)
      if (runtimeStore.globalState.active_preset === oldName) {
        await runtimeStore.setPresetName(newName)
      }
    }
    showMetaModal.value = false
    syncEditorText()
    message.success('元数据已保存')
  } catch (e) {
    message.error(String(e))
  }
}

async function saveCurrentSection() {
  if (!store.currentPreset) return

  try {
    const parsed = editorText.value.trim() ? JSON.parse(editorText.value) : {}
    const preset = store.currentPreset as PresetFile
    preset[store.currentSection] = parsed
    await store.savePreset(preset)
    message.success('预设已保存')
  } catch (e) {
    message.error(`JSON 无效：${String(e)}`)
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

async function deleteCurrentPreset() {
  if (!store.currentPreset) return

  const deletedName = store.currentPreset.name
  try {
    await store.deletePreset(deletedName)
    if (runtimeStore.globalState.active_preset === deletedName) {
      await runtimeStore.setPresetName('Default')
    }
    if (store.presetList[0]) {
      await store.loadPreset(store.presetList[0].name)
    }
    syncEditorText()
    message.success('预设已删除')
  } catch (e) {
    message.error(String(e))
  }
}

async function handleImport(options: UploadCustomRequestOptions) {
  const file = options.file.file
  if (!file) return

  try {
    await store.importPreset(file)
    syncEditorText()
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
</script>

<template>
  <div class="presets-view">
    <NCard class="preset-editor-panel" :title="currentTitle">
      <template #header-extra>
        <NSpace size="small">
          <NTag v-if="isActivePreset" size="small" type="success">当前</NTag>
          <NButton size="small" @click="showCreateModal = true">
            新建
          </NButton>
          <NUpload
            :show-file-list="false"
            accept=".json"
            :custom-request="handleImport"
          >
            <NButton size="small">导入</NButton>
          </NUpload>
          <NButton size="small" :disabled="!store.currentPreset" @click="openMetaModal">
            元数据
          </NButton>
          <NButton size="small" :disabled="!store.currentPreset" @click="handleExport">
            导出
          </NButton>
          <NButton
            size="small"
            :disabled="!store.currentPreset || isActivePreset"
            @click="activateCurrentPreset"
          >
            设为当前
          </NButton>
          <NButton
            size="small"
            type="primary"
            :disabled="!store.currentPreset"
            @click="saveCurrentSection"
          >
            保存
          </NButton>
          <NPopconfirm @positive-click="deleteCurrentPreset">
            <template #trigger>
              <NButton
                size="small"
                type="error"
                :disabled="!store.currentPreset || isDefaultPreset"
              >
                删除
              </NButton>
            </template>
            确定删除当前预设？
          </NPopconfirm>
        </NSpace>
      </template>

      <div v-if="store.currentPreset" class="editor-content">
        <div class="preset-summary">
          <div>
            <h2 class="section-title">{{ currentSectionLabel }}</h2>
            <p class="section-desc">{{ currentSectionDescription }}</p>
          </div>
          <NSpace>
            <NStatistic label="分区" :value="configuredSectionCount" />
            <NStatistic label="文件" :value="store.presetList.length" />
          </NSpace>
        </div>

        <NInput
          v-model:value="editorText"
          type="textarea"
          class="json-editor"
          placeholder="{}"
          :autosize="false"
        />
      </div>

      <NEmpty v-else class="empty-state" description="暂无预设" />
    </NCard>

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

    <NModal
      v-model:show="showMetaModal"
      preset="dialog"
      title="预设元数据"
      positive-text="保存"
      negative-text="取消"
      @positive-click="saveMeta"
    >
      <NForm>
        <NFormItem label="名称" required>
          <NInput v-model:value="metaName" placeholder="预设名称" :disabled="isDefaultPreset" />
        </NFormItem>
      </NForm>
    </NModal>
  </div>
</template>

<style scoped>
.presets-view {
  height: 100%;
  min-height: 0;
  padding: 16px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.preset-editor-panel {
  flex: 1;
  min-height: 0;
}

.preset-editor-panel :deep(.n-card__content) {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  flex: 1;
}

.editor-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.preset-summary {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 24px;
  padding: 4px 0 16px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  margin-bottom: 12px;
}

.section-title {
  margin: 0 0 8px;
  font-size: 20px;
  font-weight: 600;
}

.section-desc {
  margin: 0;
  font-size: 13px;
  color: var(--color-text-secondary, #6b7280);
}

.json-editor {
  flex: 1;
  min-height: 0;
}

.json-editor :deep(textarea) {
  height: 100% !important;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  line-height: 1.5;
}

.empty-state {
  height: 100%;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
