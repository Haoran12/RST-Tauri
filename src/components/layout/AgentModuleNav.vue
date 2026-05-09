<script setup lang="ts">
import { computed, type Component } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { NIcon } from 'naive-ui'
import {
  LocationOutline,
  BookOutline,
  PeopleOutline,
  LinkOutline,
  SettingsOutline,
  ChatbubbleOutline,
} from '@vicons/ionicons5'

interface ModuleItem {
  label: string
  routeName: string
  icon: Component
}

const route = useRoute()
const router = useRouter()

const modules: ModuleItem[] = [
  { label: '地点', routeName: 'agent-locations', icon: LocationOutline },
  { label: 'Knowledge', routeName: 'agent-knowledge', icon: BookOutline },
  { label: '人物', routeName: 'agent-characters', icon: PeopleOutline },
  { label: '关系', routeName: 'agent-relationships', icon: LinkOutline },
  { label: '世界规则', routeName: 'agent-rules', icon: SettingsOutline },
  { label: '会话', routeName: 'agent-sessions', icon: ChatbubbleOutline },
]

const activeKey = computed(() => {
  const name = route.name as string
  // agent-chat 和 agent-world-editor 不在这个导航中
  return name
})

function navigateTo(item: ModuleItem) {
  router.push({ name: item.routeName })
}
</script>

<template>
  <nav class="agent-module-nav" aria-label="Agent 模块导航">
    <a
      v-for="item in modules"
      :key="item.routeName"
      class="nav-item"
      :class="{ active: activeKey === item.routeName }"
      :aria-label="item.label"
      :aria-current="activeKey === item.routeName ? 'page' : undefined"
      @click="navigateTo(item)"
    >
      <NIcon class="nav-icon" :component="item.icon" />
      <span class="nav-tooltip" role="tooltip">{{ item.label }}</span>
    </a>
  </nav>
</template>

<style scoped>
.agent-module-nav {
  width: 52px;
  height: 100%;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 10px 4px;
  background-color: var(--color-bg-app, #f0f2f5);
  border-right: 1px solid var(--color-border-subtle, #e0e0e6);
  overflow: visible;
  z-index: 5;
}

.nav-item {
  position: relative;
  width: 40px;
  height: 40px;
  flex: 0 0 40px;
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
  width: 20px;
  height: 20px;
  flex: 0 0 20px;
  line-height: 20px;
}

.nav-icon :deep(svg) {
  width: 20px;
  height: 20px;
  flex: 0 0 20px;
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
</style>
