<script setup lang="ts">
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme, lightTheme } from 'naive-ui'
import { computed, onMounted, onUnmounted, watch } from 'vue'
import { useAppShellStore } from '@/stores/appShell'
import AppLayout from '@/components/layout/AppLayout.vue'

const appShell = useAppShellStore()

// 检测系统主题偏好
function getSystemPrefersDark(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

const theme = computed(() => {
  if (appShell.theme === 'dark') {
    return darkTheme
  }
  if (appShell.theme === 'light') {
    return lightTheme
  }
  // system: 检测系统偏好
  return getSystemPrefersDark() ? darkTheme : lightTheme
})

// 更新 HTML class
function updateHtmlClass() {
  const html = document.documentElement
  const themeValue = appShell.theme
  html.classList.remove('dark', 'light')
  if (themeValue === 'dark') {
    html.classList.add('dark')
  } else if (themeValue === 'light') {
    html.classList.add('light')
  } else {
    // system: 根据系统偏好设置 class
    if (getSystemPrefersDark()) {
      html.classList.add('dark')
    }
  }
}

// 监听系统主题变化
let mediaQuery: MediaQueryList | null = null
function handleSystemThemeChange() {
  if (appShell.theme === 'system') {
    updateHtmlClass()
  }
}

onMounted(() => {
  updateHtmlClass()
  mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  mediaQuery.addEventListener('change', handleSystemThemeChange)
})

onUnmounted(() => {
  if (mediaQuery) {
    mediaQuery.removeEventListener('change', handleSystemThemeChange)
  }
})

watch(
  () => appShell.theme,
  () => {
    updateHtmlClass()
  }
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
