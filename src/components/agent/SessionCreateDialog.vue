<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  NButton,
  NCard,
  NForm,
  NFormItem,
  NInput,
  NSelect,
  NSpace,
  NRadioGroup,
  NRadio,
  NIcon,
  NAlert,
  NDivider,
  NInputNumber,
  useMessage,
} from 'naive-ui'
import { PersonOutline, VideocamOutline, TimeOutline } from '@vicons/ionicons5'
import type {
  PlayerMode,
  AgentSession,
  AgentSessionKind,
  TimeAnchor,
} from '@/types/agent/session'
import {
  createAgentSession,
  determineSessionKind,
  validateSession,
} from '@/types/agent/session'

const props = defineProps<{
  worldId: string
  characters: Array<{ id: string; name: string; description?: string }>
  mainlineTimeAnchor: TimeAnchor
  availableTimeAnchors?: TimeAnchor[]
}>()

const emit = defineEmits<{
  (e: 'create', session: AgentSession): void
  (e: 'cancel'): void
}>()

const message = useMessage()

// Form state
const title = ref('')
const playerMode = ref<PlayerMode>('Character')
const selectedCharacterId = ref<string | null>(null)
const selectedTimeAnchorIndex = ref<number>(0)
const customTimeOrdinal = ref<number | null>(null)
const customTimeDisplay = ref('')

// Computed
const characterOptions = computed(() =>
  props.characters.map((c) => ({
    label: c.name,
    value: c.id,
    description: c.description,
  }))
)

const timeAnchorOptions = computed(() => {
  if (props.availableTimeAnchors && props.availableTimeAnchors.length > 0) {
    return props.availableTimeAnchors.map((ta, index) => ({
      label: ta.display_text,
      value: index,
    }))
  }
  return [{ label: props.mainlineTimeAnchor.display_text, value: 0 }]
})

const currentTimeAnchor = computed((): TimeAnchor => {
  if (customTimeOrdinal.value !== null && customTimeDisplay.value) {
    return {
      calendar_id: props.mainlineTimeAnchor.calendar_id,
      ordinal: customTimeOrdinal.value,
      precision: 'Exact',
      display_text: customTimeDisplay.value,
    }
  }
  if (props.availableTimeAnchors && props.availableTimeAnchors.length > 0) {
    return props.availableTimeAnchors[selectedTimeAnchorIndex.value]
  }
  return props.mainlineTimeAnchor
})

const sessionKind = computed((): AgentSessionKind => {
  return determineSessionKind(currentTimeAnchor.value, props.mainlineTimeAnchor)
})

const sessionKindLabel = computed(() => {
  switch (sessionKind.value) {
    case 'Mainline':
      return { text: '当前主线', tone: 'success' as const }
    case 'Retrospective':
      return { text: '过去线', tone: 'warning' as const }
    case 'FuturePreview':
      return { text: '未来预演', tone: 'info' as const }
  }
})

const validationError = computed(() => {
  if (playerMode.value === 'Character' && !selectedCharacterId.value) {
    return '角色模式下必须选择一个角色'
  }
  if (playerMode.value === 'Director' && selectedCharacterId.value) {
    return '导演模式下不能选择角色'
  }
  return null
})

const canCreate = computed(() => {
  return title.value.trim() && !validationError.value
})

// Methods
function handleCreate() {
  if (!canCreate.value) return

  const session = createAgentSession(
    props.worldId,
    title.value.trim(),
    sessionKind.value,
    currentTimeAnchor.value,
    playerMode.value,
    playerMode.value === 'Character' ? selectedCharacterId.value : null
  )

  if (!session) {
    message.error('创建会话失败：参数无效')
    return
  }

  const error = validateSession(session)
  if (error) {
    message.error(error)
    return
  }

  emit('create', session)
}

function handleCancel() {
  emit('cancel')
}

// Reset character selection when switching to Director mode
watch(playerMode, (newMode) => {
  if (newMode === 'Director') {
    selectedCharacterId.value = null
  }
})
</script>

<template>
  <NCard title="创建 Agent 会话" size="medium" style="max-width: 600px">
    <NForm label-placement="left" label-width="100">
      <NFormItem label="会话标题" required>
        <NInput
          v-model:value="title"
          placeholder="输入会话标题"
          :maxlength="100"
          show-count
        />
      </NFormItem>

      <NDivider style="margin: 12px 0">会话视角</NDivider>

      <NFormItem label="视角模式">
        <NRadioGroup v-model:value="playerMode">
          <NSpace>
            <NRadio value="Character">
              <NSpace align="center" :size="6">
                <NIcon :size="16"><PersonOutline /></NIcon>
                <span>扮演角色</span>
              </NSpace>
            </NRadio>
            <NRadio value="Director">
              <NSpace align="center" :size="6">
                <NIcon :size="16"><VideocamOutline /></NIcon>
                <span>导演模式</span>
              </NSpace>
            </NRadio>
          </NSpace>
        </NRadioGroup>
      </NFormItem>

      <NFormItem v-if="playerMode === 'Character'" label="扮演角色" required>
        <NSelect
          v-model:value="selectedCharacterId"
          :options="characterOptions"
          placeholder="选择要扮演的角色"
          clearable
        />
      </NFormItem>

      <NAlert v-if="playerMode === 'Director'" type="info" style="margin-bottom: 12px">
        导演模式下，你将以世界外观察者身份输入场景描述、导演提示和元命令，不直接扮演任何角色。
      </NAlert>

      <NDivider style="margin: 12px 0">时间锚点</NDivider>

      <NFormItem label="时间点">
        <NSpace vertical style="width: 100%">
          <NSelect
            v-model:value="selectedTimeAnchorIndex"
            :options="timeAnchorOptions"
            placeholder="选择时间锚点"
          />
          <NSpace align="center">
            <NIcon :size="16"><TimeOutline /></NIcon>
            <span>会话类型：</span>
            <NButton
              size="tiny"
              :type="sessionKindLabel.tone"
              disabled
            >
              {{ sessionKindLabel.text }}
            </NButton>
          </NSpace>
        </NSpace>
      </NFormItem>

      <NFormItem label="自定义时间">
        <NSpace>
          <NInput
            v-model:value="customTimeDisplay"
            placeholder="显示文本（如：第三章开头）"
            style="width: 200px"
          />
          <NInputNumber
            v-model:value="customTimeOrdinal"
            placeholder="时间序号"
            :min="0"
            style="width: 120px"
          />
        </NSpace>
      </NFormItem>

      <NAlert v-if="sessionKind === 'Retrospective'" type="warning" style="margin-bottom: 12px">
        过去线会话的细节需要经过正史资格校验才能成为正史。硬冲突可能导致会话变为非正史。
      </NAlert>

      <NAlert v-if="sessionKind === 'FuturePreview'" type="info" style="margin-bottom: 12px">
        未来预演会话默认不入正史，只用于探索可能的发展方向。
      </NAlert>

      <NAlert v-if="validationError" type="error" style="margin-bottom: 12px">
        {{ validationError }}
      </NAlert>
    </NForm>

    <template #footer>
      <NSpace justify="end">
        <NButton @click="handleCancel">取消</NButton>
        <NButton
          type="primary"
          :disabled="!canCreate"
          @click="handleCreate"
        >
          创建会话
        </NButton>
      </NSpace>
    </template>
  </NCard>
</template>

<style scoped>
:deep(.n-form-item) {
  margin-bottom: 16px;
}
</style>