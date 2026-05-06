import { computed } from 'vue'

/**
 * 弹窗尺寸类型
 * - editor: 编辑内容类弹窗（需要较大空间）
 * - form: 表单类弹窗（中等尺寸）
 * - dialog: 简单确认/提示类弹窗（较小尺寸）
 */
export type ModalSizeType = 'editor' | 'form' | 'dialog'

/**
 * 弹窗尺寸配置
 */
export interface ModalSizeConfig {
  width: string
  maxWidth: string
  height?: string
  maxHeight?: string
}

/**
 * 计算弹窗尺寸的 composable
 *
 * 全屏模式（窗口宽度 >= 1200px）:
 * - editor: 60% 宽度, 80% 高度
 * - form: 50% 宽度, auto 高度
 * - dialog: 400px 宽度, auto 高度
 *
 * 非全屏模式（窗口宽度 < 1200px）:
 * - editor: 90vw 宽度, 85vh 高度
 * - form: 85vw 宽度, auto 高度
 * - dialog: min(400px, 90vw) 宽度
 */
export function useModalSize(type: ModalSizeType = 'dialog') {
  const isFullscreen = computed(() => {
    if (typeof window === 'undefined') return false
    return window.innerWidth >= 1200
  })

  const sizeConfig = computed<ModalSizeConfig>(() => {
    if (isFullscreen.value) {
      // 全屏模式
      switch (type) {
        case 'editor':
          return {
            width: '60%',
            maxWidth: '900px',
            height: '80vh',
            maxHeight: '80vh',
          }
        case 'form':
          return {
            width: '50%',
            maxWidth: '600px',
          }
        case 'dialog':
          return {
            width: '400px',
            maxWidth: '90vw',
          }
      }
    } else {
      // 非全屏模式（小窗口/移动端）
      switch (type) {
        case 'editor':
          return {
            width: '92vw',
            maxWidth: '92vw',
            height: '85vh',
            maxHeight: '85vh',
          }
        case 'form':
          return {
            width: '85vw',
            maxWidth: '600px',
          }
        case 'dialog':
          return {
            width: 'min(400px, 90vw)',
            maxWidth: '90vw',
          }
      }
    }
  })

  /**
   * 生成 style 字符串，用于 NModal 的 style 属性
   */
  const modalStyle = computed(() => {
    const config = sizeConfig.value
    const parts: string[] = []

    parts.push(`width: ${config.width}`)
    if (config.maxWidth) parts.push(`max-width: ${config.maxWidth}`)
    if (config.height) parts.push(`height: ${config.height}`)
    if (config.maxHeight) parts.push(`max-height: ${config.maxHeight}`)

    return parts.join('; ')
  })

  return {
    isFullscreen,
    sizeConfig,
    modalStyle,
  }
}

/**
 * 预定义的弹窗尺寸样式字符串
 * 可直接用于 NModal 的 style 属性
 */
export const modalSizeStyles = {
  /**
   * 编辑器/表单类弹窗（内容编辑、表单填写等）
   * 全屏: 60% 宽度, 80% 高度
   * 非全屏: 92vw 宽度, 85vh 高度
   */
  editor: 'width: min(60vw, 900px); height: 80vh; max-height: 80vh;',

  /**
   * 简单对话框（确认、提示等）
   */
  dialog: 'width: min(400px, 90vw);',
}