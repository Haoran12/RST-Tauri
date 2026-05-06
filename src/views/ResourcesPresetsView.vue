<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  NButton,
  NCard,
  NEmpty,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NModal,
  NSpace,
  NSelect,
  NSwitch,
  NTag,
  NUpload,
  useMessage,
  type UploadCustomRequestOptions,
} from 'naive-ui'
import { usePresetsStore, type PresetSectionKey } from '@/stores/presets'
import { useRuntimeStore } from '@/stores/runtime'
import type { PromptItem, PromptOrder, PromptOrderItem } from '@/types/preset'

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

const fixedPromptIdentifiers = new Set([
  'main',
  'nsfw',
  'dialogueExamples',
  'jailbreak',
  'chatHistory',
  'worldInfoAfter',
  'worldInfoBefore',
  'enhanceDefinitions',
  'charDescription',
  'charPersonality',
  'scenario',
  'personaDescription',
])

const currentSectionLabel = computed(() => sectionLabels[store.currentSection])
const currentSectionDescription = computed(() => sectionDescriptions[store.currentSection])
const isActivePreset = computed(
  () => !!store.currentPreset && runtimeStore.globalState.active_preset === store.currentPreset.name,
)

const roleOptions = [
  { label: 'System', value: 'system' },
  { label: 'User', value: 'user' },
  { label: 'Assistant', value: 'assistant' },
]

const namesBehaviorOptions = [
  { label: 'None', value: 'none' },
  { label: 'Force', value: 'force' },
  { label: 'Always', value: 'always' },
]

const editablePromptItems = computed(() => {
  const prompts = store.currentPreset?.prompts ?? []
  const orderItems = activePromptOrder.value?.order ?? []
  const enabledMap = new Map(orderItems.map((item) => [item.identifier, item.enabled !== false]))
  return prompts.map((item) => ({
    ...item,
    enabled: enabledMap.get(item.identifier) ?? true,
    fixed: fixedPromptIdentifiers.has(item.identifier),
  }))
})

const activePromptOrder = computed<PromptOrder | null>(() => {
  const preset = store.currentPreset
  if (!preset?.prompt_order?.length) return null
  return preset.prompt_order.find((item) => item.character_id === 100000) ?? preset.prompt_order[0] ?? null
})

function openCreateModal() {
  showCreateModal.value = true
}

function ensurePromptOrder(): PromptOrder {
  const preset = store.currentPreset!
  if (!preset.prompt_order || preset.prompt_order.length === 0) {
    preset.prompt_order = [{ character_id: 100000, order: [] }]
  }
  let order = preset.prompt_order.find((item) => item.character_id === 100000)
  if (!order) {
    order = { character_id: 100000, order: [] }
    preset.prompt_order.unshift(order)
  }
  if (!order.order) {
    order.order = []
  }
  return order
}

function ensurePromptOrderItem(identifier: string): PromptOrderItem {
  const order = ensurePromptOrder()
  let item = order.order!.find((entry) => entry.identifier === identifier)
  if (!item) {
    item = { identifier, enabled: true }
    order.order!.push(item)
  }
  return item
}

function openEditModal(item: PromptItem) {
  editIdentifier.value = item.identifier
  editName.value = item.name
  editRole.value = item.role
  editContent.value = item.content || ''
  showEditModal.value = true
}

async function persistCurrentPreset(successText = '预设已保存') {
  if (!store.currentPreset) return
  try {
    await store.savePreset(store.currentPreset)
    message.success(successText)
  } catch (e) {
    message.error(String(e))
  }
}

function setPresetField<K extends keyof NonNullable<typeof store.currentPreset>>(key: K, value: NonNullable<typeof store.currentPreset>[K]) {
  if (!store.currentPreset) return
  store.currentPreset[key] = value
}

function updateNested<T extends object, K extends keyof T>(target: T | undefined, key: K, value: T[K]) {
  if (!target) return
  target[key] = value
}

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
  if (!store.currentPreset?.prompts) return

  const prompts = store.currentPreset.prompts
  const index = prompts.findIndex((p) => p.identifier === editIdentifier.value)
  if (index >= 0) {
    prompts[index] = {
      ...prompts[index],
      identifier: editIdentifier.value,
      name: editName.value,
      role: editRole.value,
      content: editContent.value,
    }
    await persistCurrentPreset('提示词已保存')
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

function handleEditPromptItem(event: CustomEvent<PromptItem>) {
  openEditModal(event.detail)
}

async function togglePromptEnabled(identifier: string, enabled: boolean) {
  ensurePromptOrderItem(identifier).enabled = enabled
  await persistCurrentPreset('提示词启用状态已更新')
}

onMounted(async () => {
  window.addEventListener('open-preset-create', openCreateModal)
  window.addEventListener('edit-prompt-item', handleEditPromptItem as EventListener)
  try {
    await Promise.all([store.loadPresetList(), runtimeStore.loadGlobalState()])
    if (!store.currentPreset && store.presetList[0]) {
      const activeName = runtimeStore.globalState.active_preset
      const preferred = store.presetList.find((preset) => preset.name === activeName)
      await store.loadPreset((preferred ?? store.presetList[0]).name)
    }
  } catch (e) {
    console.error('Failed to load presets:', e)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('open-preset-create', openCreateModal)
  window.removeEventListener('edit-prompt-item', handleEditPromptItem as EventListener)
})
</script>

<template>
  <div class="presets-view">
    <div v-if="!store.currentPreset" class="empty-state">
      <NCard class="empty-card">
        <div class="empty-content">
          <NEmpty description="请在左侧面板选择或创建预设" />
        </div>
      </NCard>
    </div>

    <template v-else>
      <div class="editor-header">
        <div class="header-left">
          <h2 class="preset-title">{{ store.currentPreset.name }}</h2>
          <NSpace>
            <NTag v-if="isActivePreset" size="small" type="success">当前</NTag>
          </NSpace>
        </div>
        <NSpace class="header-actions">
          <NUpload :show-file-list="false" accept=".json" :custom-request="handleImport">
            <NButton size="small">导入</NButton>
          </NUpload>
          <NButton size="small" @click="handleExport">导出</NButton>
          <NButton size="small" @click="persistCurrentPreset()">保存</NButton>
          <NButton size="small" :disabled="isActivePreset" @click="activateCurrentPreset">
            设为当前
          </NButton>
        </NSpace>
      </div>

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

      <NCard class="section-editor-card">
        <div class="section-header">
          <h3 class="section-title">{{ currentSectionLabel }}</h3>
          <p class="section-desc">{{ currentSectionDescription }}</p>
        </div>

        <template v-if="store.currentSection === 'sampler'">
          <NForm label-placement="top" class="preset-form">
            <div class="form-grid">
              <NFormItem label="Temperature">
                <NInputNumber
                  :value="store.currentPreset.temperature"
                  :step="0.1"
                  @update:value="value => setPresetField('temperature', value ?? 1)"
                />
              </NFormItem>
              <NFormItem label="Top P">
                <NInputNumber
                  :value="store.currentPreset.top_p"
                  :step="0.05"
                  @update:value="value => setPresetField('top_p', value ?? 1)"
                />
              </NFormItem>
              <NFormItem label="Top K">
                <NInputNumber
                  :value="store.currentPreset.top_k"
                  @update:value="value => setPresetField('top_k', value ?? 0)"
                />
              </NFormItem>
              <NFormItem label="Top A">
                <NInputNumber
                  :value="store.currentPreset.top_a"
                  :step="0.05"
                  @update:value="value => setPresetField('top_a', value ?? 0)"
                />
              </NFormItem>
              <NFormItem label="Min P">
                <NInputNumber
                  :value="store.currentPreset.min_p"
                  :step="0.05"
                  @update:value="value => setPresetField('min_p', value ?? 0)"
                />
              </NFormItem>
              <NFormItem label="Typical P">
                <NInputNumber
                  :value="store.currentPreset.typical_p"
                  :step="0.05"
                  @update:value="value => setPresetField('typical_p', value ?? 0)"
                />
              </NFormItem>
              <NFormItem label="TFS">
                <NInputNumber
                  :value="store.currentPreset.tfs"
                  :step="0.05"
                  @update:value="value => setPresetField('tfs', value ?? 0)"
                />
              </NFormItem>
              <NFormItem label="Repetition Penalty">
                <NInputNumber
                  :value="store.currentPreset.repetition_penalty"
                  :step="0.05"
                  @update:value="value => setPresetField('repetition_penalty', value ?? 1)"
                />
              </NFormItem>
              <NFormItem label="Frequency Penalty">
                <NInputNumber
                  :value="store.currentPreset.frequency_penalty"
                  :step="0.05"
                  @update:value="value => setPresetField('frequency_penalty', value ?? 0)"
                />
              </NFormItem>
              <NFormItem label="Presence Penalty">
                <NInputNumber
                  :value="store.currentPreset.presence_penalty"
                  :step="0.05"
                  @update:value="value => setPresetField('presence_penalty', value ?? 0)"
                />
              </NFormItem>
              <NFormItem label="OpenAI Max Context">
                <NInputNumber
                  :value="store.currentPreset.openai_max_context"
                  @update:value="value => setPresetField('openai_max_context', value ?? 4095)"
                />
              </NFormItem>
              <NFormItem label="OpenAI Max Tokens">
                <NInputNumber
                  :value="store.currentPreset.openai_max_tokens"
                  @update:value="value => setPresetField('openai_max_tokens', value ?? 300)"
                />
              </NFormItem>
            </div>
            <NFormItem label="Negative Prompt">
              <NInput
                type="textarea"
                :value="store.currentPreset.negative_prompt"
                :autosize="{ minRows: 3, maxRows: 8 }"
                @update:value="value => setPresetField('negative_prompt', value)"
              />
            </NFormItem>
            <div class="switch-grid">
              <NFormItem label="Use System Prompt">
                <NSwitch
                  :value="store.currentPreset.use_sysprompt"
                  @update:value="value => setPresetField('use_sysprompt', value)"
                />
              </NFormItem>
              <NFormItem label="Stream OpenAI">
                <NSwitch
                  :value="store.currentPreset.stream_openai"
                  @update:value="value => setPresetField('stream_openai', value)"
                />
              </NFormItem>
              <NFormItem label="Max Context Unlocked">
                <NSwitch
                  :value="store.currentPreset.max_context_unlocked"
                  @update:value="value => setPresetField('max_context_unlocked', value)"
                />
              </NFormItem>
              <NFormItem label="Temperature Last">
                <NSwitch
                  :value="store.currentPreset.temperature_last"
                  @update:value="value => setPresetField('temperature_last', value)"
                />
              </NFormItem>
            </div>
          </NForm>
        </template>

        <template v-else-if="store.currentSection === 'instruct' && store.currentPreset.instruct">
          <NForm label-placement="top" class="preset-form">
            <div class="form-grid">
              <NFormItem label="Input Sequence">
                <NInput
                  :value="store.currentPreset.instruct.input_sequence"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'input_sequence', value)"
                />
              </NFormItem>
              <NFormItem label="Output Sequence">
                <NInput
                  :value="store.currentPreset.instruct.output_sequence"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'output_sequence', value)"
                />
              </NFormItem>
              <NFormItem label="System Sequence">
                <NInput
                  :value="store.currentPreset.instruct.system_sequence"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'system_sequence', value)"
                />
              </NFormItem>
              <NFormItem label="Stop Sequence">
                <NInput
                  :value="store.currentPreset.instruct.stop_sequence"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'stop_sequence', value)"
                />
              </NFormItem>
              <NFormItem label="Input Suffix">
                <NInput
                  :value="store.currentPreset.instruct.input_suffix"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'input_suffix', value)"
                />
              </NFormItem>
              <NFormItem label="Output Suffix">
                <NInput
                  :value="store.currentPreset.instruct.output_suffix"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'output_suffix', value)"
                />
              </NFormItem>
              <NFormItem label="System Suffix">
                <NInput
                  :value="store.currentPreset.instruct.system_suffix"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'system_suffix', value)"
                />
              </NFormItem>
              <NFormItem label="Names Behavior">
                <NSelect
                  :value="store.currentPreset.instruct.names_behavior"
                  :options="namesBehaviorOptions"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'names_behavior', value)"
                />
              </NFormItem>
            </div>
            <NFormItem label="Story String Prefix">
              <NInput
                :value="store.currentPreset.instruct.story_string_prefix"
                @update:value="value => updateNested(store.currentPreset?.instruct, 'story_string_prefix', value)"
              />
            </NFormItem>
            <NFormItem label="Story String Suffix">
              <NInput
                :value="store.currentPreset.instruct.story_string_suffix"
                @update:value="value => updateNested(store.currentPreset?.instruct, 'story_string_suffix', value)"
              />
            </NFormItem>
            <NFormItem label="Activation Regex">
              <NInput
                :value="store.currentPreset.instruct.activation_regex"
                @update:value="value => updateNested(store.currentPreset?.instruct, 'activation_regex', value)"
              />
            </NFormItem>
            <div class="switch-grid">
              <NFormItem label="Wrap">
                <NSwitch
                  :value="store.currentPreset.instruct.wrap"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'wrap', value)"
                />
              </NFormItem>
              <NFormItem label="Macro">
                <NSwitch
                  :value="store.currentPreset.instruct.macro"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'macro', value)"
                />
              </NFormItem>
              <NFormItem label="System Same As User">
                <NSwitch
                  :value="store.currentPreset.instruct.system_same_as_user"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'system_same_as_user', value)"
                />
              </NFormItem>
              <NFormItem label="Skip Examples">
                <NSwitch
                  :value="store.currentPreset.instruct.skip_examples"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'skip_examples', value)"
                />
              </NFormItem>
              <NFormItem label="Sequences As Stop Strings">
                <NSwitch
                  :value="store.currentPreset.instruct.sequences_as_stop_strings"
                  @update:value="value => updateNested(store.currentPreset?.instruct, 'sequences_as_stop_strings', value)"
                />
              </NFormItem>
            </div>
          </NForm>
        </template>

        <template v-else-if="store.currentSection === 'context' && store.currentPreset.context">
          <NForm label-placement="top" class="preset-form">
            <NFormItem label="Story String">
              <NInput
                type="textarea"
                :value="store.currentPreset.context.story_string"
                :autosize="{ minRows: 6, maxRows: 16 }"
                @update:value="value => updateNested(store.currentPreset?.context, 'story_string', value)"
              />
            </NFormItem>
            <div class="form-grid">
              <NFormItem label="Example Separator">
                <NInput
                  :value="store.currentPreset.context.example_separator"
                  @update:value="value => updateNested(store.currentPreset?.context, 'example_separator', value)"
                />
              </NFormItem>
              <NFormItem label="Chat Start">
                <NInput
                  :value="store.currentPreset.context.chat_start"
                  @update:value="value => updateNested(store.currentPreset?.context, 'chat_start', value)"
                />
              </NFormItem>
              <NFormItem label="Story Position">
                <NInputNumber
                  :value="store.currentPreset.context.story_string_position"
                  @update:value="value => updateNested(store.currentPreset?.context, 'story_string_position', value ?? 0)"
                />
              </NFormItem>
              <NFormItem label="Story Depth">
                <NInputNumber
                  :value="store.currentPreset.context.story_string_depth"
                  @update:value="value => updateNested(store.currentPreset?.context, 'story_string_depth', value ?? 4)"
                />
              </NFormItem>
              <NFormItem label="Story Role">
                <NInputNumber
                  :value="store.currentPreset.context.story_string_role"
                  @update:value="value => updateNested(store.currentPreset?.context, 'story_string_role', value ?? 0)"
                />
              </NFormItem>
            </div>
            <div class="switch-grid">
              <NFormItem label="Use Stop Strings">
                <NSwitch
                  :value="store.currentPreset.context.use_stop_strings"
                  @update:value="value => updateNested(store.currentPreset?.context, 'use_stop_strings', value)"
                />
              </NFormItem>
              <NFormItem label="Names As Stop Strings">
                <NSwitch
                  :value="store.currentPreset.context.names_as_stop_strings"
                  @update:value="value => updateNested(store.currentPreset?.context, 'names_as_stop_strings', value)"
                />
              </NFormItem>
              <NFormItem label="Always Force Name2">
                <NSwitch
                  :value="store.currentPreset.context.always_force_name2"
                  @update:value="value => updateNested(store.currentPreset?.context, 'always_force_name2', value)"
                />
              </NFormItem>
              <NFormItem label="Trim Sentences">
                <NSwitch
                  :value="store.currentPreset.context.trim_sentences"
                  @update:value="value => updateNested(store.currentPreset?.context, 'trim_sentences', value)"
                />
              </NFormItem>
              <NFormItem label="Single Line">
                <NSwitch
                  :value="store.currentPreset.context.single_line"
                  @update:value="value => updateNested(store.currentPreset?.context, 'single_line', value)"
                />
              </NFormItem>
            </div>
          </NForm>
        </template>

        <template v-else-if="store.currentSection === 'sysprompt' && store.currentPreset.sysprompt">
          <NForm label-placement="top" class="preset-form">
            <NFormItem label="System Prompt Content">
              <NInput
                type="textarea"
                :value="store.currentPreset.sysprompt.content"
                :autosize="{ minRows: 10, maxRows: 20 }"
                @update:value="value => updateNested(store.currentPreset?.sysprompt, 'content', value)"
              />
            </NFormItem>
          </NForm>
        </template>

        <template v-else-if="store.currentSection === 'reasoning' && store.currentPreset.reasoning">
          <NForm label-placement="top" class="preset-form">
            <div class="form-grid">
              <NFormItem label="Prefix">
                <NInput
                  :value="store.currentPreset.reasoning.prefix"
                  @update:value="value => updateNested(store.currentPreset?.reasoning, 'prefix', value)"
                />
              </NFormItem>
              <NFormItem label="Suffix">
                <NInput
                  :value="store.currentPreset.reasoning.suffix"
                  @update:value="value => updateNested(store.currentPreset?.reasoning, 'suffix', value)"
                />
              </NFormItem>
              <NFormItem label="Separator">
                <NInput
                  :value="store.currentPreset.reasoning.separator"
                  @update:value="value => updateNested(store.currentPreset?.reasoning, 'separator', value)"
                />
              </NFormItem>
            </div>
          </NForm>
        </template>

        <template v-else-if="store.currentSection === 'prompt'">
          <NForm label-placement="top" class="preset-form">
            <div class="form-grid">
              <NFormItem label="World Info Format">
                <NInput
                  :value="store.currentPreset.wi_format"
                  @update:value="value => setPresetField('wi_format', value)"
                />
              </NFormItem>
              <NFormItem label="Scenario Format">
                <NInput
                  :value="store.currentPreset.scenario_format"
                  @update:value="value => setPresetField('scenario_format', value)"
                />
              </NFormItem>
              <NFormItem label="Personality Format">
                <NInput
                  :value="store.currentPreset.personality_format"
                  @update:value="value => setPresetField('personality_format', value)"
                />
              </NFormItem>
            </div>

            <NFormItem label="New Chat Prompt">
              <NInput
                type="textarea"
                :value="store.currentPreset.new_chat_prompt"
                :autosize="{ minRows: 2, maxRows: 6 }"
                @update:value="value => setPresetField('new_chat_prompt', value)"
              />
            </NFormItem>
            <NFormItem label="New Group Chat Prompt">
              <NInput
                type="textarea"
                :value="store.currentPreset.new_group_chat_prompt"
                :autosize="{ minRows: 2, maxRows: 6 }"
                @update:value="value => setPresetField('new_group_chat_prompt', value)"
              />
            </NFormItem>
            <NFormItem label="New Example Chat Prompt">
              <NInput
                type="textarea"
                :value="store.currentPreset.new_example_chat_prompt"
                :autosize="{ minRows: 2, maxRows: 6 }"
                @update:value="value => setPresetField('new_example_chat_prompt', value)"
              />
            </NFormItem>
            <NFormItem label="Continue Nudge Prompt">
              <NInput
                type="textarea"
                :value="store.currentPreset.continue_nudge_prompt"
                :autosize="{ minRows: 2, maxRows: 6 }"
                @update:value="value => setPresetField('continue_nudge_prompt', value)"
              />
            </NFormItem>
            <NFormItem label="Group Nudge Prompt">
              <NInput
                type="textarea"
                :value="store.currentPreset.group_nudge_prompt"
                :autosize="{ minRows: 2, maxRows: 6 }"
                @update:value="value => setPresetField('group_nudge_prompt', value)"
              />
            </NFormItem>
            <NFormItem label="Impersonation Prompt">
              <NInput
                type="textarea"
                :value="store.currentPreset.impersonation_prompt"
                :autosize="{ minRows: 3, maxRows: 8 }"
                @update:value="value => setPresetField('impersonation_prompt', value)"
              />
            </NFormItem>

            <div class="prompt-items">
              <div
                v-for="item in editablePromptItems"
                :key="item.identifier"
                class="prompt-item-card"
              >
                <div class="prompt-item-header">
                  <div class="prompt-item-meta">
                    <strong>{{ item.name }}</strong>
                    <span class="prompt-item-id">{{ item.identifier }}</span>
                  </div>
                  <NSpace>
                    <NTag v-if="item.fixed" size="small">固定</NTag>
                    <NTag size="small" :type="item.role === 'system' ? 'warning' : item.role === 'user' ? 'info' : 'success'">
                      {{ item.role }}
                    </NTag>
                    <NSwitch
                      :value="item.enabled"
                      size="small"
                      @update:value="value => togglePromptEnabled(item.identifier, value)"
                    />
                    <NButton size="tiny" @click="openEditModal(item)">编辑</NButton>
                  </NSpace>
                </div>
                <div class="prompt-item-preview">{{ item.content || '动态内容 / 空内容' }}</div>
              </div>
            </div>
          </NForm>
        </template>
      </NCard>
    </template>

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
      v-model:show="showEditModal"
      preset="card"
      title="编辑提示词"
      style="width: 640px"
    >
      <NForm label-placement="left" label-width="96px">
        <NFormItem label="标识符">
          <NInput :value="editIdentifier" disabled />
        </NFormItem>
        <NFormItem label="名称">
          <NInput v-model:value="editName" placeholder="提示词名称" />
        </NFormItem>
        <NFormItem label="角色">
          <NSelect v-model:value="editRole" :options="roleOptions" placeholder="选择角色" />
        </NFormItem>
        <NFormItem label="内容">
          <NInput
            v-model:value="editContent"
            type="textarea"
            placeholder="提示词内容"
            :autosize="{ minRows: 6, maxRows: 16 }"
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
  min-height: 0;
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
  flex-wrap: wrap;
  gap: 8px;
  flex-shrink: 0;
}

.section-editor-card {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.section-editor-card :deep(.n-card-content) {
  min-height: 0;
  height: 100%;
  overflow-y: auto;
  scrollbar-gutter: stable;
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

.preset-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
}

.switch-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
}

.prompt-items {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.prompt-item-card {
  border: 1px solid rgba(148, 163, 184, 0.28);
  border-radius: 12px;
  padding: 12px;
  background: rgba(248, 250, 252, 0.7);
}

.prompt-item-header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
  margin-bottom: 8px;
}

.prompt-item-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.prompt-item-id {
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
}

.prompt-item-preview {
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 13px;
  color: var(--color-text-secondary, #4b5563);
}
</style>
