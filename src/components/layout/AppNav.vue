<script setup lang="ts">
import { NMenu, NButton, NIcon, NTooltip } from 'naive-ui'
import { h, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAppShellStore } from '@/stores/appShell'
import {
  HomeOutline,
  ChatbubbleOutline,
  BookOutline,
  SettingsOutline,
  TerminalOutline,
  KeyOutline,
  MapOutline,
  PersonOutline,
} from '@vicons/ionicons5'

const router = useRouter()
const route = useRoute()
const appShell = useAppShellStore()

const menuOptions = [
  {
    label: '资源工作台',
    key: 'library',
    icon: () => h(NIcon, null, { default: () => h(HomeOutline) }),
  },
  {
    label: 'ST 聊天',
    key: 'st-chat',
    icon: () => h(NIcon, null, { default: () => h(ChatbubbleOutline) }),
  },
  {
    label: 'Agent',
    key: 'agent-worlds',
    icon: () => h(NIcon, null, { default: () => h(MapOutline) }),
  },
  {
    type: 'divider',
    key: 'd1',
  },
  {
    label: '角色卡',
    key: 'resources-characters',
    icon: () => h(NIcon, null, { default: () => h(PersonOutline) }),
  },
  {
    label: '世界书',
    key: 'resources-worldbooks',
    icon: () => h(NIcon, null, { default: () => h(BookOutline) }),
  },
  {
    label: '预设',
    key: 'resources-presets',
    icon: () => h(NIcon, null, { default: () => h(SettingsOutline) }),
  },
  {
    type: 'divider',
    key: 'd2',
  },
  {
    label: 'API 配置',
    key: 'api-configs',
    icon: () => h(NIcon, null, { default: () => h(KeyOutline) }),
  },
  {
    label: '日志',
    key: 'logs',
    icon: () => h(NIcon, null, { default: () => h(TerminalOutline) }),
  },
]

const activeKey = computed(() => route.name as string)

function handleMenuSelect(key: string) {
  router.push({ name: key })
}
</script>

<template>
  <div class="app-nav">
    <NMenu
      :options="menuOptions"
      :value="activeKey"
      :collapsed="appShell.navCollapsed"
      :collapsed-width="52"
      :collapsed-icon-size="22"
      @update:value="handleMenuSelect"
    />

    <div class="nav-footer">
      <NTooltip trigger="hover" placement="right">
        <template #trigger>
          <NButton
            quaternary
            size="small"
            @click="router.push({ name: 'settings' })"
          >
            <template #icon>
              <NIcon :size="18">
                <SettingsOutline />
              </NIcon>
            </template>
          </NButton>
        </template>
        设置
      </NTooltip>
    </div>
  </div>
</template>

<style scoped>
.app-nav {
  width: 72px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg-app, #f0f2f5);
  border-right: 1px solid var(--color-border-subtle, #e0e0e6);
}

.app-nav :deep(.n-menu) {
  flex: 1;
}

.nav-footer {
  padding: 8px;
  display: flex;
  justify-content: center;
}
</style>