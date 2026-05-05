<script setup lang="ts">
import { NCard, NEmpty } from 'naive-ui'
</script>

<template>
  <div class="placeholder-view">
    <NCard>
      <NEmpty description="Regex 管理 - 待实现" />
    </NCard>
  </div>
</template>

<style scoped>
.placeholder-view {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 24px;
  overflow: hidden;
}

.placeholder-view :deep(.n-card) {
  flex: 1;
  min-height: 0;
}
</style>