<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useRoute } from 'vue-router'
import {
  NButton,
  NCard,
  NDynamicInput,
  NForm,
  NFormItem,
  NGrid,
  NGi,
  NInput,
  NInputNumber,
  NSelect,
  NSpace,
  NTag,
  useMessage,
} from 'naive-ui'
import StructuredTextEditor from '@/components/shared/structured-text-editor/StructuredTextEditor.vue'
import type { StructuredTextDiagnostic, StructuredTextLanguageId } from '@/types/structuredText'
import { DEFAULT_BINDINGS } from '@/types/structuredText'
import { validateStructuredText } from '@/services/storage'
import { useAgentStore } from '@/stores/agent'

const route = useRoute()
const message = useMessage()
const agentStore = useAgentStore()
const worldId = computed(() => {
  const routeWorldId = route.params.worldId
  if (typeof routeWorldId === 'string' && routeWorldId.length > 0) return routeWorldId
  return agentStore.currentWorldId ?? ''
})

const locationLevels = [
  'world_root',
  'realm',
  'continent',
  'natural_region',
  'polity',
  'major_region',
  'local_region',
  'settlement',
  'district_or_site',
  'room_or_subsite',
].map(value => ({ label: value, value }))

const draft = reactive({
  location: {
    location_id: '',
    name: '',
    parent_id: '',
    canonical_level: 'settlement',
    type_label: '',
    aliases: [] as string[],
  },
  knowledge: {
    knowledge_id: '',
    kind: 'world_fact',
    subject_type: 'world',
    access_scope: 'public',
    content_mode: 'json' as StructuredTextLanguageId,
    content_text: '{\n  "summary_text": ""\n}',
  },
  character: {
    character_id: '',
    mind_model_card_knowledge_id: '',
    physical: 100,
    agility: 100,
    endurance: 100,
    insight: 100,
    mana_power: 0,
    soul_strength: 100,
  },
})

const knowledgeDiagnostics = ref<StructuredTextDiagnostic[]>([])
const knowledgeParsedValue = ref<unknown>({ summary_text: '' })
const knowledgeEditorRef = ref<InstanceType<typeof StructuredTextEditor> | null>(null)

const knowledgeContentObject = computed<Record<string, unknown> | null>(() => {
  const value = knowledgeParsedValue.value
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>
  }
  return null
})

const knowledgeSummaryText = computed(() => {
  const summary = knowledgeContentObject.value?.summary_text
  return typeof summary === 'string' ? summary.trim() : ''
})

const validationItems = computed(() => [
  {
    label: 'Location ID',
    ok: draft.location.location_id.trim().length > 0,
  },
  {
    label: 'Location parent',
    ok:
      !draft.location.parent_id ||
      draft.location.parent_id !== draft.location.location_id,
  },
  {
    label: 'Knowledge payload',
    ok:
      draft.knowledge.knowledge_id.trim().length > 0 &&
      knowledgeDiagnostics.value.every(item => item.severity !== 'blocker') &&
      knowledgeSummaryText.value.length > 0,
  },
  {
    label: 'Character attributes',
    ok: [
      draft.character.physical,
      draft.character.agility,
      draft.character.endurance,
      draft.character.insight,
      draft.character.mana_power,
      draft.character.soul_strength,
    ].every(value => Number.isFinite(value) && value >= 0),
  },
])

const canExport = computed(() => validationItems.value.every(item => item.ok))

const patchPreview = computed(() => {
  return JSON.stringify(
    {
      world_id: worldId.value,
      location_creates: draft.location.location_id
        ? [
            {
              ...draft.location,
              aliases: draft.location.aliases
                .filter(alias => alias.trim())
                .map(alias => ({
                  alias,
                  locale: null,
                  normalized_alias: alias.trim().toLowerCase(),
                })),
              parent_id: draft.location.parent_id || null,
              tags: [],
              status: 'active',
              metadata: {},
              schema_version: '0.1',
            },
          ]
        : [],
      knowledge_creates: draft.knowledge.knowledge_id
        ? [
            {
              ...draft.knowledge,
              content: knowledgeContentObject.value ?? {},
              access_policy: {
                known_by: [],
                scope: [draft.knowledge.access_scope],
                conditions: [],
              },
            },
          ]
        : [],
      character_creates: draft.character.character_id
        ? [
            {
              character_id: draft.character.character_id,
              mind_model_card_knowledge_id:
                draft.character.mind_model_card_knowledge_id,
              base_attributes: {
                physical: draft.character.physical,
                agility: draft.character.agility,
                endurance: draft.character.endurance,
                insight: draft.character.insight,
                mana_power: draft.character.mana_power,
                soul_strength: draft.character.soul_strength,
              },
            },
          ]
        : [],
    },
    null,
    2,
  )
})

async function validateKnowledgeEditor() {
  const result = knowledgeEditorRef.value
    ? await knowledgeEditorRef.value.validate()
    : await validateStructuredText({
        text: draft.knowledge.content_text,
        mode: draft.knowledge.content_mode,
        binding: DEFAULT_BINDINGS.agent_knowledge_content,
      })

  draft.knowledge.content_text = result.text
  knowledgeDiagnostics.value = result.diagnostics
  knowledgeParsedValue.value = result.parsedValue
  return result
}

async function exportPatch() {
  const result = await validateKnowledgeEditor()
  if (result.diagnostics.some(item => item.severity === 'blocker')) {
    message.error('KnowledgeEntry content 仍有 blocker，无法导出补丁。')
    return
  }

  message.success('Patch 已通过结构化内容复检。')
}
</script>

<template>
  <div class="agent-editor-view">
    <header class="page-header">
      <div>
        <h1>Agent World Editor</h1>
        <div class="world-id">{{ worldId }}</div>
      </div>
      <NSpace>
        <NTag :type="canExport ? 'success' : 'warning'">
          {{ canExport ? 'Patch ready' : 'Needs fields' }}
        </NTag>
        <NButton secondary :disabled="!canExport" @click="exportPatch">导出补丁</NButton>
      </NSpace>
    </header>

    <section class="editor-grid">
      <div class="form-column">
        <NCard size="small" title="LocationNode">
          <NForm label-placement="left" label-width="110">
            <NFormItem label="location_id">
              <NInput v-model:value="draft.location.location_id" />
            </NFormItem>
            <NFormItem label="name">
              <NInput v-model:value="draft.location.name" />
            </NFormItem>
            <NFormItem label="parent_id">
              <NInput v-model:value="draft.location.parent_id" />
            </NFormItem>
            <NGrid :cols="2" :x-gap="12">
              <NGi>
                <NFormItem label="level">
                  <NSelect
                    v-model:value="draft.location.canonical_level"
                    :options="locationLevels"
                  />
                </NFormItem>
              </NGi>
              <NGi>
                <NFormItem label="type_label">
                  <NInput v-model:value="draft.location.type_label" />
                </NFormItem>
              </NGi>
            </NGrid>
            <NFormItem label="aliases">
              <NDynamicInput
                v-model:value="draft.location.aliases"
                placeholder="alias"
              />
            </NFormItem>
          </NForm>
        </NCard>

        <NCard size="small" title="KnowledgeEntry">
          <NForm label-placement="left" label-width="110">
            <NFormItem label="knowledge_id">
              <NInput v-model:value="draft.knowledge.knowledge_id" />
            </NFormItem>
            <NGrid :cols="3" :x-gap="12">
              <NGi>
                <NFormItem label="kind">
                  <NSelect
                    v-model:value="draft.knowledge.kind"
                    :options="[
                      { label: 'world_fact', value: 'world_fact' },
                      { label: 'region_fact', value: 'region_fact' },
                      { label: 'character_facet', value: 'character_facet' },
                      { label: 'memory', value: 'memory' },
                    ]"
                  />
                </NFormItem>
              </NGi>
              <NGi>
                <NFormItem label="subject">
                  <NSelect
                    v-model:value="draft.knowledge.subject_type"
                    :options="[
                      { label: 'world', value: 'world' },
                      { label: 'region', value: 'region' },
                      { label: 'character', value: 'character' },
                    ]"
                  />
                </NFormItem>
              </NGi>
              <NGi>
                <NFormItem label="scope">
                  <NSelect
                    v-model:value="draft.knowledge.access_scope"
                    :options="[
                      { label: 'public', value: 'public' },
                      { label: 'god_only', value: 'god_only' },
                    ]"
                  />
                </NFormItem>
              </NGi>
            </NGrid>
            <NFormItem label="summary">
              <StructuredTextEditor
                ref="knowledgeEditorRef"
                :model-value="draft.knowledge.content_text"
                :mode="draft.knowledge.content_mode"
                :binding="DEFAULT_BINDINGS.agent_knowledge_content"
                :min-height="240"
                :use-backend-validation="true"
                @update:model-value="(value) => { draft.knowledge.content_text = value }"
                @update:mode="(mode) => { draft.knowledge.content_mode = mode }"
                @diagnostics-change="(diagnostics) => { knowledgeDiagnostics = diagnostics }"
                @parsed-value-change="(value) => { knowledgeParsedValue = value }"
              />
            </NFormItem>
          </NForm>
        </NCard>

        <NCard size="small" title="CharacterRecord">
          <NForm label-placement="left" label-width="150">
            <NFormItem label="character_id">
              <NInput v-model:value="draft.character.character_id" />
            </NFormItem>
            <NFormItem label="mind_model_card">
              <NInput v-model:value="draft.character.mind_model_card_knowledge_id" />
            </NFormItem>
            <NGrid :cols="3" :x-gap="12" :y-gap="6">
              <NGi v-for="key in ['physical', 'agility', 'endurance', 'insight', 'mana_power', 'soul_strength']" :key="key">
                <NFormItem :label="key">
                  <NInputNumber v-model:value="(draft.character as any)[key]" :min="0" />
                </NFormItem>
              </NGi>
            </NGrid>
          </NForm>
        </NCard>
      </div>

      <aside class="preview-column">
        <NCard size="small" title="Validation">
          <div class="check-list">
            <div v-for="item in validationItems" :key="item.label" class="check-row">
              <NTag size="small" :type="item.ok ? 'success' : 'error'">
                {{ item.ok ? 'OK' : 'MISS' }}
              </NTag>
              <span>{{ item.label }}</span>
            </div>
          </div>
        </NCard>

        <NCard size="small" title="Patch Preview">
          <pre>{{ patchPreview }}</pre>
        </NCard>
      </aside>
    </section>
  </div>
</template>

<style scoped>
.agent-editor-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-app, #f0f2f5);
}

.page-header {
  padding: 18px 24px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--color-bg-surface, #fff);
}

h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.world-id {
  margin-top: 4px;
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
}

.editor-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(520px, 1fr) 380px;
  gap: 12px;
  padding: 16px;
  overflow: hidden;
}

.form-column,
.preview-column {
  min-height: 0;
  overflow: auto;
  display: grid;
  align-content: start;
  gap: 12px;
  scrollbar-width: thin;
  scrollbar-gutter: stable;
}

.form-column::-webkit-scrollbar,
.preview-column::-webkit-scrollbar {
  width: 8px;
}

.form-column::-webkit-scrollbar-track,
.preview-column::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.05);
  border-radius: 4px;
}

.form-column::-webkit-scrollbar-thumb,
.preview-column::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.5);
  border-radius: 4px;
  min-height: 30px;
}

.form-column::-webkit-scrollbar-thumb:hover,
.preview-column::-webkit-scrollbar-thumb:hover {
  background: rgba(128, 128, 128, 0.7);
}

.check-list {
  display: grid;
  gap: 8px;
}

.check-row {
  display: grid;
  grid-template-columns: 54px 1fr;
  align-items: center;
  gap: 8px;
}

pre {
  max-height: 520px;
  overflow: auto;
  margin: 0;
  padding: 12px;
  background: #101828;
  color: #e6edf3;
  border-radius: 6px;
  font-size: 12px;
  line-height: 1.45;
}

@media (max-width: 1100px) {
  .editor-grid {
    grid-template-columns: 1fr;
    overflow: auto;
  }
}
</style>
