<script setup lang="ts">
import { computed, ref } from 'vue'
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

const worldRulesBinding = DEFAULT_BINDINGS.agent_world_rules

const draftYaml = ref<string>('')

const isNewDraft = computed(() => {
  return editorStore.selectedEntityType === 'world_rules' && editorStore.draft?.isNew !== false
})

// Initialize draft when world_rules is selected
if (editorStore.selectedEntityType === 'world_rules') {
  if (editorStore.draft?.draft) {
    draftYaml.value = String(editorStore.draft.draft)
  } else {
    draftYaml.value = `# World Base Rules
# 编辑后需通过 ConfigValidator 校验方可提交

world_name: ""
calendar:
  default_calendar_id: ""
  eras: []
rules:
  combat_enabled: true
  mana_system_enabled: true
  knowledge_reveal_enabled: true
llm_profile:
  default_preset_id: ""
  scene_initializer_prompt: ""
`
    editorStore.initDraft('world_rules', 'world_base', draftYaml.value, true)
  }
}

function updateYaml(value: string) {
  draftYaml.value = value
  editorStore.updateDraftField('world_base', value)
}

async function handleValidateYaml() {
  message.info('YAML 格式校验需通过后端 ConfigValidator 完成')
}
</script>

<template>
  <div class="world-rules-editor">
    <div class="editor-header">
      <div class="header-main">
        <NTag size="small" :type="isNewDraft ? 'success' : 'default'">
          {{ isNewDraft ? '新建' : '编辑' }}
        </NTag>
        <span class="entity-id">world_base.yaml</span>
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
        <span>World Rules 保存前必须经过 ConfigValidator 和 World Editor validation。顶层 structured content 不允许以 Plain 模式提交。</span>
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
