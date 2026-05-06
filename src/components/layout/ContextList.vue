<script setup lang="ts">
import {
  NList,
  NListItem,
  NEmpty,
  NSpin,
  NInput,
  NButton,
  NDropdown,
  NForm,
  NFormItem,
  NIcon,
  NSelect,
  NSwitch,
  NModal,
  NPopconfirm,
  NText,
  NTag,
  useMessage,
  useDialog,
} from 'naive-ui'
import { computed, ref, watch, h } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { SearchOutline, AddOutline, TrashOutline, SettingsOutline, ReorderFourOutline, EllipsisHorizontalOutline, SwapHorizontalOutline } from '@vicons/ionicons5'
import { useAppShellStore } from '@/stores/appShell'
import { useCharactersStore } from '@/stores/characters'
import { useChatStore } from '@/stores/chat'
import { usePresetsStore, type PresetSectionKey } from '@/stores/presets'
import { useWorldbooksStore } from '@/stores/worldbooks'
import type { WorldInfoEntry } from '@/types/st'
import { WorldInfoPosition } from '@/types/st'
import type { ChatSession } from '@/types/st'
import type { PromptItem } from '@/types/preset'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const dialog = useDialog()
const appShellStore = useAppShellStore()
const charactersStore = useCharactersStore()
const chatStore = useChatStore()
const presetsStore = usePresetsStore()
const worldbooksStore = useWorldbooksStore()
const searchQuery = ref('')
const editingStSessionId = ref<string | null>(null)
const editSessionName = ref('')
const editSessionCharacterId = ref<string | null>(null)
const editSessionWorldbooks = ref<string[]>([])
const editPersonaName = ref('')
const editPersonaDescription = ref('')
const isSavingSessionSettings = ref(false)
const switchingSessionId = ref<string | null>(null)

// Drag and drop state for prompt items
const draggedItem = ref<PromptItem | null>(null)
const dragOverItem = ref<PromptItem | null>(null)

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

type ContextItem = {
  id: string
  name: string
  type: string
  meta?: string
  active?: boolean
  session?: ChatSession
  action: () => unknown
}

const presetSectionLabels: Record<PresetSectionKey, string> = {
  sampler: 'Sampler',
  instruct: 'Instruct',
  context: 'Context',
  sysprompt: 'System Prompt',
  reasoning: 'Reasoning',
  prompt: 'Prompt',
}

// Computed page type
const isWorldbooksPage = computed(() => route.name === 'resources-worldbooks')
const isPresetsPage = computed(() => route.name === 'resources-presets')

// Worldbook file options for selector
const worldbookOptions = computed(() => {
  return worldbooksStore.worldbookList.map((wb) => ({
    label: wb.name || '未命名世界书',
    value: wb.id,
  }))
})

const characterOptions = computed(() => {
  return charactersStore.characters.map((item) => ({
    label: item.character.data.name || '未命名角色',
    value: item.id,
  }))
})

// Preset file options for selector
const presetOptions = computed(() => {
  return presetsStore.presetList.map((p) => ({
    label: p.name,
    value: p.name,
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
    'st-home': 'ST',
    'st-chat': '会话',
    'resources-characters': '角色卡',
    'resources-worldbooks': '世界书',
    'resources-presets': '预设',
    'resources-regex': 'Regex',
    'api-configs': 'API 配置',
    'logs': '日志',
  }
  return titles[route.name as string] || '列表'
})

const contextItems = computed<ContextItem[]>(() => {
  switch (route.name) {
    case 'st-home':
      return [
        ...appShellStore.recentSessions
          .filter(item => item.type === 'st')
          .map((item) => ({
          id: `session:${item.type}:${item.id}`,
          name: item.name,
          type: 'ST 会话',
          meta: formatShortTime(item.updatedAt),
          action: () => router.push({ name: 'st-chat', params: { sessionId: item.id } }),
        })),
        ...appShellStore.recentResources.map((item) => ({
          id: `resource:${item.type}:${item.id}`,
          name: item.name,
          type: item.type,
          meta: formatShortTime(item.updatedAt),
          action: () => undefined,
        })),
      ]
    case 'st-chat':
      return chatStore.sessions
        .slice()
        .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
        .map((session) => ({
          id: session.id,
          name: session.name || '未命名会话',
          type: '',
          meta: formatShortTime(session.updated_at),
          active: route.params.sessionId === session.id,
          session,
          action: () => router.push({ name: 'st-chat', params: { sessionId: session.id } }),
        }))
    case 'resources-characters':
      return charactersStore.characters.map((item) => ({
        id: item.id,
        name: item.character.data.name || '未命名角色',
        type: '角色卡',
        meta: item.character.data.creator_notes || item.character.data.description || undefined,
        active: route.query.character === item.id,
        action: () => router.replace({
          name: 'resources-characters',
          query: { character: item.id },
        }),
      }))
    case 'resources-presets':
      return [
        ...presetsStore.presetList.map((preset) => ({
          id: `preset:${preset.name}`,
          name: preset.name,
          type: '预设',
          active: presetsStore.currentPreset?.name === preset.name,
          action: () => presetsStore.loadPreset(preset.name),
        })),
        ...Object.entries(presetSectionLabels).map(([key, label]) => ({
          id: `section:${key}`,
          name: label,
          type: '分区',
          active: presetsStore.currentSection === key,
          action: () => presetsStore.selectSection(key as PresetSectionKey),
        })),
      ]
    default:
      return []
  }
})

const filteredItems = computed(() => {
  if (!searchQuery.value) return contextItems.value
  const query = searchQuery.value.toLowerCase()
  return contextItems.value.filter(item =>
    item.name.toLowerCase().includes(query) ||
    item.type.toLowerCase().includes(query) ||
    item.meta?.toLowerCase().includes(query)
  )
})

const isDefaultLoading = computed(() => {
  switch (route.name) {
    case 'resources-characters':
      return charactersStore.isLoading
    case 'resources-presets':
      return presetsStore.isLoading
    default:
      return false
  }
})

const showDefaultAddButton = computed(() => {
  return ['st-chat', 'resources-characters', 'resources-presets'].includes(route.name as string)
})

const defaultEmptyDescription = computed(() => {
  switch (route.name) {
    case 'st-chat':
      return '暂无会话，点击上方按钮创建'
    case 'resources-characters':
      return '暂无角色卡，请导入'
    case 'resources-presets':
      return '暂无预设'
    case 'resources-regex':
      return 'Regex 管理待实现'
    default:
      return '暂无数据'
  }
})

async function handleDefaultAdd() {
  switch (route.name) {
    case 'st-chat': {
      const name = `新会话 ${new Date().toLocaleString()}`
      await chatStore.createSession(name)
      if (chatStore.currentSession) {
        await router.push({ name: 'st-chat', params: { sessionId: chatStore.currentSession.id } })
      }
      break
    }
    case 'resources-characters':
      window.dispatchEvent(new CustomEvent('open-character-import'))
      break
    case 'resources-presets':
      window.dispatchEvent(new CustomEvent('open-preset-create'))
      break
  }
}

function openSessionSettings(session: ChatSession) {
  const metadata = session.chat_metadata ?? {}
  editingStSessionId.value = session.id
  editSessionName.value = session.name || '未命名会话'
  editSessionCharacterId.value = session.character_id ?? null
  editSessionWorldbooks.value = metadata.enabled_world_info ?? (metadata.world_info ? [metadata.world_info] : [])
  editPersonaName.value = metadata.user_persona?.name ?? ''
  editPersonaDescription.value = metadata.user_persona?.description ?? ''
}

async function saveSessionSettings() {
  if (!editingStSessionId.value) return
  isSavingSessionSettings.value = true
  try {
    await chatStore.updateSessionSettings(editingStSessionId.value, {
      name: editSessionName.value,
      character_id: editSessionCharacterId.value,
      enabled_world_info: editSessionWorldbooks.value,
      user_persona: {
        name: editPersonaName.value,
        description: editPersonaDescription.value,
      },
    })
    editingStSessionId.value = null
    message.success('会话设置已保存')
  } catch (err) {
    message.error(String(err))
  } finally {
    isSavingSessionSettings.value = false
  }
}

function handleSessionMenuSelect(key: string, session: ChatSession) {
  if (key === 'switch') {
    switchingSessionId.value = session.id
  } else if (key === 'delete') {
    confirmDeleteSession(session)
  }
}

function canSwitchSession(session: ChatSession): boolean {
  // 如果点击的是当前会话，不需要检查
  if (route.params.sessionId === session.id) return true
  // 检查当前会话是否有未完成的操作
  return !chatStore.isGenerating && !editingStSessionId.value
}

async function confirmSwitchSession() {
  if (!switchingSessionId.value) return
  const session = chatStore.sessions.find(s => s.id === switchingSessionId.value)
  if (!session) {
    switchingSessionId.value = null
    return
  }
  if (!canSwitchSession(session)) {
    message.warning('当前会话有未完成的操作，无法切换')
    switchingSessionId.value = null
    return
  }
  switchingSessionId.value = null
  await router.push({ name: 'st-chat', params: { sessionId: session.id } })
}

async function confirmDeleteSession(session: ChatSession) {
  dialog.warning({
    title: '删除会话',
    content: `确定删除会话「${session.name || '未命名会话'}」？此操作不可恢复。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await chatStore.deleteSession(session.id)
      message.success('会话已删除')
    },
  })
}

function getSessionMenuOptions(session: ChatSession) {
  const isCurrentSession = route.params.sessionId === session.id
  const canSwitch = canSwitchSession(session)
  return [
    {
      label: isCurrentSession ? '当前会话' : '切换会话',
      key: 'switch',
      disabled: isCurrentSession || !canSwitch,
      icon: () => h(NIcon, null, { default: () => h(SwapHorizontalOutline) }),
    },
    {
      label: '删除会话',
      key: 'delete',
      icon: () => h(NIcon, null, { default: () => h(TrashOutline) }),
    },
  ]
}

function formatShortTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleString()
}

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

// Handle preset selection
async function handlePresetSelect(name: string | null) {
  if (name) {
    await presetsStore.loadPreset(name)
  } else {
    presetsStore.clearCurrentPreset()
  }
}

// Select prompt item for right-side editing
function selectPromptItem(identifier: string) {
  presetsStore.selectPromptItem(identifier)
}

// Create new preset
function createPreset() {
  window.dispatchEvent(new CustomEvent('open-preset-create'))
}

// Delete current preset
async function deleteCurrentPreset() {
  if (!presetsStore.currentPreset) return
  await presetsStore.deletePreset(presetsStore.currentPreset.name)
}

function isFixedPromptItem(identifier: string): boolean {
  return fixedPromptIdentifiers.has(identifier)
}

// Check if prompt item is enabled
function isPromptEnabled(identifier: string): boolean {
  const order = presetsStore.currentPreset?.prompt_order?.[0]?.order
  if (!order) return true // Default enabled if no order specified
  const item = order.find((o) => o.identifier === identifier)
  return item?.enabled !== false
}

// Toggle prompt item enabled state
async function togglePromptEnabled(identifier: string, enabled: boolean) {
  if (!presetsStore.currentPreset) return
  const preset = presetsStore.currentPreset
  if (!preset.prompt_order || preset.prompt_order.length === 0) {
    preset.prompt_order = [{ order: [] }]
  }
  const order = preset.prompt_order[0].order || []
  const existingIndex = order.findIndex((o) => o.identifier === identifier)
  if (existingIndex >= 0) {
    order[existingIndex].enabled = enabled
  } else {
    order.push({ identifier, enabled })
  }
  preset.prompt_order[0].order = order
  await presetsStore.savePreset(preset)
}

// Delete prompt item
async function deletePromptItem(identifier: string) {
  if (isFixedPromptItem(identifier)) return
  if (!presetsStore.currentPreset?.prompts) return
  const preset = presetsStore.currentPreset
  // Remove from prompts array
  const prompts = (preset.prompts ?? []).filter((p) => p.identifier !== identifier)
  preset.prompts = prompts
  // Remove from order array
  if (preset.prompt_order?.[0]?.order) {
    preset.prompt_order[0].order = preset.prompt_order[0].order.filter(
      (o) => o.identifier !== identifier
    )
  }
  if (presetsStore.currentPromptIdentifier === identifier) {
    presetsStore.selectPromptItem(prompts[0]?.identifier ?? null)
  }
  await presetsStore.savePreset(preset)
}

// Create new prompt item
async function createPromptItem() {
  if (!presetsStore.currentPreset) return
  const preset = presetsStore.currentPreset
  if (!preset.prompts) {
    preset.prompts = []
  }
  // Generate unique identifier
  const existingIds = new Set(preset.prompts.map((p) => p.identifier))
  let counter = 1
  while (existingIds.has(`prompt_${counter}`)) {
    counter++
  }
  const newIdentifier = `prompt_${counter}`
  preset.prompts.push({
    identifier: newIdentifier,
    name: `新提示词 ${counter}`,
    role: 'system',
    content: '',
  })
  presetsStore.selectPromptItem(newIdentifier)
  await presetsStore.savePreset(preset)
}

// ============================================================================
// Drag and drop for prompt items
// ============================================================================

function onDragStart(event: DragEvent, item: PromptItem) {
  draggedItem.value = item
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', item.identifier)
  }
}

function onDragEnd() {
  draggedItem.value = null
  dragOverItem.value = null
}

function onDragOver(event: DragEvent, item: PromptItem) {
  event.preventDefault()
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
  if (draggedItem.value && draggedItem.value.identifier !== item.identifier) {
    dragOverItem.value = item
  }
}

function onDragLeave() {
  dragOverItem.value = null
}

async function onDrop(event: DragEvent, targetItem: PromptItem) {
  event.preventDefault()
  if (!draggedItem.value || draggedItem.value.identifier === targetItem.identifier) {
    return
  }

  const preset = presetsStore.currentPreset
  if (!preset?.prompts) return

  // 确保 prompt_order 存在
  if (!preset.prompt_order || preset.prompt_order.length === 0) {
    preset.prompt_order = [{ character_id: 100000, order: [] }]
  }

  // 获取当前 order 数组
  const currentOrder = preset.prompt_order[0].order ?? []

  // 创建新的 order 数组（确保响应式更新）
  const newOrder = [...currentOrder]

  // 获取 dragged 和 target 在 newOrder 中的索引
  const draggedOrderIndex = newOrder.findIndex(o => o.identifier === draggedItem.value!.identifier)
  const targetOrderIndex = newOrder.findIndex(o => o.identifier === targetItem.identifier)

  // 如果两个都在 order 中，直接重新排序
  if (draggedOrderIndex >= 0 && targetOrderIndex >= 0) {
    const [removed] = newOrder.splice(draggedOrderIndex, 1)
    newOrder.splice(targetOrderIndex, 0, removed)
  } else {
    // 如果其中一个不在 order 中，需要添加到 order 中
    // 先确保所有 prompts 都在 order 中
    preset.prompts.forEach((p) => {
      if (!newOrder.some(o => o.identifier === p.identifier)) {
        newOrder.push({ identifier: p.identifier, enabled: true })
      }
    })
    // 然后重新排序
    const newDraggedIndex = newOrder.findIndex(o => o.identifier === draggedItem.value!.identifier)
    const newTargetIndex = newOrder.findIndex(o => o.identifier === targetItem.identifier)
    if (newDraggedIndex >= 0 && newTargetIndex >= 0) {
      const [removed] = newOrder.splice(newDraggedIndex, 1)
      newOrder.splice(newTargetIndex, 0, removed)
    }
  }

  // 清除所有 position 字段，使用数组顺序
  newOrder.forEach(item => {
    delete item.position
  })

  // 更新所有 prompt_order 条目（保持一致性）
  preset.prompt_order = preset.prompt_order.map((orderEntry) => ({
    ...orderEntry,
    order: newOrder.map((item) => {
      // 保留原有的 enabled 状态
      const existing = orderEntry.order?.find((o) => o.identifier === item.identifier)
      return {
        identifier: item.identifier,
        enabled: existing?.enabled ?? true,
      }
    }),
  }))

  await presetsStore.savePreset(preset)

  draggedItem.value = null
  dragOverItem.value = null
}

// Get sorted prompt items with position from prompt_order
const sortedPromptItems = computed(() => {
  const prompts = presetsStore.currentPreset?.prompts
  if (!prompts) return []

  const order = presetsStore.currentPreset?.prompt_order?.[0]?.order
  if (!order) return [...prompts]

  // 构建 identifier -> 排序位置的映射
  // ST 格式：order 数组的顺序就是排序顺序，position 字段用于用户自定义覆盖
  const positionMap = new Map<string, number>()
  order.forEach((item, index) => {
    // 优先使用 position 字段，否则使用数组索引
    const pos = item.position ?? index
    positionMap.set(item.identifier, pos)
  })

  // 对于不在 order 中的 prompts，使用 prompts 数组中的原始索引加上一个偏移量
  const maxPosition = Math.max(...Array.from(positionMap.values()), -1)
  prompts.forEach((p, index) => {
    if (!positionMap.has(p.identifier)) {
      positionMap.set(p.identifier, maxPosition + 1 + index)
    }
  })

  const sorted = [...prompts].sort((a, b) => {
    const posA = positionMap.get(a.identifier) ?? 0
    const posB = positionMap.get(b.identifier) ?? 0
    return posA - posB
  })

  if (!searchQuery.value) {
    return sorted
  }

  const query = searchQuery.value.toLowerCase()
  return sorted.filter((item) => {
    return item.name.toLowerCase().includes(query) ||
      item.identifier.toLowerCase().includes(query) ||
      (item.content?.toLowerCase().includes(query) ?? false)
  })
})

// Load worldbooks when entering the page
watch(() => route.name, async (newName) => {
  if (newName === 'resources-worldbooks') {
    await worldbooksStore.loadWorldbooks()
  } else if (newName === 'st-chat') {
    await Promise.all([
      chatStore.loadSessions(),
      worldbooksStore.loadWorldbooks(),
      charactersStore.loadCharacters(),
    ])
  } else if (newName === 'st-home') {
    await Promise.all([
      chatStore.loadSessions(),
      worldbooksStore.loadWorldbooks(),
      charactersStore.loadCharacters(),
      presetsStore.loadPresetList(),
    ])
  } else if (newName === 'resources-characters') {
    await charactersStore.loadCharacters()
  } else if (newName === 'resources-presets') {
    await presetsStore.loadPresetList()
    if (!presetsStore.currentPreset && presetsStore.presetList[0]) {
      await presetsStore.loadPreset(presetsStore.presetList[0].name)
    }
  }
}, { immediate: true })
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
            添加
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

    <!-- Preset-specific layout -->
    <template v-else-if="isPresetsPage">
      <!-- File selector header -->
      <div class="list-header">
        <span class="list-title-preset">预设</span>
      </div>

      <!-- Preset file selector -->
      <div class="file-selector">
        <NSelect
          :value="presetsStore.currentPreset?.name"
          :options="presetOptions"
          placeholder="选择预设..."
          clearable
          size="small"
          @update:value="handlePresetSelect"
        />
      </div>

      <!-- File action buttons -->
      <div class="file-actions">
        <NButton size="small" type="primary" @click="createPreset">
          <template #icon>
            <NIcon><AddOutline /></NIcon>
          </template>
          新建
        </NButton>
        <NPopconfirm
          v-if="presetsStore.currentPreset && presetsStore.currentPreset.name !== 'Default'"
          @positive-click="deleteCurrentPreset"
        >
          <template #trigger>
            <NButton size="small" type="error">
              <template #icon>
                <NIcon><TrashOutline /></NIcon>
              </template>
              删除
            </NButton>
          </template>
          确定删除此预设吗？
        </NPopconfirm>
      </div>

      <!-- Prompt items when preset is selected -->
      <template v-if="presetsStore.currentPreset">
        <!-- Prompt actions -->
        <div class="entry-actions">
          <NText depth="3" class="entry-count">
            条目: {{ sortedPromptItems.length }}
          </NText>
          <NButton size="small" type="primary" @click="createPromptItem">
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
            placeholder="搜索提示词..."
            clearable
            size="small"
          >
            <template #prefix>
              <NIcon :size="16"><SearchOutline /></NIcon>
            </template>
          </NInput>
        </div>

        <!-- Prompt list with drag and drop -->
        <div class="list-content">
          <NSpin :show="presetsStore.isLoading">
            <div v-if="sortedPromptItems.length > 0" class="entry-list">
              <div
                v-for="item in sortedPromptItems"
                :key="item.identifier"
                class="entry-item"
                :class="{
                  'entry-item-dragging': draggedItem?.identifier === item.identifier,
                  'entry-item-drag-over': dragOverItem?.identifier === item.identifier,
                  'entry-selected': presetsStore.currentPromptIdentifier === item.identifier
                }"
                draggable="true"
                @dragstart="(e) => onDragStart(e, item)"
                @dragend="onDragEnd"
                @dragover="(e) => onDragOver(e, item)"
                @dragleave="onDragLeave"
                @drop="(e) => onDrop(e, item)"
              >
                <!-- Drag handle -->
                <div class="entry-drag-handle">
                  <NIcon :size="16" class="drag-icon">
                    <ReorderFourOutline />
                  </NIcon>
                </div>

                <!-- Enable switch -->
                <div class="entry-switch">
                  <NSwitch
                    :value="isPromptEnabled(item.identifier)"
                    size="small"
                    @update:value="(v: boolean) => togglePromptEnabled(item.identifier, v)"
                  />
                </div>

                <!-- Prompt info -->
                <div class="entry-info" @click="selectPromptItem(item.identifier)">
                  <div class="entry-header">
                    <span class="entry-name">{{ item.name }}</span>
                  </div>
                </div>

                <!-- Delete button (only for non-builtin items) -->
                <NPopconfirm
                  v-if="!isFixedPromptItem(item.identifier)"
                  @positive-click="deletePromptItem(item.identifier)"
                >
                  <template #trigger>
                    <NButton quaternary circle size="tiny" type="error" class="delete-btn">
                      <template #icon>
                        <NIcon><TrashOutline /></NIcon>
                      </template>
                    </NButton>
                  </template>
                  确定删除此提示词吗？
                </NPopconfirm>
              </div>
            </div>
            <NEmpty v-else description="暂无提示词" />
          </NSpin>
        </div>
      </template>

      <!-- Empty state when no preset selected -->
      <div v-else class="empty-state">
        <NEmpty description="请选择或创建预设" />
      </div>
    </template>

    <!-- Default layout for other pages -->
    <template v-else>
      <div class="list-header">
        <span class="list-title">{{ pageTitle }}</span>
        <NButton v-if="showDefaultAddButton" quaternary size="small" @click="handleDefaultAdd">
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
        <NSpin :show="isDefaultLoading">
          <NList v-if="filteredItems.length > 0" hoverable clickable>
            <NListItem
              v-for="item in filteredItems"
              :key="item.id"
              class="context-item"
              :class="{ 'context-item-active': item.active }"
              @click="item.session ? openSessionSettings(item.session) : item.action()"
            >
              <div class="context-item-row">
                <div class="context-item-main">
                  <span class="context-item-name">{{ item.name }}</span>
                  <NTag v-if="item.type" size="tiny" :bordered="false">{{ item.type }}</NTag>
                </div>
                <div v-if="route.name === 'st-chat' && item.session" class="context-item-actions">
                  <NDropdown
                    trigger="click"
                    :options="getSessionMenuOptions(item.session)"
                    @select="(key: string) => handleSessionMenuSelect(key, item.session!)"
                  >
                    <NButton
                      quaternary
                      circle
                      size="tiny"
                      class="context-item-menu"
                      @click.stop
                    >
                      <template #icon>
                        <NIcon><EllipsisHorizontalOutline /></NIcon>
                      </template>
                    </NButton>
                  </NDropdown>
                </div>
              </div>
              <NText v-if="item.meta" depth="3" class="context-item-meta">
                {{ item.meta }}
              </NText>
            </NListItem>
          </NList>
          <NEmpty v-else :description="defaultEmptyDescription" />
        </NSpin>
      </div>
    </template>

    <NModal
      :show="editingStSessionId !== null"
      preset="card"
      title="编辑 ST 会话"
      class="session-settings-modal"
      @update:show="value => { if (!value) editingStSessionId = null }"
    >
      <NForm label-placement="top">
        <NFormItem label="会话名称">
          <NInput v-model:value="editSessionName" placeholder="会话名称" />
        </NFormItem>
        <NFormItem label="绑定角色卡">
          <NSelect
            v-model:value="editSessionCharacterId"
            :options="characterOptions"
            filterable
            clearable
            placeholder="选择角色卡"
          />
        </NFormItem>
        <NFormItem label="选用的世界书">
          <NSelect
            v-model:value="editSessionWorldbooks"
            :options="worldbookOptions"
            multiple
            filterable
            clearable
            placeholder="选择一个或多个世界书"
          />
        </NFormItem>
        <NFormItem label="User name">
          <NInput v-model:value="editPersonaName" placeholder="User name" />
        </NFormItem>
        <NFormItem label="Persona Description">
          <NInput
            v-model:value="editPersonaDescription"
            type="textarea"
            :autosize="{ minRows: 4, maxRows: 8 }"
            placeholder="User 角色描述"
          />
        </NFormItem>
      </NForm>
      <div class="modal-actions">
        <NButton @click="editingStSessionId = null">取消</NButton>
        <NButton type="primary" :loading="isSavingSessionSettings" @click="saveSessionSettings">
          保存
        </NButton>
      </div>
    </NModal>

    <!-- 切换会话确认弹窗 -->
    <NModal
      :show="switchingSessionId !== null"
      preset="card"
      title="切换会话"
      class="session-switch-modal"
      @update:show="value => { if (!value) switchingSessionId = null }"
    >
      <NText>确定切换到该会话？</NText>
      <NText v-if="chatStore.isGenerating" depth="3" class="switch-warning">
        当前会话正在生成回复，切换将中断当前操作。
      </NText>
      <div class="modal-actions">
        <NButton @click="switchingSessionId = null">取消</NButton>
        <NButton type="primary" @click="confirmSwitchSession">
          切换
        </NButton>
      </div>
    </NModal>
  </div>
</template>

<style scoped>
.context-list {
  height: 100%;
  min-height: 0;
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

.list-title-preset {
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
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  padding: 0 4px;
}

.context-item {
  border-radius: 6px;
  cursor: pointer;
}

.context-item:hover {
  background-color: var(--n-color-hover);
}

.context-item-active {
  background-color: rgba(24, 160, 88, 0.1);
}

.context-item-main {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.context-item-row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.context-item-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-weight: 500;
}

.context-item-menu {
  flex: 0 0 auto;
  opacity: 1;
}

.context-item-actions {
  flex: 0 0 auto;
  margin-left: auto;
}

.context-item-meta {
  display: block;
  margin-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
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
  cursor: grab;
  transition: all 0.2s ease;
  gap: 8px;
  user-select: none;
}

.entry-item:active {
  cursor: grabbing;
}

.entry-item:hover {
  background-color: var(--n-color-hover);
}

.entry-item-dragging {
  opacity: 0.5;
  transform: scale(0.98);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.entry-item-drag-over {
  border-top: 2px solid var(--color-primary, #18a058);
  background-color: rgba(24, 160, 88, 0.08);
}

.entry-selected {
  background-color: rgba(24, 160, 88, 0.1);
}

.entry-drag-handle {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  cursor: grab;
  opacity: 0.4;
  transition: opacity 0.2s;
}

.entry-drag-handle:hover {
  opacity: 0.8;
}

.entry-drag-handle:active {
  cursor: grabbing;
}

.drag-icon {
  color: var(--color-text-secondary, #6b7280);
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

.session-settings-modal {
  width: min(420px, 60vw, calc(100vw - 32px));
}

.session-switch-modal {
  width: min(400px, calc(100vw - 32px));
}

.switch-warning {
  display: block;
  margin-top: 8px;
  font-size: 12px;
  color: var(--n-warning-color);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
}
</style>
