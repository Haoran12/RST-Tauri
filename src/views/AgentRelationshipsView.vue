<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import {
  NButton,
  NEmpty,
  NIcon,
  NSpin,
  NTag,
  useMessage,
} from 'naive-ui'
import {
  CheckmarkCircleOutline,
  SaveOutline,
  LinkOutline,
} from '@vicons/ionicons5'
import { useAgentStore } from '@/stores/agent'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'

const message = useMessage()
const agentStore = useAgentStore()
const editorStore = useAgentWorldEditorStore()

const worldId = computed(() => agentStore.currentWorldId ?? '')

async function loadData() {
  if (!worldId.value) return
  try {
    await editorStore.loadSnapshot(worldId.value)
    editorStore.selectEntity('relationship', null)
  } catch (e) {
    message.error(`加载关系数据失败: ${String(e)}`)
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
            <NIcon :size="16"><LinkOutline /></NIcon>
            关系
          </span>
        </div>
        <div class="list-content">
          <NSpin :show="editorStore.isLoading">
            <NEmpty size="small" description="关系编辑即将上线" />
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
            >
              <template #icon><NIcon><CheckmarkCircleOutline /></NIcon></template>
              校验
            </NButton>
            <NButton
              size="small"
              type="primary"
              :disabled="!editorStore.canCommit"
              :loading="editorStore.isSaving"
            >
              <template #icon><NIcon><SaveOutline /></NIcon></template>
              提交
            </NButton>
          </div>
        </div>

        <!-- Editor Content -->
        <div class="editor-body">
          <NEmpty description="关系编辑器即将上线" />
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
  padding: 16px;
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
