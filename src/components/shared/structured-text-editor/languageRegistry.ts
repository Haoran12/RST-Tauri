import type { Extension } from '@codemirror/state'
import { json } from '@codemirror/lang-json'
import { yaml as yamlLanguage } from '@codemirror/lang-yaml'
import type {
  StructuredTextBinding,
  StructuredTextLanguageId,
  StructuredTextLanguagePack,
} from '@/types/structuredText'

const registry = new Map<StructuredTextLanguageId, StructuredTextLanguagePack>()

function register(pack: StructuredTextLanguagePack) {
  registry.set(pack.languageId, pack)
}

register({
  languageId: 'plain',
  label: 'Plain',
  source: 'builtin',
  storageKinds: ['string'],
  load: async (): Promise<Extension[]> => [],
  supportsFormat: false,
  supportsLint: true,
  supportsAutoIndent: false,
})

register({
  languageId: 'json',
  label: 'JSON',
  source: 'builtin',
  storageKinds: ['string', 'json_value'],
  load: async (): Promise<Extension[]> => [json()],
  canParseToJsonValue: true,
  supportsFormat: true,
  supportsLint: true,
  supportsAutoIndent: true,
})

register({
  languageId: 'yaml',
  label: 'YAML',
  source: 'builtin',
  storageKinds: ['string', 'json_value', 'yaml_file'],
  load: async (): Promise<Extension[]> => [yamlLanguage()],
  canParseToJsonValue: true,
  supportsFormat: true,
  supportsLint: true,
  supportsAutoIndent: true,
})

export function getStructuredTextLanguagePack(
  languageId: StructuredTextLanguageId,
): StructuredTextLanguagePack | undefined {
  return registry.get(languageId)
}

export function listStructuredTextLanguagePacks(): StructuredTextLanguagePack[] {
  return Array.from(registry.values())
}

export function getStructuredTextLanguageOptions(binding: StructuredTextBinding) {
  return binding.allowedModes
    .map(mode => {
      const pack = registry.get(mode)
      if (!pack) {
        return null
      }
      return {
        label: pack.label,
        value: pack.languageId,
      }
    })
    .filter((option): option is { label: string; value: string } => option !== null)
}

export async function loadStructuredTextLanguageExtensions(
  languageId: StructuredTextLanguageId,
): Promise<Extension[]> {
  const pack = registry.get(languageId)
  if (!pack) {
    return []
  }

  return pack.load()
}

export function isStructuredTextModeAllowed(
  binding: StructuredTextBinding,
  languageId: StructuredTextLanguageId,
) {
  return binding.allowedModes.includes(languageId)
}
