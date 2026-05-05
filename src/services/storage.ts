import { invoke } from '@tauri-apps/api/core'
import type {
  ApiConfig,
  TavernCardV3,
  ChatSession,
  ChatAttachmentRecord,
  AttachmentUploadCacheDiagnostics,
  ClearAttachmentUploadCacheResult,
} from '@/types/st'
import type {
  StructuredTextBackendRequest,
  StructuredTextValidationResult,
} from '@/types/structuredText'

// ============================================================================
// Model Info
// ============================================================================

export interface ModelInfo {
  id: string
  display_name: string | null
  owned_by: string | null
  max_input_tokens: number | null
  max_output_tokens: number | null
  capabilities: Record<string, unknown> | null
}

// ============================================================================
// API Config
// ============================================================================

export async function listApiConfigs(): Promise<ApiConfig[]> {
  return await invoke<ApiConfig[]>('list_api_configs')
}

export async function getApiConfig(id: string): Promise<ApiConfig> {
  return await invoke<ApiConfig>('get_api_config', { id })
}

export async function saveApiConfig(config: ApiConfig): Promise<void> {
  return await invoke('save_api_config', { config })
}

export async function deleteApiConfig(id: string): Promise<void> {
  return await invoke('delete_api_config', { id })
}

export async function listModels(configId: string): Promise<ModelInfo[]> {
  return await invoke<ModelInfo[]>('list_models', { configId })
}

// ============================================================================
// Character
// ============================================================================

export interface CharacterImportResult {
  id: string
  character: TavernCardV3
  has_embedded_worldbook: boolean
  avatar_filename: string
}

export interface CharacterListItem {
  id: string
  character: TavernCardV3
}

export async function listCharacters(): Promise<CharacterListItem[]> {
  return await invoke<CharacterListItem[]>('list_characters')
}

export async function getCharacter(id: string): Promise<TavernCardV3> {
  return await invoke<TavernCardV3>('get_character', { id })
}

export async function saveCharacter(
  id: string,
  character: TavernCardV3
): Promise<void> {
  return await invoke('save_character', { id, character })
}

export async function deleteCharacter(id: string): Promise<void> {
  return await invoke('delete_character', { id })
}

export async function importCharacterFromPng(
  pngData: number[],
  filename: string
): Promise<CharacterImportResult> {
  return await invoke<CharacterImportResult>('import_character_from_png', {
    pngData,
    filename,
  })
}

export async function importCharacterFromJson(
  jsonData: number[],
  avatarPng: number[] | null,
  filename: string
): Promise<CharacterImportResult> {
  return await invoke<CharacterImportResult>('import_character_from_json', {
    jsonData,
    avatarPng,
    filename,
  })
}

export async function exportCharacterAsPng(id: string): Promise<number[]> {
  return await invoke<number[]>('export_character_as_png', { id })
}

export async function exportCharacterAsJson(id: string): Promise<number[]> {
  return await invoke<number[]>('export_character_as_json', { id })
}

export async function importEmbeddedWorldbook(
  characterId: string
): Promise<string> {
  return await invoke<string>('import_embedded_worldbook', { characterId })
}

export async function updateCharacterAvatar(
  id: string,
  pngData: number[]
): Promise<void> {
  return await invoke('update_character_avatar', { id, pngData })
}

export async function getCharacterAvatar(id: string): Promise<number[]> {
  return await invoke<number[]>('get_character_avatar', { id })
}

// ============================================================================
// Worldbook (WorldInfoFile - ST compatible format)
// ============================================================================

import type { WorldInfoFile, WorldInfoEntry } from '@/types/st'

export interface WorldbookListItem {
  id: string
  name: string
  description: string
  entry_count: number
}

export async function listWorldbooks(): Promise<WorldbookListItem[]> {
  return await invoke<WorldbookListItem[]>('list_worldbooks')
}

export async function getWorldbook(id: string): Promise<WorldInfoFile> {
  return await invoke<WorldInfoFile>('get_worldbook', { id })
}

export async function createWorldbook(name: string): Promise<string> {
  return await invoke<string>('create_worldbook', { name })
}

export async function saveWorldbook(
  id: string,
  worldbook: WorldInfoFile
): Promise<void> {
  return await invoke('save_worldbook', { id, worldbook })
}

export async function deleteWorldbook(id: string): Promise<void> {
  return await invoke('delete_worldbook', { id })
}

export async function updateWorldbookMeta(
  id: string,
  name: string,
  description: string
): Promise<void> {
  return await invoke('update_worldbook_meta', { id, name, description })
}

// Entry-level operations
export async function createWorldbookEntry(worldbookId: string): Promise<number> {
  return await invoke<number>('create_worldbook_entry', { worldbookId })
}

export async function updateWorldbookEntry(
  worldbookId: string,
  uid: number,
  entry: WorldInfoEntry
): Promise<void> {
  return await invoke('update_worldbook_entry', { worldbookId, uid, entry })
}

export async function deleteWorldbookEntry(
  worldbookId: string,
  uid: number
): Promise<void> {
  return await invoke('delete_worldbook_entry', { worldbookId, uid })
}

export async function reorderWorldbookEntries(
  worldbookId: string,
  uidOrder: number[]
): Promise<void> {
  return await invoke('reorder_worldbook_entries', { worldbookId, uidOrder })
}

export async function importWorldbook(
  jsonData: number[],
  filename: string
): Promise<string> {
  return await invoke<string>('import_worldbook', { jsonData, filename })
}

export async function exportWorldbook(id: string): Promise<number[]> {
  return await invoke<number[]>('export_worldbook', { id })
}

// ============================================================================
// Chat Session
// ============================================================================

export async function listChatSessions(): Promise<ChatSession[]> {
  return await invoke<ChatSession[]>('list_chat_sessions')
}

export async function getChatSession(id: string): Promise<ChatSession> {
  return await invoke<ChatSession>('get_chat_session', { id })
}

export async function saveChatSession(session: ChatSession): Promise<void> {
  return await invoke('save_chat_session', { session })
}

export async function deleteChatSession(id: string): Promise<void> {
  return await invoke('delete_chat_session', { id })
}

export async function saveChatAttachment(
  filename: string,
  mimeType: string,
  data: number[]
): Promise<ChatAttachmentRecord> {
  return await invoke<ChatAttachmentRecord>('save_chat_attachment', {
    input: {
      filename,
      mimeType,
      data,
    },
  })
}

export async function getChatAttachment(attachmentId: string): Promise<ChatAttachmentRecord> {
  return await invoke<ChatAttachmentRecord>('get_chat_attachment', { attachmentId })
}

export async function getChatAttachmentBlob(attachmentId: string): Promise<number[]> {
  return await invoke<number[]>('get_chat_attachment_blob', { attachmentId })
}

export async function listAttachmentUploadCache(
  attachmentId: string
): Promise<AttachmentUploadCacheDiagnostics> {
  return await invoke<AttachmentUploadCacheDiagnostics>('list_attachment_upload_cache', {
    attachmentId,
  })
}

export async function clearAttachmentUploadCache(
  attachmentId: string,
  apiConfigId?: string | null
): Promise<ClearAttachmentUploadCacheResult> {
  return await invoke<ClearAttachmentUploadCacheResult>('clear_attachment_upload_cache', {
    attachmentId,
    apiConfigId: apiConfigId ?? null,
  })
}

// ============================================================================
// Structured Text
// ============================================================================

export async function validateStructuredText(
  input: StructuredTextBackendRequest
): Promise<StructuredTextValidationResult> {
  return await invoke<StructuredTextValidationResult>('validate_structured_text', { input })
}

export async function formatStructuredText(
  input: StructuredTextBackendRequest
): Promise<StructuredTextValidationResult> {
  return await invoke<StructuredTextValidationResult>('format_structured_text', { input })
}

// ============================================================================
// Presets
// ============================================================================

import type { PresetFile } from '@/types/preset'

export interface PresetListItem {
  name: string
  source_api_id?: string
}

export async function listPresets(): Promise<PresetListItem[]> {
  return await invoke<PresetListItem[]>('list_presets')
}

export async function loadPreset(name: string): Promise<PresetFile> {
  return await invoke<PresetFile>('load_preset', { name })
}

export async function savePreset(preset: PresetFile): Promise<void> {
  return await invoke('save_preset', { preset })
}

export async function deletePreset(name: string): Promise<void> {
  return await invoke('delete_preset', { name })
}
