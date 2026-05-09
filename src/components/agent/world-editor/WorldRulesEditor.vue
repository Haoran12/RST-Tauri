<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRoute } from 'vue-router'
import {
  NButton,
  NCard,
  NTag,
  NIcon,
  NSpace,
  useMessage,
} from 'naive-ui'
import {
  WarningOutline,
  CodeDownloadOutline,
} from '@vicons/ionicons5'
import StructuredTextEditor from '@/components/shared/structured-text-editor/StructuredTextEditor.vue'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import { DEFAULT_BINDINGS } from '@/types/structuredText'

const editorStore = useAgentWorldEditorStore()
const message = useMessage()
const route = useRoute()

const worldRulesBinding = DEFAULT_BINDINGS.agent_world_rules

const draftYaml = ref<string>('')
const worldId = computed(() => {
  const id = route.params.worldId
  return typeof id === 'string' ? id : ''
})

const isNewDraft = computed(() => {
  return editorStore.selectedEntityType === 'world_rules' && editorStore.draft?.isNew !== false
})

const FALLBACK_WORLD_ARGUMENT = `schema_version: 1
world:
  display_name: ""
calendar:
  default_calendar_id: ""
  eras: []
attribute_rules:
  tier_thresholds:
    mundane: [0.0, 200.0]
    awakened: [200.0, 1000.0]
    adept: [1000.0, 1800.0]
    master: [1800.0, 2600.0]
    ascendant: [2600.0, 5600.0]
    transcendent: [5600.0, null]
  delta_thresholds:
    indistinguishable_abs_lt: 150.0
    slight_abs_lt: 300.0
    notable_abs_lt: 1000.0
    far_abs_lt: 2000.0
mana_rules:
  display_ratio_clamp: [0.0, 2.0]
  tendency_factors:
    inward: -0.5
    neutral: -0.2
    expressive: 0.1
  mode_factors:
    sealed: -0.7
    suppressed: -0.3
    natural: 0.0
    released: 0.2
    dominating: 0.4
  expression_modes:
    sealed: { radius: self_only, pressure_multiplier: 0.0 }
    suppressed: { radius: close, pressure_multiplier: 0.5 }
    natural: { radius: room, pressure_multiplier: 1.0 }
    released: { radius: area, pressure_multiplier: 1.15 }
    dominating: { radius: scene, pressure_multiplier: 1.3 }
  concealment_suspected_gap: 200.0
combat_rules:
  delta_thresholds:
    indistinguishable_abs_lt: 150.0
    slight_abs_lt: 300.0
    marked_abs_lt: 1000.0
  min_effectiveness: 0.1
  soul_tier_factors:
    mundane: 0.8
    awakened: 0.9
    adept: 1.0
    master: 1.05
    ascendant: 1.1
    transcendent: 1.15
  soul_damage_floor: 0.2
`

async function loadWorldArgumentDraft() {
  if (editorStore.selectedEntityType !== 'world_rules') return
  if (editorStore.draft?.draft) {
    draftYaml.value = String(editorStore.draft.draft)
    return
  }

  try {
    const yaml = await invoke<string>('get_world_argument_detail', {
      worldId: worldId.value,
    })
    draftYaml.value = yaml
    editorStore.initDraft('world_rules', 'world_argument', yaml, false)
  } catch (e) {
    draftYaml.value = FALLBACK_WORLD_ARGUMENT
    editorStore.initDraft('world_rules', 'world_argument', FALLBACK_WORLD_ARGUMENT, true)
    message.error(`加载 ${'world_argument.yaml'} 失败: ${String(e)}`)
  }
}

function updateYaml(value: string) {
  draftYaml.value = value
  editorStore.updateDraftField('world_argument', value)
}

async function handleValidateYaml() {
  message.info('world_argument.yaml 的格式与 schema 校验会在后端 validation 阶段执行')
}

onMounted(() => {
  void loadWorldArgumentDraft()
})
</script>

<template>
  <div class="world-rules-editor">
    <div class="editor-header">
      <div class="header-main">
        <NTag size="small" :type="isNewDraft ? 'success' : 'default'">
          {{ isNewDraft ? '新建' : '编辑' }}
        </NTag>
        <span class="entity-id">world_argument.yaml</span>
      </div>
      <NSpace>
        <NTag size="small" type="warning">
          <template #icon><NIcon><WarningOutline /></NIcon></template>
          YAML
        </NTag>
      </NSpace>
    </div>

    <NCard size="small" title="世界规则配置">
      <template #header-extra>
        <NSpace size="small">
          <NButton size="tiny" quaternary @click="handleValidateYaml">
            <template #icon><NIcon><CodeDownloadOutline /></NIcon></template>
            格式检查
          </NButton>
        </NSpace>
      </template>

      <div class="yaml-editor-wrapper">
        <StructuredTextEditor
          :model-value="draftYaml"
          mode="yaml"
          :binding="worldRulesBinding"
          :min-height="500"
          @update:model-value="updateYaml"
        />
      </div>

      <div class="yaml-hint">
        <NTag size="tiny" :bordered="false" type="info">
          <template #icon><NIcon><WarningOutline /></NIcon></template>
          提示
        </NTag>
        <span>World Rules 保存前必须经过后端 YAML 解析、schema 校验和 World Editor validation。顶层 structured content 不允许以 Plain 模式提交。</span>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.world-rules-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-bottom: 24px;
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 4px 2px;
}

.header-main {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.entity-id {
  font-family: monospace;
  font-size: 13px;
  color: var(--color-text-secondary, #6b7280);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.yaml-editor-wrapper {
  border: 1px solid var(--color-border-subtle, #e0e0e6);
  border-radius: 8px;
  overflow: hidden;
}

.yaml-hint {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-top: 12px;
  padding: 10px;
  background: #f6ffed;
  border-radius: 6px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-secondary, #6b7280);
}
</style>
