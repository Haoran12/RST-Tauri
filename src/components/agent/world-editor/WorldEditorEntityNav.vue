<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import {
  NButton,
  NInput,
  NTree,
  NList,
  NListItem,
  NSpace,
  NTag,
  NIcon,
  NEmpty,
  NSelect,
  NDivider,
} from 'naive-ui'
import type { TreeOption } from 'naive-ui'
import {
  AddOutline,
  LocationOutline,
  BookOutline,
  PeopleOutline,
  LinkOutline,
  SettingsOutline,
} from '@vicons/ionicons5'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import {
  KNOWLEDGE_KIND_LABELS,
  createDefaultKnowledgeEntry,
  createMindModelCardKnowledgeEntry,
} from '@/types/agent/knowledge'
import type { EditorEntityType } from '@/types/agent/worldEditor'
import { createCharacterRecord } from '@/types/agent/character'

const editorStore = useAgentWorldEditorStore()
const route = useRoute()

const entityTabs: { type: EditorEntityType; label: string; icon: any }[] = [
  { type: 'location', label: '地点', icon: LocationOutline },
  { type: 'knowledge', label: 'Knowledge', icon: BookOutline },
  { type: 'character', label: '角色', icon: PeopleOutline },
  { type: 'relationship', label: '关系', icon: LinkOutline },
  { type: 'world_rules', label: '世界规则', icon: SettingsOutline },
]

const activeTab = computed<EditorEntityType>({
  get: () => editorStore.selectedEntityType === 'none' ? 'knowledge' : editorStore.selectedEntityType,
  set: (v) => {
    editorStore.selectEntity(v, null)
  },
})

const worldId = computed(() => {
  const id = route.params.worldId
  return typeof id === 'string' ? id : ''
})

// Location Tree
const locationTreeOptions = computed<TreeOption[]>(() => {
  function buildNodes(nodes: typeof editorStore.locationTree): TreeOption[] {
    return nodes.map(n => ({
      key: n.location_id,
      label: `${n.name} (${n.canonical_level})`,
      children: buildNodes(n.children),
    }))
  }
  return buildNodes(editorStore.locationTree)
})

// Knowledge
const knowledgeKindFilterOptions = [
  { label: '全部', value: '' },
  ...Object.entries(KNOWLEDGE_KIND_LABELS).map(([value, label]) => ({ label, value })),
]

const selectedLocationKeys = computed(() => {
  return editorStore.selectedEntityType === 'location' && editorStore.selectedEntityId
    ? [editorStore.selectedEntityId]
    : []
})

async function handleLocationSelect(keys: string[]) {
  const id = keys[0] ?? null
  if (id) {
    editorStore.selectEntity('location', id)
    if (!worldId.value) return
    const loc = await editorStore.loadLocationDetail(worldId.value, id)
    if (loc) {
      editorStore.initDraft('location', id, { ...loc }, false)
    }
  }
}

async function handleKnowledgeSelect(id: string) {
  editorStore.selectEntity('knowledge', id)
  if (!worldId.value) return
  const knowledge = await editorStore.loadKnowledgeDetail(worldId.value, id)
  if (knowledge) {
    editorStore.initDraft('knowledge', id, { ...knowledge }, false)
  }
}

async function handleCharacterSelect(id: string) {
  editorStore.selectEntity('character', id)
  if (!worldId.value) return
  const character = await editorStore.loadCharacterDetail(worldId.value, id)
  if (character) {
    let linkedKnowledge = null
    if (character.mind_model_card_knowledge_id) {
      linkedKnowledge = await editorStore.loadKnowledgeDetail(
        worldId.value,
        character.mind_model_card_knowledge_id
      )
    }
    editorStore.initCharacterDraft(
      id,
      { ...character },
      false,
      linkedKnowledge ? { ...linkedKnowledge } : null,
      false
    )
  }
}

function createNewKnowledge() {
  const id = `knowledge_${Date.now()}`
  editorStore.selectEntity('knowledge', id)
  editorStore.initDraft('knowledge', id, createDefaultKnowledgeEntry(id), true)
}

function createNewCharacter() {
  const id = `character_${Date.now()}`
  const mindModelKnowledgeId = `knowledge_mind_model_${Date.now()}`
  const character = createCharacterRecord(id, {
    mind_model_card_knowledge_id: mindModelKnowledgeId,
  })
  const linkedKnowledge = createMindModelCardKnowledgeEntry(mindModelKnowledgeId, id)
  editorStore.selectEntity('character', id)
  editorStore.initCharacterDraft(id, character, true, linkedKnowledge, true)
}
</script>

<template>
  <div class="entity-nav">
    <!-- Tabs -->
    <div class="nav-tabs">
      <button
        v-for="tab in entityTabs"
        :key="tab.type"
        class="tab-btn"
        :class="{ active: activeTab === tab.type }"
        type="button"
        @click="activeTab = tab.type"
      >
        <NIcon :size="14"><component :is="tab.icon" /></NIcon>
        <span>{{ tab.label }}</span>
      </button>
    </div>

    <NDivider style="margin: 8px 0" />

    <!-- Location List -->
    <div v-if="activeTab === 'location'" class="nav-section">
      <div class="section-header">
        <span class="section-title">地点树</span>
        <NButton size="tiny" quaternary>
          <template #icon><NIcon><AddOutline /></NIcon></template>
        </NButton>
      </div>
      <NTree
        :data="locationTreeOptions"
        :selected-keys="selectedLocationKeys"
        block-line
        expand-on-click
        @update:selected-keys="handleLocationSelect"
      />
    </div>

    <!-- Knowledge List -->
    <div v-if="activeTab === 'knowledge'" class="nav-section">
      <div class="section-header">
        <span class="section-title">Knowledge</span>
        <NButton size="tiny" quaternary @click="createNewKnowledge">
          <template #icon><NIcon><AddOutline /></NIcon></template>
        </NButton>
      </div>
      <NSpace vertical size="small" style="padding: 0 8px 8px">
        <NInput
          v-model:value="editorStore.knowledgeFilterSearch"
          size="tiny"
          placeholder="搜索 ID / 摘要"
          clearable
        />
        <NSelect
          v-model:value="editorStore.knowledgeFilterKind"
          size="tiny"
          :options="knowledgeKindFilterOptions"
          placeholder="类型筛选"
          clearable
        />
      </NSpace>
      <NList hoverable clickable style="background: transparent">
        <NListItem
          v-for="item in editorStore.filteredKnowledgeList"
          :key="item.knowledge_id"
          :class="{ active: editorStore.selectedEntityId === item.knowledge_id }"
          @click="handleKnowledgeSelect(item.knowledge_id)"
        >
          <div class="knowledge-list-item">
            <div class="knowledge-title">
              <NTag size="tiny" :type="item.access_scope_summary === 'GodOnly' ? 'error' : 'default'">
                {{ item.kind.slice(0, 4) }}
              </NTag>
              <span class="knowledge-id" :title="item.knowledge_id">{{ item.knowledge_id }}</span>
            </div>
            <div class="knowledge-summary">{{ item.summary_text || '(无摘要)' }}</div>
          </div>
        </NListItem>
        <NEmpty v-if="!editorStore.filteredKnowledgeList.length" size="small" description="无数据" />
      </NList>
    </div>

    <!-- Character List -->
    <div v-if="activeTab === 'character'" class="nav-section">
      <div class="section-header">
        <span class="section-title">角色</span>
        <NButton size="tiny" quaternary @click="createNewCharacter">
          <template #icon><NIcon><AddOutline /></NIcon></template>
        </NButton>
      </div>
      <NList hoverable clickable style="background: transparent">
        <NListItem
          v-for="item in editorStore.characterList"
          :key="item.character_id"
          :class="{ active: editorStore.selectedEntityId === item.character_id }"
          @click="handleCharacterSelect(item.character_id)"
        >
          <div class="character-list-item">
            <span class="character-id">{{ item.character_id }}</span>
            <NTag size="tiny" type="info">
              {{ item.base_attributes_summary }}
            </NTag>
          </div>
        </NListItem>
        <NEmpty v-if="!editorStore.characterList.length" size="small" description="无角色" />
      </NList>
    </div>

    <!-- Relationship List -->
    <div v-if="activeTab === 'relationship'" class="nav-section">
      <div class="section-header">
        <span class="section-title">关系</span>
      </div>
      <NEmpty size="small" description="关系编辑即将上线" />
    </div>

    <!-- World Rules -->
    <div v-if="activeTab === 'world_rules'" class="nav-section">
      <div class="section-header">
        <span class="section-title">世界规则</span>
      </div>
      <NList hoverable clickable style="background: transparent">
        <NListItem
          :class="{ active: editorStore.selectedEntityType === 'world_rules' }"
          @click="editorStore.selectEntity('world_rules', 'world_argument')"
        >
          <div class="rule-list-item">
            <NIcon :size="14"><SettingsOutline /></NIcon>
            <span>world_argument.yaml</span>
          </div>
        </NListItem>
      </NList>
    </div>
  </div>
</template>

<style scoped>
.entity-nav {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.nav-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
  padding: 6px;
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 8px;
  border: none;
  border-radius: 6px;
  background: transparent;
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
  cursor: pointer;
  transition: all 0.15s;
}

.tab-btn:hover {
  background: var(--color-bg-hover, #f2f3f5);
}

.tab-btn.active {
  background: var(--color-primary-light, #e6f7ff);
  color: var(--color-primary, #1890ff);
  font-weight: 600;
}

.nav-section {
  flex: 1;
  overflow: auto;
  min-height: 0;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary, #6b7280);
}

.section-title {
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.knowledge-list-item,
.character-list-item,
.rule-list-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 2px 0;
}

.knowledge-title,
.character-list-item,
.rule-list-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.knowledge-id,
.character-id {
  font-family: monospace;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.knowledge-summary {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

:deep(.n-list-item.active) {
  background: var(--color-primary-light, #e6f7ff);
}
</style>
