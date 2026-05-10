<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NCard, NIcon, NPagination, NSelect, NSpace, useDialog, useMessage } from 'naive-ui'
import { AddOutline, ChatbubblesOutline, TrashOutline } from '@vicons/ionicons5'
import { useChatStore } from '@/stores/chat'
import { useCharactersStore } from '@/stores/characters'
import type { ChatSession } from '@/types/st'

const props = defineProps<{
  showCreateButton?: boolean
}>()

const emit = defineEmits<{
  (e: 'create'): void
  (e: 'sessionSelected', session: ChatSession): void
}>()

const router = useRouter()
const chatStore = useChatStore()
const charactersStore = useCharactersStore()
const dialog = useDialog()
const message = useMessage()

const page = ref(1)
const pageSize = ref(8)
const selectedCharacterFilter = ref('')

const recentSessions = computed(() =>
  [...chatStore.sessions]
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()),
)

const characterFilterOptions = computed(() => [
  { label: '全部角色', value: '' },
  ...charactersStore.characters.map((item) => ({
    label: item.character.data.name || '未命名角色',
    value: item.id,
  })),
])

const filteredSessions = computed(() => {
  if (!selectedCharacterFilter.value) return recentSessions.value
  return recentSessions.value.filter((s) => s.character_id === selectedCharacterFilter.value)
})

const paginatedSessions = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return filteredSessions.value.slice(start, start + pageSize.value)
})

function formatTime(value: string) {
  return new Date(value).toLocaleString()
}

function confirmDeleteSession(session: { id: string; name: string }) {
  dialog.warning({
    title: '删除会话',
    content: `确定删除会话 "${session.name}"？此操作不可恢复。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await chatStore.deleteSession(session.id)
        message.success('会话已删除')
        const totalPages = Math.ceil(filteredSessions.value.length / pageSize.value)
        if (page.value > totalPages && page.value > 1) {
          page.value = totalPages || 1
        }
      } catch (e) {
        message.error(`删除失败: ${String(e)}`)
      }
    },
  })
}

function openSession(session: ChatSession) {
  emit('sessionSelected', session)
  router.push({ name: 'st-chat', params: { sessionId: session.id } })
}
</script>

<template>
  <div class="session-list-panel">
    <div class="panel-header">
      <NSelect
        v-model:value="selectedCharacterFilter"
        :options="characterFilterOptions"
        placeholder="按角色卡筛选"
        size="small"
        style="width: 160px"
        @update:value="page = 1"
      />
      <NButton v-if="showCreateButton" size="small" type="success" @click="emit('create')">
        <template #icon>
          <NIcon><AddOutline /></NIcon>
        </template>
        新建会话
      </NButton>
    </div>

    <div v-if="paginatedSessions.length" class="session-grid">
      <NCard
        v-for="session in paginatedSessions"
        :key="session.id"
        size="small"
        class="session-card"
        hoverable
        @click="openSession(session)"
      >
        <div class="session-card-content">
          <div class="session-info">
            <div class="session-name">{{ session.name }}</div>
            <div class="session-meta">{{ formatTime(session.updated_at) }}</div>
          </div>
          <NSpace @click.stop>
            <NButton size="small" secondary @click="openSession(session)">
              打开
            </NButton>
            <NButton size="small" type="error" secondary @click="confirmDeleteSession(session)">
              <template #icon>
                <NIcon><TrashOutline /></NIcon>
              </template>
            </NButton>
          </NSpace>
        </div>
      </NCard>
    </div>
    <div v-else class="empty-state">
      <div class="empty-icon">
        <NIcon :component="ChatbubblesOutline" :size="48" />
      </div>
      <div class="empty-title">暂无 ST 会话</div>
      <div class="empty-desc">创建一个会话，开始与角色进行沉浸式的对话体验</div>
      <NButton v-if="showCreateButton" size="medium" type="primary" @click="emit('create')">
        <template #icon>
          <NIcon><AddOutline /></NIcon>
        </template>
        创建第一个会话
      </NButton>
    </div>

    <div v-if="filteredSessions.length > pageSize" class="session-pagination">
      <NPagination
        v-model:page="page"
        :page-size="pageSize"
        :item-count="filteredSessions.length"
        size="small"
      />
    </div>
  </div>
</template>

<style scoped>
.session-list-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.session-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 12px;
  align-content: start;
}

.session-card {
  cursor: pointer;
  transition: box-shadow 0.2s, transform 0.2s, border-color 0.2s;
  border: 1px solid var(--n-border-color);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
  background: var(--n-card-color);
}

.session-card:hover {
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.1);
  transform: translateY(-2px);
  border-color: color-mix(in srgb, var(--n-primary-color) 30%, transparent);
}

.session-card-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.session-info {
  min-width: 0;
  flex: 1;
}

.session-name {
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.session-meta {
  font-size: 12px;
  color: var(--color-text-secondary, #6b7280);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 64px 20px;
  text-align: center;
}

.empty-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--n-primary-color) 8%, transparent);
  color: var(--n-primary-color);
  margin-bottom: 4px;
}

.empty-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--n-text-color);
}

.empty-desc {
  font-size: 14px;
  color: var(--n-text-color-3);
  max-width: 320px;
  line-height: 1.5;
}

.session-pagination {
  display: flex;
  justify-content: flex-end;
  flex-shrink: 0;
}
</style>
