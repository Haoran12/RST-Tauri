<script setup lang="ts">
import { computed } from 'vue'
import {
  NButton,
  NCard,
  NForm,
  NFormItem,
  NGrid,
  NGi,
  NIcon,
  NInput,
  NSelect,
  NSpace,
  NTag,
  NTooltip,
  NDynamicInput,
  NSwitch,
} from 'naive-ui'
import {
  EyeOutline,
  EyeOffOutline,
  WarningOutline,
  ShieldCheckmarkOutline,
  PersonOutline,
  GlobeOutline,
  LocationOutline,
  BusinessOutline,
} from '@vicons/ionicons5'
import StructuredTextEditor from '@/components/shared/structured-text-editor/StructuredTextEditor.vue'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import type { KnowledgeEntry, KnowledgeSubjectType } from '@/types/agent/knowledge'
import { KNOWLEDGE_KIND_LABELS, CHARACTER_FACET_LABELS } from '@/types/agent/knowledge'
import { DEFAULT_BINDINGS } from '@/types/structuredText'

const editorStore = useAgentWorldEditorStore()

const draft = computed(() => {
  if (editorStore.selectedEntityType !== 'knowledge') return null
  return editorStore.draft?.draft as KnowledgeEntry | undefined
})

const isNew = computed(() => editorStore.draft?.isNew ?? false)

const kindOptions = Object.entries(KNOWLEDGE_KIND_LABELS).map(([value, label]) => ({
  label,
  value,
}))

const subjectTypeOptions: { label: string; value: KnowledgeSubjectType; icon: any }[] = [
  { label: '世界', value: 'world', icon: GlobeOutline },
  { label: '地区', value: 'region', icon: LocationOutline },
  { label: '势力', value: 'faction', icon: BusinessOutline },
  { label: '角色', value: 'character', icon: PersonOutline },
  { label: '事件', value: 'event', icon: WarningOutline },
]

const facetTypeOptions = Object.entries(CHARACTER_FACET_LABELS).map(([value, label]) => ({
  label,
  value,
}))

const scopeOptions = [
  { label: '公开 (Public)', value: 'Public' },
  { label: '仅编排器 (GodOnly)', value: 'GodOnly', type: 'error' },
  { label: '地区限定', value: 'Region' },
  { label: '势力限定', value: 'Faction' },
  { label: '修为门槛', value: 'Realm' },
  { label: '职位限定', value: 'Role' },
  { label: '血脉限定', value: 'Bloodline' },
]

function updateField(path: string, value: unknown) {
  editorStore.updateDraftField(path, value)
}

function updateAccessPolicy(field: string, value: unknown) {
  const policy = { ...(draft.value?.access_policy ?? { known_by: [], scope: [], conditions: [] }) }
  ;(policy as Record<string, unknown>)[field] = value
  updateField('access_policy', policy)
}

function addScope() {
  const scope = [...(draft.value?.access_policy.scope ?? [])]
  scope.push({ type: 'Public' })
  updateAccessPolicy('scope', scope)
}

function removeScope(index: number) {
  const scope = [...(draft.value?.access_policy.scope ?? [])]
  scope.splice(index, 1)
  updateAccessPolicy('scope', scope)
}

function updateScope(index: number, patch: Partial<{ type: string; value?: string }>) {
  const scope = [...(draft.value?.access_policy.scope ?? [])]
  scope[index] = { ...scope[index], ...patch } as any
  updateAccessPolicy('scope', scope)
}

function updateKnownBy(value: unknown) {
  updateAccessPolicy('known_by', value as string[])
}

// Content bindings
const contentBinding = DEFAULT_BINDINGS.agent_knowledge_content
</script>

<template>
  <div v-if="!draft" class="empty-editor">
    <NCard size="small" class="empty-card">
      <div class="empty-content">
        <NTag type="info">Knowledge Editor</NTag>
        <p>请在左侧实体导航中选择一条 Knowledge，或点击新建。</p>
      </div>
    </NCard>
  </div>

  <div v-else class="knowledge-editor">
    <!-- Header -->
    <div class="editor-header">
      <div class="header-main">
        <NTag size="small" :type="isNew ? 'success' : 'default'">
          {{ isNew ? '新建' : '编辑' }}
        </NTag>
        <span class="entity-id">{{ draft.knowledge_id }}</span>
      </div>
      <NSpace>
        <NTag
          v-if="draft.access_policy.scope.some(s => s.type === 'GodOnly')"
          size="small"
          type="error"
        >
          <template #icon><NIcon><EyeOffOutline /></NIcon></template>
          GodOnly
        </NTag>
        <NTag
          v-else-if="draft.access_policy.scope.some(s => s.type === 'Public')"
          size="small"
          type="success"
        >
          <template #icon><NIcon><EyeOutline /></NIcon></template>
          Public
        </NTag>
        <NTag v-else size="small" type="warning">受限</NTag>
      </NSpace>
    </div>

    <!-- Basic Info -->
    <NCard size="small" title="基础信息">
      <NForm label-placement="left" label-width="110">
        <NGrid :cols="2" :x-gap="16">
          <NGi>
            <NFormItem label="ID">
              <NInput
                v-model:value="draft.knowledge_id"
                size="small"
                placeholder="knowledge_001"
                @update:value="v => updateField('knowledge_id', v)"
              />
            </NFormItem>
          </NGi>
          <NGi>
            <NFormItem label="类型">
              <NSelect
                v-model:value="draft.kind"
                size="small"
                :options="kindOptions"
                @update:value="v => updateField('kind', v)"
              />
            </NFormItem>
          </NGi>
        </NGrid>

        <!-- Subject Selection -->
        <NFormItem label="所属对象">
          <NGrid :cols="2" :x-gap="12">
            <NGi>
              <NSelect
                v-model:value="draft.subject_type"
                size="small"
                :options="subjectTypeOptions"
                @update:value="v => updateField('subject_type', v)"
              />
            </NGi>
            <NGi>
              <NInput
                v-model:value="draft.subject_id"
                size="small"
                placeholder="输入对象 ID（角色/地区/势力 ID）"
                :disabled="draft.subject_type === 'world'"
                @update:value="v => updateField('subject_id', v || null)"
              />
            </NGi>
          </NGrid>
        </NFormItem>

        <!-- Facet Type (for character_facet) -->
        <NFormItem v-if="draft.kind === 'character_facet'" label="分面类型">
          <NSelect
            v-model:value="draft.facet_type"
            size="small"
            :options="facetTypeOptions"
            @update:value="v => updateField('facet_type', v)"
          />
        </NFormItem>
      </NForm>
    </NCard>

    <!-- Access Policy -->
    <NCard size="small" title="访问策略">
      <template #header-extra>
        <NTooltip>
          <template #trigger>
            <NTag size="tiny" :bordered="false">
              <template #icon><NIcon><ShieldCheckmarkOutline /></NIcon></template>
              三谓词 OR
            </NTag>
          </template>
          known_by / scope / conditions 任一为真即可访问；GodOnly 为 hard deny
        </NTooltip>
      </template>

      <!-- known_by -->
      <NFormItem label="已知者名单 (known_by)">
        <NDynamicInput
          v-model:value="draft.access_policy.known_by"
          placeholder="character_id"
          @update:value="updateKnownBy"
        />
      </NFormItem>

      <!-- scope -->
      <NFormItem label="范围标签 (scope)">
        <div class="scope-list">
          <div
            v-for="(scope, index) in draft.access_policy.scope"
            :key="index"
            class="scope-row"
          >
            <NSelect
              v-model:value="scope.type"
              size="small"
              :options="scopeOptions"
              style="width: 160px"
              @update:value="v => updateScope(index, { type: v })"
            />
            <NInput
              v-if="scope.type !== 'Public' && scope.type !== 'GodOnly'"
              v-model:value="scope.value"
              size="small"
              placeholder="范围值，如地区ID"
              style="width: 140px"
              @update:value="v => updateScope(index, { value: v })"
            />
            <NButton quaternary size="tiny" @click="removeScope(index)">移除</NButton>
          </div>
          <NButton size="small" @click="addScope">
            <template #icon><NIcon><ShieldCheckmarkOutline /></NIcon></template>
            添加范围
          </NButton>
        </div>
      </NFormItem>

      <!-- GodOnly Warning -->
      <NCard
        v-if="draft.access_policy.scope.some(s => s.type === 'GodOnly')"
        size="small"
        type="error"
        class="godonly-warning"
      >
        <strong>GodOnly 警告</strong>
        <p>此 Knowledge 被设为仅编排器可读。所有角色均不可访问。</p>
        <p>若要通过剧情揭示给角色，必须使用运行时 KnowledgeRevealEvent，不得在编辑器中直接追加 known_by。</p>
      </NCard>
    </NCard>

    <!-- Subject Awareness -->
    <NCard
      v-if="draft.subject_type === 'character'"
      size="small"
      title="自我认知 (SubjectAwareness)"
    >
      <NFormItem label="角色是否自知">
        <NSwitch
          :value="(draft.subject_awareness as any)?.kind === 'Aware'"
          @update:value="v => {
            updateField('subject_awareness', v ? { kind: 'Aware' } : { kind: 'Unaware', self_belief: { summary_text: '' } })
          }"
        >
          <template #checked>自知 (Aware)</template>
          <template #unchecked>不自知 (Unaware)</template>
        </NSwitch>
      </NFormItem>
      <p class="hint-text">
        若角色不自知，该角色在构建 AccessibleKnowledge 时将看到 self_belief 版本，而非客观 content。
      </p>
    </NCard>

    <!-- Content -->
    <NCard size="small" title="内容 (content)">
      <StructuredTextEditor
        :model-value="JSON.stringify(draft.content ?? { summary_text: '' }, null, 2)"
        mode="json"
        :binding="contentBinding"
        :min-height="280"
        @update:model-value="(text) => {
          try { updateField('content', JSON.parse(text)) } catch { /* ignore parse errors during editing */ }
        }"
      />
    </NCard>

    <!-- Apparent Content -->
    <NCard size="small" title="表象 (apparent_content)">
      <template #header-extra>
        <NTag size="tiny" :bordered="false">可选</NTag>
      </template>
      <p class="hint-text">当观察者通过观察获得此 Knowledge 时，默认看到的版本。留空则与 content 一致。</p>
      <StructuredTextEditor
        :model-value="JSON.stringify(draft.apparent_content ?? { summary_text: '' }, null, 2)"
        mode="json"
        :binding="contentBinding"
        :min-height="200"
        @update:model-value="(text) => {
          try { updateField('apparent_content', JSON.parse(text)) } catch { }
        }"
      />
    </NCard>
  </div>
</template>

<style scoped>
.knowledge-editor {
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

.scope-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.scope-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.godonly-warning {
  margin-top: 8px;
}

.godonly-warning p {
  margin: 4px 0 0;
  font-size: 12px;
}

.hint-text {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
  line-height: 1.5;
}
</style>
