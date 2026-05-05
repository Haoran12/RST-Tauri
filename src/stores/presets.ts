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
  const isLoading = ref(false)

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
    } catch (e) {
      console.error(`Failed to load preset "${name}":`, e)
      currentPreset.value = null
    } finally {
      isLoading.value = false
    }
  }

  async function savePreset(preset: PresetFile) {
    if (!preset.name) return
    await storage.savePreset(preset)
    await loadPresetList()
  }

  async function deletePreset(name: string) {
    await storage.deletePreset(name)
    await loadPresetList()
    if (currentPreset.value?.name === name) {
      currentPreset.value = null
    }
  }

  function createNewPreset(name: string) {
    currentPreset.value = createDefaultPresetFile(name)
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
  }

  /**
   * 将导入的数据转换为 PresetFile 格式
   *
   * 支持以下格式：
   * 1. RST 新格式（PresetFile）- 包含 sampler/instruct/context 等字段
   * 2. ST Master 格式 - 包含 instruct/context/sysprompt/reasoning 等 section
   * 3. ST 单类型格式 - 单独的 sampler/instruct/context 等文件
   * 4. Text Completion 预设 - temp/top_k/top_p/rep_pen 等字段
   */
  function convertToPresetFile(data: Record<string, unknown>): PresetFile {
    // 1. RST 新格式：已有 name 且包含预设类型字段
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

    // 3.5 Text Completion / Sampler 预设
    if (isPossiblySamplerData(data)) {
      return {
        name: (data.name as string) || 'Imported Sampler',
        sampler: extractSamplerPreset(data),
        source_api_id: detectSourceApiId(data),
      }
    }

    // 4. 无法识别，尝试作为新格式处理
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
    return (
      !!data.name &&
      typeof data.name === 'string' &&
      !!(
        data.sampler ||
        data.instruct ||
        data.context ||
        data.sysprompt ||
        data.reasoning ||
        data.prompt
      )
    )
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
    // ST 可能有其他标识字段
    return undefined
  }

  // ============================================================================
  // 格式转换函数
  // ============================================================================

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

    // 提取 preset section (Text Completion)
    if (data.preset && typeof data.preset === 'object') {
      const presetData = data.preset as Record<string, unknown>
      result.sampler = extractSamplerPreset(presetData)
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

  function extractSamplerPreset(data: Record<string, unknown>): import('@/types/preset').SamplerPreset {
    // 处理 ST Text Completion 预设的字段名映射
    return {
      // ST Text Completion 使用 temp，RST 使用 temperature
      temperature: (data.temperature ?? data.temp) as number | undefined,
      top_p: (data.top_p ?? data.top_p) as number | undefined,
      top_k: data.top_k as number | undefined,
      top_a: data.top_a as number | undefined,
      min_p: data.min_p as number | undefined,
      typical_p: data.typical_p as number | undefined,
      tfs: data.tfs as number | undefined,
      epsilon_cutoff: data.epsilon_cutoff as number | undefined,
      eta_cutoff: data.eta_cutoff as number | undefined,
      // ST Text Completion 使用 rep_pen，RST 使用 repetition_penalty
      repetition_penalty: (data.repetition_penalty ?? data.rep_pen) as number | undefined,
      rep_pen_range: data.rep_pen_range as number | undefined,
      rep_pen_decay: data.rep_pen_decay as number | undefined,
      rep_pen_slope: data.rep_pen_slope as number | undefined,
      frequency_penalty: data.frequency_penalty as number | undefined,
      presence_penalty: data.presence_penalty as number | undefined,
      encoder_rep_pen: data.encoder_rep_pen as number | undefined,
      dry_allowed_length: data.dry_allowed_length as number | undefined,
      dry_multiplier: data.dry_multiplier as number | undefined,
      dry_base: data.dry_base as number | undefined,
      dry_sequence_breakers: data.dry_sequence_breakers as string | undefined,
      mirostat_mode: data.mirostat_mode as number | undefined,
      mirostat_tau: data.mirostat_tau as number | undefined,
      mirostat_eta: data.mirostat_eta as number | undefined,
      no_repeat_ngram_size: data.no_repeat_ngram_size as number | undefined,
      guidance_scale: data.guidance_scale as number | undefined,
      negative_prompt: data.negative_prompt as string | undefined,
      sampler_priority: data.sampler_priority as string[] | undefined,
      temperature_last: data.temperature_last as boolean | undefined,
      provider_overrides: data.provider_overrides as Record<string, Record<string, unknown>> | undefined,
    }
  }

  function clearCurrentPreset() {
    currentPreset.value = null
  }

  function selectSection(section: PresetSectionKey) {
    currentSection.value = section
  }

  return {
    presetList,
    currentPreset,
    currentSection,
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
  }
})
