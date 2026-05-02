<script setup lang="ts">
import { NList, NListItem, NEmpty, NSpin, NInput, NButton, NIcon } from 'naive-ui'
import { computed, ref } from 'vue'
import { useRoute } from 'vue-router'
import { SearchOutline, AddOutline } from '@vicons/ionicons5'

const route = useRoute()
const searchQuery = ref('')
const loading = ref(false)

// Placeholder data - will be replaced with actual data from stores
const items = ref<Array<{ id: string; name: string; type: string }>>([])

const filteredItems = computed(() => {
  if (!searchQuery.value) return items.value
  const query = searchQuery.value.toLowerCase()
  return items.value.filter(item =>
    item.name.toLowerCase().includes(query)
  )
})

const pageTitle = computed(() => {
  const titles: Record<string, string> = {
    'library': '最近',
    'st-chat': '会话',
    'agent-worlds': 'Worlds',
    'resources-characters': '角色卡',
    'resources-worldbooks': '世界书',
    'resources-presets': '预设',
    'resources-regex': 'Regex',
    'api-configs': 'API 配置',
    'logs': '日志',
  }
  return titles[route.name as string] || '列表'
})
</script>

<template>
  <div class="context-list">
    <div class="list-header">
      <span class="list-title">{{ pageTitle }}</span>
      <NButton quaternary size="small">
        <template #icon>
          <NIcon><AddOutline /></NIcon>
        </template>
      </NButton>
    </div>

    <div class="list-search">
      <NInput
        v-model:value="searchQuery"
        placeholder="搜索..."
        clearable
        size="small"
      >
        <template #prefix>
          <NIcon :size="16"><SearchOutline /></NIcon>
        </template>
      </NInput>
    </div>

    <div class="list-content">
      <NSpin :show="loading">
        <NList v-if="filteredItems.length > 0" hoverable clickable>
          <NListItem v-for="item in filteredItems" :key="item.id">
            {{ item.name }}
          </NListItem>
        </NList>
        <NEmpty v-else description="暂无数据" />
      </NSpin>
    </div>
  </div>
</template>

<style scoped>
.context-list {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.list-header {
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
}

.list-title {
  font-weight: 500;
  font-size: 14px;
}

.list-search {
  padding: 8px 12px;
}

.list-content {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px;
}
</style>