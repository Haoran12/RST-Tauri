<script setup lang="ts">
import { NLayout, NLayoutSider, NLayoutContent } from 'naive-ui'
import { computed, defineAsyncComponent } from 'vue'
import { useRoute } from 'vue-router'
import { useAppShellStore } from '@/stores/appShell'
import AppNav from './AppNav.vue'
import PanelLoading from './PanelLoading.vue'

const route = useRoute()
const appShell = useAppShellStore()
const STContextList = defineAsyncComponent({
  loader: () => import('./ContextList.vue'),
  loadingComponent: PanelLoading,
  delay: 80,
})
const AgentContextList = defineAsyncComponent({
  loader: () => import('./AgentContextList.vue'),
  loadingComponent: PanelLoading,
  delay: 80,
})
const STInspectPanel = defineAsyncComponent({
  loader: () => import('./InspectPanel.vue'),
  loadingComponent: PanelLoading,
  delay: 80,
})
const AgentInspectPanel = defineAsyncComponent({
  loader: () => import('./AgentInspectPanel.vue'),
  loadingComponent: PanelLoading,
  delay: 80,
})

const isAgentMode = computed(() => route.path.startsWith('/agent'))

const isWorldEditor = computed(() => route.name === 'agent-world-editor')

const showContextList = computed(() => {
  // World Editor has its own 3-column layout; do not show AppLayout context list
  if (isWorldEditor.value) return false
  const contextPages = [
    'st-home',
    'st-chat',
    'agent-home',
    'agent-worlds',
    'resources-characters',
    'resources-worldbooks',
    'resources-presets',
    'resources-regex',
  ]
  return contextPages.includes(route.name as string)
})

const showInspectPanel = computed(() => {
  // World Editor has its own validation panel; do not show AppLayout inspect panel
  if (isWorldEditor.value) return false
  const inspectPages = ['st-chat', 'agent-worlds']
  return inspectPages.includes(route.name as string) && appShell.inspectPanelOpen
})

const contextSiderContentStyle = {
  height: '100%',
  width: '100%',
  minHeight: '0',
  minWidth: '0',
  display: 'flex',
  flexDirection: 'column',
  overflow: 'hidden',
} as const

const mainLayoutContentStyle = {
  height: '100%',
  width: '100%',
  minHeight: '0',
  minWidth: '0',
  display: 'flex',
  overflow: 'hidden',
} as const

const mainContentStyle = {
  height: '100%',
  width: '100%',
  minHeight: '0',
  minWidth: '0',
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
    <NLayout
      class="main-layout"
      has-sider
      :native-scrollbar="true"
      :content-style="mainLayoutContentStyle"
    >
      <!-- Context List -->
      <NLayoutSider
        v-if="showContextList"
        bordered
        :width="appShell.contextListWidth"
        :collapsed-width="220"
        :content-style="contextSiderContentStyle"
        class="context-sider"
      >
        <component :is="isAgentMode ? AgentContextList : STContextList" />
      </NLayoutSider>

      <!-- Main Content -->
      <NLayoutContent
        class="main-content"
        :native-scrollbar="false"
        :content-style="mainContentStyle"
      >
        <div class="route-host">
          <router-view v-slot="{ Component }">
            <transition name="route-fade" mode="out-in">
              <component :is="Component" :key="route.name" />
            </transition>
          </router-view>
        </div>
      </NLayoutContent>

      <!-- Inspect Panel -->
      <NLayoutSider
        v-if="showInspectPanel"
        bordered
        :width="appShell.inspectPanelWidth"
        class="inspect-sider"
      >
        <component :is="isAgentMode ? AgentInspectPanel : STInspectPanel" />
      </NLayoutSider>
    </NLayout>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100vh;
  width: 100vw;
  min-width: 0;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.main-layout {
  flex: 1;
  min-width: 0;
  min-height: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.context-sider {
  flex-shrink: 0;
  min-height: 0;
  background-color: var(--color-bg-surface, #f5f7fa);
}

.context-sider :deep(.n-layout-sider-scroll-container) {
  overflow: hidden !important;
}

.main-content {
  flex: 1;
  height: 100%;
  width: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.main-content :deep(.n-layout-scroll-container) {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.route-host {
  flex: 1 1 auto;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.route-host > :deep(*) {
  flex: 1 1 0;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
}

/* Route transition animations */
.route-fade-enter-active,
.route-fade-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.route-fade-enter-from {
  opacity: 0;
  transform: translateX(8px);
}

.route-fade-leave-to {
  opacity: 0;
  transform: translateX(-4px);
}

.inspect-sider {
  flex-shrink: 0;
  min-height: 0;
  background-color: var(--color-bg-surface, #f5f7fa);
}
</style>
