// ST Runtime Assembly Service
// 运行时组装服务：调用 Tauri 命令

import { invoke } from '@tauri-apps/api/core';
import type {
  GlobalAppState,
  STWorldInfoSettings,
  AssembleRequestInput,
  AssembleRequestOutput,
  WorldInfoInjectionInput,
  WorldInfoInjectionResult,
  AssembledRequest,
} from '@/types/runtime';
import type { ChatResponse } from '@/services/api';
import type {
  SamplerPreset,
  InstructTemplate,
  ContextTemplate,
  SystemPrompt,
  ReasoningTemplate,
  PromptPreset,
} from '@/types/preset';

// ============================================================================
// 全局应用状态
// ============================================================================

/**
 * 获取全局应用状态
 */
export async function getGlobalState(): Promise<GlobalAppState> {
  return await invoke<GlobalAppState>('get_global_state');
}

/**
 * 保存全局应用状态
 */
export async function saveGlobalState(state: GlobalAppState): Promise<void> {
  return await invoke('save_global_state', { state });
}

/**
 * 设置激活的 API 配置
 */
export async function setActiveApiConfig(apiConfigId: string | null): Promise<void> {
  return await invoke('set_active_api_config', { apiConfigId });
}

/**
 * 设置激活的完整预设
 */
export async function setActivePreset(presetName: string): Promise<void> {
  return await invoke('set_active_preset', { presetName });
}

// ============================================================================
// 预设加载
// ============================================================================

/**
 * 加载 Sampler 预设
 */
export async function loadSamplerPreset(name: string): Promise<SamplerPreset> {
  return await invoke<SamplerPreset>('load_sampler_preset', { name });
}

/**
 * 加载 Instruct 模板
 */
export async function loadInstructTemplate(name: string): Promise<InstructTemplate> {
  return await invoke<InstructTemplate>('load_instruct_template', { name });
}

/**
 * 加载 Context 模板
 */
export async function loadContextTemplate(name: string): Promise<ContextTemplate> {
  return await invoke<ContextTemplate>('load_context_template', { name });
}

/**
 * 加载 System Prompt
 */
export async function loadSystemPrompt(name: string): Promise<SystemPrompt> {
  return await invoke<SystemPrompt>('load_system_prompt', { name });
}

/**
 * 加载 Reasoning 模板
 */
export async function loadReasoningTemplate(name: string): Promise<ReasoningTemplate> {
  return await invoke<ReasoningTemplate>('load_reasoning_template', { name });
}

/**
 * 加载 Prompt 预设
 */
export async function loadPromptPreset(name: string): Promise<PromptPreset> {
  return await invoke<PromptPreset>('load_prompt_preset', { name });
}

// ============================================================================
// 运行时组装
// ============================================================================

/**
 * 组装 ST 聊天请求
 */
export async function assembleSTRequest(input: AssembleRequestInput): Promise<AssembleRequestOutput> {
  return await invoke<AssembleRequestOutput>('assemble_st_request', { input });
}

/**
 * 经过 ST runtime assembly gate 后发送聊天请求
 */
export async function sendAssembledSTChatMessage(input: AssembleRequestInput): Promise<ChatResponse> {
  return await invoke<ChatResponse>('send_assembled_st_chat_message', { input });
}

/**
 * 执行世界书注入
 */
export async function runWorldInfoInjection(input: WorldInfoInjectionInput): Promise<WorldInfoInjectionResult> {
  return await invoke<WorldInfoInjectionResult>('run_world_info_injection', { input });
}

/**
 * 映射请求到 Provider 格式
 */
export async function mapRequestToProvider(
  request: AssembledRequest,
  apiConfigId: string,
  providerType: string,
  model: string
): Promise<Record<string, unknown>> {
  return await invoke<Record<string, unknown>>('map_request_to_provider', {
    request,
    apiConfigId,
    providerType,
    model,
  });
}

// ============================================================================
// 辅助函数
// ============================================================================

/**
 * 构建完整的聊天请求流程
 *
 * 包含：加载配置 → 组装请求 → 世界书注入 → Provider 映射
 */
export async function buildCompleteChatRequest(
  apiConfigId: string,
  sessionId: string,
  characterId: string | null,
  worldInfoSettings: STWorldInfoSettings,
  options?: {
    presetName?: string;
    chatLoreId?: string;
    globalLoreIds?: string[];
    maxContext?: number;
  }
): Promise<{
  assembledRequest: AssembledRequest;
  providerRequest: Record<string, unknown>;
  worldInfoResult: WorldInfoInjectionResult | null;
  providerType: string;
  model: string;
}> {
  // 1. 组装基础请求
  const assembleInput: AssembleRequestInput = {
    api_config_id: apiConfigId,
    character_id: characterId,
    session_id: sessionId,
    preset_name: options?.presetName ?? null,
    world_info_settings: worldInfoSettings,
    chat_lore_id: options?.chatLoreId ?? null,
    global_lore_ids: options?.globalLoreIds ?? [],
    max_context: options?.maxContext ?? 8192,
  };

  const assembleOutput = await assembleSTRequest(assembleInput);

  // 2. 映射到 Provider 格式
  const providerRequest = await mapRequestToProvider(
    assembleOutput.request,
    apiConfigId,
    assembleOutput.provider_type,
    assembleOutput.model
  );

  return {
    assembledRequest: assembleOutput.request,
    providerRequest,
    worldInfoResult: assembleOutput.world_info_result,
    providerType: assembleOutput.provider_type,
    model: assembleOutput.model,
  };
}
