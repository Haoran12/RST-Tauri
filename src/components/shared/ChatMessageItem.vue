<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from 'vue'
import { NButton, NIcon, NTooltip } from 'naive-ui'
import { CopyOutline, CreateOutline, TrashOutline } from '@vicons/ionicons5'

const props = withDefaults(defineProps<{
  role: 'user' | 'assistant' | 'system'
  name: string
  content: string
  createdAt?: string
  floor: number
  pending?: boolean
  editable?: boolean
  deletable?: boolean
}>(), {
  createdAt: '',
  pending: false,
  editable: true,
  deletable: true,
})

const emit = defineEmits<{
  copy: []
  edit: []
  delete: []
}>()

const tokenEstimate = computed(() => estimateTokens(props.content))

// Throttled markdown rendering for streaming content
const renderedHtml = ref<string>('')
let renderScheduled = false
let lastRenderedContent = ''

function scheduleMarkdownRender() {
  if (renderScheduled) return
  renderScheduled = true

  // Use requestAnimationFrame for smooth updates
  requestAnimationFrame(() => {
    renderScheduled = false
    if (props.content !== lastRenderedContent) {
      lastRenderedContent = props.content
      renderedHtml.value = renderMarkdown(props.content)
    }
  })
}

// For non-streaming content, use computed directly
const markdownHtml = computed(() => {
  if (props.pending) {
    // For pending/streaming content, use the throttled version
    scheduleMarkdownRender()
    return renderedHtml.value || renderMarkdown(props.content)
  }
  return renderMarkdown(props.content)
})

// Watch for content changes in streaming mode
watch(() => props.content, (newContent) => {
  if (props.pending && newContent !== lastRenderedContent) {
    scheduleMarkdownRender()
  }
}, { immediate: true })

onUnmounted(() => {
  renderScheduled = false
})

const dateLabel = computed(() => {
  if (!props.createdAt) return ''
  const date = new Date(props.createdAt)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleDateString()
})

const timeLabel = computed(() => {
  if (props.pending) return '生成中'
  if (!props.createdAt) return ''
  const date = new Date(props.createdAt)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleTimeString()
})

function estimateTokens(input: string) {
  const chars = Array.from(input)
  let ascii = 0
  let nonAscii = 0
  for (const ch of chars) {
    if (ch.charCodeAt(0) <= 0x7f) ascii += 1
    else nonAscii += 1
  }
  return Math.max(1, Math.ceil(ascii / 4) + nonAscii)
}

function renderMarkdown(input: string) {
  if (!input.trim()) return '<p class="md-empty">...</p>'

  const lines = input.replace(/\r\n?/g, '\n').split('\n')
  const html: string[] = []
  let paragraph: string[] = []
  let listType: 'ul' | 'ol' | null = null
  let inCode = false
  let codeLang = ''
  let codeLines: string[] = []

  const closeParagraph = () => {
    if (!paragraph.length) return
    html.push(`<p>${paragraph.map(renderInline).join('<br>')}</p>`)
    paragraph = []
  }

  const closeList = () => {
    if (!listType) return
    html.push(`</${listType}>`)
    listType = null
  }

  for (const line of lines) {
    const fence = line.match(/^```([A-Za-z0-9_-]+)?\s*$/)
    if (fence) {
      if (inCode) {
        html.push(`<pre><code${codeLang ? ` data-lang="${escapeHtml(codeLang)}"` : ''}>${escapeHtml(codeLines.join('\n'))}</code></pre>`)
        inCode = false
        codeLang = ''
        codeLines = []
      } else {
        closeParagraph()
        closeList()
        inCode = true
        codeLang = fence[1] ?? ''
      }
      continue
    }

    if (inCode) {
      codeLines.push(line)
      continue
    }

    if (!line.trim()) {
      closeParagraph()
      closeList()
      continue
    }

    const heading = line.match(/^(#{1,3})\s+(.+)$/)
    if (heading) {
      closeParagraph()
      closeList()
      const level = heading[1].length + 2
      html.push(`<h${level}>${renderInline(heading[2])}</h${level}>`)
      continue
    }

    const quote = line.match(/^>\s?(.+)$/)
    if (quote) {
      closeParagraph()
      closeList()
      html.push(`<blockquote>${renderInline(quote[1])}</blockquote>`)
      continue
    }

    const unordered = line.match(/^\s*[-*+]\s+(.+)$/)
    if (unordered) {
      closeParagraph()
      if (listType !== 'ul') {
        closeList()
        html.push('<ul>')
        listType = 'ul'
      }
      html.push(`<li>${renderInline(unordered[1])}</li>`)
      continue
    }

    const ordered = line.match(/^\s*\d+\.\s+(.+)$/)
    if (ordered) {
      closeParagraph()
      if (listType !== 'ol') {
        closeList()
        html.push('<ol>')
        listType = 'ol'
      }
      html.push(`<li>${renderInline(ordered[1])}</li>`)
      continue
    }

    closeList()
    paragraph.push(line)
  }

  if (inCode) {
    html.push(`<pre><code${codeLang ? ` data-lang="${escapeHtml(codeLang)}"` : ''}>${escapeHtml(codeLines.join('\n'))}</code></pre>`)
  }
  closeParagraph()
  closeList()

  return html.join('')
}

function renderInline(raw: string) {
  const codes: string[] = []
  const masked = raw.replace(/`([^`]+)`/g, (_, code: string) => {
    const index = codes.push(`<code>${escapeHtml(code)}</code>`) - 1
    return `\uE000${index}\uE000`
  })

  let text = escapeHtml(masked)
  text = text.replace(/\[([^\]]+)]\(([^)\s]+)\)/g, (_, label: string, href: string) => {
    const safeHref = sanitizeHref(href)
    if (!safeHref) return label
    return `<a href="${safeHref}" target="_blank" rel="noreferrer">${label}</a>`
  })
  text = text.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
  text = text.replace(/\*([^*]+)\*/g, '<em>$1</em>')
  text = text.replace(/~~([^~]+)~~/g, '<del>$1</del>')
  text = text.replace(/&quot;([^&]+?)&quot;/g, '&quot;<span class="md-quote">$1</span>&quot;')
  text = text.replace(/“([^”]+?)”/g, '“<span class="md-quote">$1</span>”')
  text = text.replace(/\uE000(\d+)\uE000/g, (_, index: string) => codes[Number(index)] ?? '')
  return text
}

function sanitizeHref(href: string) {
  const value = href.replace(/&amp;/g, '&').trim()
  if (/^(https?:|mailto:|#|\/)/i.test(value)) return escapeHtml(value)
  return ''
}

function escapeHtml(raw: string) {
  return raw
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}
</script>

<template>
  <article :class="['chat-message-item', role]">
    <div class="message-shell">
      <div class="message-header">
        <span class="message-author">{{ name }}</span>
        <span class="message-meta">
          <span>#{{ floor }}</span>
          <span v-if="dateLabel">{{ dateLabel }}</span>
          <span v-if="timeLabel">{{ timeLabel }}</span>
          <span>~{{ tokenEstimate }}t</span>
        </span>
      </div>
      <div class="message-bubble">
        <div class="message-markdown" v-html="markdownHtml" />
        <slot name="attachments" />
      </div>
      <div class="message-actions">
        <NTooltip trigger="hover">
          <template #trigger>
            <NButton quaternary circle size="tiny" @click="emit('copy')">
              <template #icon><NIcon :component="CopyOutline" /></template>
            </NButton>
          </template>
          复制
        </NTooltip>
        <NTooltip v-if="editable" trigger="hover">
          <template #trigger>
            <NButton quaternary circle size="tiny" @click="emit('edit')">
              <template #icon><NIcon :component="CreateOutline" /></template>
            </NButton>
          </template>
          修改
        </NTooltip>
        <NTooltip v-if="deletable" trigger="hover">
          <template #trigger>
            <NButton quaternary circle size="tiny" type="error" @click="emit('delete')">
              <template #icon><NIcon :component="TrashOutline" /></template>
            </NButton>
          </template>
          删除
        </NTooltip>
      </div>
    </div>
  </article>
</template>

<style scoped>
.chat-message-item {
  display: flex;
  width: 100%;
  margin-bottom: 22px;
}

.chat-message-item.user {
  justify-content: flex-end;
}

.chat-message-item.assistant,
.chat-message-item.system {
  justify-content: center;
}

.message-shell {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  max-width: min(760px, 82%);
  min-width: 180px;
}

.chat-message-item.user .message-shell {
  align-items: flex-end;
}

.chat-message-item.assistant .message-shell,
.chat-message-item.system .message-shell {
  align-items: flex-start;
}

.message-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 5px;
}

.chat-message-item.user .message-header {
  flex-direction: row-reverse;
}

.chat-message-item.assistant .message-header,
.chat-message-item.system .message-header {
  flex-direction: row;
}

.message-author {
  font-size: 12px;
  font-weight: 600;
  color: var(--n-text-color-2);
}

.message-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 8px;
  font-size: 11px;
  color: var(--n-text-color-3);
}

.message-bubble {
  max-width: 100%;
  padding: 12px 15px;
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  background: var(--n-color);
  color: var(--n-text-color);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
}

.chat-message-item.user .message-bubble {
  background: color-mix(in srgb, var(--chat-user-bubble-color, var(--n-primary-color)) var(--chat-user-bubble-opacity, 16%), var(--n-color));
  border-color: color-mix(in srgb, var(--chat-user-bubble-color, var(--n-primary-color)) var(--chat-user-bubble-border-opacity, 28%), var(--n-border-color));
}

.chat-message-item.assistant .message-bubble {
  background: color-mix(in srgb, var(--chat-assistant-bubble-color, var(--n-success-color)) var(--chat-assistant-bubble-opacity, 7%), var(--n-color));
  border-color: color-mix(in srgb, var(--chat-assistant-bubble-color, var(--n-success-color)) var(--chat-assistant-bubble-border-opacity, 16%), var(--n-border-color));
}

.chat-message-item.system .message-bubble {
  background: color-mix(in srgb, var(--chat-system-bubble-color, var(--n-text-color-3)) var(--chat-system-bubble-opacity, 9%), var(--n-color));
  border-color: color-mix(in srgb, var(--chat-system-bubble-color, var(--n-text-color-3)) var(--chat-system-bubble-border-opacity, 18%), var(--n-border-color));
}

.message-actions {
  display: flex;
  gap: 2px;
  margin-top: 4px;
}

.chat-message-item.assistant .message-actions,
.chat-message-item.system .message-actions {
  justify-content: center;
}

.message-markdown {
  line-height: 1.65;
  word-break: break-word;
  overflow-wrap: anywhere;
  color: var(--chat-md-paragraph-color, var(--n-text-color));
  font-size: var(--chat-md-paragraph-font-size, 14px);
  font-style: var(--chat-md-paragraph-font-style, normal);
  font-weight: var(--chat-md-paragraph-font-weight, 400);
}

.message-markdown :deep(p) {
  margin: 0;
  color: var(--chat-md-paragraph-color, var(--n-text-color));
  font-size: var(--chat-md-paragraph-font-size, 14px);
  font-style: var(--chat-md-paragraph-font-style, normal);
  font-weight: var(--chat-md-paragraph-font-weight, 400);
}

.message-markdown :deep(p + p),
.message-markdown :deep(p + ul),
.message-markdown :deep(p + ol),
.message-markdown :deep(ul + p),
.message-markdown :deep(ol + p),
.message-markdown :deep(pre + p),
.message-markdown :deep(blockquote + p) {
  margin-top: 10px;
}

.message-markdown :deep(h3),
.message-markdown :deep(h4),
.message-markdown :deep(h5) {
  margin: 0 0 8px;
  line-height: 1.35;
  color: var(--chat-md-heading-color, var(--n-text-color));
  font-size: var(--chat-md-heading-font-size, 16px);
  font-style: var(--chat-md-heading-font-style, normal);
  font-weight: var(--chat-md-heading-font-weight, 700);
}

.message-markdown :deep(ul),
.message-markdown :deep(ol) {
  margin: 0;
  padding-left: 20px;
}

.message-markdown :deep(li + li) {
  margin-top: 3px;
}

.message-markdown :deep(blockquote) {
  margin: 0;
  padding: 4px 0 4px 10px;
  border-left: 3px solid var(--n-border-color);
  color: var(--n-text-color-2);
}

.message-markdown :deep(pre) {
  max-width: 100%;
  margin: 0;
  padding: 10px 12px;
  overflow: auto;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.06);
}

.message-markdown :deep(code) {
  padding: 1px 4px;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.07);
  font-family: var(--font-mono);
  font-size: 0.92em;
}

.message-markdown :deep(em) {
  color: var(--chat-md-italic-color, inherit);
  font-size: var(--chat-md-italic-font-size, inherit);
  font-style: var(--chat-md-italic-font-style, italic);
  font-weight: var(--chat-md-italic-font-weight, inherit);
}

.message-markdown :deep(strong) {
  color: var(--chat-md-bold-color, inherit);
  font-size: var(--chat-md-bold-font-size, inherit);
  font-style: var(--chat-md-bold-font-style, normal);
  font-weight: var(--chat-md-bold-font-weight, 700);
}

.message-markdown :deep(.md-quote) {
  color: var(--chat-md-quoted-color, inherit);
  font-size: var(--chat-md-quoted-font-size, inherit);
  font-style: var(--chat-md-quoted-font-style, normal);
  font-weight: var(--chat-md-quoted-font-weight, 500);
}

.message-markdown :deep(pre code) {
  padding: 0;
  background: transparent;
}

.message-markdown :deep(a) {
  color: var(--n-primary-color);
}

.md-empty {
  color: var(--n-text-color-3);
}

@media (max-width: 720px) {
  .message-shell {
    max-width: 92%;
  }
}
</style>
