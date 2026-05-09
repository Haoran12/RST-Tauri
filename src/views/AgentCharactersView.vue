<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import {
  NButton,
  NEmpty,
  NIcon,
  NList,
  NListItem,
  NSpin,
  NTag,
  useMessage,
} from 'naive-ui'
import {
  AddOutline,
  CheckmarkCircleOutline,
  SaveOutline,
  PeopleOutline,
} from '@vicons/ionicons5'
import { useAgentStore } from '@/stores/agent'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import { createCharacterRecord } from '@/types/agent/character'
import { createMindModelCardKnowledgeEntry } from '@/types/agent/knowledge'
import CharacterRecordEditor from '@/components/agent/world-editor/CharacterRecordEditor.vue'

const message = useMessage()
const agentStore = useAgentStore()
const editorStore = useAgentWorldEditorStore()

const worldId = computed(() => agentStore.currentWorldId ?? '')

async function loadData() {
  if (!worldId.value) return
  try {
    await editorStore.loadSnapshot(worldId.value)
    editorStore.selectEntity('character', null)
  } catch (e) {
    message.error(`加载人物数据失败: ${String(e)}`)
  }
}

async function handleCharacterSelect(id: string) {
  editorStore.selectEntity('character', id)
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

async function handleValidate() {
  if (!worldId.value) return
  try {
    const result = await editorStore.validateDraft(worldId.value)
    if (result.blockers.length === 0) {
      message.success('校验通过')
    } else {
      message.error(`校验发现 ${result.blockers.length} 个阻断问题`)
    }
  } catch (e) {
    message.error(`校验失败: ${String(e)}`)
  }
}

async function handleCommit() {
  if (!worldId.value) return
  try {
    const result = await editorStore.commitDraft(worldId.value)
    if (result.success) {
      message.success('提交成功')
      await editorStore.loadSnapshot(worldId.value)
    } else {
      message.error(`提交失败: ${result.error ?? '未知错误'}`)
    }
  } catch (e) {
    message.error(`提交失败: ${String(e)}`)
  }
}

watch(worldId, (newId, oldId) => {
  if (newId && newId !== oldId) {
    editorStore.clearDraft()
    loadData()
  }
})

onMounted(() => {
  loadData()
})
</script>

<template>
  <div class="agent-module-view">
    <div v-if="!worldId" class="empty-world">
      <NEmpty description="请先选择一个 World" size="large">
        <template #extra>
          <p>使用顶部 World 选择器切换</p>
        </template>
      </NEmpty>
    </div>
    <div v-else class="module-layout">
      <!-- Left: Entity List -->
      <div class="module-list">
        <div class="list-header">
          <span class="list-title">
            <NIcon :size="16"><PeopleOutline /></NIcon>
            人物
          </span>
          <NButton size="tiny" quaternary @click="createNewCharacter">
            <template #icon><NIcon><AddOutline /></NIcon></template>
          </NButton>
        </div>
        <div class="list-content">
          <NSpin :show="editorStore.isLoading">
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
          </NSpin>
        </div>
      </div>

      <!-- Right: Editor -->
      <div class="module-editor">
        <!-- Toolbar -->
        <div class="editor-toolbar">
          <div class="toolbar-left">
            <NTag size="small" :type="editorStore.draft?.isDirty ? 'warning' : 'default'">
              {{ editorStore.draft?.isDirty ? '未保存' : '已同步' }}
            </NTag>
          </div>
          <div class="toolbar-right">
            <NButton
              size="small"
              secondary
              :disabled="!editorStore.canValidate"
              :loading="editorStore.isValidating"
              @click="handleValidate"
            >
              <template #icon><NIcon><CheckmarkCircleOutline /></NIcon></template>
              校验
            </NButton>
            <NButton
              size="small"
              type="primary"
              :disabled="!editorStore.canCommit"
              :loading="editorStore.isSaving"
              @click="handleCommit"
            >
              <template #icon><NIcon><SaveOutline /></NIcon></template>
              提交
            </NButton>
          </div>
        </div>

        <!-- Editor Content -->
        <div class="editor-body">
          <CharacterRecordEditor />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.agent-module-view {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.empty-world {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.module-layout {
  display: flex;
  height: 100%;
  min-height: 0;
}

.module-list {
  width: 260px;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border-subtle, #e0e0e6);
  background: var(--color-bg-surface, #fff);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.list-title {
  font-weight: 600;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.list-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 4px;
}

.module-editor {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  background: var(--color-bg-surface, #fff);
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.editor-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 12px;
}

.character-list-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
}

.character-id {
  font-family: monospace;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

:deep(.n-list-item.active) {
  background: var(--color-primary-light, #e6f7ff);
}
</style>
