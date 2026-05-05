import { autocompletion } from '@codemirror/autocomplete'
import { history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { bracketMatching, defaultHighlightStyle, indentOnInput, syntaxHighlighting } from '@codemirror/language'
import { lintGutter, linter, type Diagnostic } from '@codemirror/lint'
import { searchKeymap } from '@codemirror/search'
import { Compartment, EditorState, type Extension } from '@codemirror/state'
import {
  EditorView,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from '@codemirror/view'
import { defaultKeymap } from '@codemirror/commands'
import type { StructuredTextDiagnostic } from '@/types/structuredText'

export interface StructuredTextEditorController {
  view: EditorView
  languageCompartment: Compartment
  lintCompartment: Compartment
  readOnlyCompartment: Compartment
  updateDoc: (text: string) => void
  reconfigureLanguage: (extensions: Extension[]) => void
  reconfigureLinter: (diagnostics: () => StructuredTextDiagnostic[]) => void
  setReadOnly: (value: boolean) => void
  destroy: () => void
}

interface CreateStructuredTextEditorOptions {
  parent: HTMLElement
  doc: string
  languageExtensions: Extension[]
  readOnly?: boolean
  minHeight?: number
  onDocChange: (text: string) => void
  onBlur?: () => void
  diagnosticsProvider: () => StructuredTextDiagnostic[]
}

export function createStructuredTextEditor(
  options: CreateStructuredTextEditorOptions,
): StructuredTextEditorController {
  const languageCompartment = new Compartment()
  const lintCompartment = new Compartment()
  const readOnlyCompartment = new Compartment()

  const theme = EditorView.theme({
    '&': {
      height: '100%',
      minHeight: `${options.minHeight ?? 220}px`,
      backgroundColor: 'var(--color-bg-surface)',
      color: 'var(--color-text-primary)',
      borderRadius: '10px',
      border: '1px solid var(--color-border-subtle)',
      overflow: 'hidden',
    },
    '.cm-scroller': {
      fontFamily: 'var(--font-mono)',
      lineHeight: '1.55',
      overflow: 'auto',
      scrollbarWidth: 'thin',
    },
    '.cm-scroller::-webkit-scrollbar': {
      width: '8px',
      height: '8px',
    },
    '.cm-scroller::-webkit-scrollbar-track': {
      background: 'rgba(0, 0, 0, 0.05)',
      borderRadius: '4px',
    },
    '.cm-scroller::-webkit-scrollbar-thumb': {
      background: 'rgba(128, 128, 128, 0.5)',
      borderRadius: '4px',
      minHeight: '30px',
      minWidth: '30px',
    },
    '.cm-scroller::-webkit-scrollbar-thumb:hover': {
      background: 'rgba(128, 128, 128, 0.7)',
    },
    '.cm-scroller::-webkit-scrollbar-corner': {
      background: 'transparent',
    },
    '.cm-content': {
      padding: '12px 0',
      caretColor: 'var(--color-status-info)',
    },
    '.cm-gutters': {
      backgroundColor: 'var(--color-bg-subtle)',
      borderRight: '1px solid var(--color-border-subtle)',
      color: 'var(--color-text-secondary)',
    },
    '.cm-activeLine': {
      backgroundColor: 'rgba(32, 128, 240, 0.06)',
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'rgba(32, 128, 240, 0.06)',
    },
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
      backgroundColor: 'rgba(32, 128, 240, 0.2)',
    },
    '&.cm-focused': {
      outline: '2px solid rgba(32, 128, 240, 0.18)',
      outlineOffset: '0',
      borderColor: 'rgba(32, 128, 240, 0.45)',
    },
    '.cm-tooltip': {
      border: '1px solid var(--color-border-subtle)',
      backgroundColor: 'var(--color-bg-surface)',
      color: 'var(--color-text-primary)',
    },
  })

  const lintExtension = linter(view =>
    toCmDiagnostics(view.state.doc.toString(), options.diagnosticsProvider()),
  )

  const state = EditorState.create({
    doc: options.doc,
    extensions: [
      lineNumbers(),
      history(),
      drawSelection(),
      dropCursor(),
      bracketMatching(),
      indentOnInput(),
      autocompletion(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      lintGutter(),
      theme,
      EditorView.lineWrapping,
      EditorState.tabSize.of(2),
      keymap.of([
        indentWithTab,
        ...defaultKeymap,
        ...historyKeymap,
        ...searchKeymap,
      ]),
      languageCompartment.of(options.languageExtensions),
      lintCompartment.of(lintExtension),
      readOnlyCompartment.of(EditorState.readOnly.of(Boolean(options.readOnly))),
      EditorView.updateListener.of(update => {
        if (update.docChanged) {
          options.onDocChange(update.state.doc.toString())
        }
      }),
    ],
  })

  const view = new EditorView({
    state,
    parent: options.parent,
  })

  if (options.onBlur) {
    view.dom.addEventListener('focusout', options.onBlur)
  }

  return {
    view,
    languageCompartment,
    lintCompartment,
    readOnlyCompartment,
    updateDoc(text: string) {
      const current = view.state.doc.toString()
      if (current === text) {
        return
      }

      view.dispatch({
        changes: {
          from: 0,
          to: current.length,
          insert: text,
        },
      })
    },
    reconfigureLanguage(extensions: Extension[]) {
      view.dispatch({
        effects: languageCompartment.reconfigure(extensions),
      })
    },
    reconfigureLinter(diagnosticsProvider: () => StructuredTextDiagnostic[]) {
      view.dispatch({
        effects: lintCompartment.reconfigure(
          linter(cmView =>
            toCmDiagnostics(cmView.state.doc.toString(), diagnosticsProvider()),
          ),
        ),
      })
    },
    setReadOnly(value: boolean) {
      view.dispatch({
        effects: readOnlyCompartment.reconfigure(EditorState.readOnly.of(value)),
      })
    },
    destroy() {
      if (options.onBlur) {
        view.dom.removeEventListener('focusout', options.onBlur)
      }
      view.destroy()
    },
  }
}

function toCmDiagnostics(
  text: string,
  diagnostics: StructuredTextDiagnostic[],
): Diagnostic[] {
  return diagnostics.map(item => {
    const from = positionToOffset(text, item.line, item.column)
    const to = Math.max(from + (item.length ?? 1), from + 1)
    return {
      from,
      to,
      severity: item.severity === 'blocker' ? 'error' : item.severity,
      message: item.message,
    }
  })
}

function positionToOffset(text: string, line: number, column: number) {
  if (line <= 1) {
    return Math.max(0, column - 1)
  }

  const lines = text.split(/\r?\n/)
  let offset = 0
  for (let index = 0; index < Math.min(line - 1, lines.length); index += 1) {
    if (index === line - 1) {
      break
    }
    offset += lines[index].length + 1
  }

  return offset + Math.max(0, column - 1)
}
