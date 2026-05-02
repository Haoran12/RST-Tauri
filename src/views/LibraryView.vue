<script setup lang="ts">
import { NCard, NGrid, NGi, NButton, NIcon, NEmpty, NSpin } from 'naive-ui'
import { useRouter } from 'vue-router'
import { useSettingsStore } from '@/stores/settings'
import { onMounted } from 'vue'
import {
  ChatbubbleOutline,
  PeopleOutline,
  BookOutline,
  KeyOutline,
  AlertCircleOutline,
} from '@vicons/ionicons5'

const router = useRouter()
const settings = useSettingsStore()

onMounted(() => {
  settings.loadApiConfigs()
})
</script>

<template>
  <div class="library-view">
    <div class="page-header">
      <h1 class="page-title">资源工作台</h1>
    </div>

    <div class="page-content">
      <!-- API Config Status -->
      <NCard class="status-card" size="small">
        <div class="status-header">
          <NIcon :size="20"><KeyOutline /></NIcon>
          <span>API 配置状态</span>
        </div>
        <NSpin :show="settings.loading">
          <div v-if="settings.apiConfigs.length === 0" class="status-warning">
            <NIcon :size="16" color="#f0a020"><AlertCircleOutline /></NIcon>
            <span>未配置 API，请先添加 API 配置</span>
            <NButton size="small" type="primary" @click="router.push({ name: 'api-configs' })">
              添加配置
            </NButton>
          </div>
          <div v-else class="status-ok">
            <span>已配置 {{ settings.apiConfigs.length }} 个 API</span>
          </div>
        </NSpin>
      </NCard>

      <!-- Quick Actions -->
      <div class="section-title">快捷入口</div>
      <NGrid :cols="4" :x-gap="16" :y-gap="16">
        <NGi>
          <NCard class="action-card" hoverable @click="router.push({ name: 'st-chat' })">
            <div class="action-icon">
              <NIcon :size="32"><ChatbubbleOutline /></NIcon>
            </div>
            <div class="action-label">新建 ST 会话</div>
          </NCard>
        </NGi>
        <NGi>
          <NCard class="action-card" hoverable @click="router.push({ name: 'agent-worlds' })">
            <div class="action-icon">
              <NIcon :size="32"><PeopleOutline /></NIcon>
            </div>
            <div class="action-label">Agent World</div>
          </NCard>
        </NGi>
        <NGi>
          <NCard class="action-card" hoverable @click="router.push({ name: 'resources-characters' })">
            <div class="action-icon">
              <NIcon :size="32"><BookOutline /></NIcon>
            </div>
            <div class="action-label">角色卡</div>
          </NCard>
        </NGi>
        <NGi>
          <NCard class="action-card" hoverable @click="router.push({ name: 'api-configs' })">
            <div class="action-icon">
              <NIcon :size="32"><KeyOutline /></NIcon>
            </div>
            <div class="action-label">API 配置</div>
          </NCard>
        </NGi>
      </NGrid>

      <!-- Recent Sessions -->
      <div class="section-title">最近会话</div>
      <NCard size="small">
        <NEmpty description="暂无最近会话" />
      </NCard>

      <!-- Recent Resources -->
      <div class="section-title">最近资源</div>
      <NCard size="small">
        <NEmpty description="暂无最近资源" />
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.library-view {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  padding: 16px 24px;
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
}

.page-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.page-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

.status-card {
  margin-bottom: 24px;
}

.status-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  font-weight: 500;
}

.status-warning {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #f0a020;
}

.status-ok {
  color: #18a058;
}

.section-title {
  font-size: 14px;
  font-weight: 500;
  margin: 24px 0 12px 0;
  color: var(--color-text-secondary, #666);
}

.action-card {
  cursor: pointer;
  text-align: center;
  padding: 24px;
}

.action-icon {
  margin-bottom: 12px;
  color: var(--color-text-primary, #333);
}

.action-label {
  font-size: 14px;
  color: var(--color-text-secondary, #666);
}
</style>