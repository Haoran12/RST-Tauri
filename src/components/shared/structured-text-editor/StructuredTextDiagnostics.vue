<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'
import type { StructuredTextDiagnostic } from '@/types/structuredText'

const props = defineProps<{
  diagnostics: StructuredTextDiagnostic[]
}>()

const grouped = computed(() => props.diagnostics)

function tagType(severity: StructuredTextDiagnostic['severity']) {
  if (severity === 'blocker') {
    return 'error'
  }
  if (severity === 'warning') {
    return 'warning'
  }
  return 'info'
}
</script>

<template>
  <div v-if="grouped.length > 0" class="diagnostics-panel">
    <div
      v-for="(item, index) in grouped"
      :key="`${item.code}-${item.line}-${item.column}-${index}`"
      class="diagnostic-row"
    >
      <NTag size="small" :type="tagType(item.severity)">
        {{ item.severity }}
      </NTag>
      <span class="diagnostic-message">{{ item.message }}</span>
      <span class="diagnostic-position">L{{ item.line }}:C{{ item.column }}</span>
    </div>
  </div>
</template>

<style scoped>
.diagnostics-panel {
  display: grid;
  gap: 6px;
  padding: 10px 12px;
  border: 1px solid var(--color-border-subtle);
  border-radius: 10px;
  background: var(--color-bg-subtle);
}

.diagnostic-row {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 8px;
  align-items: center;
  font-size: 12px;
}

.diagnostic-message {
  color: var(--color-text-primary);
}

.diagnostic-position {
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
}
</style>
