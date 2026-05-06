// ST Runtime Assembly Service
// 运行时组装服务：调用 Tauri 命令

import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
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
// 流式传输事件类型
// ============================================================================

export interface StreamStartEvent {
  stream_id: string;
  request_id: string;
}

export interface StreamChunkEvent {
  stream_id: string;
  delta: string;
  finish_reason: string | null;
}

export interface StreamErrorEvent {
  stream_id: string;
  error: string;
}

export interface StreamCallbacks {
  onStart?: (event: StreamStartEvent) => void;
  onChunk?: (event: StreamChunkEvent) => void;
  onError?: (event: StreamErrorEvent) => void;
  onEnd?: (streamId: string) => void;
}

export interface StreamController {
  streamId: string;
  abort: () => void;
}

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
 * 启动流式聊天请求
 *
 * 返回流控制器，通过 callbacks 接收流事件：
 * - onStart: 流开始，包含 stream_id 和 request_id
 * - onChunk: 收到内容块，包含 delta 增量文本
 * - onError: 发生错误
 * - onEnd: 流结束
 */
export async function startSTChatStream(
  input: AssembleRequestInput,
  callbacks: StreamCallbacks
): Promise<StreamController> {
  // 注册事件监听器
  const unlisteners: UnlistenFn[] = [];
  let currentStreamId: string | null = null;
  let aborted = false;

  // 监听开始事件
  unlisteners.push(
    await listen<StreamStartEvent>('st-stream-start', (event) => {
      if (aborted) return;
      currentStreamId = event.payload.stream_id;
      callbacks.onStart?.(event.payload);
    })
  );

  // 监听内容块事件
  unlisteners.push(
    await listen<StreamChunkEvent>('st-stream-chunk', (event) => {
      if (aborted || event.payload.stream_id !== currentStreamId) return;
      callbacks.onChunk?.(event.payload);
    })
  );

  // 监听错误事件
  unlisteners.push(
    await listen<StreamErrorEvent>('st-stream-error', (event) => {
      if (aborted || event.payload.stream_id !== currentStreamId) return;
      callbacks.onError?.(event.payload);
    })
  );

  // 监听结束事件
  unlisteners.push(
    await listen<string>('st-stream-end', (streamId) => {
      if (aborted || streamId.payload !== currentStreamId) return;
      // 清理所有监听器
      unlisteners.forEach((unlisten) => unlisten());
      callbacks.onEnd?.(streamId.payload);
    })
  );

  // 启动流
  const streamId = await invoke<string>('start_st_chat_stream', { input });

  return {
    streamId,
    abort: () => {
      aborted = true;
      unlisteners.forEach((unlisten) => unlisten());
      callbacks.onEnd?.(streamId);
    },
  };
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
