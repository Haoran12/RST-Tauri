<script setup lang="ts">
import { NEmpty, NButton, NIcon } from 'naive-ui'
import { useAppShellStore } from '@/stores/appShell'
import { CloseOutline } from '@vicons/ionicons5'

const appShell = useAppShellStore()
</script>

<template>
  <div class="inspect-panel">
    <div class="panel-header">
      <span class="panel-title">检查面板</span>
      <NButton quaternary size="small" @click="appShell.toggleInspectPanel">
        <template #icon>
          <NIcon><CloseOutline /></NIcon>
        </template>
      </NButton>
    </div>

    <div class="panel-content">
      <NEmpty description="选择项目查看详情" />
    </div>
  </div>
</template>

<style scoped>
.inspect-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.panel-header {
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
}

.panel-title {
  font-weight: 500;
  font-size: 14px;
}

.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}
</style>