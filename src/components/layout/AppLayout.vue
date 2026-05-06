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

const isAgentMode = computed(() => appShell.currentMode === 'agent')

const showContextList = computed(() => {
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
  // Pages that show inspect panel
  const inspectPages = ['st-chat', 'agent-worlds', 'agent-world-editor']
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
        :native-scrollbar="true"
        :content-style="mainContentStyle"
      >
        <div class="route-host">
          <router-view v-slot="{ Component }">
            <Suspense>
              <component :is="Component" />
              <template #fallback>
                <div class="route-loading">
                  <div class="route-loading-header" />
                  <div class="route-loading-grid">
                    <div class="route-loading-card route-loading-card-wide" />
                    <div class="route-loading-card" />
                    <div class="route-loading-card" />
                  </div>
                </div>
              </template>
            </Suspense>
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
  flex: 1;
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

.route-loading {
  height: 100%;
  min-height: 0;
  padding: 18px 20px;
  display: grid;
  grid-template-rows: 74px 1fr;
  gap: 16px;
  background: var(--color-bg-app, #f0f2f5);
}

.route-loading-header,
.route-loading-card {
  border-radius: 8px;
  background: linear-gradient(
    90deg,
    var(--color-bg-surface, #fff),
    var(--color-bg-subtle, #f5f7fa),
    var(--color-bg-surface, #fff)
  );
  background-size: 220% 100%;
  border: 1px solid var(--color-border-subtle, #e0e0e6);
  animation: route-loading-shimmer 1.2s ease-in-out infinite;
}

.route-loading-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  align-content: start;
}

.route-loading-card {
  min-height: 150px;
}

.route-loading-card-wide {
  grid-column: 1 / -1;
}

@keyframes route-loading-shimmer {
  0% {
    background-position: 120% 0;
  }
  100% {
    background-position: -120% 0;
  }
}

.inspect-sider {
  flex-shrink: 0;
  min-height: 0;
  background-color: var(--color-bg-surface, #f5f7fa);
}
</style>
