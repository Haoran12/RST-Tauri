<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  NCollapse,
  NCollapseItem,
  NEmpty,
  NInput,
  NModal,
  NScrollbar,
  NSpin,
  NText,
} from 'naive-ui'
import type { PromptPreviewOutput } from '@/types/runtime'
import { previewSTPrompt } from '@/services/runtime'
import type { AssembleRequestInput } from '@/types/runtime'

const props = defineProps<{
  show: boolean
  input: AssembleRequestInput | null
}>()

const emit = defineEmits<{
  'update:show': [value: boolean]
}>()

const isLoading = ref(false)
const error = ref<string | null>(null)
const previewData = ref<PromptPreviewOutput | null>(null)
const expandedKeys = ref<string[]>([])
const searchQuery = ref('')

const totalTokens = computed(() => previewData.value?.total_estimated_tokens ?? 0)

// 搜索过滤后的条目
const filteredItems = computed(() => {
  const items = previewData.value?.prompt_items ?? []
  if (!searchQuery.value.trim()) {
    return items
  }

  const query = searchQuery.value.toLowerCase()
  return items.filter(item => {
    // 搜索名称
    if (item.name.toLowerCase().includes(query)) return true
    // 搜索标识符
    if (item.identifier.toLowerCase().includes(query)) return true
    // 搜索内容
    if (item.content.toLowerCase().includes(query)) return true
    return false
  })
})

// 匹配计数
const matchCount = computed(() => {
  if (!searchQuery.value.trim()) return 0
  return filteredItems.value.length
})

const hasContent = computed(() => {
  return (previewData.value?.prompt_items ?? []).length > 0
})

async function loadPreview() {
  if (!props.input) return

  isLoading.value = true
  error.value = null
  previewData.value = null
  searchQuery.value = ''

  try {
    const result = await previewSTPrompt(props.input)
    previewData.value = result
    // 默认展开前 3 个条目
    expandedKeys.value = result.prompt_items.slice(0, 3).map(p => p.identifier)
  } catch (e) {
    error.value = String(e)
  } finally {
    isLoading.value = false
  }
}

function handleClose() {
  emit('update:show', false)
}

// 高亮搜索文本
function highlightText(text: string): string {
  if (!searchQuery.value.trim()) return text

  const query = searchQuery.value.trim()
  const regex = new RegExp(`(${escapeRegex(query)})`, 'gi')
  return text.replace(regex, '<mark class="highlight">$1</mark>')
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

// 搜索时自动展开所有匹配的条目
watch(searchQuery, (query) => {
  if (query.trim() && previewData.value) {
    // 展开所有匹配的条目
    expandedKeys.value = filteredItems.value.map(p => p.identifier)
  }
})

watch(() => props.show, (show) => {
  if (show && props.input) {
    loadPreview()
  }
})
</script>

<template>
  <NModal
    :show="show"
    preset="card"
    title="提示词预览"
    style="width: 90%; max-width: 900px; height: 80vh"
    @update:show="handleClose"
  >
    <div class="preview-container">
      <NSpin :show="isLoading">
        <template v-if="error">
          <div class="error-state">
            <NText type="error">{{ error }}</NText>
          </div>
        </template>

        <template v-else-if="!hasContent && !isLoading">
          <div class="empty-state">
            <NEmpty description="暂无提示词预览" />
          </div>
        </template>

        <template v-else>
          <!-- 搜索框和统计信息 -->
          <div class="toolbar">
            <NInput
              v-model:value="searchQuery"
              placeholder="搜索提示词..."
              clearable
              class="search-input"
            />
            <div class="stats">
              <NText depth="3">
                总计约 <strong>{{ totalTokens }}</strong> tokens
                <template v-if="searchQuery.trim()">
                  <span class="search-result">，匹配 <strong>{{ matchCount }}</strong> 个条目</span>
                </template>
              </NText>
            </div>
          </div>

          <!-- 提示词条目列表 -->
          <NScrollbar style="max-height: calc(80vh - 220px)">
            <div class="prompt-list">
              <template v-if="filteredItems.length > 0">
                <NCollapse v-model:expanded-names="expandedKeys">
                  <NCollapseItem
                    v-for="item in filteredItems"
                    :key="item.identifier"
                    :name="item.identifier"
                  >
                    <template #header>
                      <div class="item-header">
                        <span
                          class="item-name"
                          v-html="highlightText(item.name)"
                        ></span>
                        <span class="item-tokens">~{{ item.estimated_tokens }} tokens</span>
                        <span v-if="item.marker" class="item-marker">标记</span>
                        <span v-if="!item.enabled" class="item-disabled">已禁用</span>
                      </div>
                    </template>

                    <div class="item-content">
                      <template v-if="item.content">
                        <pre
                          class="content-text"
                          v-html="highlightText(item.content)"
                        ></pre>
                      </template>
                      <template v-else>
                        <NText depth="3" italic>（空内容）</NText>
                      </template>
                    </div>
                  </NCollapseItem>
                </NCollapse>
              </template>
              <template v-else>
                <div class="no-match">
                  <NText depth="3">未找到匹配的提示词</NText>
                </div>
              </template>
            </div>
          </NScrollbar>
        </template>
      </NSpin>
    </div>
  </NModal>
</template>

<style scoped>
.preview-container {
  min-height: 200px;
}

.error-state,
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 200px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.search-input {
  flex: 1;
  min-width: 200px;
  max-width: 300px;
}

.stats {
  flex-shrink: 0;
}

.search-result {
  color: var(--n-primary-color);
}

.prompt-list {
  padding-right: 8px;
}

.no-match {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
}

.item-header {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.item-name {
  font-weight: 500;
}

.item-name :deep(.highlight) {
  background: #ffeb3b;
  color: #000;
  padding: 0 2px;
  border-radius: 2px;
}

.item-tokens {
  font-size: 12px;
  color: var(--n-text-color-3);
}

.item-marker {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--n-info-color);
  color: white;
}

.item-disabled {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--n-error-color);
  color: white;
}

.item-content {
  margin-top: 8px;
  border-radius: 6px;
  overflow: hidden;
  background: var(--n-color-embedded);
}

.content-text {
  margin: 0;
  padding: 12px;
  font-family: var(--font-mono, 'SF Mono', Monaco, 'Cascadia Code', monospace);
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 300px;
  overflow-y: auto;
}

.content-text :deep(.highlight) {
  background: #ffeb3b;
  color: #000;
  padding: 0 2px;
  border-radius: 2px;
}

/* 折叠卡片整体样式 */
:deep(.n-collapse-item) {
  margin-bottom: 8px;
  border-radius: 8px;
  background: var(--n-color-embedded);
  border: 1px solid var(--n-border-color);
}

:deep(.n-collapse-item__header) {
  padding: 14px 16px;
  min-height: 48px;
}

:deep(.n-collapse-item__content-inner) {
  padding: 0 16px 14px;
}
</style>
