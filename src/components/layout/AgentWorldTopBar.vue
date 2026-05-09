<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { NSelect, NTag } from 'naive-ui'
import { useAgentStore } from '@/stores/agent'

const router = useRouter()
const agentStore = useAgentStore()

const worldOptions = computed(() => {
  return agentStore.worlds.map((w) => ({
    label: `${w.world_id} (${w.session_count} 会话, ${w.character_count} 角色)`,
    value: w.world_id,
  }))
})

const currentWorld = computed(() =>
  agentStore.worlds.find((w) => w.world_id === agentStore.currentWorldId)
)

function switchWorld(worldId: string) {
  if (!worldId || worldId === agentStore.currentWorldId) return
  agentStore.loadWorld(worldId)
  // 切换到工作区
  router.push({ name: 'agent-worlds', params: { worldId } })
}
</script>

<template>
  <div class="agent-world-topbar">
    <div class="topbar-left">
      <span class="topbar-label">World</span>
      <NSelect
        :value="agentStore.currentWorldId ?? undefined"
        :options="worldOptions"
        size="small"
        placeholder="选择 World..."
        style="width: 240px"
        @update:value="switchWorld"
      />
      <div v-if="currentWorld" class="world-meta">
        <NTag size="tiny" :bordered="false">
          主线 {{ currentWorld.mainline_time_anchor?.display_text ?? '未初始化' }}
        </NTag>
        <NTag size="tiny" :bordered="false">
          {{ currentWorld.character_count }} 角色
        </NTag>
      </div>
    </div>
    <div class="topbar-right">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.agent-world-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 16px;
  background: var(--color-bg-surface, #fff);
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.topbar-label {
  font-weight: 600;
  font-size: 13px;
  color: var(--color-text-secondary, #6b7280);
  white-space: nowrap;
}

.world-meta {
  display: flex;
  align-items: center;
  gap: 6px;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
</style>
