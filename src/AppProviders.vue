<script setup lang="ts">
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme, lightTheme } from 'naive-ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useAppShellStore } from '@/stores/appShell'
import AppLayout from '@/components/layout/AppLayout.vue'

const appShell = useAppShellStore()

// 响应式的系统主题偏好 - 立即检测，不在 onMounted 中
const systemPrefersDark = ref(
  typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches
)

// NaiveUI 主题
const theme = computed(() => {
  if (appShell.theme === 'dark') {
    return darkTheme
  }
  if (appShell.theme === 'light') {
    return lightTheme
  }
  // system: 使用响应式的系统偏好
  return systemPrefersDark.value ? darkTheme : lightTheme
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
    if (systemPrefersDark.value) {
      html.classList.add('dark')
    }
  }
}

// 监听系统主题变化
let mediaQuery: MediaQueryList | null = null
function handleSystemThemeChange(e: MediaQueryListEvent) {
  systemPrefersDark.value = e.matches
  if (appShell.theme === 'system') {
    updateHtmlClass()
  }
}

// Ctrl+S 快捷键处理
function handleKeyDown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    triggerSave()
  }
}

/**
 * 查找并触发当前焦点区域的保存按钮
 * 优先级: 弹窗 > 页面区域
 */
function triggerSave() {
  // 1. 查找 NaiveUI 弹窗 (NModal preset="card" 使用 .n-card)
  // 弹窗通常在 .n-modal-body 或 .n-card 内
  const modalCards = document.querySelectorAll('.n-modal .n-card')
  let targetContainer: Element | null = null

  // 找到最顶层（DOM顺序最后）可见的弹窗卡片
  if (modalCards.length > 0) {
    for (let i = modalCards.length - 1; i >= 0; i--) {
      const card = modalCards[i]
      // 检查弹窗是否可见（父级 .n-modal 是否显示）
      const modal = card.closest('.n-modal')
      if (modal) {
        const style = window.getComputedStyle(modal)
        if (style.display !== 'none' && style.visibility !== 'hidden') {
          targetContainer = card
          break
        }
      }
    }
  }

  // 2. 如果没有弹窗，查找页面主内容区域
  if (!targetContainer) {
    // 查找主内容区域或侧边栏面板
    targetContainer = document.querySelector('.main-content') ||
                      document.querySelector('.n-layout-content') ||
                      document.body
  }

  // 3. 在目标容器中查找保存按钮
  if (targetContainer) {
    const saveButtons = targetContainer.querySelectorAll('button')

    // 优先查找带有 "保存" 文字的按钮
    for (const btn of saveButtons) {
      const text = btn.textContent?.trim()
      if (text === '保存' || text?.startsWith('保存')) {
        ;(btn as HTMLButtonElement).click()
        return
      }
    }

    // 查找 type="primary" 的按钮（通常是主要操作按钮）
    for (const btn of saveButtons) {
      if (btn.classList.contains('n-button--primary-type')) {
        const text = btn.textContent?.trim()
        // 排除明显的非保存按钮
        if (text && !['取消', '关闭', '删除', '移除', '确定', '创建', '新建'].includes(text)) {
          ;(btn as HTMLButtonElement).click()
          return
        }
      }
    }
  }
}

onMounted(() => {
  updateHtmlClass()
  mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  mediaQuery.addEventListener('change', handleSystemThemeChange)
  window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  if (mediaQuery) {
    mediaQuery.removeEventListener('change', handleSystemThemeChange)
  }
  window.removeEventListener('keydown', handleKeyDown)
})

watch(
  () => appShell.theme,
  () => {
    updateHtmlClass()
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
