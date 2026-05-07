<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { markdown } from '@codemirror/lang-markdown'
import { createStructuredTextEditor, type StructuredTextEditorController } from './structured-text-editor/cm6Setup'

const props = defineProps<{
  content: string
  maxHeight?: number
}>()

const containerRef = ref<HTMLElement | null>(null)
let controller: StructuredTextEditorController | null = null

function initEditor() {
  if (!containerRef.value) return

  if (controller) {
    controller.destroy()
    controller = null
  }

  controller = createStructuredTextEditor({
    parent: containerRef.value,
    doc: props.content,
    languageExtensions: [markdown()],
    readOnly: true,
    minHeight: 100,
    maxHeight: props.maxHeight ?? 400,
    onDocChange: () => {},
    diagnosticsProvider: () => [],
  })
}

function updateContent() {
  if (controller) {
    controller.updateDoc(props.content)
  }
}

watch(() => props.content, updateContent)

onMounted(initEditor)

onUnmounted(() => {
  if (controller) {
    controller.destroy()
    controller = null
  }
})
</script>

<template>
  <div ref="containerRef" class="markdown-viewer"></div>
</template>

<style scoped>
.markdown-viewer {
  width: 100%;
}
</style>
