<script setup lang="ts">
import { computed } from 'vue'
import {
  NCard,
  NForm,
  NFormItem,
  NGrid,
  NGi,
  NIcon,
  NInput,
  NInputNumber,
  NSelect,
  NSpace,
  NTag,
  NDivider,
} from 'naive-ui'
import {
  SparklesOutline,
} from '@vicons/ionicons5'
import StructuredTextEditor from '@/components/shared/structured-text-editor/StructuredTextEditor.vue'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import { DEFAULT_BINDINGS } from '@/types/structuredText'

const editorStore = useAgentWorldEditorStore()

const draft = computed(() => {
  if (editorStore.selectedEntityType !== 'character') return null
  return (editorStore.draft?.draft as any) ?? null
})

const isNew = computed(() => editorStore.draft?.isNew ?? false)

const manaTendencyOptions = [
  { label: '内敛 (Inward)', value: 'Inward' },
  { label: '中性 (Neutral)', value: 'Neutral' },
  { label: '外放 (Expressive)', value: 'Expressive' },
]

function updateField(path: string, value: unknown) {
  editorStore.updateDraftField(path, value)
}

function updateBaseAttribute(key: string, value: number | null) {
  if (value === null) return
  const attrs = { ...(draft.value?.base_attributes ?? {}) }
  ;(attrs as Record<string, number>)[key] = value
  updateField('base_attributes', attrs)
}

function updateSensoryBaseline(key: string, value: number | null) {
  if (value === null) return
  const baseline = { ...(draft.value?.baseline_body_profile?.mana_sense_baseline ?? { acuity: 0.5, overload_threshold: 1.0, attribute_bias: null }) }
  ;(baseline as Record<string, number>)[key] = value
  const profile = { ...(draft.value?.baseline_body_profile ?? {}) }
  profile.mana_sense_baseline = baseline as any
  updateField('baseline_body_profile', profile)
}

function updateBodyProfileField(key: string, value: unknown) {
  const profile = { ...(draft.value?.baseline_body_profile ?? {}) }
  ;(profile as Record<string, unknown>)[key] = value
  updateField('baseline_body_profile', profile)
}

const comfortTemperatureRange = computed<[number, number]>(() => {
  const raw = draft.value?.baseline_body_profile?.comfort_temperature_range
  if (Array.isArray(raw) && raw.length >= 2) {
    return [Number(raw[0] ?? 0), Number(raw[1] ?? 0)]
  }
  return [0, 0]
})

const manaSenseBaseline = computed(() => {
  return draft.value?.baseline_body_profile?.mana_sense_baseline ?? {
    acuity: 0.5,
    overload_threshold: 1.0,
    attribute_bias: null,
  }
})

const temporaryStateBinding = DEFAULT_BINDINGS.agent_knowledge_content
</script>

<template>
  <div v-if="!draft" class="empty-editor">
    <NCard size="small" class="empty-card">
      <div class="empty-content">
        <NTag type="info">Character Editor</NTag>
        <p>请在左侧实体导航中选择一个角色，或点击新建。</p>
      </div>
    </NCard>
  </div>

  <div v-else class="character-editor">
    <!-- Header -->
    <div class="editor-header">
      <div class="header-main">
        <NTag size="small" :type="isNew ? 'success' : 'default'">
          {{ isNew ? '新建' : '编辑' }}
        </NTag>
        <span class="entity-id">{{ draft.character_id }}</span>
      </div>
      <NSpace>
        <NTag size="small" type="info">
          <template #icon><NIcon><SparklesOutline /></NIcon></template>
          {{ draft.mana_expression_tendency }}
        </NTag>
      </NSpace>
    </div>

    <!-- Basic Info -->
    <NCard size="small" title="基础信息">
      <NForm label-placement="left" label-width="120">
        <NFormItem label="角色 ID">
          <NInput
            v-model:value="draft.character_id"
            size="small"
            placeholder="character_001"
            @update:value="v => updateField('character_id', v)"
          />
        </NFormItem>

        <NFormItem label="灵力显露倾向">
          <NSelect
            v-model:value="draft.mana_expression_tendency"
            size="small"
            :options="manaTendencyOptions"
            @update:value="v => updateField('mana_expression_tendency', v)"
          />
        </NFormItem>

        <NFormItem label="MindModelCard">
          <NInput
            v-model:value="draft.mind_model_card_knowledge_id"
            size="small"
            placeholder="指向 KnowledgeEntry (kind=CharacterFacet, facet=MindModelCard)"
            @update:value="v => updateField('mind_model_card_knowledge_id', v)"
          />
        </NFormItem>
      </NForm>
    </NCard>

    <!-- Base Attributes -->
    <NCard size="small" title="六项基础属性">
      <template #header-extra>
        <NTag size="tiny" :bordered="false">f64 存储，UI 展示为整数</NTag>
      </template>
      <NGrid :cols="3" :x-gap="12" :y-gap="12">
        <NGi>
          <NFormItem label="体质 (physical)">
            <NInputNumber
              :value="draft.base_attributes.physical"
              size="small"
              :min="0"
              :max="100"
              :precision="0"
              @update:value="v => updateBaseAttribute('physical', v)"
            />
          </NFormItem>
        </NGi>
        <NGi>
          <NFormItem label="敏捷 (agility)">
            <NInputNumber
              :value="draft.base_attributes.agility"
              size="small"
              :min="0"
              :max="100"
              :precision="0"
              @update:value="v => updateBaseAttribute('agility', v)"
            />
          </NFormItem>
        </NGi>
        <NGi>
          <NFormItem label="耐力 (endurance)">
            <NInputNumber
              :value="draft.base_attributes.endurance"
              size="small"
              :min="0"
              :max="100"
              :precision="0"
              @update:value="v => updateBaseAttribute('endurance', v)"
            />
          </NFormItem>
        </NGi>
        <NGi>
          <NFormItem label="洞察 (insight)">
            <NInputNumber
              :value="draft.base_attributes.insight"
              size="small"
              :min="0"
              :max="100"
              :precision="0"
              @update:value="v => updateBaseAttribute('insight', v)"
            />
          </NFormItem>
        </NGi>
        <NGi>
          <NFormItem label="灵力 (mana_power)">
            <NInputNumber
              :value="draft.base_attributes.mana_power"
              size="small"
              :min="0"
              :max="100"
              :precision="0"
              @update:value="v => updateBaseAttribute('mana_power', v)"
            />
          </NFormItem>
        </NGi>
        <NGi>
          <NFormItem label="魂强 (soul_strength)">
            <NInputNumber
              :value="draft.base_attributes.soul_strength"
              size="small"
              :min="0"
              :max="100"
              :precision="0"
              @update:value="v => updateBaseAttribute('soul_strength', v)"
            />
          </NFormItem>
        </NGi>
      </NGrid>
    </NCard>

    <!-- Body Baseline -->
    <NCard size="small" title="身体基线">
      <NGrid :cols="3" :x-gap="12" :y-gap="12">
        <NGi>
          <NFormItem label="种族">
            <NInput
              :value="draft.baseline_body_profile?.species"
              size="small"
              @update:value="v => updateBodyProfileField('species', v)"
            />
          </NFormItem>
        </NGi>
        <NGi>
          <NFormItem label="舒适温度下限">
            <NInputNumber
              :value="comfortTemperatureRange[0]"
              size="small"
              @update:value="v => updateBodyProfileField('comfort_temperature_range', [v ?? 0, comfortTemperatureRange[1]])"
            />
          </NFormItem>
        </NGi>
        <NGi>
          <NFormItem label="舒适温度上限">
            <NInputNumber
              :value="comfortTemperatureRange[1]"
              size="small"
              @update:value="v => updateBodyProfileField('comfort_temperature_range', [comfortTemperatureRange[0], v ?? 0])"
            />
          </NFormItem>
        </NGi>
      </NGrid>

      <NDivider style="margin: 8px 0" />

      <div class="sensory-baseline">
        <div class="sensory-title">Mana Sense 基线</div>
        <NGrid :cols="2" :x-gap="12" :y-gap="12">
          <NGi>
            <NFormItem label="Acuity">
              <NInputNumber
                :value="manaSenseBaseline.acuity"
                size="small"
                :min="0"
                :max="2"
                :step="0.1"
                @update:value="v => updateSensoryBaseline('acuity', v)"
              />
            </NFormItem>
          </NGi>
          <NGi>
            <NFormItem label="Overload Threshold">
              <NInputNumber
                :value="manaSenseBaseline.overload_threshold"
                size="small"
                :min="0"
                :step="0.1"
                @update:value="v => updateSensoryBaseline('overload_threshold', v)"
              />
            </NFormItem>
          </NGi>
        </NGrid>
      </div>
    </NCard>

    <!-- Temporary State (read-only-ish, editable via JSON) -->
    <NCard size="small" title="临时状态 (temporary_state)">
      <template #header-extra>
        <NTag size="tiny" :bordered="false">JSON 编辑</NTag>
      </template>
      <StructuredTextEditor
        :model-value="JSON.stringify(draft.temporary_state, null, 2)"
        mode="json"
        :binding="temporaryStateBinding"
        :min-height="200"
        @update:model-value="(text) => {
          try { updateField('temporary_state', JSON.parse(text)) } catch { }
        }"
      />
    </NCard>
  </div>
</template>

<style scoped>
.character-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-bottom: 24px;
}

.empty-editor {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-card {
  max-width: 420px;
}

.empty-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 24px;
  text-align: center;
  color: var(--color-text-secondary, #6b7280);
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

.sensory-baseline {
  padding-top: 4px;
}

.sensory-title {
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text-secondary, #6b7280);
  margin-bottom: 8px;
}
</style>
