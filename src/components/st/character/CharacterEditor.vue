<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed, watch } from 'vue'
import {
  NForm,
  NFormItem,
  NInput,
  NButton,
  NSpin,
  NSpace,
  NUpload,
  NText,
  useMessage,
  type UploadFileInfo,
} from 'naive-ui'
import { useCharactersStore } from '@/stores/characters'
import { getCharacter, saveCharacter } from '@/services/storage'
import type { TavernCardV3 } from '@/types/st'
import type {
  StructuredTextBinding,
  StructuredTextLanguageId,
} from '@/types/structuredText'
import StructuredTextEditor from '@/components/shared/structured-text-editor/StructuredTextEditor.vue'

const props = defineProps<{
  characterId: string
  closeOnSave?: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const store = useCharactersStore()
const message = useMessage()

const form = ref<TavernCardV3 | null>(null)
const avatarUrl = ref<string | null>(null)
const isLoading = ref(false)
const isSaving = ref(false)
const loadError = ref<string | null>(null)
const isDirty = ref(false)
const dirtyVersion = ref(0)
const textModes = ref<Record<string, StructuredTextLanguageId>>({})
const editorRefs = ref<Record<string, InstanceType<typeof StructuredTextEditor> | null>>({})

type CharacterTextFieldKey =
  | 'description'
  | 'personality'
  | 'scenario'
  | 'first_mes'
  | 'mes_example'
  | 'system_prompt'
  | 'post_history_instructions'
  | 'creator_notes'

interface CharacterTextFieldConfig {
  key: CharacterTextFieldKey
  label: string
  rows: number
}

const textFieldConfigs: CharacterTextFieldConfig[] = [
  { key: 'description', label: '描述', rows: 4 },
  { key: 'personality', label: '性格', rows: 3 },
  { key: 'scenario', label: '场景', rows: 3 },
  { key: 'first_mes', label: '第一条消息', rows: 4 },
  { key: 'mes_example', label: '示例对话', rows: 4 },
  { key: 'system_prompt', label: '系统提示词', rows: 3 },
  { key: 'post_history_instructions', label: '后历史指令', rows: 2 },
  { key: 'creator_notes', label: '创作者备注', rows: 2 },
]

const stringBinding: StructuredTextBinding = {
  resourceKind: 'st_preset',
  fieldPath: 'content',
  allowedModes: ['plain', 'json', 'yaml'],
  defaultMode: 'plain',
  storageKind: 'string',
}

onMounted(() => {
  void loadCharacterForEditor()
})

watch(
  () => props.characterId,
  async (_newId, oldId) => {
    await persistCurrentCharacter({ characterId: oldId, silent: true, closeAfterSave: false })
    await loadCharacterForEditor()
  },
)

onBeforeUnmount(() => {
  void persistCurrentCharacter({ silent: true, closeAfterSave: false })
})

/**
 * 安全深拷贝，处理 structuredClone 无法克隆的对象
 * 使用 JSON 序列化/反序列化作为 fallback
 */
function safeDeepClone<T>(obj: T): T {
  try {
    return structuredClone(obj)
  } catch {
    return JSON.parse(JSON.stringify(obj))
  }
}

async function loadCharacterForEditor() {
  isLoading.value = true
  loadError.value = null

  try {
    const character = await getCharacter(props.characterId)
    store.currentCharacter = character
    form.value = safeDeepClone(character)
    avatarUrl.value = await store.getAvatarUrl(props.characterId)
    textModes.value = Object.fromEntries(
      textFieldConfigs.map(field => [field.key, 'plain' as StructuredTextLanguageId]),
    )
    isDirty.value = false
  } catch (e) {
    form.value = null
    loadError.value = String(e)
  } finally {
    isLoading.value = false
  }
}

const hasEmbeddedWorldbook = computed(
  () => form.value?.data.character_book != null
)

function markDirty() {
  isDirty.value = true
  dirtyVersion.value += 1
}

function updateTextField(key: CharacterTextFieldKey, value: string) {
  if (!form.value) return
  form.value.data[key] = value
  markDirty()
}

function updateStringField(
  key: 'name' | 'creator' | 'character_version',
  value: string,
) {
  if (!form.value) return
  form.value.data[key] = value
  markDirty()
}

function updateTags(value: string) {
  if (!form.value) return
  form.value.data.tags = value
    .split(',')
    .map(t => t.trim())
    .filter(Boolean)
  markDirty()
}

async function collectEditorText(silent: boolean) {
  if (!form.value) return false

  for (const field of textFieldConfigs) {
    const editor = editorRefs.value[field.key]
    if (!editor) {
      continue
    }

    const result = await editor.validate()
    if (result.diagnostics.some(item => item.severity === 'blocker')) {
      if (!silent) {
        message.error(`字段“${field.label}”存在 blocker，修复后才能保存。`)
      }
      return false
    }

    form.value.data[field.key] = String(result.text ?? '')
  }

  return true
}

function syncCharacterListItem(id: string, character: TavernCardV3) {
  const index = store.characters.findIndex(item => item.id === id)
  if (index >= 0) {
    store.characters[index] = { id, character }
  }
}

async function persistCurrentCharacter(options: {
  characterId?: string
  silent?: boolean
  closeAfterSave?: boolean
} = {}) {
  if (!form.value || isLoading.value || isSaving.value || !isDirty.value) {
    return
  }

  const silent = options.silent ?? false
  const saveVersion = dirtyVersion.value
  const id = options.characterId ?? props.characterId
  isSaving.value = true

  try {
    const canSave = await collectEditorText(silent)
    if (!canSave || !form.value) {
      return
    }

    const character = safeDeepClone(form.value)
    store.currentCharacter = character
    syncCharacterListItem(id, character)
    await saveCharacter(id, character)
    if (dirtyVersion.value === saveVersion) {
      isDirty.value = false
    }
    if (!silent) {
      message.success('保存成功')
    }
    if (options.closeAfterSave ?? (props.closeOnSave ?? true)) {
      emit('close')
    }
  } catch (e) {
    message.error(`${silent ? '自动保存失败' : '保存失败'}: ${e}`)
  } finally {
    isSaving.value = false
  }
}

async function handleSave() {
  await persistCurrentCharacter({ silent: false })
}

function handleEditorFocusOut(event: FocusEvent) {
  const nextTarget = event.relatedTarget
  const currentTarget = event.currentTarget
  if (
    nextTarget instanceof Node &&
    currentTarget instanceof HTMLElement &&
    currentTarget.contains(nextTarget)
  ) {
    return
  }

  void persistCurrentCharacter({ silent: true, closeAfterSave: false })
}

async function handleAvatarUpload(options: { file: UploadFileInfo }) {
  const file = options.file.file
  if (!file) return

  try {
    await store.updateAvatar(props.characterId, file)
    avatarUrl.value = await store.getAvatarUrl(props.characterId)
    message.success('头像更新成功')
  } catch (e) {
    message.error(`头像更新失败: ${e}`)
  }
}

async function handleImportWorldbook() {
  try {
    const loreId = await store.importWorldbook(props.characterId)
    message.success(`世界书导入成功，ID: ${loreId}`)
    await loadCharacterForEditor()
  } catch (e) {
    message.error(`导入世界书失败: ${e}`)
  }
}

function setEditorRef(
  key: CharacterTextFieldKey,
  instance: InstanceType<typeof StructuredTextEditor> | null,
) {
  editorRefs.value[key] = instance
}
</script>

<template>
  <div class="character-editor" @focusout="handleEditorFocusOut">
    <NSpin :show="isLoading">
      <div v-if="form" class="editor-content">
      <!-- Avatar Section -->
      <div class="avatar-section">
        <div class="avatar-preview">
          <img v-if="avatarUrl" :src="avatarUrl" class="avatar" />
          <div v-else class="avatar-placeholder">
            <NText>无头像</NText>
          </div>
        </div>
        <NUpload
          accept="image/png"
          :custom-request="handleAvatarUpload"
          :show-file-list="false"
        >
          <NButton size="small">更换头像</NButton>
        </NUpload>
      </div>

      <!-- Form Section -->
      <NForm label-placement="top">
        <NFormItem label="名称">
          <NInput
            :value="form.data.name"
            @update:value="value => updateStringField('name', value)"
          />
        </NFormItem>

        <NFormItem
          v-for="field in textFieldConfigs"
          :key="field.key"
          :label="field.label"
        >
          <StructuredTextEditor
            :ref="(instance) => setEditorRef(field.key, instance as InstanceType<typeof StructuredTextEditor> | null)"
            :model-value="form.data[field.key] ?? ''"
            :mode="textModes[field.key] ?? 'plain'"
            :binding="stringBinding"
            :min-height="Math.max(180, field.rows * 30 + 60)"
            :use-backend-validation="true"
            @update:model-value="value => updateTextField(field.key, value)"
            @update:mode="(mode) => { textModes[field.key] = mode }"
          />
        </NFormItem>

        <NFormItem label="标签">
          <NInput
            :value="form.data.tags?.join(', ')"
            @update:value="updateTags"
          />
        </NFormItem>

        <NFormItem label="创作者">
          <NInput
            :value="form.data.creator"
            @update:value="value => updateStringField('creator', value)"
          />
        </NFormItem>

        <NFormItem label="角色版本">
          <NInput
            :value="form.data.character_version"
            @update:value="value => updateStringField('character_version', value)"
          />
        </NFormItem>
      </NForm>

      <!-- Embedded Worldbook Section -->
      <div v-if="hasEmbeddedWorldbook" class="worldbook-section">
        <NText strong>内嵌世界书</NText>
        <NText depth="3">
          该角色卡包含内嵌世界书，点击下方按钮可将其导入为外部世界书。
        </NText>
        <NButton type="info" @click="handleImportWorldbook">
          导入为外部世界书
        </NButton>
      </div>

      <!-- Actions -->
      <NSpace justify="end">
        <NButton @click="emit('close')">取消</NButton>
        <NButton type="primary" :loading="isSaving" @click="handleSave">保存</NButton>
      </NSpace>
      </div>

      <div v-else class="editor-empty">
        <NText v-if="loadError" type="error">{{ loadError }}</NText>
        <NText v-else>加载中...</NText>
      </div>
    </NSpin>
  </div>
</template>

<style scoped>
.character-editor {
  height: 100%;
  overflow-y: auto;
  padding: 16px;
  scrollbar-width: thin;
}

.character-editor::-webkit-scrollbar {
  width: 6px;
}

.character-editor::-webkit-scrollbar-track {
  background: transparent;
}

.character-editor::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 3px;
}

.character-editor::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

.editor-content {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.avatar-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.avatar-preview {
  width: 200px;
  height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg-subtle, #f5f5f5);
  border-radius: 8px;
}

.avatar {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.avatar-placeholder {
  color: var(--color-text-secondary, #999);
}

.worldbook-section {
  padding: 16px;
  background: var(--color-bg-subtle, #f9f9f9);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.editor-empty {
  padding: 24px;
  text-align: center;
}
</style>
