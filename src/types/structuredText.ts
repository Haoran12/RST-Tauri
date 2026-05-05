/**
 * 结构化文本编辑器类型定义
 * @see docs/42_structured_text_editor.md
 */

import type { Extension } from '@codemirror/state'

// ============ 基础类型 ============

export type BuiltinStructuredTextMode = 'plain' | 'json' | 'yaml';
export type StructuredTextLanguageId = string;
export type StructuredTextSeverity = 'info' | 'warning' | 'blocker';

// ============ 诊断模型 ============

export type StructuredTextDiagnosticCode =
  | 'unmatched_bracket'
  | 'unclosed_quote'
  | 'invalid_escape'
  | 'parse_error'
  | 'unsupported_yaml_feature'
  | 'auto_fix_available'
  | 'auto_fix_applied'
  | 'schema_type_mismatch';

export interface StructuredTextDiagnostic {
  severity: StructuredTextSeverity;
  code: StructuredTextDiagnosticCode;
  message: string;
  line: number;
  column: number;
  length?: number;
}

export interface StructuredTextValidationResult {
  text: string;
  diagnostics: StructuredTextDiagnostic[];
  parsedValue?: unknown;
}

// ============ 绑定模型 ============

export type StructuredTextResourceKind =
  | 'st_worldbook_entry'
  | 'st_characterbook_entry'
  | 'st_preset'
  | 'st_regex_script'
  | 'agent_knowledge_entry'
  | 'agent_world_rules'
  | 'generic_extensions';

export type StructuredTextStorageKind = 'string' | 'json_value' | 'yaml_file';
export type RequiredValueShape = 'string' | 'object' | 'array' | 'any';

export interface StructuredTextBinding {
  resourceKind: StructuredTextResourceKind;
  fieldPath: string;
  allowedModes: StructuredTextLanguageId[];
  defaultMode: StructuredTextLanguageId;
  storageKind: StructuredTextStorageKind;
  requiredValueShape?: RequiredValueShape;
}

// ============ 草稿模型 ============

export interface StructuredTextDraft {
  binding: StructuredTextBinding;
  mode: StructuredTextLanguageId;
  text: string;
  diagnostics: StructuredTextDiagnostic[];
  isDirty: boolean;
  lastFormattedAt?: string;
}

// ============ 语言包模型 ============

export type LanguagePackSource = 'builtin' | 'bundled' | 'trusted_plugin';

export interface StructuredTextLanguagePack {
  languageId: string;
  label: string;
  source: LanguagePackSource;
  storageKinds: StructuredTextStorageKind[];
  load: () => Promise<Extension[]>;
  canParseToJsonValue?: boolean;
  supportsFormat?: boolean;
  supportsLint?: boolean;
  supportsAutoIndent?: boolean;
}

// ============ 编辑器配置 ============

export interface StructuredTextEditorConfig {
  mode: StructuredTextLanguageId;
  readonly?: boolean;
  lineNumbers?: boolean;
  lineWrapping?: boolean;
  tabSize?: number;
  theme?: 'light' | 'dark';
}

// ============ Quick Fix ============

export interface QuickFixAction {
  label: string;
  apply: () => void;
}

export interface StructuredTextBackendRequest {
  text: string;
  mode: StructuredTextLanguageId;
  binding: StructuredTextBinding;
}

// ============ 预定义绑定 ============

export const DEFAULT_BINDINGS: Record<string, StructuredTextBinding> = {
  // ST 模式
  st_worldbook_content: {
    resourceKind: 'st_worldbook_entry',
    fieldPath: 'content',
    allowedModes: ['plain', 'json', 'yaml'],
    defaultMode: 'plain',
    storageKind: 'string',
  },
  st_characterbook_content: {
    resourceKind: 'st_characterbook_entry',
    fieldPath: 'content',
    allowedModes: ['plain', 'json', 'yaml'],
    defaultMode: 'plain',
    storageKind: 'string',
  },
  st_preset_content: {
    resourceKind: 'st_preset',
    fieldPath: 'content',
    allowedModes: ['plain', 'json', 'yaml'],
    defaultMode: 'plain',
    storageKind: 'string',
  },
  st_regex_pattern: {
    resourceKind: 'st_regex_script',
    fieldPath: 'findRegex',
    allowedModes: ['plain'],
    defaultMode: 'plain',
    storageKind: 'string',
  },
  st_extensions: {
    resourceKind: 'generic_extensions',
    fieldPath: 'extensions',
    allowedModes: ['json'],
    defaultMode: 'json',
    storageKind: 'json_value',
    requiredValueShape: 'object',
  },
  // Agent 模式
  agent_knowledge_content: {
    resourceKind: 'agent_knowledge_entry',
    fieldPath: 'content',
    allowedModes: ['json', 'yaml'],
    defaultMode: 'json',
    storageKind: 'json_value',
  },
  agent_knowledge_apparent: {
    resourceKind: 'agent_knowledge_entry',
    fieldPath: 'apparent_content',
    allowedModes: ['json', 'yaml'],
    defaultMode: 'json',
    storageKind: 'json_value',
  },
  agent_knowledge_self_belief: {
    resourceKind: 'agent_knowledge_entry',
    fieldPath: 'self_belief',
    allowedModes: ['json', 'yaml'],
    defaultMode: 'json',
    storageKind: 'json_value',
  },
  agent_world_rules: {
    resourceKind: 'agent_world_rules',
    fieldPath: 'world_base.yaml',
    allowedModes: ['yaml'],
    defaultMode: 'yaml',
    storageKind: 'yaml_file',
  },
  agent_llm_readable: {
    resourceKind: 'agent_knowledge_entry',
    fieldPath: 'summary_text',
    allowedModes: ['plain'],
    defaultMode: 'plain',
    storageKind: 'string',
  },
};
