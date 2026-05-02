/**
 * WorldBook keyword matching system
 *
 * 关键词匹配系统，支持基础匹配、正则匹配和匹配目标扩展。
 * 参考: SillyTavern/public/scripts/world-info.js
 */

import type { WorldInfoEntry } from '@/types/st'

/**
 * 关键词匹配结果
 */
export interface MatchResult {
  entryUid: number
  matchedKeys: string[]
  matchedSecondary: string[]
}

/**
 * 全局扫描数据
 *
 * 包含角色卡各部分文本，用于 match_* 扩展匹配目标。
 */
export interface GlobalScanData {
  personaDescription: string
  characterDescription: string
  characterPersonality: string
  characterDepthPrompt: string
  scenario: string
  creatorNotes: string
  trigger?: string
}

/**
 * 匹配上下文
 */
export interface MatchContext {
  /** 扫描文本（聊天历史） */
  scanText: string
  /** 全局扫描数据 */
  globalScanData: GlobalScanData
  /** 全局大小写敏感设置 */
  globalCaseSensitive: boolean
  /** 全局全词匹配设置 */
  globalMatchWholeWords: boolean
  /** 全局扫描深度 */
  globalScanDepth: number
}

/**
 * 关键词匹配器
 */
export class KeywordMatcher {
  private regexCache: Map<string, RegExp> = new Map()
  private cacheMaxSize = 1000

  /**
   * 检查单个词条是否匹配
   *
   * 返回匹配结果，如果不匹配则返回 null。
   */
  matchEntry(entry: WorldInfoEntry, context: MatchContext): MatchResult | null {
    // 跳过禁用的词条
    if (entry.disable) {
      return null
    }

    // 检查触发器
    if (entry.triggers && entry.triggers.length > 0) {
      if (!context.globalScanData.trigger) {
        return null
      }
      if (!entry.triggers.includes(context.globalScanData.trigger)) {
        return null
      }
    }

    // 获取匹配设置
    const caseSensitive = entry.case_sensitive ?? context.globalCaseSensitive
    const matchWholeWords = entry.match_whole_words ?? context.globalMatchWholeWords

    // 匹配主关键词
    // 来源可以是：扫描文本、或启用的扩展目标
    const matchedKeys = this.matchKeysWithExtensions(
      entry.key ?? [],
      context.scanText,
      caseSensitive,
      matchWholeWords,
      entry,
      context.globalScanData
    )

    if (!matchedKeys) {
      return null
    }

    // 如果启用 selective，检查次关键词逻辑
    let matchedSecondary: string[] = []

    if (entry.selective && entry.keysecondary && entry.keysecondary.length > 0) {
      const secondaryMatches = this.matchKeys(
        entry.keysecondary,
        context.scanText,
        caseSensitive,
        matchWholeWords
      )

      const hasSecondary = secondaryMatches !== null
      const logic = entry.selective_logic ?? 0 // 默认 AND_ANY

      switch (logic) {
        case 0: // AND_ANY
          if (!hasSecondary) return null
          matchedSecondary = secondaryMatches ?? []
          break

        case 1: // NOT_ALL
          if (
            hasSecondary &&
            secondaryMatches!.length === entry.keysecondary.length
          ) {
            return null
          }
          matchedSecondary = secondaryMatches ?? []
          break

        case 2: // NOT_ANY
          if (hasSecondary) return null
          matchedSecondary = []
          break

        case 3: // AND_ALL
          if (!hasSecondary || secondaryMatches!.length !== entry.keysecondary.length) {
            return null
          }
          matchedSecondary = secondaryMatches ?? []
          break

        default:
          if (!hasSecondary) return null
          matchedSecondary = secondaryMatches ?? []
      }
    }

    return {
      entryUid: entry.uid,
      matchedKeys,
      matchedSecondary,
    }
  }

  /**
   * 匹配关键词（含扩展目标）
   *
   * 关键词可以在扫描文本或启用的扩展目标中匹配。
   */
  private matchKeysWithExtensions(
    keys: string[],
    scanText: string,
    caseSensitive: boolean,
    matchWholeWords: boolean,
    entry: WorldInfoEntry,
    globalScanData: GlobalScanData
  ): string[] | null {
    if (keys.length === 0) {
      return null
    }

    // 首先在扫描文本中匹配
    const scanMatches = this.matchKeys(keys, scanText, caseSensitive, matchWholeWords)
    if (scanMatches) {
      return scanMatches
    }

    // 如果扫描文本没有匹配，检查扩展目标
    const matched: string[] = []

    for (const key of keys) {
      if (!key) continue

      let found = false

      // 检查各扩展目标
      if (entry.match_persona_description && globalScanData.personaDescription) {
        if (this.matchSingleKey(key, globalScanData.personaDescription, caseSensitive)) {
          found = true
        }
      }

      if (!found && entry.match_character_description && globalScanData.characterDescription) {
        if (this.matchSingleKey(key, globalScanData.characterDescription, caseSensitive)) {
          found = true
        }
      }

      if (!found && entry.match_character_personality && globalScanData.characterPersonality) {
        if (this.matchSingleKey(key, globalScanData.characterPersonality, caseSensitive)) {
          found = true
        }
      }

      if (!found && entry.match_character_depth_prompt && globalScanData.characterDepthPrompt) {
        if (this.matchSingleKey(key, globalScanData.characterDepthPrompt, caseSensitive)) {
          found = true
        }
      }

      if (!found && entry.match_scenario && globalScanData.scenario) {
        if (this.matchSingleKey(key, globalScanData.scenario, caseSensitive)) {
          found = true
        }
      }

      if (!found && entry.match_creator_notes && globalScanData.creatorNotes) {
        if (this.matchSingleKey(key, globalScanData.creatorNotes, caseSensitive)) {
          found = true
        }
      }

      if (found) {
        matched.push(key)
      }
    }

    return matched.length > 0 ? matched : null
  }

  /**
   * 匹配关键词列表
   *
   * 返回匹配到的关键词列表，如果没有匹配则返回 null。
   */
  private matchKeys(
    keys: string[],
    text: string,
    caseSensitive: boolean,
    matchWholeWords: boolean
  ): string[] | null {
    if (keys.length === 0) {
      return null
    }

    const searchText = caseSensitive ? text : text.toLowerCase()
    const matched: string[] = []

    for (const key of keys) {
      if (!key) continue

      // 检查是否是正则表达式（以 / 开头）
      if (key.startsWith('/')) {
        const regexMatch = this.matchRegex(key, text, caseSensitive)
        if (regexMatch) {
          matched.push(key)
        }
      } else {
        const searchKey = caseSensitive ? key : key.toLowerCase()

        const found = matchWholeWords
          ? this.matchWholeWord(searchText, searchKey)
          : searchText.includes(searchKey)

        if (found) {
          matched.push(key)
        }
      }
    }

    return matched.length > 0 ? matched : null
  }

  /**
   * 匹配单个关键词
   */
  private matchSingleKey(key: string, text: string, caseSensitive: boolean): boolean {
    if (!key || !text) return false

    // 检查是否是正则表达式
    if (key.startsWith('/')) {
      return this.matchRegex(key, text, caseSensitive) !== null
    } else {
      const searchText = caseSensitive ? text : text.toLowerCase()
      const searchKey = caseSensitive ? key : key.toLowerCase()
      return searchText.includes(searchKey)
    }
  }

  /**
   * 匹配正则表达式
   */
  private matchRegex(pattern: string, text: string, caseSensitive: boolean): string | null {
    // 解析 /pattern/flags 格式
    const { regexPattern, flags } = this.parseRegexPattern(pattern, caseSensitive)

    // 构建缓存 key
    const cacheKey = `${regexPattern}|${flags}`

    // 尝试从缓存获取或编译正则
    let regex: RegExp
    if (this.regexCache.has(cacheKey)) {
      regex = this.regexCache.get(cacheKey)!
    } else {
      try {
        regex = new RegExp(regexPattern, flags)
        // 管理缓存大小
        if (this.regexCache.size >= this.cacheMaxSize) {
          const firstKey = this.regexCache.keys().next().value
          if (firstKey) {
            this.regexCache.delete(firstKey)
          }
        }
        this.regexCache.set(cacheKey, regex)
      } catch {
        return null
      }
    }

    // 执行匹配
    return regex.test(text) ? pattern : null
  }

  /**
   * 解析正则表达式模式
   *
   * 支持 /pattern/flags 格式，也支持普通 pattern。
   */
  private parseRegexPattern(
    pattern: string,
    caseSensitive: boolean
  ): { regexPattern: string; flags: string } {
    // 检查是否是 /pattern/flags 格式
    if (pattern.startsWith('/')) {
      const lastSlash = pattern.lastIndexOf('/')
      if (lastSlash > 1) {
        const inner = pattern.slice(1, lastSlash)
        const flagsStr = pattern.slice(lastSlash + 1)

        // 处理 flags
        let hasI = false
        let finalFlags = ''

        for (const c of flagsStr) {
          switch (c) {
            case 'i':
              hasI = true
              break
            case 'm':
            case 's':
              finalFlags += c
              break
          }
        }

        // 如果需要大小写不敏感
        if (!caseSensitive || hasI) {
          finalFlags = 'i' + finalFlags
        }

        return { regexPattern: inner, flags: finalFlags }
      }
    }

    // 普通 pattern
    return {
      regexPattern: pattern,
      flags: caseSensitive ? '' : 'i',
    }
  }

  /**
   * 全词匹配
   */
  private matchWholeWord(text: string, word: string): boolean {
    // 使用单词边界匹配
    const pattern = `\\b${this.escapeRegex(word)}\\b`
    try {
      const regex = new RegExp(pattern, 'i')
      return regex.test(text)
    } catch {
      return text.includes(word)
    }
  }

  /**
   * 转义正则特殊字符
   */
  private escapeRegex(str: string): string {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  /**
   * 清除正则缓存
   */
  clearCache(): void {
    this.regexCache.clear()
  }
}

/**
 * 创建默认的匹配器实例
 */
export function createKeywordMatcher(): KeywordMatcher {
  return new KeywordMatcher()
}

/**
 * 创建默认的全局扫描数据
 */
export function createDefaultGlobalScanData(): GlobalScanData {
  return {
    personaDescription: '',
    characterDescription: '',
    characterPersonality: '',
    characterDepthPrompt: '',
    scenario: '',
    creatorNotes: '',
  }
}

/**
 * 创建默认的匹配上下文
 */
export function createDefaultMatchContext(scanText: string): MatchContext {
  return {
    scanText,
    globalScanData: createDefaultGlobalScanData(),
    globalCaseSensitive: false,
    globalMatchWholeWords: false,
    globalScanDepth: 4,
  }
}
