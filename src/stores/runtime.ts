// ST Runtime Store
// 运行时状态管理

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type {
  GlobalAppState,
  STWorldInfoSettings,
  WorldInfoInjectionResult,
} from '@/types/runtime';
import {
  getGlobalState,
  saveGlobalState,
  setActiveApiConfig,
  setActivePreset,
  buildCompleteChatRequest,
} from '@/services/runtime';
import { createDefaultGlobalAppState } from '@/types/runtime';
import type { ApiConfig } from '@/types/st';

export const useRuntimeStore = defineStore('runtime', () => {
  // State
  const globalState = ref<GlobalAppState>(createDefaultGlobalAppState());
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  // World info injection result (per-request)
  const lastWorldInfoResult = ref<WorldInfoInjectionResult | null>(null);

  // Computed
  const activeApiConfigId = computed(() => globalState.value.active_api_config_id);
  const activePresetName = computed(() => globalState.value.active_preset);

  const worldInfoSettings = computed(() => globalState.value.world_info_settings);

  const hasActiveApiConfig = computed(() => globalState.value.active_api_config_id !== null);

  // Actions

  /**
   * 加载全局状态
   */
  async function loadGlobalState() {
    isLoading.value = true;
    error.value = null;

    try {
      const state = await getGlobalState();
      globalState.value = state;
    } catch (e) {
      error.value = String(e);
      // 使用默认值
      globalState.value = createDefaultGlobalAppState();
    } finally {
      isLoading.value = false;
    }
  }

  /**
   * 保存全局状态
   */
  async function persistGlobalState() {
    try {
      await saveGlobalState(globalState.value);
    } catch (e) {
      error.value = String(e);
    }
  }

  /**
   * 设置激活的 API 配置
   */
  async function setApiConfigId(id: string | null) {
    globalState.value.active_api_config_id = id;
    await setActiveApiConfig(id);
  }

  /**
   * 设置激活的完整预设
   */
  async function setPresetName(presetName: string) {
    const name = presetName.trim();
    if (!name) {
      return;
    }
    globalState.value.active_preset = name;
    await setActivePreset(name);
  }

  /**
   * 更新世界书设置
   */
  async function updateWorldInfoSettings(settings: Partial<STWorldInfoSettings>) {
    globalState.value.world_info_settings = {
      ...globalState.value.world_info_settings,
      ...settings,
    };
    await persistGlobalState();
  }

  /**
   * 设置全局选中的世界书
   */
  async function setGlobalLoreSelection(loreIds: string[]) {
    globalState.value.world_info_settings.global_select = loreIds;
    await persistGlobalState();
  }

  /**
   * 构建完整的聊天请求
   */
  async function buildChatRequest(
    apiConfig: ApiConfig,
    sessionId: string,
    characterId: string | null,
    options?: {
      chatLoreId?: string;
      globalLoreIds?: string[];
      maxContext?: number;
    }
  ) {
    isLoading.value = true;
    error.value = null;

    try {
      const result = await buildCompleteChatRequest(
        apiConfig.id,
        sessionId,
        characterId,
        globalState.value.world_info_settings,
        {
          presetName: globalState.value.active_preset || undefined,
          ...options,
        }
      );

      lastWorldInfoResult.value = result.worldInfoResult;
      return result;
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      isLoading.value = false;
    }
  }

  /**
   * 重置为默认状态
   */
  async function resetToDefaults() {
    globalState.value = createDefaultGlobalAppState();
    await persistGlobalState();
  }

  return {
    // State
    globalState,
    isLoading,
    error,
    lastWorldInfoResult,

    // Computed
    activeApiConfigId,
    activePresetName,
    worldInfoSettings,
    hasActiveApiConfig,

    // Actions
    loadGlobalState,
    persistGlobalState,
    setApiConfigId,
    setPresetName,
    updateWorldInfoSettings,
    setGlobalLoreSelection,
    buildChatRequest,
    resetToDefaults,
  };
});
