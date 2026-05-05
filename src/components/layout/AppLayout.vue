<script setup lang="ts">
import { NLayout, NLayoutSider, NLayoutContent } from 'naive-ui'
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useAppShellStore } from '@/stores/appShell'
import AppNav from './AppNav.vue'
import ContextList from './ContextList.vue'
import InspectPanel from './InspectPanel.vue'

const route = useRoute()
const appShell = useAppShellStore()

const showContextList = computed(() => {
  // Pages that show context list
  // Note: 'api-configs' has its own list layout, so excluded here
  const contextPages = ['library', 'st-chat', 'agent-worlds', 'resources-characters', 'resources-worldbooks', 'resources-presets', 'resources-regex']
  return contextPages.includes(route.name as string)
})

const showInspectPanel = computed(() => {
  // Pages that show inspect panel
  const inspectPages = ['st-chat', 'agent-worlds', 'agent-world-editor']
  return inspectPages.includes(route.name as string) && appShell.inspectPanelOpen
})

const contextSiderContentStyle = {
  height: '100%',
  minHeight: '0',
  display: 'flex',
  flexDirection: 'column',
  overflow: 'hidden',
} as const
</script>

<template>
  <div class="app-shell">
    <!-- Primary Navigation -->
    <AppNav />

    <!-- Main Layout Area -->
    <NLayout class="main-layout" has-sider :native-scrollbar="false">
      <!-- Context List -->
      <NLayoutSider
        v-if="showContextList"
        bordered
        :width="appShell.contextListWidth"
        :collapsed-width="220"
        :content-style="contextSiderContentStyle"
        class="context-sider"
      >
        <ContextList />
      </NLayoutSider>

      <!-- Main Content -->
      <NLayoutContent class="main-content" :native-scrollbar="false">
        <router-view />
      </NLayoutContent>

      <!-- Inspect Panel -->
      <NLayoutSider
        v-if="showInspectPanel"
        bordered
        :width="appShell.inspectPanelWidth"
        class="inspect-sider"
      >
        <InspectPanel />
      </NLayoutSider>
    </NLayout>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100vh;
  width: 100vw;
  display: flex;
  overflow: hidden;
}

.main-layout {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.context-sider {
  background-color: var(--color-bg-surface, #f5f7fa);
}

.context-sider :deep(.n-layout-sider-scroll-container) {
  overflow: hidden !important;
}

.main-content {
  height: 100%;
}

.main-content :deep(.n-layout-scroll-container) {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.inspect-sider {
  background-color: var(--color-bg-surface, #f5f7fa);
}
</style>
