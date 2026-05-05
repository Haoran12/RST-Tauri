<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NSelect, NSpace, NTag } from 'naive-ui'
import type {
  StructuredTextDiagnostic,
  StructuredTextLanguageId,
} from '@/types/structuredText'
import { getDiagnosticSummary } from './modeAdapters'

const props = defineProps<{
  diagnostics: StructuredTextDiagnostic[]
  mode: StructuredTextLanguageId
  modeOptions: Array<{ label: string; value: string }>
  formatDisabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:mode', mode: StructuredTextLanguageId): void
  (e: 'format'): void
}>()

const summary = computed(() => getDiagnosticSummary(props.diagnostics))
</script>

<template>
  <div class="structured-toolbar">
    <NSpace align="center" :wrap="true">
      <NSelect
        :value="mode"
        size="small"
        style="width: 120px"
        :options="modeOptions"
        @update:value="(value) => emit('update:mode', value as StructuredTextLanguageId)"
      />
      <NButton size="small" secondary :disabled="formatDisabled" @click="emit('format')">
        Format
      </NButton>
      <NTag size="small" :type="summary.type">
        {{ summary.label }}
      </NTag>
    </NSpace>
  </div>
</template>

<style scoped>
.structured-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
</style>
