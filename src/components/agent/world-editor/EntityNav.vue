<script setup lang="ts">
import { computed } from 'vue'
import { NIcon, NTag } from 'naive-ui'
import {
  GlobeOutline,
  BookOutline,
  PersonOutline,
  GitNetworkOutline,
  LinkOutline,
  SettingsOutline,
} from '@vicons/ionicons5'
import { useAgentWorldEditorStore } from '@/stores/agentWorldEditor'
import type { EditorEntityType } from '@/types/agent/worldEditor'

const editorStore = useAgentWorldEditorStore()

interface NavItem {
  type: EditorEntityType
  label: string
  icon: any
  count: number
  badge?: string
}

const items = computed<NavItem[]>(() => [
  {
    type: 'world_settings',
    label: 'World 设置',
    icon: GlobeOutline,
    count: 0,
  },
  {
    type: 'location',
    label: '地点层级',
    icon: GitNetworkOutline,
    count: editorStore.snapshot?.locations.length ?? 0,
  },
  {
    type: 'knowledge',
    label: 'Knowledge',
    icon: BookOutline,
    count: editorStore.filteredKnowledgeList.length,
    badge: editorStore.knowledgeFilterKind
      ? ` kind:${editorStore.knowledgeFilterKind.slice(0, 8)}`
      : undefined,
  },
  {
    type: 'character',
    label: '角色',
    icon: PersonOutline,
    count: editorStore.characterList.length,
  },
  {
    type: 'relationship',
    label: '关系',
    icon: LinkOutline,
    count: editorStore.snapshot?.relationships.length ?? 0,
  },
  {
    type: 'world_rules',
    label: '世界规则',
    icon: SettingsOutline,
    count: 0,
  },
])

function select(type: EditorEntityType) {
  editorStore.selectEntity(type, null)
}

const activeType = computed(() => editorStore.selectedEntityType)
</script>

<template>
  <div class="entity-nav">
    <div class="nav-header">
      <span class="nav-title">实体导航</span>
    </div>
    <div class="nav-list">
      <button
        v-for="item in items"
        :key="item.type"
        class="nav-item"
        :class="{ active: activeType === item.type }"
        @click="select(item.type)"
      >
        <NIcon :size="18"><component :is="item.icon" /></NIcon>
        <span class="nav-label">{{ item.label }}</span>
        <NTag v-if="item.count > 0" size="tiny" :bordered="false">{{ item.count }}</NTag>
        <NTag v-else-if="item.badge" size="tiny" type="info" :bordered="false">
          {{ item.badge }}
        </NTag>
      </button>
    </div>
  </div>
</template>

<style scoped>
.entity-nav {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-surface, #fff);
  border-right: 1px solid var(--color-border-subtle, #e0e0e6);
  min-width: 180px;
  width: 100%;
}

.nav-header {
  padding: 12px 14px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.nav-title {
  font-weight: 600;
  font-size: 14px;
}

.nav-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: transparent;
  cursor: pointer;
  text-align: left;
  color: var(--color-text-primary, #1f2937);
  transition: background-color 0.12s ease;
}

.nav-item:hover {
  background: var(--n-color-hover);
}

.nav-item.active {
  background: color-mix(in srgb, var(--n-primary-color, #2080f0) 12%, transparent);
  color: var(--n-primary-color, #2080f0);
  font-weight: 500;
}

.nav-label {
  flex: 1;
  font-size: 13px;
}
</style>
