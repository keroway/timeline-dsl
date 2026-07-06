import { useMemo } from 'react'
import CodeMirror from '@uiw/react-codemirror'
import { oneDark } from '@codemirror/theme-one-dark'
import { EditorView } from '@codemirror/view'
import { autocompletion } from '@codemirror/autocomplete'
import { bracketMatching } from '@codemirror/language'
import { search } from '@codemirror/search'
import { lintGutter } from '@codemirror/lint'
import type { Extension } from '@codemirror/state'
import { tdsl } from '../lang-tdsl'
import { tdslHover } from '../lang-tdsl/hover'
import { makeTdslCompletionSource } from '../editor/completions'
import { lineHighlightField } from '../editor/extensions'
import type { ColorScheme } from '../lib/settings'
import { createTranslator, type Locale } from '../lib/i18n'

type EditorPaneProps = {
  source: string
  colorScheme: ColorScheme
  lineWrap: boolean
  splitRatio: number
  hidden: boolean
  fullscreen: boolean
  cursorLineExtension: Extension
  tdslLinterExtension: Extension
  onChange: (value: string) => void
  onCreateEditor: (view: EditorView) => void
  locale: Locale
}

export function EditorPane(props: EditorPaneProps) {
  const {
    source,
    colorScheme,
    lineWrap,
    splitRatio,
    hidden,
    fullscreen,
    cursorLineExtension,
    tdslLinterExtension,
    onChange,
    onCreateEditor,
    locale,
  } = props

  const t = useMemo(() => createTranslator(locale), [locale])

  return (
    <div
      className={`editor-pane${hidden ? ' mobile-hidden' : ''}`}
      style={{ flex: `0 0 ${splitRatio * 100}%`, ...(fullscreen ? { display: 'none' } : {}) }}
    >
      <CodeMirror
        value={source}
        height="100%"
        theme={colorScheme === 'dark' ? oneDark : 'light'}
        extensions={[
          tdsl(),
          search({ top: true }),
          bracketMatching(),
          autocompletion({ override: [makeTdslCompletionSource(() => source, t)] }),
          tdslHover(() => source),
          lineHighlightField,
          cursorLineExtension,
          tdslLinterExtension,
          lintGutter(),
          ...(lineWrap ? [EditorView.lineWrapping] : []),
        ]}
        onChange={onChange}
        onCreateEditor={(view) => { onCreateEditor(view) }}
        basicSetup={{
          lineNumbers: true,
          foldGutter: false,
          dropCursor: false,
          allowMultipleSelections: false,
          indentOnInput: true,
        }}
      />
    </div>
  )
}
