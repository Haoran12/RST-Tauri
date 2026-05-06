/**
 * Presets store
 *
 * 预设管理 store，每个预设文件包含六类配置。
 */

import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { PresetFile } from '@/types/preset'
import { createDefaultPresetFile } from '@/types/preset'
import * as storage from '@/services/storage'

export interface PresetListItem {
  name: string
  source_api_id?: string
}

export type PresetSectionKey = 'sampler' | 'instruct' | 'context' | 'sysprompt' | 'reasoning' | 'prompt'

export const usePresetsStore = defineStore('presets', () => {
  // State
  const presetList = ref<PresetListItem[]>([])
  const currentPreset = ref<PresetFile | null>(null)
  const currentSection = ref<PresetSectionKey>('sampler')
  const currentPromptIdentifier = ref<string | null>(null)
  const isLoading = ref(false)

  function firstPromptIdentifier(preset: PresetFile | null) {
    return preset?.prompts?.[0]?.identifier ?? null
  }

  function ensureCurrentPromptSelection() {
    const prompts = currentPreset.value?.prompts ?? []
    if (prompts.length === 0) {
      currentPromptIdentifier.value = null
      return
    }

    if (
      !currentPromptIdentifier.value ||
      !prompts.some((item) => item.identifier === currentPromptIdentifier.value)
    ) {
      currentPromptIdentifier.value = prompts[0].identifier
    }
  }

  // Actions
  async function loadPresetList() {
    isLoading.value = true
    try {
      const list = await storage.listPresets()
      presetList.value = list
    } catch (e) {
      console.error('Failed to load presets:', e)
      presetList.value = []
    } finally {
      isLoading.value = false
    }
  }

  async function loadPreset(name: string) {
    isLoading.value = true
    try {
      const preset = await storage.loadPreset(name)
      currentPreset.value = preset
      ensureCurrentPromptSelection()
    } catch (e) {
      console.error(`Failed to load preset "${name}":`, e)
      currentPreset.value = null
      currentPromptIdentifier.value = null
    } finally {
      isLoading.value = false
    }
  }

  async function savePreset(preset: PresetFile) {
    if (!preset.name) return
    await storage.savePreset(preset)
    await loadPresetList()
    if (currentPreset.value?.name === preset.name) {
      await loadPreset(preset.name)
    }
  }

  async function deletePreset(name: string) {
    await storage.deletePreset(name)
    await loadPresetList()
    if (currentPreset.value?.name === name) {
      currentPreset.value = null
      currentPromptIdentifier.value = null
    }
  }

  function createNewPreset(name: string) {
    currentPreset.value = createDefaultPresetFile(name)
    currentPromptIdentifier.value = firstPromptIdentifier(currentPreset.value)
  }

  async function renamePreset(oldName: string, newName: string) {
    const preset = await storage.loadPreset(oldName)
    if (preset) {
      preset.name = newName
      await storage.savePreset(preset)
      await storage.deletePreset(oldName)
      await loadPresetList()
      if (currentPreset.value?.name === oldName) {
        currentPreset.value = preset
        ensureCurrentPromptSelection()
      }
    }
  }

  async function exportPreset(name: string): Promise<Blob> {
    const preset = await storage.loadPreset(name)
    if (!preset) {
      throw new Error(`Preset "${name}" not found`)
    }
    const json = JSON.stringify(preset, null, 2)
    return new Blob([json], { type: 'application/json' })
  }

  async function importPreset(file: File): Promise<void> {
    const text = await file.text()
    const data = JSON.parse(text)

    // 检测并转换格式
    const preset = convertToPresetFile(data)

    if (!preset.name) {
      throw new Error('Invalid preset file: missing name')
    }

    await storage.savePreset(preset)
    await loadPresetList()
    currentPreset.value = preset
    ensureCurrentPromptSelection()
  }

  /**
   * 将导入的数据转换为 PresetFile 格式
   *
   * 支持以下格式：
   * 1. RST/ST 新格式（PresetFile）- 扁平结构，采样参数和 prompts 在顶层
   * 2. ST Master 格式 - 包含 instruct/context/sysprompt/reasoning 等 section
   * 3. ST 单类型格式 - 单独的 sampler/instruct/context 等文件
   * 4. Text Completion 预设 - temp/top_k/top_p/rep_pen 等字段
   */
  function convertToPresetFile(data: Record<string, unknown>): PresetFile {
    // 1. RST/ST 新格式：已有 name 且包含 prompts 数组或采样参数
    if (isPresetFileFormat(data)) {
      return data as unknown as PresetFile
    }

    // 2. ST Master 格式：包含多个 section
    if (isSTMasterFormat(data)) {
      return convertFromSTMaster(data)
    }

    // 3. ST 单类型格式检测
    // 3.1 Instruct Template
    if (isPossiblyInstructData(data)) {
      return {
        name: (data.name as string) || 'Imported Instruct',
        instruct: extractInstructTemplate(data),
      }
    }

    // 3.2 Context Template
    if (isPossiblyContextData(data)) {
      return {
        name: (data.name as string) || 'Imported Context',
        context: extractContextTemplate(data),
      }
    }

    // 3.3 System Prompt
    if (isPossiblySystemPromptData(data)) {
      return {
        name: (data.name as string) || 'Imported System Prompt',
        sysprompt: extractSystemPrompt(data),
      }
    }

    // 3.4 Reasoning Template
    if (isPossiblyReasoningData(data)) {
      return {
        name: (data.name as string) || 'Imported Reasoning',
        reasoning: extractReasoningTemplate(data),
      }
    }

    // 4. Text Completion / Sampler 预设
    if (isPossiblySamplerData(data)) {
      const result: PresetFile = {
        name: (data.name as string) || 'Imported Sampler',
      }
      // 直接提取采样参数到顶层
      extractSamplerParams(data, result)
      return result
    }

    // 5. 无法识别，尝试作为新格式处理
    if (data.name && typeof data.name === 'string') {
      console.warn('Unrecognized preset format, treating as PresetFile')
      return data as unknown as PresetFile
    }

    throw new Error('Unable to recognize preset format')
  }

  // ============================================================================
  // 格式检测函数
  // ============================================================================

  function isPresetFileFormat(data: Record<string, unknown>): boolean {
    // RST/ST 新格式：有 name 且包含 prompts 数组或采样参数
    return (
      !!data.name &&
      typeof data.name === 'string' &&
      !!(Array.isArray(data.prompts) || hasSamplerParams(data))
    )
  }

  function hasSamplerParams(data: Record<string, unknown>): boolean {
    // 检测是否有采样参数
    const samplerKeys = [
      'temperature', 'top_p', 'top_k', 'top_a', 'min_p', 'typical_p', 'tfs',
      'repetition_penalty', 'frequency_penalty', 'presence_penalty',
      'mirostat_mode', 'mirostat_tau', 'mirostat_eta'
    ]
    return samplerKeys.some(key => key in data && typeof data[key] === 'number')
  }

  function isSTMasterFormat(data: Record<string, unknown>): boolean {
    // ST Master 格式包含 instruct/context/sysprompt/reasoning/preset 等 section
    const masterKeys = ['instruct', 'context', 'sysprompt', 'reasoning', 'preset']
    return masterKeys.some((key) => data[key] && typeof data[key] === 'object')
  }

  function isPossiblyInstructData(data: Record<string, unknown>): boolean {
    // ST Instruct Template 必须有 name, input_sequence, output_sequence
    return (
      !!data.name &&
      typeof data.name === 'string' &&
      ('input_sequence' in data || 'output_sequence' in data || 'system_sequence' in data)
    )
  }

  function isPossiblyContextData(data: Record<string, unknown>): boolean {
    // ST Context Template 必须有 name, story_string
    return (
      !!data.name &&
      typeof data.name === 'string' &&
      'story_string' in data
    )
  }

  function isPossiblySystemPromptData(data: Record<string, unknown>): boolean {
    // ST System Prompt 必须有 name, content
    return (
      !!data.name &&
      typeof data.name === 'string' &&
      'content' in data &&
      !('prompts' in data) // 排除 PromptPreset
    )
  }

  function isPossiblyReasoningData(data: Record<string, unknown>): boolean {
    // ST Reasoning Template 必须有 name, prefix, suffix, separator
    return (
      !!data.name &&
      typeof data.name === 'string' &&
      'prefix' in data &&
      'suffix' in data
    )
  }

  function isPossiblySamplerData(data: Record<string, unknown>): boolean {
    // Text Completion 预设通常有 temp, top_k, top_p, rep_pen
    // 或者 ST sampler 的 temperature, top_p 等
    return (
      'temp' in data ||
      'temperature' in data ||
      'top_k' in data ||
      'top_p' in data ||
      'rep_pen' in data ||
      'repetition_penalty' in data
    )
  }

  function detectSourceApiId(data: Record<string, unknown>): string | undefined {
    // 从数据中检测来源 API（用于迁移追踪）
    if (data.source_api_id) return data.source_api_id as string
    return undefined
  }

  // ============================================================================
  // 格式转换函数
  // ============================================================================

  /**
   * 提取采样参数到 PresetFile 顶层
   */
  function extractSamplerParams(data: Record<string, unknown>, result: PresetFile) {
    // 基础采样参数
    if (data.temperature !== undefined) result.temperature = data.temperature as number
    if (data.temp !== undefined) result.temperature = data.temp as number
    if (data.top_p !== undefined) result.top_p = data.top_p as number
    if (data.top_k !== undefined) result.top_k = data.top_k as number
    if (data.top_a !== undefined) result.top_a = data.top_a as number
    if (data.min_p !== undefined) result.min_p = data.min_p as number
    if (data.typical_p !== undefined) result.typical_p = data.typical_p as number
    if (data.tfs !== undefined) result.tfs = data.tfs as number
    if (data.epsilon_cutoff !== undefined) result.epsilon_cutoff = data.epsilon_cutoff as number
    if (data.eta_cutoff !== undefined) result.eta_cutoff = data.eta_cutoff as number

    // 重复惩罚
    if (data.repetition_penalty !== undefined) result.repetition_penalty = data.repetition_penalty as number
    if (data.rep_pen !== undefined) result.repetition_penalty = data.rep_pen as number
    if (data.rep_pen_range !== undefined) result.rep_pen_range = data.rep_pen_range as number
    if (data.rep_pen_decay !== undefined) result.rep_pen_decay = data.rep_pen_decay as number
    if (data.rep_pen_slope !== undefined) result.rep_pen_slope = data.rep_pen_slope as number
    if (data.frequency_penalty !== undefined) result.frequency_penalty = data.frequency_penalty as number
    if (data.presence_penalty !== undefined) result.presence_penalty = data.presence_penalty as number
    if (data.encoder_rep_pen !== undefined) result.encoder_rep_pen = data.encoder_rep_pen as number

    // DRY
    if (data.dry_allowed_length !== undefined) result.dry_allowed_length = data.dry_allowed_length as number
    if (data.dry_multiplier !== undefined) result.dry_multiplier = data.dry_multiplier as number
    if (data.dry_base !== undefined) result.dry_base = data.dry_base as number
    if (data.dry_sequence_breakers !== undefined) result.dry_sequence_breakers = data.dry_sequence_breakers as string

    // Mirostat
    if (data.mirostat_mode !== undefined) result.mirostat_mode = data.mirostat_mode as number
    if (data.mirostat_tau !== undefined) result.mirostat_tau = data.mirostat_tau as number
    if (data.mirostat_eta !== undefined) result.mirostat_eta = data.mirostat_eta as number

    // 其他
    if (data.no_repeat_ngram_size !== undefined) result.no_repeat_ngram_size = data.no_repeat_ngram_size as number
    if (data.guidance_scale !== undefined) result.guidance_scale = data.guidance_scale as number
    if (data.negative_prompt !== undefined) result.negative_prompt = data.negative_prompt as string
    if (data.sampler_priority !== undefined) result.sampler_priority = data.sampler_priority as string[]
    if (data.temperature_last !== undefined) result.temperature_last = data.temperature_last as boolean
  }

  function convertFromSTMaster(data: Record<string, unknown>): PresetFile {
    const result: PresetFile = {
      name: 'Imported Master',
    }

    // 提取 instruct section
    if (data.instruct && typeof data.instruct === 'object') {
      const instructData = data.instruct as Record<string, unknown>
      result.name = (instructData.name as string) || result.name
      result.instruct = extractInstructTemplate(instructData)
    }

    // 提取 context section
    if (data.context && typeof data.context === 'object') {
      const contextData = data.context as Record<string, unknown>
      result.name = (contextData.name as string) || result.name
      result.context = extractContextTemplate(contextData)
    }

    // 提取 sysprompt section
    if (data.sysprompt && typeof data.sysprompt === 'object') {
      const syspromptData = data.sysprompt as Record<string, unknown>
      result.name = (syspromptData.name as string) || result.name
      result.sysprompt = extractSystemPrompt(syspromptData)
    }

    // 提取 reasoning section
    if (data.reasoning && typeof data.reasoning === 'object') {
      const reasoningData = data.reasoning as Record<string, unknown>
      result.name = (reasoningData.name as string) || result.name
      result.reasoning = extractReasoningTemplate(reasoningData)
    }

    // 提取 preset section (Text Completion) - 采样参数直接到顶层
    if (data.preset && typeof data.preset === 'object') {
      const presetData = data.preset as Record<string, unknown>
      extractSamplerParams(presetData, result)
      result.source_api_id = detectSourceApiId(presetData)
    }

    return result
  }

  function extractInstructTemplate(data: Record<string, unknown>): import('@/types/preset').InstructTemplate {
    return {
      input_sequence: data.input_sequence as string | undefined,
      output_sequence: data.output_sequence as string | undefined,
      system_sequence: data.system_sequence as string | undefined,
      stop_sequence: data.stop_sequence as string | undefined,
      input_suffix: data.input_suffix as string | undefined,
      output_suffix: data.output_suffix as string | undefined,
      system_suffix: data.system_suffix as string | undefined,
      first_input_sequence: data.first_input_sequence as string | undefined,
      last_input_sequence: data.last_input_sequence as string | undefined,
      first_output_sequence: data.first_output_sequence as string | undefined,
      last_output_sequence: data.last_output_sequence as string | undefined,
      story_string_prefix: data.story_string_prefix as string | undefined,
      story_string_suffix: data.story_string_suffix as string | undefined,
      wrap: data.wrap as boolean | undefined,
      macro: data.macro as boolean | undefined,
      names_behavior: data.names_behavior as 'none' | 'force' | 'always' | undefined,
      system_same_as_user: data.system_same_as_user as boolean | undefined,
      skip_examples: data.skip_examples as boolean | undefined,
      sequences_as_stop_strings: data.sequences_as_stop_strings as boolean | undefined,
      activation_regex: data.activation_regex as string | undefined,
    }
  }

  function extractContextTemplate(data: Record<string, unknown>): import('@/types/preset').ContextTemplate {
    return {
      story_string: data.story_string as string | undefined,
      example_separator: data.example_separator as string | undefined,
      chat_start: data.chat_start as string | undefined,
      use_stop_strings: data.use_stop_strings as boolean | undefined,
      names_as_stop_strings: data.names_as_stop_strings as boolean | undefined,
      story_string_position: data.story_string_position as number | undefined,
      story_string_depth: data.story_string_depth as number | undefined,
      story_string_role: data.story_string_role as number | undefined,
      always_force_name2: data.always_force_name2 as boolean | undefined,
      trim_sentences: data.trim_sentences as boolean | undefined,
      single_line: data.single_line as boolean | undefined,
    }
  }

  function extractSystemPrompt(data: Record<string, unknown>): import('@/types/preset').SystemPrompt {
    return {
      content: data.content as string | undefined,
    }
  }

  function extractReasoningTemplate(data: Record<string, unknown>): import('@/types/preset').ReasoningTemplate {
    return {
      prefix: data.prefix as string | undefined,
      suffix: data.suffix as string | undefined,
      separator: data.separator as string | undefined,
    }
  }

  function clearCurrentPreset() {
    currentPreset.value = null
    currentPromptIdentifier.value = null
  }

  function selectSection(section: PresetSectionKey) {
    currentSection.value = section
    if (section === 'prompt') {
      ensureCurrentPromptSelection()
    }
  }

  function selectPromptItem(identifier: string | null) {
    currentPromptIdentifier.value = identifier
    if (identifier) {
      currentSection.value = 'prompt'
    }
  }

  return {
    presetList,
    currentPreset,
    currentSection,
    currentPromptIdentifier,
    isLoading,
    loadPresetList,
    loadPreset,
    savePreset,
    deletePreset,
    createNewPreset,
    renamePreset,
    exportPreset,
    importPreset,
    clearCurrentPreset,
    selectSection,
    selectPromptItem,
  }
})
