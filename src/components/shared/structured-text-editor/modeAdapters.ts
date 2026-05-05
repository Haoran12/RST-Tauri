import YAML from 'yaml'
import type {
  RequiredValueShape,
  StructuredTextBinding,
  StructuredTextDiagnostic,
  StructuredTextLanguageId,
} from '@/types/structuredText'

interface AnalyzeResult {
  diagnostics: StructuredTextDiagnostic[]
  parsedValue?: unknown
}

interface FormatResult {
  text: string
  diagnostics: StructuredTextDiagnostic[]
  parsedValue?: unknown
}

const BLOCKER = 'blocker'
const WARNING = 'warning'
const INFO = 'info'
const INFER_SAMPLE_LIMIT = 8192

export function inferInitialStructuredTextMode(
  text: string,
  binding: StructuredTextBinding,
  explicitMode?: StructuredTextLanguageId,
): StructuredTextLanguageId {
  if (explicitMode && binding.allowedModes.includes(explicitMode)) {
    return explicitMode
  }

  const fallback = binding.allowedModes.includes(binding.defaultMode)
    ? binding.defaultMode
    : binding.allowedModes[0] ?? 'plain'
  const sample = text.slice(0, INFER_SAMPLE_LIMIT).trim()
  if (!sample) {
    return fallback
  }

  if (binding.allowedModes.includes('json') && looksLikeJson(sample)) {
    return 'json'
  }

  if (binding.allowedModes.includes('yaml') && looksLikeYaml(sample, binding)) {
    return 'yaml'
  }

  return fallback
}

export function analyzeStructuredText(
  text: string,
  mode: StructuredTextLanguageId,
  binding: StructuredTextBinding,
): AnalyzeResult {
  switch (mode) {
    case 'json':
      return analyzeJson(text, binding)
    case 'yaml':
      return analyzeYaml(text, binding)
    case 'plain':
    default:
      return analyzePlain(text, binding)
  }
}

export function formatStructuredText(
  text: string,
  mode: StructuredTextLanguageId,
  binding: StructuredTextBinding,
): FormatResult {
  switch (mode) {
    case 'json':
      return formatJson(text, binding)
    case 'yaml':
      return formatYaml(text, binding)
    case 'plain':
    default:
      return {
        text,
        diagnostics: analyzePlain(text, binding).diagnostics,
      }
  }
}

export function hasBlockingDiagnostics(diagnostics: StructuredTextDiagnostic[]) {
  return diagnostics.some(item => item.severity === BLOCKER)
}

export function getDiagnosticSummary(diagnostics: StructuredTextDiagnostic[]) {
  const blockers = diagnostics.filter(item => item.severity === BLOCKER).length
  const warnings = diagnostics.filter(item => item.severity === WARNING).length
  const infos = diagnostics.filter(item => item.severity === INFO).length

  if (blockers > 0) {
    return {
      label: `${blockers} blocker${blockers > 1 ? 's' : ''}`,
      type: 'error' as const,
    }
  }

  if (warnings > 0) {
    return {
      label: `${warnings} warning${warnings > 1 ? 's' : ''}`,
      type: 'warning' as const,
    }
  }

  if (infos > 0) {
    return {
      label: `${infos} info`,
      type: 'info' as const,
    }
  }

  return {
    label: 'Passed',
    type: 'success' as const,
  }
}

function analyzePlain(text: string, binding: StructuredTextBinding): AnalyzeResult {
  const diagnostics = scanBracketAndQuoteBalance(text)

  if (binding.storageKind === 'json_value') {
    diagnostics.push({
      severity: BLOCKER,
      code: 'schema_type_mismatch',
      message: '当前字段要求结构化值，不能以 Plain 模式保存。',
      line: 1,
      column: 1,
    })
  }

  return { diagnostics }
}

function analyzeJson(text: string, binding: StructuredTextBinding): AnalyzeResult {
  const diagnostics: StructuredTextDiagnostic[] = []

  try {
    const parsed = JSON.parse(text)
    const shapeDiagnostic = validateBindingShape(parsed, binding)
    if (shapeDiagnostic) {
      diagnostics.push(shapeDiagnostic)
    }

    return {
      diagnostics,
      parsedValue: parsed,
    }
  } catch (error) {
    diagnostics.push(jsonParseDiagnostic(error, text))

    const normalized = normalizeJsonCandidate(text)
    if (normalized !== text) {
      try {
        JSON.parse(normalized)
        diagnostics.push({
          severity: INFO,
          code: 'auto_fix_available',
          message: '检测到可安全修复的 JSON key 引号问题，可使用 Format 自动整理。',
          line: 1,
          column: 1,
        })
      } catch {
        // ignore
      }
    }

    return { diagnostics }
  }
}

function formatJson(text: string, binding: StructuredTextBinding): FormatResult {
  const raw = text.trim()
  const normalized = normalizeJsonCandidate(raw)
  const source = normalized.length > 0 ? normalized : raw
  const parsed = JSON.parse(source)
  const diagnostics: StructuredTextDiagnostic[] = []
  const shapeDiagnostic = validateBindingShape(parsed, binding)

  if (shapeDiagnostic) {
    diagnostics.push(shapeDiagnostic)
  }

  if (normalized !== raw) {
    diagnostics.push({
      severity: INFO,
      code: 'auto_fix_applied',
      message: '已应用安全的 JSON key 引号修复。',
      line: 1,
      column: 1,
    })
  }

  return {
    text: JSON.stringify(parsed, null, 2),
    diagnostics,
    parsedValue: parsed,
  }
}

function analyzeYaml(text: string, binding: StructuredTextBinding): AnalyzeResult {
  const diagnostics: StructuredTextDiagnostic[] = []
  diagnostics.push(...collectYamlIndentDiagnostics(text))

  const featureDiagnostic = collectYamlFeatureDiagnostic(text, binding)
  if (featureDiagnostic) {
    diagnostics.push(featureDiagnostic)
  }

  const document = YAML.parseDocument(text, {
    prettyErrors: true,
    uniqueKeys: true,
  })

  for (const error of document.errors) {
    diagnostics.push(yamlMessageDiagnostic(error.message, text))
  }

  for (const warning of document.warnings) {
    diagnostics.push(yamlMessageDiagnostic(warning.message, text, WARNING))
  }

  if (document.errors.length > 0) {
    return { diagnostics }
  }

  const parsed = document.toJS()
  const shapeDiagnostic = validateBindingShape(parsed, binding)
  if (shapeDiagnostic) {
    diagnostics.push(shapeDiagnostic)
  }

  return {
    diagnostics,
    parsedValue: parsed,
  }
}

function formatYaml(text: string, binding: StructuredTextBinding): FormatResult {
  const document = YAML.parseDocument(text, {
    prettyErrors: true,
    uniqueKeys: true,
  })

  if (document.errors.length > 0) {
    throw new Error(document.errors[0]?.message ?? 'YAML parse error')
  }

  const featureDiagnostic = collectYamlFeatureDiagnostic(text, binding)
  const diagnostics = collectYamlIndentDiagnostics(text)
  if (featureDiagnostic) {
    diagnostics.push(featureDiagnostic)
  }

  const parsed = document.toJS()
  const shapeDiagnostic = validateBindingShape(parsed, binding)
  if (shapeDiagnostic) {
    diagnostics.push(shapeDiagnostic)
  }

  return {
    text: YAML.stringify(parsed, {
      indent: 2,
      lineWidth: 0,
      minContentWidth: 0,
    }).trimEnd(),
    diagnostics,
    parsedValue: parsed,
  }
}

function scanBracketAndQuoteBalance(text: string): StructuredTextDiagnostic[] {
  const diagnostics: StructuredTextDiagnostic[] = []
  const stack: Array<{ char: string; index: number }> = []
  let activeQuote: { char: string; index: number } | null = null
  let escaped = false

  const pairs: Record<string, string> = {
    ')': '(',
    ']': '[',
    '}': '{',
  }

  for (let index = 0; index < text.length; index += 1) {
    const char = text[index]

    if (activeQuote) {
      if (escaped) {
        escaped = false
        continue
      }

      if (char === '\\') {
        escaped = true
        continue
      }

      if (char === activeQuote.char) {
        activeQuote = null
      }
      continue
    }

    if (char === '"' || char === '\'' || char === '`') {
      activeQuote = { char, index }
      continue
    }

    if (char === '(' || char === '[' || char === '{') {
      stack.push({ char, index })
      continue
    }

    if (char === ')' || char === ']' || char === '}') {
      const last = stack.pop()
      if (!last || last.char !== pairs[char]) {
        const position = offsetToPosition(text, index)
        diagnostics.push({
          severity: WARNING,
          code: 'unmatched_bracket',
          message: `未匹配的括号 ${char}`,
          line: position.line,
          column: position.column,
          length: 1,
        })
      }
    }
  }

  if (activeQuote) {
    const position = offsetToPosition(text, activeQuote.index)
    diagnostics.push({
      severity: WARNING,
      code: 'unclosed_quote',
      message: `未闭合的引号 ${activeQuote.char}`,
      line: position.line,
      column: position.column,
      length: 1,
    })
  }

  for (const item of stack) {
    const position = offsetToPosition(text, item.index)
    diagnostics.push({
      severity: WARNING,
      code: 'unmatched_bracket',
      message: `未闭合的括号 ${item.char}`,
      line: position.line,
      column: position.column,
      length: 1,
    })
  }

  return diagnostics
}

function validateBindingShape(
  value: unknown,
  binding: StructuredTextBinding,
): StructuredTextDiagnostic | null {
  if (binding.storageKind !== 'json_value' && binding.storageKind !== 'yaml_file') {
    return null
  }

  const shape = resolveValueShape(value)
  const requiredShape = binding.requiredValueShape ?? 'any'

  if (requiredShape === 'any') {
    return null
  }

  if (shape !== requiredShape) {
    return {
      severity: BLOCKER,
      code: 'schema_type_mismatch',
      message: `字段要求 ${requiredShape}，当前解析结果为 ${shape}。`,
      line: 1,
      column: 1,
    }
  }

  return null
}

function resolveValueShape(value: unknown): RequiredValueShape {
  if (Array.isArray(value)) {
    return 'array'
  }

  if (value !== null && typeof value === 'object') {
    return 'object'
  }

  if (typeof value === 'string') {
    return 'string'
  }

  return 'any'
}

function looksLikeJson(sample: string) {
  if (!sample.startsWith('{') && !sample.startsWith('[')) {
    return false
  }

  try {
    JSON.parse(sample)
    return true
  } catch {
    // continue to JSON-like repair probe
  }

  const normalized = normalizeJsonCandidate(sample)
  if (normalized === sample) {
    return false
  }

  try {
    JSON.parse(normalized)
    return true
  } catch {
    return false
  }
}

function looksLikeYaml(sample: string, binding: StructuredTextBinding) {
  if (sample.startsWith('---')) {
    return true
  }

  const meaningfulLines = sample
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter(line => line && !line.startsWith('#'))
  const yamlLineCount = meaningfulLines.filter(line =>
    /^-\s+\S/.test(line) || /^[A-Za-z_][\w.-]*\s*:/.test(line) || /^['"][^'"\r\n]+['"]\s*:/.test(line),
  ).length

  if (yamlLineCount === 0) {
    return false
  }

  if (binding.storageKind === 'json_value' || binding.storageKind === 'yaml_file') {
    return true
  }

  return yamlLineCount >= 2 || meaningfulLines.some(line => /^-\s+\S/.test(line))
}

function normalizeJsonCandidate(text: string) {
  let index = 0
  let output = ''
  const stack: Array<{ type: 'object' | 'array'; expectingKey: boolean }> = []

  while (index < text.length) {
    const top = stack[stack.length - 1]

    if (top?.type === 'object' && top.expectingKey) {
      const key = readJsonLikeKey(text, index)
      if (key) {
        output += text.slice(index, key.start)
        output += `"${escapeJsonKey(key.key)}"`
        output += text.slice(key.end, key.colonIndex + 1)
        top.expectingKey = false
        index = key.colonIndex + 1
        continue
      }
    }

    const char = text[index]

    if (char === '"') {
      const end = readJsonStringEnd(text, index, '"')
      output += text.slice(index, end)
      index = end
      continue
    }

    if (char === '{') {
      stack.push({ type: 'object', expectingKey: true })
      output += char
      index += 1
      continue
    }

    if (char === '[') {
      stack.push({ type: 'array', expectingKey: false })
      output += char
      index += 1
      continue
    }

    if (char === '}' || char === ']') {
      stack.pop()
      output += char
      index += 1
      continue
    }

    if (char === ':' && top?.type === 'object') {
      top.expectingKey = false
      output += char
      index += 1
      continue
    }

    if (char === ',' && top?.type === 'object') {
      top.expectingKey = true
      output += char
      index += 1
      continue
    }

    output += char
    index += 1
  }

  return output
}

function readJsonLikeKey(text: string, startIndex: number) {
  const start = skipJsonWhitespace(text, startIndex)
  const char = text[start]

  if (char === "'") {
    const end = readJsonStringEnd(text, start, "'")
    if (end > text.length || text[end - 1] !== "'") {
      return null
    }

    const colonIndex = skipJsonWhitespace(text, end)
    if (text[colonIndex] !== ':') {
      return null
    }

    return {
      start,
      end,
      colonIndex,
      key: unescapeSingleQuotedKey(text.slice(start + 1, end - 1)),
    }
  }

  if (!isJsonBareKeyStart(char)) {
    return null
  }

  let end = start + 1
  while (end < text.length && isJsonBareKeyPart(text[end])) {
    end += 1
  }

  const colonIndex = skipJsonWhitespace(text, end)
  if (text[colonIndex] !== ':') {
    return null
  }

  return {
    start,
    end,
    colonIndex,
    key: text.slice(start, end),
  }
}

function skipJsonWhitespace(text: string, index: number) {
  let current = index
  while (current < text.length && /\s/.test(text[current])) {
    current += 1
  }
  return current
}

function readJsonStringEnd(text: string, start: number, quote: '"' | "'") {
  let escaped = false
  for (let index = start + 1; index < text.length; index += 1) {
    const char = text[index]
    if (escaped) {
      escaped = false
      continue
    }
    if (char === '\\') {
      escaped = true
      continue
    }
    if (char === quote) {
      return index + 1
    }
  }
  return text.length + 1
}

function isJsonBareKeyStart(char: string | undefined) {
  return Boolean(char && /[A-Za-z_\u00A0-\uFFFF]/u.test(char))
}

function isJsonBareKeyPart(char: string | undefined) {
  return Boolean(char && /[\w.\-\u00A0-\uFFFF]/u.test(char))
}

function unescapeSingleQuotedKey(key: string) {
  return key.replace(/\\'/g, "'").replace(/\\\\/g, '\\')
}

function escapeJsonKey(key: string) {
  return key.replace(/\\/g, '\\\\').replace(/"/g, '\\"')
}

function jsonParseDiagnostic(error: unknown, text: string): StructuredTextDiagnostic {
  const message = error instanceof Error ? error.message : 'Invalid JSON'
  const match = message.match(/position\s+(\d+)/i)
  const offset = match ? Number(match[1]) : 0
  const position = offsetToPosition(text, Number.isFinite(offset) ? offset : 0)

  return {
    severity: BLOCKER,
    code: 'parse_error',
    message,
    line: position.line,
    column: position.column,
    length: 1,
  }
}

function collectYamlIndentDiagnostics(text: string): StructuredTextDiagnostic[] {
  const diagnostics: StructuredTextDiagnostic[] = []
  const lines = text.split(/\r?\n/)

  lines.forEach((line, index) => {
    const leadingTabs = line.match(/^\t+/)
    if (leadingTabs) {
      diagnostics.push({
        severity: BLOCKER,
        code: 'parse_error',
        message: 'YAML 缩进不能使用 tab。',
        line: index + 1,
        column: 1,
        length: leadingTabs[0].length,
      })
    }

    const colonWithoutSpace = line.match(/^(\s*[^#\s][^:#]*):(?![\s\]|>]|$)/)
    if (colonWithoutSpace) {
      diagnostics.push({
        severity: WARNING,
        code: 'auto_fix_available',
        message: '建议在 YAML 冒号后补空格。',
        line: index + 1,
        column: colonWithoutSpace[0].length,
        length: 1,
      })
    }
  })

  return diagnostics
}

function collectYamlFeatureDiagnostic(
  text: string,
  binding: StructuredTextBinding,
): StructuredTextDiagnostic | null {
  const anchorLike = /(^|\s)[&*][A-Za-z0-9_-]+/m.test(text)
  const tagLike = /(^|\s)![A-Za-z<]/m.test(text)

  if (!anchorLike && !tagLike) {
    return null
  }

  const unsupportedForStructured = binding.storageKind === 'json_value'
  return {
    severity: unsupportedForStructured ? BLOCKER : WARNING,
    code: 'unsupported_yaml_feature',
    message: unsupportedForStructured
      ? '当前字段保存为结构化值，暂不支持 YAML anchor / alias / tag。'
      : 'YAML anchor / alias / tag 仅作为文本保留，不会参与业务结构展开。',
    line: 1,
    column: 1,
  }
}

function yamlMessageDiagnostic(
  message: string,
  text: string,
  severity: StructuredTextDiagnostic['severity'] = BLOCKER,
): StructuredTextDiagnostic {
  const match = message.match(/at line (\d+), column (\d+)/i)
  const line = match ? Number(match[1]) : 1
  const column = match ? Number(match[2]) : 1

  if (!match) {
    const fallback = offsetToPosition(text, 0)
    return {
      severity,
      code: 'parse_error',
      message,
      line: fallback.line,
      column: fallback.column,
    }
  }

  return {
    severity,
    code: 'parse_error',
    message,
    line,
    column,
  }
}

function offsetToPosition(text: string, rawOffset: number) {
  const offset = Math.max(0, Math.min(rawOffset, text.length))
  const lines = text.slice(0, offset).split(/\r?\n/)
  return {
    line: lines.length,
    column: lines[lines.length - 1]?.length + 1 || 1,
  }
}
