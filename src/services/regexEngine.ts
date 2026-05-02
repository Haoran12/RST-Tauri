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
    _options: RegexRunOptions
  ): SourcedScript[] {
    const result: SourcedScript[] = []

    // 1. Global 脚本：全部可见
    for (const script of settings.regex) {
      result.push({
        script,
        source: ScriptSource.Global,
      })
    }

    // 2. Preset 脚本：需要通过 preset_allowed_regex
    // TODO: 从当前预设加载脚本
    // 当前预设名需要在 options.presetKey 中传递
    // if (options.presetKey) {
    //   const allowedIds = settings.preset_allowed_regex[options.presetKey]
    //   if (allowedIds) {
    //     // 只添加在 allow list 中的脚本
    //   }
    // }

    // 3. Scoped 脚本：需要通过 character_allowed_regex
    // TODO: 从当前角色卡加载脚本
    // 当前角色名需要在 options.characterName 中传递
    // if (options.characterName) {
    //   if (settings.character_allowed_regex.includes(options.characterName)) {
    //     // 添加角色卡内嵌脚本
    //   }
    // }

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

    // 执行替换
    let result = text.replace(regex, script.replace_string)

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
