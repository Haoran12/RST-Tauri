<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useMessage } from 'naive-ui'
import WorldEditorShell from '@/components/agent/world-editor/WorldEditorShell.vue'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'

const route = useRoute()
const message = useMessage()
const editorStore = useAgentWorldEditorStore()

const worldId = computed(() => {
  const id = route.params.worldId
  return typeof id === 'string' ? id : ''
})

async function loadWorldEditor() {
  if (!worldId.value) {
    message.warning('未指定 World ID')
    return
  }
  try {
    await editorStore.loadSnapshot(worldId.value)
  } catch (e) {
    message.error(`加载 World Editor 失败: ${String(e)}`)
  }
}

watch(worldId, (newId, oldId) => {
  if (newId && newId !== oldId) {
    editorStore.clearDraft()
    loadWorldEditor()
  }
})

onMounted(() => {
  loadWorldEditor()
})
</script>

<template>
  <div class="agent-world-editor-view">
    <WorldEditorShell />
  </div>
</template>

<style scoped>
.agent-world-editor-view {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: var(--color-bg-app, #f0f2f5);
}
</style>
