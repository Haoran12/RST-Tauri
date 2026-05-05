<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import type {
  StructuredTextDiagnostic,
  StructuredTextLanguageId,
  StructuredTextBinding,
  StructuredTextValidationResult,
} from '@/types/structuredText'
import {
  getStructuredTextLanguageOptions,
  isStructuredTextModeAllowed,
  loadStructuredTextLanguageExtensions,
  getStructuredTextLanguagePack,
} from './languageRegistry'
import { analyzeStructuredText, formatStructuredText, inferInitialStructuredTextMode } from './modeAdapters'
import { createStructuredTextEditor, type StructuredTextEditorController } from './cm6Setup'
import StructuredTextToolbar from './StructuredTextToolbar.vue'
import StructuredTextDiagnostics from './StructuredTextDiagnostics.vue'
import {
  formatStructuredText as formatStructuredTextBackend,
  validateStructuredText as validateStructuredTextBackend,
} from '@/services/storage'

const props = withDefaults(
  defineProps<{
    modelValue: string
    binding: StructuredTextBinding
    mode?: StructuredTextLanguageId
    readonly?: boolean
    minHeight?: number
    useBackendValidation?: boolean
  }>(),
  {
    mode: undefined,
    readonly: false,
    minHeight: 220,
    useBackendValidation: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'update:mode', mode: StructuredTextLanguageId): void
  (e: 'diagnostics-change', diagnostics: StructuredTextDiagnostic[]): void
  (e: 'parsed-value-change', value: unknown | undefined): void
  (e: 'validation-result', result: StructuredTextValidationResult): void
  (e: 'blur'): void
}>()

const host = ref<HTMLElement | null>(null)
const controller = shallowRef<StructuredTextEditorController | null>(null)
const currentMode = ref<StructuredTextLanguageId>(
  inferInitialStructuredTextMode(props.modelValue, props.binding, props.mode),
)
const diagnostics = ref<StructuredTextDiagnostic[]>([])
const parsedValue = ref<unknown>()

const modeOptions = computed(() => getStructuredTextLanguageOptions(props.binding))
const formatDisabled = computed(() => {
  const pack = getStructuredTextLanguagePack(currentMode.value)
  return !pack?.supportsFormat
})

function emitAnalysis(text: string) {
  const result = analyzeStructuredText(text, currentMode.value, props.binding)
  diagnostics.value = result.diagnostics
  parsedValue.value = result.parsedValue
  emit('diagnostics-change', result.diagnostics)
  emit('parsed-value-change', result.parsedValue)
}

function applyValidationResult(result: StructuredTextValidationResult) {
  diagnostics.value = result.diagnostics
  parsedValue.value = result.parsedValue
  emit('diagnostics-change', result.diagnostics)
  emit('parsed-value-change', result.parsedValue)
  emit('validation-result', result)
}

function getCurrentText() {
  return controller.value?.view.state.doc.toString() ?? props.modelValue
}

async function ensureController() {
  if (!host.value || controller.value) {
    return
  }

  const languageExtensions = await loadStructuredTextLanguageExtensions(currentMode.value)
  controller.value = createStructuredTextEditor({
    parent: host.value,
    doc: props.modelValue,
    languageExtensions,
    readOnly: props.readonly,
    minHeight: props.minHeight,
    diagnosticsProvider: () => diagnostics.value,
    onDocChange: text => {
      emit('update:modelValue', text)
      emitAnalysis(text)
    },
    onBlur: () => {
      void validateBeforeBlur()
    },
  })
  if (!props.mode) {
    emit('update:mode', currentMode.value)
  }
  emitAnalysis(props.modelValue)
}

async function switchMode(mode: StructuredTextLanguageId) {
  if (!isStructuredTextModeAllowed(props.binding, mode)) {
    return
  }

  currentMode.value = mode
  emit('update:mode', mode)
  const extensions = await loadStructuredTextLanguageExtensions(mode)
  controller.value?.reconfigureLanguage(extensions)
  controller.value?.reconfigureLinter(() => diagnostics.value)
  emitAnalysis(controller.value?.view.state.doc.toString() ?? props.modelValue)
}

function formatCurrentText() {
  const text = getCurrentText()
  if (props.useBackendValidation) {
    void runBackendFormatWithFallback(text)
    return
  }

  const result = formatStructuredText(text, currentMode.value, props.binding)

  controller.value?.updateDoc(result.text)
  emit('update:modelValue', result.text)
  applyValidationResult(result)
}

async function runBackendValidation(text: string) {
  const result = await validateStructuredTextBackend({
    text,
    mode: currentMode.value,
    binding: props.binding,
  })
  applyValidationResult(result)
  return result
}

async function runBackendFormat(text: string) {
  const result = await formatStructuredTextBackend({
    text,
    mode: currentMode.value,
    binding: props.binding,
  })
  controller.value?.updateDoc(result.text)
  emit('update:modelValue', result.text)
  applyValidationResult(result)
  return result
}

async function runBackendFormatWithFallback(text: string) {
  try {
    return await runBackendFormat(text)
  } catch (error) {
    try {
      const result = formatStructuredText(text, currentMode.value, props.binding)
      result.diagnostics.unshift({
        severity: 'warning',
        code: 'parse_error',
        message: `后端格式化失败，已使用前端格式化：${error instanceof Error ? error.message : String(error)}`,
        line: 1,
        column: 1,
      })
      controller.value?.updateDoc(result.text)
      emit('update:modelValue', result.text)
      applyValidationResult(result)
      return result
    } catch (fallbackError) {
      const result: StructuredTextValidationResult = {
        text,
        diagnostics: [
          {
            severity: 'blocker',
            code: 'parse_error',
            message: `Format 失败：${fallbackError instanceof Error ? fallbackError.message : String(fallbackError)}`,
            line: 1,
            column: 1,
          },
        ],
      }
      applyValidationResult(result)
      return result
    }
  }
}

async function validateBeforeBlur() {
  if (!props.useBackendValidation) {
    emit('blur')
    return
  }

  const text = getCurrentText()
  await runBackendValidation(text)
  emit('blur')
}

async function validateEditor() {
  const text = getCurrentText()
  if (props.useBackendValidation) {
    return await runBackendValidation(text)
  }

  const result = {
    text,
    ...analyzeStructuredText(text, currentMode.value, props.binding),
  }
  applyValidationResult(result)
  return result
}

async function formatEditor() {
  const text = getCurrentText()
  if (props.useBackendValidation) {
    return await runBackendFormatWithFallback(text)
  }

  const result = formatStructuredText(text, currentMode.value, props.binding)
  controller.value?.updateDoc(result.text)
  emit('update:modelValue', result.text)
  applyValidationResult(result)
  return result
}

defineExpose({
  validate: validateEditor,
  format: formatEditor,
  getState: () => ({
    text: getCurrentText(),
    mode: currentMode.value,
    diagnostics: diagnostics.value,
    parsedValue: parsedValue.value,
  }),
})

watch(
  () => props.modelValue,
  value => {
    if (!controller.value) {
      emitAnalysis(value)
      return
    }

    if (controller.value.view.state.doc.toString() !== value) {
      controller.value.updateDoc(value)
      emitAnalysis(value)
    }
  },
)

watch(
  () => props.mode,
  value => {
    if (value && value !== currentMode.value) {
      void switchMode(value)
    }
  },
)

watch(
  () => props.readonly,
  value => {
    controller.value?.setReadOnly(Boolean(value))
  },
)

watch(
  () => props.binding,
  binding => {
    if (!isStructuredTextModeAllowed(binding, currentMode.value)) {
      void switchMode(binding.defaultMode)
      return
    }
    emitAnalysis(controller.value?.view.state.doc.toString() ?? props.modelValue)
  },
  { deep: true },
)

onMounted(() => {
  void ensureController()
})

onBeforeUnmount(() => {
  controller.value?.destroy()
  controller.value = null
})
</script>

<template>
  <div class="structured-editor">
    <StructuredTextToolbar
      :diagnostics="diagnostics"
      :mode="currentMode"
      :mode-options="modeOptions"
      :format-disabled="formatDisabled"
      @update:mode="(mode) => switchMode(mode)"
      @format="formatCurrentText"
    />
    <div ref="host" class="editor-host" />
    <StructuredTextDiagnostics :diagnostics="diagnostics" />
  </div>
</template>

<style scoped>
.structured-editor {
  display: grid;
  gap: 10px;
}

.editor-host {
  min-height: 220px;
}
</style>
