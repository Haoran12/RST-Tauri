<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import {
  NCard,
  NSelect,
  NButton,
  NSpin,
  NAlert,
  NTooltip,
  NIcon,
  NDivider,
} from 'naive-ui'
import { InformationCircleOutline } from '@vicons/ionicons5'
import type { ApiConfig } from '@/types/st'
import type { AgentLlmProfile, AgentLlmNodeType } from '@/types/agent/llm-config'
import { AGENT_LLM_NODE_INFO, createDefaultAgentLlmProfile } from '@/types/agent/llm-config'
import { listApiConfigs } from '@/services/storage'

const props = defineProps<{
  worldId: string
  profile?: AgentLlmProfile | null
}>()

const emit = defineEmits<{
  (e: 'update:profile', profile: AgentLlmProfile): void
  (e: 'save'): void
}>()

// State
const apiConfigs = ref<ApiConfig[]>([])
const isLoading = ref(false)
const error = ref<string | null>(null)
const localProfile = ref<AgentLlmProfile>(createDefaultAgentLlmProfile(props.worldId))

// Computed
const apiConfigOptions = computed(() => {
  const options: Array<{ label: string; value: string | null }> = [
    { label: '继承默认配置', value: null },
  ]
  for (const config of apiConfigs.value) {
    if (config.enabled) {
      options.push({
        label: `${config.name} (${config.provider}/${config.model})`,
        value: config.id,
      })
    }
  }
  return options
})

const defaultApiConfigOptions = computed(() => {
  const options: Array<{ label: string; value: string | null }> = [
    { label: '未设置', value: null },
  ]
  for (const config of apiConfigs.value) {
    if (config.enabled) {
      options.push({
        label: `${config.name} (${config.provider}/${config.model})`,
        value: config.id,
      })
    }
  }
  return options
})

const nodeKeys: AgentLlmNodeType[] = [
  'SceneInitializer',
  'SceneStateExtractor',
  'CharacterCognitivePass',
  'OutcomePlanner',
  'SurfaceRealizer',
]

// Methods
async function loadApiConfigs() {
  isLoading.value = true
  error.value = null
  try {
    apiConfigs.value = await listApiConfigs()
  } catch (e) {
    error.value = String(e)
  } finally {
    isLoading.value = false
  }
}

function getNodeConfigKey(nodeType: AgentLlmNodeType): keyof AgentLlmProfile {
  switch (nodeType) {
    case 'SceneInitializer': return 'scene_initializer_api_config_id'
    case 'SceneStateExtractor': return 'scene_state_extractor_api_config_id'
    case 'CharacterCognitivePass': return 'character_cognitive_pass_api_config_id'
    case 'OutcomePlanner': return 'outcome_planner_api_config_id'
    case 'SurfaceRealizer': return 'surface_realizer_api_config_id'
  }
}

function getNodeConfigId(nodeType: AgentLlmNodeType): string | null {
  const key = getNodeConfigKey(nodeType)
  return localProfile.value[key] as string | null
}

function setNodeConfigId(nodeType: AgentLlmNodeType, id: string | null) {
  const key = getNodeConfigKey(nodeType)
  ;(localProfile.value as Record<string, string | null>)[key] = id
  localProfile.value.updated_at = new Date().toISOString()
  emit('update:profile', localProfile.value)
}

function getDefaultConfigId(): string | null {
  return localProfile.value.default_api_config_id
}

function setDefaultConfigId(id: string | null) {
  localProfile.value.default_api_config_id = id
  localProfile.value.updated_at = new Date().toISOString()
  emit('update:profile', localProfile.value)
}

function handleSave() {
  emit('save')
}

// Lifecycle
onMounted(() => {
  loadApiConfigs()
})

watch(() => props.profile, (newProfile) => {
  if (newProfile) {
    localProfile.value = { ...newProfile }
  } else {
    localProfile.value = createDefaultAgentLlmProfile(props.worldId)
  }
}, { immediate: true })
</script>

<template>
  <NCard title="Agent LLM 节点配置" size="small">
    <template #header-extra>
      <NButton type="primary" size="small" @click="handleSave">
        保存配置
      </NButton>
    </template>

    <NSpin :show="isLoading">
      <NAlert v-if="error" type="error" :title="error" class="mb-4" />

      <div class="config-section">
        <div class="section-title">
          <span>默认 API 配置</span>
          <NTooltip>
            <template #trigger>
              <NIcon :size="16" class="info-icon">
                <InformationCircleOutline />
              </NIcon>
            </template>
            未单独配置的节点将使用此默认配置
          </NTooltip>
        </div>
        <NSelect
          :value="getDefaultConfigId()"
          :options="defaultApiConfigOptions as any"
          :disabled="isLoading"
          placeholder="选择默认 API 配置"
          @update:value="setDefaultConfigId"
        />
      </div>

      <NDivider style="margin: 16px 0" />

      <div class="section-title mb-3">
        <span>节点独立配置</span>
        <NTooltip>
          <template #trigger>
            <NIcon :size="16" class="info-icon">
              <InformationCircleOutline />
            </NIcon>
          </template>
          为特定节点指定不同的 API 配置，未设置时继承默认配置
        </NTooltip>
      </div>

      <div class="node-config-list">
        <div
          v-for="nodeType in nodeKeys"
          :key="nodeType"
          class="node-config-item"
        >
          <div class="node-info">
            <div class="node-label">
              {{ AGENT_LLM_NODE_INFO[nodeType].label }}
            </div>
            <div class="node-description">
              {{ AGENT_LLM_NODE_INFO[nodeType].description }}
            </div>
            <div class="node-permission">
              权限: {{ AGENT_LLM_NODE_INFO[nodeType].permission }}
            </div>
          </div>
          <div class="node-select">
            <NSelect
              :value="getNodeConfigId(nodeType)"
              :options="apiConfigOptions as any"
              :disabled="isLoading"
              placeholder="继承默认"
              size="small"
              @update:value="(id: string | null) => setNodeConfigId(nodeType, id)"
            />
          </div>
        </div>
      </div>
    </NSpin>
  </NCard>
</template>

<style scoped>
.mb-3 {
  margin-bottom: 12px;
}

.mb-4 {
  margin-bottom: 16px;
}

.config-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
  font-size: 14px;
}

.info-icon {
  color: var(--color-text-secondary, #666);
  cursor: help;
}

.node-config-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.node-config-item {
  display: grid;
  grid-template-columns: 1fr 200px;
  gap: 12px;
  padding: 12px;
  background: var(--color-bg-subtle, #f9fafb);
  border-radius: 6px;
  align-items: center;
}

.node-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.node-label {
  font-weight: 500;
  font-size: 13px;
}

.node-description {
  font-size: 12px;
  color: var(--color-text-secondary, #667085);
}

.node-permission {
  font-size: 11px;
  color: var(--color-text-secondary, #98a2b3);
  font-family: monospace;
}

.node-select {
  flex-shrink: 0;
}

@media (max-width: 600px) {
  .node-config-item {
    grid-template-columns: 1fr;
  }
}
</style>
