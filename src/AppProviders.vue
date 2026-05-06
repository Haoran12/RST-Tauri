<script setup lang="ts">
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme, lightTheme } from 'naive-ui'
import { computed, watch } from 'vue'
import { useAppShellStore } from '@/stores/appShell'
import AppLayout from '@/components/layout/AppLayout.vue'

const appShell = useAppShellStore()

const theme = computed(() => {
  if (appShell.theme === 'dark') {
    return darkTheme
  }
  if (appShell.theme === 'light') {
    return lightTheme
  }
  return lightTheme
})

watch(
  () => appShell.theme,
  (themeValue) => {
    const html = document.documentElement
    html.classList.remove('dark', 'light')
    if (themeValue === 'dark') {
      html.classList.add('dark')
    } else if (themeValue === 'light') {
      html.classList.add('light')
    }
  },
  { immediate: true }
)

watch(
  () => appShell.chatBubbleAppearance,
  (appearance) => {
    const root = document.documentElement
    setBubbleVars(root, 'user', appearance.user.color, appearance.user.opacity)
    setBubbleVars(root, 'assistant', appearance.assistant.color, appearance.assistant.opacity)
    setBubbleVars(root, 'system', appearance.system.color, appearance.system.opacity)
  },
  { deep: true, immediate: true }
)

watch(
  () => appShell.chatMarkdownAppearance,
  (appearance) => {
    const root = document.documentElement
    setMarkdownVars(root, 'paragraph', appearance.paragraph)
    setMarkdownVars(root, 'heading', appearance.heading)
    setMarkdownVars(root, 'italic', appearance.italic)
    setMarkdownVars(root, 'bold', appearance.bold)
    setMarkdownVars(root, 'quoted', appearance.quoted)
  },
  { deep: true, immediate: true }
)

function setBubbleVars(root: HTMLElement, role: string, color: string, opacity: number) {
  root.style.setProperty(`--chat-${role}-bubble-color`, color)
  root.style.setProperty(`--chat-${role}-bubble-opacity`, `${opacity}%`)
  root.style.setProperty(`--chat-${role}-bubble-border-opacity`, `${Math.min(100, opacity + 12)}%`)
}

function setMarkdownVars(
  root: HTMLElement,
  role: string,
  style: { color: string; fontSize: number; fontWeight: number; fontStyle: string }
) {
  root.style.setProperty(`--chat-md-${role}-color`, style.color)
  root.style.setProperty(`--chat-md-${role}-font-size`, `${style.fontSize}px`)
  root.style.setProperty(`--chat-md-${role}-font-weight`, String(style.fontWeight))
  root.style.setProperty(`--chat-md-${role}-font-style`, style.fontStyle)
}
</script>

<template>
  <NConfigProvider :theme="theme">
    <NMessageProvider>
      <NDialogProvider>
        <AppLayout />
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>
