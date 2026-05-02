<script setup lang="ts">
import { NCard, NForm, NFormItem, NSelect } from 'naive-ui'
import { ref } from 'vue'
import { useAppShellStore } from '@/stores/appShell'

const appShell = useAppShellStore()

const themeOptions = [
  { label: '跟随系统', value: 'system' },
  { label: '亮色', value: 'light' },
  { label: '暗色', value: 'dark' },
]

const selectedTheme = ref(appShell.theme)

function handleThemeChange(value: 'system' | 'light' | 'dark') {
  appShell.setTheme(value)
}
</script>

<template>
  <div class="settings-view">
    <div class="page-header">
      <h1 class="page-title">设置</h1>
    </div>

    <div class="page-content">
      <NCard title="外观">
        <NForm label-placement="left" label-width="100">
          <NFormItem label="主题">
            <NSelect
              v-model:value="selectedTheme"
              :options="themeOptions"
              style="width: 200px"
              @update:value="handleThemeChange"
            />
          </NFormItem>
        </NForm>
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  padding: 16px 24px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
}

.page-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.page-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}
</style>