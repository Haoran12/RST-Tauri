<script setup lang="ts">
import { NButton, NCard, NForm, NFormItem, NInputNumber, NSelect } from 'naive-ui'
import { ref } from 'vue'
import { useAppShellStore, type ChatBubbleAppearance, type ChatMarkdownAppearance } from '@/stores/appShell'

const appShell = useAppShellStore()

const themeOptions = [
  { label: '跟随系统', value: 'system' },
  { label: '亮色', value: 'light' },
  { label: '暗色', value: 'dark' },
]

const selectedTheme = ref(appShell.theme)
const bubbleRoles: Array<{ key: keyof ChatBubbleAppearance; label: string }> = [
  { key: 'user', label: 'User 气泡' },
  { key: 'assistant', label: 'Assistant 气泡' },
  { key: 'system', label: 'System 气泡' },
]
const markdownParts: Array<{ key: keyof ChatMarkdownAppearance; label: string }> = [
  { key: 'paragraph', label: '常规段落' },
  { key: 'heading', label: '标题' },
  { key: 'italic', label: '斜体' },
  { key: 'bold', label: '粗体' },
  { key: 'quoted', label: '双引号内容' },
]
const fontStyleOptions = [
  { label: '常规', value: 'normal' },
  { label: '斜体', value: 'italic' },
]
const fontWeightOptions = [
  { label: '300', value: 300 },
  { label: '400', value: 400 },
  { label: '500', value: 500 },
  { label: '600', value: 600 },
  { label: '700', value: 700 },
  { label: '800', value: 800 },
  { label: '900', value: 900 },
]

function handleThemeChange(value: 'system' | 'light' | 'dark') {
  appShell.setTheme(value)
}

function updateBubbleColor(role: keyof ChatBubbleAppearance, color: string) {
  appShell.setChatBubbleAppearance({
    ...appShell.chatBubbleAppearance,
    [role]: { ...appShell.chatBubbleAppearance[role], color },
  })
}

function updateBubbleOpacity(role: keyof ChatBubbleAppearance, opacity: number | null) {
  appShell.setChatBubbleAppearance({
    ...appShell.chatBubbleAppearance,
    [role]: { ...appShell.chatBubbleAppearance[role], opacity: opacity ?? 0 },
  })
}

function updateMarkdownColor(part: keyof ChatMarkdownAppearance, color: string) {
  appShell.setChatMarkdownAppearance({
    ...appShell.chatMarkdownAppearance,
    [part]: { ...appShell.chatMarkdownAppearance[part], color },
  })
}

function updateMarkdownSize(part: keyof ChatMarkdownAppearance, fontSize: number | null) {
  appShell.setChatMarkdownAppearance({
    ...appShell.chatMarkdownAppearance,
    [part]: { ...appShell.chatMarkdownAppearance[part], fontSize: fontSize ?? 14 },
  })
}

function updateMarkdownWeight(part: keyof ChatMarkdownAppearance, fontWeight: string | number) {
  appShell.setChatMarkdownAppearance({
    ...appShell.chatMarkdownAppearance,
    [part]: { ...appShell.chatMarkdownAppearance[part], fontWeight: Number(fontWeight) },
  })
}

function updateMarkdownStyle(part: keyof ChatMarkdownAppearance, fontStyle: string | number) {
  appShell.setChatMarkdownAppearance({
    ...appShell.chatMarkdownAppearance,
    [part]: {
      ...appShell.chatMarkdownAppearance[part],
      fontStyle: fontStyle === 'italic' ? 'italic' : 'normal',
    },
  })
}
</script>

<template>
  <div class="settings-view">
    <div class="page-header">
      <h1 class="page-title">设置</h1>
    </div>

    <div class="page-content">
      <NCard title="外观">
        <NForm label-placement="left" label-width="100">
          <NFormItem label="主题">
            <NSelect
              v-model:value="selectedTheme"
              :options="themeOptions"
              style="width: 200px"
              @update:value="handleThemeChange"
            />
          </NFormItem>
        </NForm>
      </NCard>

      <NCard title="聊天消息">
        <div class="setting-section-title">气泡颜色与透明度</div>
        <div class="appearance-grid">
          <div v-for="role in bubbleRoles" :key="role.key" class="appearance-row">
            <div class="row-label">{{ role.label }}</div>
            <label class="color-field">
              <span>颜色</span>
              <input
                type="color"
                :value="appShell.chatBubbleAppearance[role.key].color"
                @input="event => updateBubbleColor(role.key, (event.target as HTMLInputElement).value)"
              >
            </label>
            <label class="number-field">
              <span>透明度</span>
              <NInputNumber
                :value="appShell.chatBubbleAppearance[role.key].opacity"
                :min="0"
                :max="100"
                :step="1"
                @update:value="value => updateBubbleOpacity(role.key, value)"
              />
            </label>
          </div>
        </div>
        <NButton size="small" @click="appShell.resetChatBubbleAppearance">重置气泡</NButton>

        <div class="setting-section-title markdown-title">Markdown 字体样式</div>
        <div class="markdown-grid">
          <div v-for="part in markdownParts" :key="part.key" class="markdown-row">
            <div class="row-label">{{ part.label }}</div>
            <label class="color-field">
              <span>颜色</span>
              <input
                type="color"
                :value="appShell.chatMarkdownAppearance[part.key].color"
                @input="event => updateMarkdownColor(part.key, (event.target as HTMLInputElement).value)"
              >
            </label>
            <label class="number-field">
              <span>字号</span>
              <NInputNumber
                :value="appShell.chatMarkdownAppearance[part.key].fontSize"
                :min="10"
                :max="28"
                :step="1"
                @update:value="value => updateMarkdownSize(part.key, value)"
              />
            </label>
            <label class="select-field">
              <span>字重</span>
              <NSelect
                :value="appShell.chatMarkdownAppearance[part.key].fontWeight"
                :options="fontWeightOptions"
                @update:value="value => updateMarkdownWeight(part.key, value)"
              />
            </label>
            <label class="select-field">
              <span>字形</span>
              <NSelect
                :value="appShell.chatMarkdownAppearance[part.key].fontStyle"
                :options="fontStyleOptions"
                @update:value="value => updateMarkdownStyle(part.key, value)"
              />
            </label>
          </div>
        </div>
        <NButton size="small" @click="appShell.resetChatMarkdownAppearance">重置 Markdown</NButton>
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.page-header {
  padding: 16px 24px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
  flex-shrink: 0;
}

.page-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.page-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  scrollbar-width: thin;
}

.page-content::-webkit-scrollbar {
  width: 6px;
}

.page-content::-webkit-scrollbar-track {
  background: transparent;
}

.page-content::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 3px;
}

.page-content::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

.setting-section-title {
  margin-bottom: 10px;
  font-size: 13px;
  font-weight: 600;
  color: var(--n-text-color-2);
}

.markdown-title {
  margin-top: 18px;
}

.appearance-grid,
.markdown-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 12px;
}

.appearance-row,
.markdown-row {
  display: grid;
  grid-template-columns: 120px repeat(4, minmax(110px, 1fr));
  gap: 10px;
  align-items: end;
}

.appearance-row {
  grid-template-columns: 120px minmax(110px, 1fr) minmax(120px, 1fr);
}

.row-label {
  padding-bottom: 6px;
  font-weight: 600;
  color: var(--n-text-color);
}

.color-field,
.number-field,
.select-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
  font-size: 12px;
  color: var(--n-text-color-3);
}

.color-field input {
  width: 48px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--n-border-color);
  border-radius: 6px;
  background: transparent;
}

@media (max-width: 860px) {
  .appearance-row,
  .markdown-row {
    grid-template-columns: 1fr 1fr;
  }

  .row-label {
    grid-column: 1 / -1;
  }
}
</style>
