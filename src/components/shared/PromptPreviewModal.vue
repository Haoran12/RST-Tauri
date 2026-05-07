<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import {
  NCollapse,
  NCollapseItem,
  NEmpty,
  NIcon,
  NModal,
  NScrollbar,
  NSpin,
  NText,
} from 'naive-ui'
import { ChevronDownOutline } from '@vicons/ionicons5'
import { markdown } from '@codemirror/lang-markdown'
import { createStructuredTextEditor, type StructuredTextEditorController } from './structured-text-editor/cm6Setup'
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

// CodeMirror 编辑器控制器
const editorControllers = ref<Map<string, StructuredTextEditorController>>(new Map())

const totalTokens = computed(() => previewData.value?.total_estimated_tokens ?? 0)

const promptItems = computed(() => previewData.value?.prompt_items ?? [])

const chatMessages = computed(() => previewData.value?.chat_messages ?? [])

const hasContent = computed(() => {
  return promptItems.value.length > 0 || chatMessages.value.length > 0
})

async function loadPreview() {
  if (!props.input) return

  isLoading.value = true
  error.value = null
  previewData.value = null

  // 清理旧的编辑器
  editorControllers.value.forEach(ctrl => ctrl.destroy())
  editorControllers.value.clear()

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

function getRoleLabel(role: string): string {
  switch (role) {
    case 'system':
      return '系统'
    case 'user':
      return '用户'
    case 'assistant':
      return '助手'
    default:
      return role
  }
}

function getRoleColor(role: string): string {
  switch (role) {
    case 'system':
      return 'var(--n-info-color)'
    case 'user':
      return 'var(--n-success-color)'
    case 'assistant':
      return 'var(--n-warning-color)'
    default:
      return 'var(--n-text-color)'
  }
}

// 初始化 CodeMirror 编辑器
function initEditor(identifier: string, content: string, container: HTMLElement | null) {
  if (!container) return

  // 销毁旧的编辑器
  const oldController = editorControllers.value.get(identifier)
  if (oldController) {
    oldController.destroy()
    editorControllers.value.delete(identifier)
  }

  // 创建新编辑器
  const controller = createStructuredTextEditor({
    parent: container,
    doc: content,
    languageExtensions: [markdown()],
    readOnly: true,
    minHeight: 50,
    maxHeight: 300,
    onDocChange: () => {},
    diagnosticsProvider: () => [],
  })

  editorControllers.value.set(identifier, controller)
}

// 清理所有编辑器
function cleanupEditors() {
  editorControllers.value.forEach(ctrl => ctrl.destroy())
  editorControllers.value.clear()
}

watch(() => props.show, (show) => {
  if (show && props.input) {
    loadPreview()
  } else if (!show) {
    cleanupEditors()
  }
})

onUnmounted(() => {
  cleanupEditors()
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
          <!-- 统计信息 -->
          <div class="stats-bar">
            <NText depth="3">
              总计约 <strong>{{ totalTokens }}</strong> tokens
            </NText>
          </div>

          <!-- 提示词条目列表 -->
          <NScrollbar style="max-height: calc(80vh - 180px)">
            <div class="prompt-list">
              <NCollapse v-model:expanded-names="expandedKeys">
                <NCollapseItem
                  v-for="item in promptItems"
                  :key="item.identifier"
                  :name="item.identifier"
                >
                  <template #header>
                    <div class="item-header">
                      <span class="item-name">{{ item.name }}</span>
                      <span
                        class="item-role"
                        :style="{ color: getRoleColor(item.role) }"
                      >
                        {{ getRoleLabel(item.role) }}
                      </span>
                      <span class="item-tokens">~{{ item.estimated_tokens }} tokens</span>
                      <span v-if="item.marker" class="item-marker">标记</span>
                      <span v-if="!item.enabled" class="item-disabled">已禁用</span>
                    </div>
                  </template>

                  <div class="item-content">
                    <template v-if="item.content">
                      <div
                        :ref="(el) => { if (el && expandedKeys.includes(item.identifier)) initEditor(item.identifier, item.content, el as HTMLElement) }"
                        class="editor-container"
                      ></div>
                    </template>
                    <template v-else>
                      <NText depth="3" italic>（空内容）</NText>
                    </template>
                  </div>
                </NCollapseItem>
              </NCollapse>

              <!-- 聊天历史 -->
              <div v-if="chatMessages.length > 0" class="chat-history-section">
                <div class="section-title">
                  <NIcon :component="ChevronDownOutline" />
                  <span>聊天历史 ({{ chatMessages.length }} 条消息)</span>
                </div>
                <div class="chat-messages">
                  <div
                    v-for="(msg, index) in chatMessages"
                    :key="index"
                    class="chat-message"
                    :class="msg.role"
                  >
                    <div class="message-header">
                      <span
                        class="message-role"
                        :style="{ color: getRoleColor(msg.role) }"
                      >
                        {{ getRoleLabel(msg.role) }}
                      </span>
                    </div>
                    <div class="message-content">{{ msg.content }}</div>
                  </div>
                </div>
              </div>
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

.stats-bar {
  padding: 8px 12px;
  margin-bottom: 12px;
  background: var(--n-color-hover);
  border-radius: 6px;
}

.prompt-list {
  padding-right: 8px;
}

.item-header {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.item-name {
  font-weight: 500;
}

.item-role {
  font-size: 12px;
  padding: 2px 6px;
  border-radius: 4px;
  background: color-mix(in srgb, currentColor 10%, transparent);
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
}

.editor-container {
  width: 100%;
  min-height: 50px;
}

.chat-history-section {
  margin-top: 16px;
  border-top: 1px solid var(--n-border-color);
  padding-top: 16px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
  margin-bottom: 12px;
  color: var(--n-text-color-2);
}

.chat-messages {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.chat-message {
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--n-color-hover);
}

.chat-message.user {
  background: color-mix(in srgb, var(--n-success-color) 8%, var(--n-color-hover));
}

.chat-message.assistant {
  background: color-mix(in srgb, var(--n-warning-color) 8%, var(--n-color-hover));
}

.chat-message.system {
  background: color-mix(in srgb, var(--n-info-color) 8%, var(--n-color-hover));
}

.message-header {
  margin-bottom: 4px;
}

.message-role {
  font-size: 12px;
  font-weight: 500;
}

.message-content {
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 150px;
  overflow-y: auto;
}
</style>
