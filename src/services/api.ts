import { invoke } from '@tauri-apps/api/core'
import type { ApiConfig } from '@/types/st'

export interface GreetResponse {
  message: string
}

export async function greet(name: string): Promise<GreetResponse> {
  return await invoke<GreetResponse>('greet', { name })
}

// Chat request types
export interface ChatRequestMessage {
  role: 'system' | 'user' | 'assistant'
  content: string
}

export interface ChatResponse {
  request_id: string
  content: string
  reasoning?: string
  token_usage?: {
    prompt_tokens: number
    completion_tokens: number
    total_tokens: number
  }
  finish_reason?: string
}

// Send chat message (non-streaming)
export async function sendChatMessage(
  _apiConfig: ApiConfig,
  _systemPrompt: string,
  _messages: ChatRequestMessage[]
): Promise<ChatResponse> {
  throw new Error('sendChatMessage 已停用；请改为通过 ST runtime assembly 的会话发送路径调用。')
}

// Send chat message (streaming)
export async function streamChatMessage(
  apiConfig: ApiConfig,
  systemPrompt: string,
  messages: ChatRequestMessage[],
  onChunk: (chunk: string) => void,
  onComplete: () => void,
  onError?: (error: string) => void
): Promise<void> {
  // For now, use non-streaming until we implement proper streaming via events
  try {
    const response = await sendChatMessage(apiConfig, systemPrompt, messages)
    onChunk(response.content)
    onComplete()
  } catch (e) {
    if (onError) {
      onError(String(e))
    } else {
      throw e
    }
  }
}

// Send structured chat message
export async function sendStructuredChatMessage(
  _apiConfig: ApiConfig,
  _systemPrompt: string,
  _messages: ChatRequestMessage[],
  _schema: Record<string, unknown>
): Promise<Record<string, unknown>> {
  throw new Error('sendStructuredChatMessage 已停用；请改为通过 Agent PromptBuilder / structured runtime 路径发送。')
}
