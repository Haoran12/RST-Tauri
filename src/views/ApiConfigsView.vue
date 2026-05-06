<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  NButton,
  NCard,
  NEmpty,
  NForm,
  NFormItem,
  NGrid,
  NGi,
  NIcon,
  NInput,
  NList,
  NListItem,
  NModal,
  NSelect,
  NSpace,
  NSwitch,
  NTag,
  NText,
  NThing,
  useDialog,
  useMessage,
} from 'naive-ui'
import { CheckmarkCircleOutline, KeyOutline, RadioButtonOnOutline, RefreshOutline } from '@vicons/ionicons5'
import StructuredTextEditor from '@/components/shared/structured-text-editor/StructuredTextEditor.vue'
import { useSettingsStore } from '@/stores/settings'
import { useRuntimeStore } from '@/stores/runtime'
import type { ApiConfig } from '@/types/st'
import { DEFAULT_BINDINGS } from '@/types/structuredText'
import type { StructuredTextDiagnostic } from '@/types/structuredText'
import * as storage from '@/services/storage'

type ProviderPreset = {
  value: ApiConfig['provider']
  label: string
  description: string
  defaultBaseUrl?: string
  modelPlaceholder: string
}

const message = useMessage()
const dialog = useDialog()
const route = useRoute()
const settingsStore = useSettingsStore()
const runtimeStore = useRuntimeStore()

const settingsEditorRef = ref<InstanceType<typeof StructuredTextEditor> | null>(null)
const selectedId = ref<string | null>(null)
const draft = ref<ApiConfig | null>(null)
const settingsText = ref('{\n  \n}')
const settingsDiagnostics = ref<StructuredTextDiagnostic[]>([])
const parsedSettingsValue = ref<unknown>({})
const showCreateModal = ref(false)
const createName = ref('')
const createProvider = ref<ApiConfig['provider']>('openai_responses')
const isSaving = ref(false)
const isLoadingModels = ref(false)
const availableModels = ref<storage.ModelInfo[]>([])

const providerPresets: ProviderPreset[] = [
  {
    value: 'openai_responses',
    label: 'OpenAI Responses',
    description: '结构化输出优先，适合通用 ST 与 Agent 节点。',
    defaultBaseUrl: 'https://api.openai.com/v1',
    modelPlaceholder: 'gpt-4.1 / o4-mini',
  },
  {
    value: 'openai_chat',
    label: 'OpenAI Chat Completions',
    description: 'OpenAI 兼容消息接口，也作为部分兼容 Provider 的基础协议。',
    defaultBaseUrl: 'https://api.openai.com/v1',
    modelPlaceholder: 'gpt-4.1-mini / gpt-4o-mini',
  },
  {
    value: 'anthropic',
    label: 'Anthropic Messages',
    description: '原生 system + messages 协议，支持结构化输出与多模态。',
    defaultBaseUrl: 'https://api.anthropic.com/v1',
    modelPlaceholder: 'claude-sonnet-4-0',
  },
  {
    value: 'gemini',
    label: 'Google Gemini',
    description: 'GenerateContent / streamGenerateContent 请求形态。',
    defaultBaseUrl: 'https://generativelanguage.googleapis.com/v1beta',
    modelPlaceholder: 'gemini-2.5-pro',
  },
  {
    value: 'deepseek',
    label: 'DeepSeek Chat',
    description: 'OpenAI Chat 兼容外形，但能力与错误处理独立适配。',
    defaultBaseUrl: 'https://api.deepseek.com/v1',
    modelPlaceholder: 'deepseek-chat',
  },
  {
    value: 'claude_code',
    label: 'Claude Code Interface',
    description: 'Claude Code 风格兼容面；变体配置通过 settings JSON 承载。',
    modelPlaceholder: 'claude-sonnet-4-0',
  },
]

const providerOptions = providerPresets.map((item) => ({
  label: item.label,
  value: item.value,
}))

const sortedConfigs = computed(() =>
  [...settingsStore.apiConfigs].sort(
    (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
  ),
)

const activeConfigId = computed(() => runtimeStore.activeApiConfigId)
const selectedProviderPreset = computed(() =>
  providerPresets.find((item) => item.value === draft.value?.provider) ?? providerPresets[0],
)
const canSaveDraft = computed(() => {
  if (!draft.value) return false
  if (!draft.value.name.trim()) return false
  return !settingsDiagnostics.value.some((item) => item.severity === 'blocker')
})

const modelOptions = computed(() => {
  if (availableModels.value.length === 0) return []
  return availableModels.value.map((m) => ({
    label: m.display_name || m.id,
    value: m.id,
  }))
})

function maskSecret(value?: string | null) {
  if (!value) return '未设置'
  if (value.length <= 8) return '已设置'
  return `${value.slice(0, 4)}...${value.slice(-4)}`
}

function providerLabel(provider: string) {
  return providerPresets.find((item) => item.value === provider)?.label ?? provider
}

function formatTime(value: string) {
  return new Date(value).toLocaleString()
}

function statusTags(config: ApiConfig) {
  const tags: Array<{ text: string; type: 'success' | 'warning' | 'default' }> = []
  if (activeConfigId.value === config.id) {
    tags.push({ text: '当前激活', type: 'success' })
  }
  if (!config.enabled) {
    tags.push({ text: '已禁用', type: 'warning' })
  }
  if (!config.api_key) {
    tags.push({ text: '缺少 Key', type: 'warning' })
  }
  if (tags.length === 0) {
    tags.push({ text: '可用', type: 'default' })
  }
  return tags
}

function createDraftFromConfig(config: ApiConfig) {
  draft.value = {
    ...config,
    settings: { ...config.settings },
  }
  settingsText.value = JSON.stringify(config.settings ?? {}, null, 2)
  parsedSettingsValue.value = { ...config.settings }
  settingsDiagnostics.value = []
  availableModels.value = []
}

function createBlankConfig(name: string, provider: ApiConfig['provider']): ApiConfig {
  const preset = providerPresets.find((item) => item.value === provider) ?? providerPresets[0]
  const now = new Date().toISOString()
  return {
    id: crypto.randomUUID(),
    name,
    provider,
    model: '',
    base_url: preset.defaultBaseUrl,
    api_key: '',
    enabled: true,
    settings: {},
    created_at: now,
    updated_at: now,
  }
}

function syncSelectedConfig() {
  if (!selectedId.value) {
    draft.value = null
    return
  }
  const config = settingsStore.apiConfigs.find((item) => item.id === selectedId.value)
  if (!config) {
    selectedId.value = sortedConfigs.value[0]?.id ?? null
    if (!selectedId.value) {
      draft.value = null
      return
    }
    return syncSelectedConfig()
  }
  createDraftFromConfig(config)
}

async function hydrate() {
  await Promise.all([
    settingsStore.loadApiConfigs(),
    runtimeStore.loadGlobalState(),
  ])
  settingsStore.setActiveApiConfig(runtimeStore.activeApiConfigId)

  if (!selectedId.value && sortedConfigs.value.length > 0) {
    const requestedId = typeof route.query.config === 'string' ? route.query.config : ''
    selectedId.value = requestedId || activeConfigId.value || sortedConfigs.value[0].id
  }
  syncSelectedConfig()
}

async function validateSettingsEditor() {
  const result = settingsEditorRef.value
    ? await settingsEditorRef.value.validate()
    : null
  if (!result) return null
  settingsText.value = result.text
  settingsDiagnostics.value = result.diagnostics
  parsedSettingsValue.value = result.parsedValue
  if (!result.parsedValue || typeof result.parsedValue !== 'object' || Array.isArray(result.parsedValue)) {
    message.error('settings 必须是 JSON 对象。')
    return null
  }
  if (result.diagnostics.some((item) => item.severity === 'blocker')) {
    message.error('settings JSON 仍有 blocker，无法保存。')
    return null
  }
  return result.parsedValue as Record<string, unknown>
}

async function handleSave() {
  if (!draft.value) return
  if (!draft.value.name.trim()) {
    message.error('名称不能为空。')
    return
  }

  const nextSettings = await validateSettingsEditor()
  if (!nextSettings) return

  isSaving.value = true
  try {
    await settingsStore.updateApiConfig(draft.value.id, {
      name: draft.value.name.trim(),
      provider: draft.value.provider,
      model: draft.value.model.trim(),
      base_url: draft.value.base_url?.trim() || undefined,
      api_key: draft.value.api_key?.trim() || undefined,
      enabled: draft.value.enabled,
      settings: nextSettings,
    })
    await settingsStore.loadApiConfigs()
    settingsStore.setActiveApiConfig(runtimeStore.activeApiConfigId)
    syncSelectedConfig()
    message.success('API 配置已保存。')
  } catch (error) {
    message.error(String(error))
  } finally {
    isSaving.value = false
  }
}

async function persistDraftConnectionForModelFetch() {
  if (!draft.value) return

  const existingSettings =
    settingsStore.apiConfigs.find((item) => item.id === draft.value?.id)?.settings ??
    draft.value.settings ??
    {}

  await settingsStore.updateApiConfig(draft.value.id, {
    name: draft.value.name.trim(),
    provider: draft.value.provider,
    model: draft.value.model.trim(),
    base_url: draft.value.base_url?.trim() || undefined,
    api_key: draft.value.api_key?.trim() || undefined,
    enabled: draft.value.enabled,
    settings: existingSettings,
  })
}

async function handleSetActive(id: string | null) {
  await runtimeStore.setApiConfigId(id)
  settingsStore.setActiveApiConfig(id)
  message.success(id ? '已切换当前 API 配置。' : '已清空当前 API 配置。')
}

async function handleDelete(id: string) {
  dialog.warning({
    title: '删除 API 配置',
    content: '删除后不会改写 ST 会话、世界书、预设或附件引用，但会让依赖此配置的后续请求失效。',
    positiveText: '删除',
    negativeText: '取消',
    async onPositiveClick() {
      try {
        if (activeConfigId.value === id) {
          await handleSetActive(null)
        }
        await settingsStore.removeApiConfig(id)
        if (selectedId.value === id) {
          selectedId.value = sortedConfigs.value.find((item) => item.id !== id)?.id ?? null
        }
        await settingsStore.loadApiConfigs()
        syncSelectedConfig()
        message.success('API 配置已删除。')
      } catch (error) {
        message.error(String(error))
      }
    },
  })
}

async function handleCreate() {
  if (!createName.value.trim()) {
    message.error('请输入配置名称。')
    return
  }

  const config = createBlankConfig(createName.value.trim(), createProvider.value)
  try {
    await settingsStore.addApiConfig(config)
    await settingsStore.loadApiConfigs()
    selectedId.value = config.id
    syncSelectedConfig()
    showCreateModal.value = false
    createName.value = ''
    createProvider.value = 'openai_responses'
    message.success('API 配置已创建。')
  } catch (error) {
    message.error(String(error))
  }
}

async function handleFetchModels() {
  if (!draft.value) return
  const configId = draft.value.id

  if (!draft.value.api_key?.trim() && draft.value.provider !== 'claude_code') {
    message.warning('请先填写 API Key。')
    return
  }

  isLoadingModels.value = true
  availableModels.value = []

  try {
    await persistDraftConnectionForModelFetch()
    await settingsStore.loadApiConfigs()
    settingsStore.setActiveApiConfig(runtimeStore.activeApiConfigId)
    syncSelectedConfig()

    const models = await storage.listModels(configId)
    availableModels.value = models
    if (models.length === 0) {
      message.warning('未获取到可用模型，请检查 API Key 和 Base URL 是否正确。')
    } else {
      message.success(`获取到 ${models.length} 个可用模型。`)
    }
  } catch (error) {
    message.error(`获取模型列表失败: ${error}`)
  } finally {
    isLoadingModels.value = false
  }
}

function handleSelectModel(modelId: string) {
  if (draft.value) {
    draft.value.model = modelId
  }
}

watch(
  () => settingsStore.apiConfigs,
  () => {
    if (!selectedId.value && sortedConfigs.value.length > 0) {
      selectedId.value = sortedConfigs.value[0].id
    }
  },
  { deep: true },
)

watch(selectedId, () => {
  syncSelectedConfig()
})

watch(
  () => route.query.config,
  (value) => {
    if (typeof value === 'string' && value && value !== selectedId.value) {
      selectedId.value = value
    }
  },
)

onMounted(async () => {
  try {
    await hydrate()
  } catch (e) {
    console.error('Failed to hydrate API configs:', e)
  }
})
</script>

<template>
  <div class="api-configs-view">
    <header class="page-header">
      <div>
        <h1>API 配置池</h1>
        <p>共享给 ST 运行时与 Agent 节点绑定；切换当前配置只影响下一次请求，不改写资源文件。</p>
      </div>
      <NSpace>
        <NButton secondary @click="showCreateModal = true">新建配置</NButton>
        <NButton
          v-if="activeConfigId"
          quaternary
          @click="handleSetActive(null)"
        >
          清空当前激活
        </NButton>
      </NSpace>
    </header>

    <section class="content-grid">
      <NCard class="list-panel" size="small" title="配置列表">
        <template #header-extra>
          <NTag :type="activeConfigId ? 'success' : 'warning'">
            {{ activeConfigId ? '已选择当前配置' : '未选择当前配置' }}
          </NTag>
        </template>

        <div v-if="sortedConfigs.length === 0" class="empty-panel">
          <NEmpty description="还没有 API 配置">
            <template #extra>
              <NButton type="primary" @click="showCreateModal = true">创建第一个配置</NButton>
            </template>
          </NEmpty>
        </div>

        <NList v-else hoverable clickable class="config-list">
          <NListItem
            v-for="config in sortedConfigs"
            :key="config.id"
            class="config-row"
            :class="{ selected: selectedId === config.id }"
            @click="selectedId = config.id"
          >
            <NThing :title="config.name" :description="providerLabel(config.provider)">
              <template #header-extra>
                <NSpace :size="6">
                  <NTag
                    v-for="tag in statusTags(config)"
                    :key="`${config.id}-${tag.text}`"
                    size="small"
                    :type="tag.type"
                  >
                    {{ tag.text }}
                  </NTag>
                </NSpace>
              </template>
              <template #description>
                <div class="config-meta">
                  <span>{{ config.model || '未填写模型' }}</span>
                  <span>{{ formatTime(config.updated_at) }}</span>
                </div>
              </template>
              <div class="config-actions">
                <NButton
                  size="tiny"
                  secondary
                  :type="activeConfigId === config.id ? 'primary' : 'default'"
                  @click.stop="handleSetActive(config.id)"
                >
                  <template #icon>
                    <NIcon><RadioButtonOnOutline /></NIcon>
                  </template>
                  设为当前
                </NButton>
                <NButton
                  size="tiny"
                  quaternary
                  type="error"
                  @click.stop="handleDelete(config.id)"
                >
                  删除
                </NButton>
              </div>
            </NThing>
          </NListItem>
        </NList>
      </NCard>

      <NCard class="editor-panel" size="small" title="配置详情">
        <div v-if="!draft" class="empty-panel">
          <NEmpty description="从左侧选择一个配置开始编辑" />
        </div>

        <template v-else>
          <div class="editor-scroll-area">
            <div class="detail-head">
            <div class="detail-title">
              <h2>{{ draft.name || '未命名配置' }}</h2>
              <NSpace :size="8">
                <NTag v-if="activeConfigId === draft.id" type="success">
                  <template #icon>
                    <NIcon><CheckmarkCircleOutline /></NIcon>
                  </template>
                  当前激活
                </NTag>
                <NTag :type="draft.enabled ? 'default' : 'warning'">
                  {{ draft.enabled ? '已启用' : '已禁用' }}
                </NTag>
                <NTag :type="draft.api_key ? 'default' : 'warning'">
                  {{ draft.api_key ? '已填写 Key' : '缺少 Key' }}
                </NTag>
              </NSpace>
            </div>
            <div class="detail-meta">
              <span>ID: {{ draft.id }}</span>
              <span>创建于 {{ formatTime(draft.created_at) }}</span>
            </div>
          </div>

          <NGrid :cols="2" :x-gap="16">
            <NGi>
              <NForm label-placement="top">
                <NFormItem label="名称" required>
                  <NInput v-model:value="draft.name" placeholder="例如：OpenAI 主账号 / DeepSeek 备用" />
                </NFormItem>
                <NFormItem label="Provider" required>
                  <NSelect v-model:value="draft.provider" :options="providerOptions" />
                </NFormItem>
                <NFormItem label="模型">
                  <NSpace vertical style="width: 100%">
                    <NSpace style="width: 100%">
                      <NInput
                        v-model:value="draft.model"
                        :placeholder="selectedProviderPreset.modelPlaceholder"
                        style="flex: 1"
                      />
                      <NButton
                        secondary
                        :loading="isLoadingModels"
                        :disabled="!draft.api_key"
                        @click="handleFetchModels"
                      >
                        <template #icon>
                          <NIcon><RefreshOutline /></NIcon>
                        </template>
                        获取模型
                      </NButton>
                    </NSpace>
                    <NSelect
                      v-if="availableModels.length > 0"
                      :value="draft.model"
                      :options="modelOptions"
                      placeholder="从可用模型中选择"
                      clearable
                      filterable
                      @update:value="handleSelectModel"
                    />
                  </NSpace>
                </NFormItem>
                <NFormItem label="Base URL">
                  <NInput
                    v-model:value="draft.base_url"
                    :placeholder="selectedProviderPreset.defaultBaseUrl || '使用默认端点'"
                  />
                </NFormItem>
              </NForm>
            </NGi>

            <NGi>
              <NForm label-placement="top">
                <NFormItem label="API Key">
                  <NInput
                    v-model:value="draft.api_key"
                    type="password"
                    show-password-on="click"
                    placeholder="不会在日志中明文落库"
                  />
                </NFormItem>
                <NFormItem label="启用状态">
                  <div class="switch-row">
                    <NSwitch v-model:value="draft.enabled" />
                    <NText depth="3">
                      {{ draft.enabled ? '允许作为可选连接参与后续发送' : '保留配置但暂不使用' }}
                    </NText>
                  </div>
                </NFormItem>
                <NFormItem label="当前摘要">
                  <div class="summary-box">
                    <div class="summary-row">
                      <span>Provider</span>
                      <strong>{{ providerLabel(draft.provider) }}</strong>
                    </div>
                    <div class="summary-row">
                      <span>Key</span>
                      <code>{{ maskSecret(draft.api_key) }}</code>
                    </div>
                    <div class="summary-row">
                      <span>用途</span>
                      <span>{{ selectedProviderPreset.description }}</span>
                    </div>
                  </div>
                </NFormItem>
              </NForm>
            </NGi>
          </NGrid>

          <NForm label-placement="top" class="settings-form">
            <NFormItem label="settings JSON">
              <StructuredTextEditor
                ref="settingsEditorRef"
                :model-value="settingsText"
                mode="json"
                :binding="DEFAULT_BINDINGS.st_extensions"
                :min-height="260"
                :use-backend-validation="true"
                @update:model-value="(value) => { settingsText = value }"
                @diagnostics-change="(diagnostics) => { settingsDiagnostics = diagnostics }"
                @parsed-value-change="(value) => { parsedSettingsValue = value }"
              />
            </NFormItem>
          </NForm>
          </div>

          <div class="footer-actions">
            <NText depth="3">
              更新 `provider` / `model` / `base_url` 只影响后续请求映射与契约缓存，不改写聊天、世界书、预设或附件引用。
            </NText>
            <NSpace>
              <NButton
                secondary
                :type="activeConfigId === draft.id ? 'primary' : 'default'"
                @click="handleSetActive(draft.id)"
              >
                设为当前配置
              </NButton>
              <NButton type="primary" :loading="isSaving" :disabled="!canSaveDraft" @click="handleSave">
                保存配置
              </NButton>
            </NSpace>
          </div>
        </template>
      </NCard>
    </section>

    <NModal
      v-model:show="showCreateModal"
      preset="card"
      title="新建 API 配置"
      style="width: min(92vw, 520px)"
    >
      <NForm label-placement="top">
        <NFormItem label="名称" required>
          <NInput v-model:value="createName" placeholder="例如：OpenAI 主账号" />
        </NFormItem>
        <NFormItem label="Provider" required>
          <NSelect v-model:value="createProvider" :options="providerOptions" />
        </NFormItem>
      </NForm>
      <div class="modal-tip">
        <KeyOutline />
        <span>创建后请补充模型、Key 和可选 settings；敏感字段只保存到本地 `./data/api_configs/`。</span>
      </div>
      <div class="modal-actions">
        <NButton @click="showCreateModal = false">取消</NButton>
        <NButton type="primary" @click="handleCreate">创建</NButton>
      </div>
    </NModal>
  </div>
</template>

<style scoped>
.api-configs-view {
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--color-bg-app, #f0f2f5);
}

.page-header {
  padding: 22px 24px 18px;
  border-bottom: 1px solid var(--color-border-subtle, rgba(15, 23, 42, 0.08));
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
  background: var(--color-bg-surface, rgba(255, 255, 255, 0.84));
  backdrop-filter: blur(16px);
  flex-shrink: 0;
}

.page-header h1 {
  margin: 0;
  font-size: 22px;
  line-height: 1.1;
}

.page-header p {
  margin: 6px 0 0;
  max-width: 760px;
  color: var(--color-text-secondary, #526071);
}

.content-grid {
  flex: 1;
  min-height: 0;
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(280px, 340px) minmax(0, 1fr);
  gap: 16px;
  padding: 16px 20px 20px;
  overflow: hidden;
}

.list-panel,
.editor-panel {
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  border-radius: 8px;
  box-shadow: 0 18px 44px rgba(15, 23, 42, 0.08);
  display: flex;
  flex-direction: column;
}

.list-panel :deep(.n-card__content),
.editor-panel :deep(.n-card__content) {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  flex: 1;
}

.editor-panel .detail-head,
.editor-panel .n-grid,
.editor-panel .settings-form {
  flex-shrink: 0;
}

.editor-panel .footer-actions {
  flex-shrink: 0;
  position: sticky;
  bottom: 0;
  background: var(--color-bg-surface, rgba(255, 255, 255, 0.95));
  padding: 16px 0;
  margin: 0;
  border-top: 1px solid rgba(15, 23, 42, 0.08);
  z-index: 10;
}

.editor-scroll-area {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-bottom: 16px;
  scrollbar-width: thin;
}

.editor-scroll-area::-webkit-scrollbar {
  width: 6px;
}

.editor-scroll-area::-webkit-scrollbar-track {
  background: transparent;
}

.editor-scroll-area::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 3px;
}

.editor-scroll-area::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

.config-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  scrollbar-width: thin;
}

.config-list::-webkit-scrollbar {
  width: 6px;
}

.config-list::-webkit-scrollbar-track {
  background: transparent;
}

.config-list::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 3px;
}

.config-list::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

.config-row {
  border-radius: 14px;
  transition: background-color 0.2s ease;
}

.config-row.selected {
  background: rgba(32, 128, 240, 0.08);
}

.config-meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 4px;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.config-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.empty-panel {
  min-height: 240px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
}

.detail-head {
  display: grid;
  gap: 8px;
  margin-bottom: 18px;
  flex-shrink: 0;
}

.detail-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.detail-title h2 {
  margin: 0;
  font-size: 20px;
}

.detail-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.switch-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.summary-box {
  display: grid;
  gap: 10px;
  width: 100%;
  padding: 14px 16px;
  border: 1px solid var(--color-border-subtle, rgba(15, 23, 42, 0.08));
  border-radius: 14px;
  background: var(--color-bg-subtle, rgba(248, 250, 252, 0.92));
}

.summary-row {
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr);
  gap: 12px;
  align-items: start;
  font-size: 13px;
}

.summary-row code {
  font-size: 12px;
}

.settings-form {
  margin-top: 8px;
  flex-shrink: 0;
}

.footer-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-shrink: 0;
}

.modal-tip {
  display: grid;
  grid-template-columns: 18px 1fr;
  gap: 10px;
  margin-top: 8px;
  padding: 12px 14px;
  border-radius: 12px;
  background: rgba(32, 128, 240, 0.08);
  color: var(--color-text-secondary, #36506b);
}

.modal-actions {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

@media (max-width: 1180px) {
  .content-grid {
    grid-template-columns: 1fr;
    overflow: auto;
  }
}

@media (max-width: 840px) {
  .page-header,
  .footer-actions,
  .detail-title {
    flex-direction: column;
    align-items: flex-start;
  }

  .content-grid {
    padding: 12px;
  }
}
</style>
