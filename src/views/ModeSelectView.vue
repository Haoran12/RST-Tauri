<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NCard, NGrid, NGi, NIcon, NSpace, NTag } from 'naive-ui'
import { ChatbubbleOutline, KeyOutline, MapOutline, SettingsOutline } from '@vicons/ionicons5'
import { useAppShellStore, type AppMode } from '@/stores/appShell'
import { useSettingsStore } from '@/stores/settings'
import { useChatStore } from '@/stores/chat'
import { useCharactersStore } from '@/stores/characters'
import { useWorldbooksStore } from '@/stores/worldbooks'
import { usePresetsStore } from '@/stores/presets'
import { useAgentStore } from '@/stores/agent'

const router = useRouter()
const appShell = useAppShellStore()
const settingsStore = useSettingsStore()
const chatStore = useChatStore()
const charactersStore = useCharactersStore()
const worldbooksStore = useWorldbooksStore()
const presetsStore = usePresetsStore()
const agentStore = useAgentStore()

const stSummary = computed(() => ({
  sessions: chatStore.sessions.length,
  characters: charactersStore.characterCount,
  worldbooks: worldbooksStore.worldbookCount,
  presets: presetsStore.presetList.length,
}))

const agentSummary = computed(() => ({
  recentSessions: agentStore.currentWorld?.session_count ?? agentStore.sessions.length,
  activeSessions: agentStore.currentWorld?.active_session_count ?? agentStore.activeSessions.length,
  currentWorld: agentStore.currentWorldId ?? '未选择',
  worldCount: agentStore.worlds.length,
}))

const apiSummary = computed(() => {
  if (settingsStore.apiConfigs.length === 0) return '未配置 API'
  if (!settingsStore.activeApiConfig) return '未选择当前配置'
  return `${settingsStore.activeApiConfig.name} · ${settingsStore.activeApiConfig.provider}`
})

async function hydrate() {
  await Promise.allSettled([
    settingsStore.loadApiConfigs(),
    chatStore.loadSessions(),
    charactersStore.loadCharacters(),
    worldbooksStore.loadWorldbooks(),
    presetsStore.loadPresetList(),
    agentStore.loadWorldList().then((worlds) => {
      const targetWorldId = agentStore.currentWorldId ?? worlds[0]?.world_id
      if (targetWorldId) {
        return agentStore.loadWorld(targetWorldId)
      }
      agentStore.clearWorld()
      return Promise.resolve()
    }),
  ])
}

function enterMode(mode: AppMode) {
  appShell.setCurrentMode(mode)
  const target = mode === 'st' ? appShell.lastStRoute || '/st' : appShell.lastAgentRoute || '/agent'
  router.push(target)
}

onMounted(() => {
  void hydrate()
})
</script>

<template>
  <div class="mode-select-view">
    <header class="page-header">
      <div>
        <h1>选择工作模式</h1>
        <p>ST 与 Agent 使用独立工作区。共享页只保留 API 配置、日志与设置。</p>
      </div>
      <NSpace>
        <NButton secondary @click="router.push({ name: 'api-configs' })">
          <template #icon><NIcon><KeyOutline /></NIcon></template>
          API 配置
        </NButton>
        <NButton secondary @click="router.push({ name: 'settings' })">
          <template #icon><NIcon><SettingsOutline /></NIcon></template>
          设置
        </NButton>
      </NSpace>
    </header>

    <div class="mode-grid">
      <NGrid :cols="2" :x-gap="18" :y-gap="18" responsive="screen">
        <NGi>
          <NCard class="mode-card" size="large">
            <div class="mode-head">
              <div class="mode-icon st">
                <NIcon :size="28"><ChatbubbleOutline /></NIcon>
              </div>
              <div class="mode-copy">
                <div class="mode-title">ST Workspace</div>
                <div class="mode-desc">面向角色卡、世界书、预设、Regex 与文本聊天。</div>
              </div>
              <NTag type="info">当前 {{ appShell.currentMode === 'st' ? '已选' : '可进入' }}</NTag>
            </div>
            <div class="mode-stats">
              <div class="stat-row"><span>ST 会话</span><strong>{{ stSummary.sessions }}</strong></div>
              <div class="stat-row"><span>角色卡</span><strong>{{ stSummary.characters }}</strong></div>
              <div class="stat-row"><span>世界书</span><strong>{{ stSummary.worldbooks }}</strong></div>
              <div class="stat-row"><span>预设</span><strong>{{ stSummary.presets }}</strong></div>
            </div>
            <div class="mode-footer">
              <span class="footer-note">当前连接：{{ apiSummary }}</span>
              <NButton type="primary" @click="enterMode('st')">进入 ST</NButton>
            </div>
          </NCard>
        </NGi>

        <NGi>
          <NCard class="mode-card" size="large">
            <div class="mode-head">
              <div class="mode-icon agent">
                <NIcon :size="28"><MapOutline /></NIcon>
              </div>
              <div class="mode-copy">
                <div class="mode-title">Agent Workspace</div>
                <div class="mode-desc">面向 World、主线状态、会话运行和编辑器工作流。</div>
              </div>
              <NTag type="success">当前 {{ appShell.currentMode === 'agent' ? '已选' : '可进入' }}</NTag>
            </div>
            <div class="mode-stats">
              <div class="stat-row"><span>World 数量</span><strong>{{ agentSummary.worldCount }}</strong></div>
              <div class="stat-row"><span>当前 World</span><strong>{{ agentSummary.currentWorld }}</strong></div>
              <div class="stat-row"><span>已加载会话</span><strong>{{ agentSummary.recentSessions }}</strong></div>
              <div class="stat-row"><span>活动会话</span><strong>{{ agentSummary.activeSessions }}</strong></div>
            </div>
            <div class="mode-footer">
              <span class="footer-note">继续进入独立 Agent 壳层</span>
              <NButton type="primary" @click="enterMode('agent')">进入 Agent</NButton>
            </div>
          </NCard>
        </NGi>
      </NGrid>
    </div>
  </div>
</template>

<style scoped>
.mode-select-view {
  height: 100%;
  min-height: 0;
  padding: 24px 28px;
  display: flex;
  flex-direction: column;
  gap: 24px;
  background:
    radial-gradient(circle at top left, rgba(32, 128, 240, 0.1), transparent 28%),
    radial-gradient(circle at top right, rgba(24, 160, 88, 0.09), transparent 32%),
    var(--color-bg-app, #f0f2f5);
}

.page-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
}

.page-header h1 {
  margin: 0 0 8px;
  font-size: 28px;
}

.page-header p {
  margin: 0;
  color: var(--color-text-secondary, #6b7280);
}

.mode-grid {
  flex: 1;
}

.mode-card {
  height: 100%;
  border-radius: 18px;
}

.mode-head {
  display: flex;
  gap: 14px;
  align-items: flex-start;
}

.mode-icon {
  width: 56px;
  height: 56px;
  border-radius: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #fff;
}

.mode-icon.st {
  background: linear-gradient(135deg, #2080f0, #1f6ed4);
}

.mode-icon.agent {
  background: linear-gradient(135deg, #18a058, #127a44);
}

.mode-copy {
  flex: 1;
}

.mode-title {
  font-size: 20px;
  font-weight: 700;
}

.mode-desc {
  margin-top: 4px;
  color: var(--color-text-secondary, #6b7280);
  line-height: 1.6;
}

.mode-stats {
  margin-top: 22px;
  display: grid;
  gap: 12px;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--color-bg-subtle, #f5f7fa);
}

.stat-row span {
  color: var(--color-text-secondary, #6b7280);
}

.mode-footer {
  margin-top: 24px;
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: center;
}

.footer-note {
  color: var(--color-text-secondary, #6b7280);
  line-height: 1.5;
}
</style>
