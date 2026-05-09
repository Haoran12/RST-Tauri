<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import {
  NCard,
  NTag,
  NIcon,
  NEmpty,
  NSpace,
  NButton,
  useMessage,
} from 'naive-ui'
import {
  WarningOutline,
  SkullOutline,
  InformationCircleOutline,
  FlashOutline,
} from '@vicons/ionicons5'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'

const editorStore = useAgentWorldEditorStore()
const message = useMessage()

const route = useRoute()
const worldId = computed(() => {
  const id = route.params.worldId
  return typeof id === 'string' ? id : ''
})

const hasImpact = computed(() => editorStore.impactSummary.length > 0)

const impactKindConfig: Record<string, { type: any; icon: any; label: string }> = {
  blocking: { type: 'error', icon: WarningOutline, label: '阻断' },
  destructive: { type: 'error', icon: SkullOutline, label: '破坏性' },
  warning: { type: 'warning', icon: WarningOutline, label: '警告' },
  cascade: { type: 'info', icon: FlashOutline, label: '级联' },
  info: { type: 'default', icon: InformationCircleOutline, label: '信息' },
}

async function runImpactAnalysis() {
  if (!worldId.value || !editorStore.selectedEntityId) {
    message.warning('请先选择一个实体')
    return
  }
  try {
    await editorStore.analyzeImpact(
      worldId.value,
      editorStore.selectedEntityType,
      editorStore.selectedEntityId
    )
  } catch (e) {
    message.error(`影响分析失败: ${String(e)}`)
  }
}

function clearImpact() {
  editorStore.clearImpactSummary()
}
</script>

<template>
  <NCard size="small" title="影响分析" class="impact-panel">
    <template #header-extra>
      <NSpace size="small">
        <NButton
          size="tiny"
          quaternary
          :loading="editorStore.isAnalyzingImpact"
          :disabled="!editorStore.selectedEntityId"
          @click="runImpactAnalysis"
        >
          分析
        </NButton>
        <NButton v-if="hasImpact" size="tiny" quaternary @click="clearImpact">
          清除
        </NButton>
      </NSpace>
    </template>

    <div v-if="!hasImpact" class="empty-state">
      <NEmpty size="small" description="删除或修改前运行影响分析" />
    </div>

    <NSpace v-else vertical size="small">
      <div
        v-for="(item, index) in editorStore.impactSummary"
        :key="index"
        class="impact-item"
      >
        <div class="impact-header">
          <NTag
            size="tiny"
            :type="impactKindConfig[item.kind]?.type ?? 'default'"
          >
            <template #icon>
              <NIcon>
                <component :is="impactKindConfig[item.kind]?.icon ?? InformationCircleOutline" />
              </NIcon>
            </template>
            {{ impactKindConfig[item.kind]?.label ?? item.kind }}
          </NTag>
          <span class="impact-target">
            {{ item.target_entity_type }}: {{ item.target_entity_id }}
          </span>
        </div>
        <div class="impact-desc">{{ item.description }}</div>
        <div v-if="item.affected_count" class="impact-count">
          影响 {{ item.affected_count }} 个关联项
        </div>
      </div>
    </NSpace>
  </NCard>
</template>

<style scoped>
.impact-panel {
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.empty-state {
  padding: 12px 0;
}

.impact-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
  border-radius: 6px;
  background: var(--color-bg-app, #f0f2f5);
}

.impact-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.impact-target {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.impact-desc {
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-primary, #1f2937);
}

.impact-count {
  font-size: 11px;
  color: var(--color-text-secondary, #6b7280);
}
</style>
