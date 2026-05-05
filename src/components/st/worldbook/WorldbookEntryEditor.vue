<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import {
  NCard,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NSwitch,
  NSelect,
  NButton,
  NSpace,
  NDivider,
  NDynamicTags,
  NCheckbox,
  NEmpty,
  NModal,
  useMessage,
} from 'naive-ui'
import type { WorldInfoEntry } from '@/types/st'
import { WorldInfoLogic, WorldInfoPosition, ExtensionPromptRole, createWorldInfoEntry } from '@/types/st'
import type {
  StructuredTextDiagnostic,
  StructuredTextLanguageId,
} from '@/types/structuredText'
import { DEFAULT_BINDINGS } from '@/types/structuredText'
import StructuredTextEditor from '@/components/shared/structured-text-editor/StructuredTextEditor.vue'
import { validateStructuredText } from '@/services/storage'

const props = defineProps<{
  entry: WorldInfoEntry | null
  groups: string[]
}>()

const emit = defineEmits<{
  (e: 'update', entry: WorldInfoEntry): void
  (e: 'delete'): void
}>()

const message = useMessage()

// Local state for editing
const localEntry = ref<WorldInfoEntry>(createWorldInfoEntry(0))
const contentMode = ref<StructuredTextLanguageId>(DEFAULT_BINDINGS.st_worldbook_content.defaultMode)

// Content overlay state
const showContentOverlay = ref(false)
const overlayContent = ref('')
const overlayContentMode = ref<StructuredTextLanguageId>(DEFAULT_BINDINGS.st_worldbook_content.defaultMode)
const overlayEditorRef = ref<InstanceType<typeof StructuredTextEditor> | null>(null)
const overlayDiagnostics = ref<StructuredTextDiagnostic[]>([])
const isSavingOverlay = ref(false)

// Watch for entry changes
watch(
  () => props.entry,
  (newEntry) => {
    if (newEntry) {
      localEntry.value = { ...newEntry }
      contentMode.value = DEFAULT_BINDINGS.st_worldbook_content.defaultMode
    } else {
      localEntry.value = createWorldInfoEntry(0)
      contentMode.value = DEFAULT_BINDINGS.st_worldbook_content.defaultMode
    }
  },
  { immediate: true }
)

// Position options
const positionOptions = [
  { label: '角色前', value: WorldInfoPosition.BEFORE_CHAR },
  { label: '角色后', value: WorldInfoPosition.AFTER_CHAR },
  { label: '作者备注顶部', value: WorldInfoPosition.AN_TOP },
  { label: '作者备注底部', value: WorldInfoPosition.AN_BOTTOM },
  { label: '指定深度', value: WorldInfoPosition.AT_DEPTH },
  { label: '示例消息顶部', value: WorldInfoPosition.EM_TOP },
  { label: '示例消息底部', value: WorldInfoPosition.EM_BOTTOM },
  { label: '出口', value: WorldInfoPosition.OUTLET },
]

// Logic options (次关键词匹配逻辑，主关键词始终为 OR)
const logicOptions = [
  { label: 'AND ANY (任一次关键词)', value: WorldInfoLogic.AND_ANY },
  { label: 'NOT ALL (非所有次关键词)', value: WorldInfoLogic.NOT_ALL },
  { label: 'NOT ANY (无次关键词)', value: WorldInfoLogic.NOT_ANY },
  { label: 'AND ALL (所有次关键词)', value: WorldInfoLogic.AND_ALL },
]

// Role options
const roleOptions = [
  { label: '系统', value: ExtensionPromptRole.SYSTEM },
  { label: '用户', value: ExtensionPromptRole.USER },
  { label: '助手', value: ExtensionPromptRole.ASSISTANT },
]

// Group options (existing groups + new)
const groupOptions = computed(() => {
  const existing = props.groups.map((g) => ({ label: g, value: g }))
  return [{ label: '(无分组)', value: '' }, ...existing]
})

// Is AT_DEPTH position
const isAtDepth = computed(() => localEntry.value.position === WorldInfoPosition.AT_DEPTH)

// Is OUTLET position
const isOutlet = computed(() => localEntry.value.position === WorldInfoPosition.OUTLET)

// Has group
const hasGroup = computed(() => localEntry.value.group && localEntry.value.group.trim())

// Save changes (auto-save on blur/leave)
async function saveChanges() {
  emit('update', { ...localEntry.value })
}

// Open content overlay
function openContentOverlay() {
  overlayContent.value = localEntry.value.content ?? ''
  overlayContentMode.value = contentMode.value
  overlayDiagnostics.value = []
  showContentOverlay.value = true
}

// Save content from overlay
async function saveOverlayContent() {
  const validation = overlayEditorRef.value
    ? await overlayEditorRef.value.validate()
    : await validateStructuredText({
        text: overlayContent.value,
        mode: overlayContentMode.value,
        binding: DEFAULT_BINDINGS.st_worldbook_content,
      })

  overlayDiagnostics.value = validation.diagnostics
  overlayContent.value = validation.text

  if (overlayDiagnostics.value.some(item => item.severity === 'blocker')) {
    message.error('内容存在 blocker，请修复后再保存。')
    return
  }

  isSavingOverlay.value = true
  try {
    localEntry.value.content = overlayContent.value
    contentMode.value = overlayContentMode.value
    await saveChanges()
    showContentOverlay.value = false
    message.success('内容已保存')
  } finally {
    isSavingOverlay.value = false
  }
}

// Cancel overlay
function cancelOverlay() {
  showContentOverlay.value = false
}
</script>

<template>
  <NCard v-if="entry" class="entry-editor" size="small">
    <NForm label-placement="left" label-width="100px" size="small">
      <!-- Basic Info -->
      <NFormItem label="条目名称">
        <NInput
          v-model:value="localEntry.comment"
          placeholder="条目名称/备注"
          @blur="saveChanges"
        />
      </NFormItem>

      <!-- Status (常驻) -->
      <NFormItem label="常驻">
        <NSwitch v-model:value="localEntry.constant" @update:value="saveChanges" />
        <span class="hint">始终包含在上下文中</span>
      </NFormItem>

      <!-- Scan Settings (常驻) -->
      <NFormItem label="扫描深度">
        <NInputNumber
          :value="localEntry.scan_depth ?? undefined"
          :min="0"
          :max="999"
          placeholder="使用全局设置"
          @update:value="(v: number | null) => { localEntry.scan_depth = v; saveChanges() }"
        />
      </NFormItem>

      <!-- Content Edit Button -->
      <NFormItem label="内容">
        <NButton v-if="localEntry.content" type="success" secondary @click="openContentOverlay">
          编辑内容
          <span class="content-preview">{{ localEntry.content.slice(0, 50) }}{{ localEntry.content.length > 50 ? '...' : '' }}</span>
        </NButton>
        <NButton v-else type="success" @click="openContentOverlay">
          编辑内容
        </NButton>
      </NFormItem>

      <NDivider />

      <!-- Keywords -->
      <NFormItem label="主关键词">
        <NDynamicTags v-model:value="localEntry.key" @change="saveChanges" />
      </NFormItem>

      <NFormItem label="次关键词">
        <NDynamicTags v-model:value="localEntry.keysecondary" @change="saveChanges" />
      </NFormItem>

      <!-- Selective & Logic on same row -->
      <NFormItem label="选择性">
        <div class="inline-row">
          <NSwitch v-model:value="localEntry.selective" @update:value="saveChanges" />
          <span class="hint">需要次关键词匹配</span>
          <NSelect
            v-if="localEntry.selective"
            v-model:value="localEntry.selective_logic"
            :options="logicOptions"
            style="width: 280px; margin-left: 12px;"
            @update:value="saveChanges"
          />
        </div>
      </NFormItem>

      <NDivider />

      <!-- Position & Order -->
      <NFormItem label="注入位置">
        <div class="inline-row">
          <NSelect
            v-model:value="localEntry.position"
            :options="positionOptions"
            style="flex: 1; min-width: 150px;"
            @update:value="saveChanges"
          />
          <div class="inline-field">
            <span class="inline-label">顺序</span>
            <NInputNumber
              v-model:value="localEntry.order"
              :min="0"
              :max="999"
              style="width: 120px;"
              @blur="saveChanges"
            />
          </div>
        </div>
      </NFormItem>

      <NFormItem v-if="isAtDepth" label="深度">
        <NInputNumber
          v-model:value="localEntry.depth"
          :min="0"
          :max="999"
          @blur="saveChanges"
        />
      </NFormItem>

      <NFormItem v-if="isAtDepth" label="角色">
        <NSelect
          v-model:value="localEntry.role"
          :options="roleOptions"
          @update:value="saveChanges"
        />
      </NFormItem>

      <NFormItem v-if="isOutlet" label="出口名称">
        <NInput
          v-model:value="localEntry.outlet_name"
          placeholder="出口标识符"
          @blur="saveChanges"
        />
      </NFormItem>

      <NDivider />

      <!-- Probability & Budget -->
      <NFormItem label="概率">
        <div class="inline-row">
          <NInputNumber
            v-model:value="localEntry.probability"
            :min="0"
            :max="100"
            :step="0.5"
            style="width: 120px;"
            @blur="saveChanges"
          />
          <span class="hint">%</span>
          <div class="inline-field">
            <span class="inline-label">启用概率</span>
            <NSwitch v-model:value="localEntry.use_probability" @update:value="saveChanges" />
          </div>
        </div>
      </NFormItem>

      <NFormItem label="忽略预算">
        <NSwitch v-model:value="localEntry.ignore_budget" @update:value="saveChanges" />
      </NFormItem>

      <NDivider />

      <!-- Group -->
      <NFormItem label="分组">
        <NSelect
          v-model:value="localEntry.group"
          :options="groupOptions"
          :tag="true"
          filterable
          @update:value="saveChanges"
        />
      </NFormItem>

      <NFormItem v-if="hasGroup" label="覆盖">
        <NSwitch v-model:value="localEntry.group_override" @update:value="saveChanges" />
        <span class="hint">替换同组其他条目</span>
      </NFormItem>

      <NFormItem v-if="hasGroup" label="权重">
        <NInputNumber
          v-model:value="localEntry.group_weight"
          :min="0"
          :max="100"
          @blur="saveChanges"
        />
      </NFormItem>

      <NDivider />

      <!-- Recursion -->
      <NFormItem label="递归设置">
        <div class="inline-row">
          <div class="inline-field">
            <NSwitch v-model:value="localEntry.exclude_recursion" @update:value="saveChanges" />
            <span class="inline-label-text">不可被递归激活</span>
          </div>
          <div class="inline-field">
            <NSwitch v-model:value="localEntry.prevent_recursion" @update:value="saveChanges" />
            <span class="inline-label-text">阻止进一步递归</span>
          </div>
        </div>
      </NFormItem>

      <NFormItem label="延迟至">
        <NInputNumber
          :value="typeof localEntry.delay_until_recursion === 'number' ? localEntry.delay_until_recursion : 0"
          :min="0"
          :max="999"
          @update:value="(v: number | null) => { if (v !== null) { localEntry.delay_until_recursion = v; saveChanges() } }"
        />
        <span class="hint">递归深度</span>
      </NFormItem>

      <NDivider />

      <!-- Time Control -->
      <NFormItem label="持续回合">
        <NInputNumber
          :value="localEntry.sticky ?? undefined"
          :min="0"
          :max="999"
          placeholder="禁用"
          @update:value="(v: number | null) => { localEntry.sticky = v; saveChanges() }"
        />
        <span class="hint">激活后持续 N 回合</span>
      </NFormItem>

      <NFormItem label="冷却">
        <NInputNumber
          :value="localEntry.cooldown ?? undefined"
          :min="0"
          :max="999"
          placeholder="禁用"
          @update:value="(v: number | null) => { localEntry.cooldown = v; saveChanges() }"
        />
        <span class="hint">再次激活前等待 N 回合</span>
      </NFormItem>

      <NFormItem label="延迟">
        <NInputNumber
          :value="localEntry.delay ?? undefined"
          :min="0"
          :max="999"
          placeholder="禁用"
          @update:value="(v: number | null) => { localEntry.delay = v; saveChanges() }"
        />
        <span class="hint">首次激活前等待 N 回合</span>
      </NFormItem>

      <NDivider />

      <!-- Match Targets -->
      <NFormItem label="匹配目标">
        <div class="match-targets-grid">
          <NCheckbox
            :checked="localEntry.match_persona_description"
            @update:checked="(v: boolean) => { localEntry.match_persona_description = v; saveChanges() }"
          >
            人物描述
          </NCheckbox>
          <NCheckbox
            :checked="localEntry.match_character_description"
            @update:checked="(v: boolean) => { localEntry.match_character_description = v; saveChanges() }"
          >
            角色描述
          </NCheckbox>
          <NCheckbox
            :checked="localEntry.match_character_personality"
            @update:checked="(v: boolean) => { localEntry.match_character_personality = v; saveChanges() }"
          >
            角色性格
          </NCheckbox>
          <NCheckbox
            :checked="localEntry.match_character_depth_prompt"
            @update:checked="(v: boolean) => { localEntry.match_character_depth_prompt = v; saveChanges() }"
          >
            角色深度提示
          </NCheckbox>
          <NCheckbox
            :checked="localEntry.match_scenario"
            @update:checked="(v: boolean) => { localEntry.match_scenario = v; saveChanges() }"
          >
            场景
          </NCheckbox>
          <NCheckbox
            :checked="localEntry.match_creator_notes"
            @update:checked="(v: boolean) => { localEntry.match_creator_notes = v; saveChanges() }"
          >
            创建者备注
          </NCheckbox>
        </div>
      </NFormItem>

      <NDivider />

      <!-- Triggers -->
      <NFormItem label="触发词">
        <NDynamicTags v-model:value="localEntry.triggers" @change="saveChanges" />
      </NFormItem>

      <NFormItem label="自动化ID">
        <NInput
          v-model:value="localEntry.automation_id"
          placeholder="自动化标识符"
          @blur="saveChanges"
        />
      </NFormItem>
    </NForm>

    <!-- Content Edit Overlay -->
    <NModal
      v-model:show="showContentOverlay"
      preset="card"
      title="编辑内容"
      style="width: min(92vw, 900px);"
      :mask-closable="false"
    >
      <div class="overlay-content">
        <StructuredTextEditor
          ref="overlayEditorRef"
          :model-value="overlayContent"
          :binding="DEFAULT_BINDINGS.st_worldbook_content"
          :mode="overlayContentMode"
          :min-height="400"
          :use-backend-validation="true"
          @update:model-value="(value) => { overlayContent = value }"
          @update:mode="(mode) => { overlayContentMode = mode }"
          @diagnostics-change="(diagnostics) => { overlayDiagnostics = diagnostics }"
        />
      </div>
      <template #footer>
        <NSpace justify="end">
          <NButton @click="cancelOverlay">取消</NButton>
          <NButton type="primary" :loading="isSavingOverlay" @click="saveOverlayContent">
            保存
          </NButton>
        </NSpace>
      </template>
    </NModal>
  </NCard>
  <NCard v-else class="entry-editor" size="small">
    <NEmpty description="选择一个条目进行编辑" />
  </NCard>
</template>

<style scoped>
.entry-editor {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.entry-editor :deep(.n-card-content) {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-gutter: stable;
}

.hint {
  color: var(--color-text-secondary, #999);
  font-size: 12px;
  margin-left: 8px;
}

.inline-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px 24px;
}

.inline-field {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 4px 0;
}

.inline-label {
  color: var(--color-text, inherit);
  font-size: 13px;
  white-space: nowrap;
}

.inline-label-text {
  color: var(--color-text, inherit);
  font-size: 13px;
  white-space: nowrap;
}

.match-targets-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px 24px;
}

.content-preview {
  margin-left: 8px;
  color: var(--color-text-secondary, #999);
  font-size: 12px;
}

.overlay-content {
  min-height: 400px;
}
</style>

<style>
/* 全局滚动条样式 - 确保 WebView 中可见 */
.entry-editor .n-card-content::-webkit-scrollbar {
  width: 8px;
}

.entry-editor .n-card-content::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.05);
  border-radius: 4px;
}

.entry-editor .n-card-content::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.5);
  border-radius: 4px;
  min-height: 30px;
}

.entry-editor .n-card-content::-webkit-scrollbar-thumb:hover {
  background: rgba(128, 128, 128, 0.7);
}

.entry-editor .n-card-content::-webkit-scrollbar-corner {
  background: transparent;
}
</style>
