/**
 * ST Regex extension system
 *
 * Regex 扩展兼容系统，支持 global/preset/scoped 三类脚本。
 * 参考: SillyTavern/public/scripts/extensions/regex/engine.js
 */

/**
 * Regex 脚本作用点
 */
export const RegexPlacement = {
  USER_INPUT: 1,
  AI_OUTPUT: 2,
  SLASH_COMMAND: 3,
  WORLD_INFO: 5,
  REASONING: 6,
} as const

export type RegexPlacementValue = (typeof RegexPlacement)[keyof typeof RegexPlacement]

/**
 * 宏替换策略
 */
export const SubstituteRegex = {
  NONE: 0,
  RAW: 1,
  ESCAPED: 2,
} as const

export type SubstituteRegexValue = (typeof SubstituteRegex)[keyof typeof SubstituteRegex]

/**
 * Regex 脚本数据
 */
export interface RegexScriptData {
  id: string
  script_name: string
  find_regex: string
  replace_string: string
  trim_strings?: string[]
  placement?: RegexPlacementValue[]
  disabled?: boolean
  markdown_only?: boolean
  prompt_only?: boolean
  run_on_edit?: boolean
  substitute_regex?: SubstituteRegexValue
  min_depth?: number | null
  max_depth?: number | null
}

/**
 * 创建新的 Regex 脚本
 */
export function createRegexScript(scriptName: string): RegexScriptData {
  return {
    id: crypto.randomUUID(),
    script_name: scriptName,
    find_regex: '',
    replace_string: '',
    trim_strings: [],
    placement: [],
    disabled: false,
    markdown_only: false,
    prompt_only: false,
    run_on_edit: true,
    substitute_regex: SubstituteRegex.NONE,
    min_depth: null,
    max_depth: null,
  }
}

/**
 * Regex Preset 条目
 */
export interface RegexPresetItem {
  id: string
}

/**
 * Regex Preset（启用脚本 ID 列表）
 */
export interface RegexPreset {
  id: string
  name: string
  is_selected?: boolean
  global?: RegexPresetItem[]
  scoped?: RegexPresetItem[]
  preset?: RegexPresetItem[]
}

/**
 * 创建新的 Regex Preset
 */
export function createRegexPreset(name: string): RegexPreset {
  return {
    id: crypto.randomUUID(),
    name,
    is_selected: false,
    global: [],
    scoped: [],
    preset: [],
  }
}

/**
 * Regex 扩展全局设置
 */
export interface RegexExtensionSettings {
  regex: RegexScriptData[]
  regex_presets: RegexPreset[]
  character_allowed_regex: string[]
  preset_allowed_regex: Record<string, string[]>
}

/**
 * 创建默认的 Regex 扩展设置
 */
export function createDefaultRegexExtensionSettings(): RegexExtensionSettings {
  return {
    regex: [],
    regex_presets: [],
    character_allowed_regex: [],
    preset_allowed_regex: {},
  }
}

/**
 * 运行时选项
 */
export interface RegexRunOptions {
  isMarkdown?: boolean
  isPrompt?: boolean
  isEdit?: boolean
  depth?: number | null
  characterOverride?: string
  /** 当前预设 key（用于 preset 脚本授权） */
  presetKey?: string
  /** 当前角色名（用于 scoped 脚本授权） */
  characterName?: string
}

/**
 * 脚本来源类型
 */
export enum ScriptSource {
  Global,
  Preset,
  Scoped,
}

/**
 * 带来源标记的脚本
 */
export interface SourcedScript {
  script: RegexScriptData
  source: ScriptSource
}

/**
 * Regex 执行引擎
 */
export class RegexEngine {
  private regexCache: Map<string, RegExp> = new Map()
  private cacheMaxSize = 1000

  /**
   * 对文本执行 Regex 替换
   *
   * `getRegexedString` 的等价实现。
   * 按照 global -> preset -> scoped 的顺序执行脚本。
   */
  getRegexedString(
    raw: string,
    placement: RegexPlacementValue,
    settings: RegexExtensionSettings,
    options: RegexRunOptions = {}
  ): string {
    // 非字符串输入返回空字符串
    if (typeof raw !== 'string') {
      return ''
    }

    // 空字符串直接返回
    if (raw.length === 0) {
      return raw
    }

    let text = raw

    // 合并允许运行的脚本（按 global -> preset -> scoped 顺序）
    const scripts = this.getAllowedScripts(settings, options)

    // 依次执行每个脚本
    for (const sourcedScript of scripts) {
      if (this.shouldRunScript(sourcedScript.script, placement, options)) {
        text = this.runScript(sourcedScript.script, text, options)
      }
    }

    return text
  }

  /**
   * 获取允许运行的脚本列表（按顺序）
   *
   * 合并顺序：global -> preset -> scoped
   */
  private getAllowedScripts(
    settings: RegexExtensionSettings,
    options: RegexRunOptions
  ): SourcedScript[] {
    const result: SourcedScript[] = []
    const seen = new Set<string>()
    const scriptById = new Map(settings.regex.map(script => [script.id, script]))
    const selectedPresets = settings.regex_presets.filter(preset => preset.is_selected)

    // 1. Global 脚本：没有启用 preset 时保留历史行为，全部 global 可见。
    if (selectedPresets.length === 0) {
      for (const script of settings.regex) {
        if (!seen.has(script.id)) {
          seen.add(script.id)
          result.push({
            script,
            source: ScriptSource.Global,
          })
        }
      }
    } else {
      for (const preset of selectedPresets) {
        for (const item of preset.global ?? []) {
          const script = scriptById.get(item.id)
          if (script && !seen.has(script.id)) {
            seen.add(script.id)
            result.push({ script, source: ScriptSource.Global })
          }
        }
      }
    }

    // 2. Preset 脚本：需要通过 preset_allowed_regex
    if (options.presetKey) {
      const allowedIds = settings.preset_allowed_regex[options.presetKey] ?? []
      for (const preset of selectedPresets) {
        for (const item of preset.preset ?? []) {
          if (!allowedIds.includes(item.id)) continue
          const script = scriptById.get(item.id)
          if (script && !seen.has(script.id)) {
            seen.add(script.id)
            result.push({ script, source: ScriptSource.Preset })
          }
        }
      }
    }

    // 3. Scoped 脚本：需要通过 character_allowed_regex
    const characterScopeEnabled = options.characterName
      ? settings.character_allowed_regex.includes(options.characterName)
      : false
    for (const preset of selectedPresets) {
      for (const item of preset.scoped ?? []) {
        if (!characterScopeEnabled && !settings.character_allowed_regex.includes(item.id)) continue
        const script = scriptById.get(item.id)
        if (script && !seen.has(script.id)) {
          seen.add(script.id)
          result.push({ script, source: ScriptSource.Scoped })
        }
      }
    }

    return result
  }

  /**
   * 判断脚本是否应该运行
   */
  private shouldRunScript(
    script: RegexScriptData,
    placement: RegexPlacementValue,
    options: RegexRunOptions
  ): boolean {
    // 禁用的脚本跳过
    if (script.disabled) {
      return false
    }

    // find_regex 为空跳过
    if (!script.find_regex) {
      return false
    }

    // placement 不包含当前作用点跳过
    if (!script.placement?.includes(placement)) {
      return false
    }

    // 编辑模式下检查 run_on_edit
    if (options.isEdit && !script.run_on_edit) {
      return false
    }

    // 深度过滤
    if (options.depth != null) {
      if (script.min_depth != null && options.depth < script.min_depth) {
        return false
      }
      if (script.max_depth != null && options.depth > script.max_depth) {
        return false
      }
    }

    // markdownOnly 只在 isMarkdown 时运行
    if (script.markdown_only && !options.isMarkdown) {
      return false
    }

    // promptOnly 只在 isPrompt 时运行
    if (script.prompt_only && !options.isPrompt) {
      return false
    }

    // 两者都为 false 时，只在非 markdown、非 prompt 的源文本阶段运行
    if (!script.markdown_only && !script.prompt_only) {
      if (options.isMarkdown || options.isPrompt) {
        return false
      }
    }

    return true
  }

  /**
   * 执行单个脚本
   */
  private runScript(
    script: RegexScriptData,
    text: string,
    _options: RegexRunOptions
  ): string {
    // 编译正则
    const regex = this.compileRegex(script.find_regex, script.substitute_regex)
    if (!regex) {
      return text
    }

    // 执行替换，支持 $1/$<name>，并补充 {{match}}。
    let result = text.replace(regex, (...args: unknown[]) => {
      const match = String(args[0] ?? '')
      const captures = args.slice(1, -2).map(value => String(value ?? ''))
      let replacement = script.replace_string.replaceAll('{{match}}', match)
      replacement = replacement.replace(/\$(\d+)/g, (_, index: string) => {
        const capture = captures[Number(index) - 1]
        return capture ?? ''
      })
      return replacement
    })

    // 处理 trim_strings
    if (script.trim_strings && script.trim_strings.length > 0) {
      result = this.applyTrimStrings(result, script.trim_strings)
    }

    return result
  }

  /**
   * 编译正则表达式
   */
  private compileRegex(pattern: string, _substitute?: SubstituteRegexValue): RegExp | null {
    // 尝试从缓存获取
    if (this.regexCache.has(pattern)) {
      return this.regexCache.get(pattern)!
    }

    // 编译正则
    try {
      // 尝试解析 /pattern/flags 格式
      const { regexPattern, flags } = this.parseRegexPattern(pattern)
      const regex = new RegExp(regexPattern, flags)

      // 管理缓存大小
      if (this.regexCache.size >= this.cacheMaxSize) {
        const firstKey = this.regexCache.keys().next().value
        if (firstKey) {
          this.regexCache.delete(firstKey)
        }
      }
      this.regexCache.set(pattern, regex)

      return regex
    } catch {
      return null
    }
  }

  /**
   * 解析正则表达式模式
   *
   * 支持 /pattern/flags 格式，也支持普通 pattern。
   */
  private parseRegexPattern(pattern: string): { regexPattern: string; flags: string } {
    // 检查是否是 /pattern/flags 格式
    if (pattern.startsWith('/')) {
      const lastSlash = pattern.lastIndexOf('/')
      if (lastSlash > 1) {
        const inner = pattern.slice(1, lastSlash)
        const flagsStr = pattern.slice(lastSlash + 1)
        return { regexPattern: inner, flags: flagsStr }
      }
    }

    // 普通 pattern
    return { regexPattern: pattern, flags: 'g' }
  }

  /**
   * 应用 trim_strings
   */
  private applyTrimStrings(text: string, trimStrings: string[]): string {
    let result = text
    for (const trimStr of trimStrings) {
      // TODO: 执行宏替换
      result = result.replaceAll(trimStr, '')
    }
    return result
  }

  /**
   * 清除缓存
   */
  clearCache(): void {
    this.regexCache.clear()
  }
}

/**
 * 创建默认的 Regex 引擎实例
 */
export function createRegexEngine(): RegexEngine {
  return new RegexEngine()
}

/**
 * 创建默认的运行时选项
 */
export function createDefaultRegexRunOptions(): RegexRunOptions {
  return {
    isMarkdown: false,
    isPrompt: false,
    isEdit: false,
    depth: null,
  }
}
