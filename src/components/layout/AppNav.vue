<script setup lang="ts">
import { NButton, NButtonGroup, NIcon } from 'naive-ui'
import { computed, type Component } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import {
  HomeOutline,
  ChatbubbleOutline,
  BookOutline,
  SettingsOutline,
  TerminalOutline,
  KeyOutline,
  MapOutline,
  PersonOutline,
  CodeSlashOutline,
  LocationOutline,
  PeopleOutline,
  LinkOutline,
} from '@vicons/ionicons5'
import { useAppShellStore, type AppMode } from '@/stores/appShell'

interface NavItem {
  label: string
  key: string
  icon: Component
}

const router = useRouter()
const route = useRoute()
const appShell = useAppShellStore()

const stSections: NavItem[][] = [
  [
    { label: 'ST 首页', key: 'st-home', icon: HomeOutline },
    { label: 'ST 聊天', key: 'st-chat', icon: ChatbubbleOutline },
  ],
  [
    { label: '角色卡', key: 'resources-characters', icon: PersonOutline },
    { label: '世界书', key: 'resources-worldbooks', icon: BookOutline },
    { label: '预设', key: 'resources-presets', icon: SettingsOutline },
    { label: 'Regex', key: 'resources-regex', icon: CodeSlashOutline },
  ],
  [
    { label: 'API 配置', key: 'api-configs', icon: KeyOutline },
    { label: '日志', key: 'logs', icon: TerminalOutline },
  ],
]

const agentSections: NavItem[][] = [
  [
    { label: 'Agent 首页', key: 'agent-home', icon: HomeOutline },
  ],
  [
    { label: '地点', key: 'agent-locations', icon: LocationOutline },
    { label: 'Knowledge', key: 'agent-knowledge', icon: BookOutline },
    { label: '人物', key: 'agent-characters', icon: PeopleOutline },
    { label: '关系', key: 'agent-relationships', icon: LinkOutline },
    { label: '世界规则', key: 'agent-rules', icon: SettingsOutline },
    { label: '会话', key: 'agent-sessions', icon: ChatbubbleOutline },
  ],
  [
    { label: 'API 配置', key: 'api-configs', icon: KeyOutline },
    { label: '日志', key: 'logs', icon: TerminalOutline },
  ],
]

const displayMode = computed<AppMode>(() => {
  if (route.path.startsWith('/agent')) return 'agent'
  if (route.path.startsWith('/st')) return 'st'
  return appShell.currentMode
})

const navSections = computed(() => displayMode.value === 'agent' ? agentSections : stSections)

const activeKey = computed(() => {
  const name = route.name as string
  if (name === 'agent-chat') return 'agent-sessions'
  if (name === 'mode-select') return ''
  return name
})

function switchMode(mode: AppMode) {
  appShell.setCurrentMode(mode)
  const target = mode === 'st' ? appShell.lastStRoute || '/st' : appShell.lastAgentRoute || '/agent'
  router.push(target)
}
</script>

<template>
  <div class="app-nav">
    <div class="nav-mode-switcher">
      <NButtonGroup vertical>
        <NButton
          size="small"
          :type="appShell.currentMode === 'st' ? 'primary' : 'default'"
          @click="switchMode('st')"
        >
          ST
        </NButton>
        <NButton
          size="small"
          :type="appShell.currentMode === 'agent' ? 'primary' : 'default'"
          @click="switchMode('agent')"
        >
          AG
        </NButton>
      </NButtonGroup>
    </div>

    <nav class="nav-menu" aria-label="主导航">
      <div
        v-for="(section, sectionIndex) in navSections"
        :key="sectionIndex"
        class="nav-section"
      >
        <RouterLink
          v-for="item in section"
          :key="item.key"
          v-slot="{ href, navigate }"
          custom
          :to="{ name: item.key }"
        >
          <a
            :href="href"
            class="nav-item"
            :class="{ active: activeKey === item.key }"
            :aria-label="item.label"
            :aria-current="activeKey === item.key ? 'page' : undefined"
            @click="navigate"
          >
            <NIcon class="nav-icon" :component="item.icon" />
            <span class="nav-tooltip" role="tooltip">{{ item.label }}</span>
          </a>
        </RouterLink>
      </div>
    </nav>

    <div class="nav-footer">
      <RouterLink
        v-slot="{ href, navigate }"
        custom
        :to="{ name: 'settings' }"
      >
        <a
          :href="href"
          class="nav-item"
          :class="{ active: activeKey === 'settings' }"
          aria-label="设置"
          :aria-current="activeKey === 'settings' ? 'page' : undefined"
          @click="navigate"
        >
          <NIcon class="nav-icon nav-icon-sm" :component="SettingsOutline" />
          <span class="nav-tooltip" role="tooltip">设置</span>
        </a>
      </RouterLink>
    </div>
  </div>
</template>

<style scoped>
.app-nav {
  width: 72px;
  height: 100%;
  flex-shrink: 0;
  position: relative;
  z-index: 10;
  display: flex;
  flex-direction: column;
  align-items: center;
  background-color: var(--color-bg-app, #f0f2f5);
  border-right: 1px solid var(--color-border-subtle, #e0e0e6);
  overflow: visible;
}

.nav-menu {
  flex: 1;
  width: 100%;
  min-height: 0;
  padding: 10px 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow: visible;
}

.nav-mode-switcher {
  width: 100%;
  padding: 10px 8px 0;
  box-sizing: border-box;
}

.nav-mode-switcher :deep(.n-button-group) {
  width: 100%;
}

.nav-mode-switcher :deep(.n-button) {
  width: 100%;
}

.nav-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
}

.nav-section:last-child {
  border-bottom: 0;
}

.nav-item {
  position: relative;
  width: 44px;
  height: 44px;
  flex: 0 0 44px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 8px;
  color: var(--n-text-color-2, #4b5563);
  background: transparent;
  cursor: pointer;
  text-decoration: none;
  overflow: visible;
  transition:
    color 0.15s ease,
    background-color 0.15s ease;
}

.nav-item:hover,
.nav-item:focus-visible {
  color: var(--n-primary-color, #2080f0);
  background: color-mix(in srgb, var(--n-primary-color, #2080f0) 10%, transparent);
}

.nav-item.active {
  color: var(--n-primary-color, #2080f0);
  background: color-mix(in srgb, var(--n-primary-color, #2080f0) 16%, transparent);
}

.nav-icon {
  width: 24px;
  height: 24px;
  flex: 0 0 24px;
  line-height: 24px;
}

.nav-icon-sm {
  width: 22px;
  height: 22px;
  flex-basis: 22px;
  line-height: 22px;
}

.nav-icon :deep(svg) {
  width: 24px;
  height: 24px;
  flex: 0 0 24px;
}

.nav-icon-sm :deep(svg) {
  width: 22px;
  height: 22px;
  flex-basis: 22px;
}

.nav-tooltip {
  position: absolute;
  left: calc(100% + 10px);
  top: 50%;
  z-index: 1000;
  max-width: 160px;
  padding: 6px 9px;
  border-radius: 6px;
  background: var(--n-popover-color, var(--color-bg-surface, #fff));
  box-shadow: var(--n-box-shadow2, 0 6px 18px rgba(0, 0, 0, 0.14));
  color: var(--n-text-color-1, var(--color-text-primary, #1f2937));
  font-size: 12px;
  line-height: 1.2;
  white-space: nowrap;
  pointer-events: auto;
  cursor: pointer;
  opacity: 0;
  transform: translate(4px, -50%);
  visibility: hidden;
  transition:
    opacity 0.12s ease,
    transform 0.12s ease,
    visibility 0.12s ease;
}

.nav-item:hover .nav-tooltip,
.nav-item:focus-visible .nav-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translate(0, -50%);
}

.nav-footer {
  width: 100%;
  padding: 8px;
  display: flex;
  justify-content: center;
  overflow: visible;
}
</style>
