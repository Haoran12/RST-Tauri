<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import {
  NButton,
  NEmpty,
  NIcon,
  NSpin,
  NTree,
  NTag,
  useMessage,
} from 'naive-ui'
import {
  AddOutline,
  CheckmarkCircleOutline,
  SaveOutline,
  LocationOutline,
} from '@vicons/ionicons5'
import type { TreeOption } from 'naive-ui'
import { useAgentStore } from '@/stores/agent'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import LocationGraphEditor from '@/components/agent/world-editor/LocationGraphEditor.vue'

const message = useMessage()
const agentStore = useAgentStore()
const editorStore = useAgentWorldEditorStore()

const worldId = computed(() => agentStore.currentWorldId ?? '')

const locationTreeOptions = computed<TreeOption[]>(() => {
  function buildNodes(nodes: typeof editorStore.locationTree): TreeOption[] {
    return nodes.map((n) => ({
      key: n.location_id,
      label: `${n.name} (${n.canonical_level})`,
      children: buildNodes(n.children),
    }))
  }
  return buildNodes(editorStore.locationTree)
})

const selectedLocationKeys = computed(() => {
  return editorStore.selectedEntityType === 'location' && editorStore.selectedEntityId
    ? [editorStore.selectedEntityId]
    : []
})

async function loadData() {
  if (!worldId.value) return
  try {
    await editorStore.loadSnapshot(worldId.value)
    editorStore.selectEntity('location', null)
  } catch (e) {
    message.error(`加载地点数据失败: ${String(e)}`)
  }
}

async function handleLocationSelect(keys: string[]) {
  const id = keys[0] ?? null
  if (id) {
    editorStore.selectEntity('location', id)
    const loc = await editorStore.loadLocationDetail(worldId.value, id)
    if (loc) {
      editorStore.initDraft('location', id, { ...loc }, false)
    }
  }
}

async function handleValidate() {
  if (!worldId.value) return
  try {
    const result = await editorStore.validateDraft(worldId.value)
    if (result.blockers.length === 0) {
      message.success('校验通过')
    } else {
      const msgs = result.blockers.map((b, i) => `${i + 1}. ${b.message}`).join('\n')
      message.error(`校验发现 ${result.blockers.length} 个阻断问题:\n${msgs}`)
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
            <NIcon :size="16"><LocationOutline /></NIcon>
            地点
          </span>
          <NButton size="tiny" quaternary>
            <template #icon><NIcon><AddOutline /></NIcon></template>
          </NButton>
        </div>
        <NSpin :show="editorStore.isLoading">
          <NTree
            :data="locationTreeOptions"
            :selected-keys="selectedLocationKeys"
            block-line
            expand-on-click
            @update:selected-keys="handleLocationSelect"
          />
          <NEmpty v-if="!locationTreeOptions.length" size="small" description="无地点数据" />
        </NSpin>
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
          <LocationGraphEditor />
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
</style>
