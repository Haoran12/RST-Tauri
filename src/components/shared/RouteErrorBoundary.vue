<script setup lang="ts">
import { computed, onErrorCaptured, ref, watch } from 'vue'
import { NButton, NCard, NCode, NIcon, NSpace } from 'naive-ui'
import { AlertCircleOutline, RefreshOutline } from '@vicons/ionicons5'

const props = defineProps<{
  resetKey: string
}>()

const errorMessage = ref('')

const hasError = computed(() => errorMessage.value.length > 0)

function resetError() {
  errorMessage.value = ''
}

onErrorCaptured((error) => {
  errorMessage.value = error instanceof Error ? error.message : String(error)
  return false
})

watch(
  () => props.resetKey,
  () => {
    resetError()
  },
)
</script>

<template>
  <div v-if="hasError" class="route-error-boundary">
    <NCard size="small" class="error-card">
      <div class="error-head">
        <NIcon :size="20"><AlertCircleOutline /></NIcon>
        <strong>页面渲染失败</strong>
      </div>
      <p class="error-copy">该页面在渲染时抛出了异常。切换路由会自动重置；也可以手动重试。</p>
      <NCode class="error-message" :word-wrap="true">{{ errorMessage }}</NCode>
      <NSpace style="margin-top: 12px">
        <NButton size="small" secondary @click="resetError">
          <template #icon><NIcon><RefreshOutline /></NIcon></template>
          重试
        </NButton>
      </NSpace>
    </NCard>
  </div>
  <slot v-else />
</template>

<style scoped>
.route-error-boundary {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.error-card {
  width: min(760px, 100%);
}

.error-head {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #d03050;
}

.error-copy {
  margin: 12px 0;
  color: var(--color-text-secondary, #6b7280);
  line-height: 1.6;
}

.error-message {
  display: block;
  white-space: pre-wrap;
}
</style>
