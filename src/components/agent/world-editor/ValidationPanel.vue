<script setup lang="ts">
import { computed } from 'vue'
import {
  NCard,
  NTag,
  NIcon,
  NEmpty,
  NSpace,
  NButton,
} from 'naive-ui'
import {
  CheckmarkCircleOutline,
  WarningOutline,
  InformationCircleOutline,
  CloseCircleOutline,
} from '@vicons/ionicons5'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'

const editorStore = useAgentWorldEditorStore()

const hasValidation = computed(() => !!editorStore.validationResult)

const severityConfig = {
  blocker: { type: 'error' as const, icon: CloseCircleOutline, label: '阻断' },
  warning: { type: 'warning' as const, icon: WarningOutline, label: '警告' },
  info: { type: 'info' as const, icon: InformationCircleOutline, label: '信息' },
}

function clearValidation() {
  editorStore.validationResult = null
}
</script>

<template>
  <NCard size="small" title="校验结果" class="validation-panel">
    <template #header-extra>
      <NButton v-if="hasValidation" size="tiny" quaternary @click="clearValidation">
        清除
      </NButton>
    </template>

    <div v-if="!hasValidation" class="empty-state">
      <NEmpty size="small" description="尚未运行校验" />
    </div>

    <div v-else-if="editorStore.hasBlockers" class="validation-summary error">
      <NIcon :size="18"><CloseCircleOutline /></NIcon>
      <span>发现 {{ editorStore.blockers.length }} 个阻断问题</span>
    </div>

    <div v-else class="validation-summary success">
      <NIcon :size="18"><CheckmarkCircleOutline /></NIcon>
      <span>校验通过</span>
    </div>

    <NSpace v-if="hasValidation" vertical size="small" style="margin-top: 8px">
      <div
        v-for="(item, index) in [...editorStore.blockers, ...editorStore.warnings, ...editorStore.infos]"
        :key="index"
        class="validation-item"
      >
        <NTag size="tiny" :type="severityConfig[item.severity].type">
          <template #icon>
            <NIcon><component :is="severityConfig[item.severity].icon" /></NIcon>
          </template>
          {{ severityConfig[item.severity].label }}
        </NTag>
        <div class="item-content">
          <div class="item-message">{{ item.message }}</div>
          <div v-if="item.code" class="item-code">{{ item.code }}</div>
          <div v-if="item.field_path" class="item-field">字段: {{ item.field_path }}</div>
        </div>
      </div>
    </NSpace>
  </NCard>
</template>

<style scoped>
.validation-panel {
  flex-shrink: 0;
}

.empty-state {
  padding: 12px 0;
}

.validation-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
}

.validation-summary.success {
  background: #f6ffed;
  color: #52c41a;
}

.validation-summary.error {
  background: #fff2f0;
  color: #ff4d4f;
}

.validation-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  background: var(--color-bg-app, #f0f2f5);
}

.item-content {
  flex: 1;
  min-width: 0;
}

.item-message {
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-primary, #1f2937);
}

.item-code,
.item-field {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  font-family: monospace;
  margin-top: 2px;
}
</style>
