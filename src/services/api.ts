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
  apiConfig: ApiConfig,
  systemPrompt: string,
  messages: ChatRequestMessage[]
): Promise<ChatResponse> {
  return await invoke<ChatResponse>('send_chat_message', {
    apiConfigId: apiConfig.id,
    systemPrompt,
    messages,
  })
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
  apiConfig: ApiConfig,
  systemPrompt: string,
  messages: ChatRequestMessage[],
  schema: Record<string, unknown>
): Promise<Record<string, unknown>> {
  return await invoke<Record<string, unknown>>('send_structured_chat_message', {
    apiConfigId: apiConfig.id,
    systemPrompt,
    messages,
    schema,
  })
}