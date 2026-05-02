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
  const contextPages = ['library', 'st-chat', 'agent-worlds', 'resources-characters', 'resources-worldbooks', 'resources-presets', 'resources-regex', 'api-configs', 'logs']
  return contextPages.includes(route.name as string)
})

const showInspectPanel = computed(() => {
  // Pages that show inspect panel
  const inspectPages = ['st-chat', 'agent-worlds', 'agent-world-editor', 'logs']
  return inspectPages.includes(route.name as string) && appShell.inspectPanelOpen
})
</script>

<template>
  <NLayout class="app-layout" has-sider>
    <!-- Primary Navigation -->
    <AppNav />

    <!-- Context List -->
    <NLayoutSider
      v-if="showContextList"
      bordered
      :width="appShell.contextListWidth"
      :collapsed-width="220"
      class="context-sider"
    >
      <ContextList />
    </NLayoutSider>

    <!-- Main Content -->
    <NLayoutContent class="main-content">
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
</template>

<style scoped>
.app-layout {
  height: 100vh;
  width: 100vw;
}

.context-sider {
  background-color: var(--color-bg-surface, #f5f7fa);
}

.main-content {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.inspect-sider {
  background-color: var(--color-bg-surface, #f5f7fa);
}
</style>