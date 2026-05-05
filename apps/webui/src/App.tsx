import { useState, useEffect, useRef, useCallback, type MouseEvent } from 'react'
import CodeMirror from '@uiw/react-codemirror'
import { markdown } from '@codemirror/lang-markdown'
import { oneDark } from '@codemirror/theme-one-dark'
import { initWasm, renderSvg, renderHtml, checkSource } from './wasmLoader'
import type { Diagnostic } from './wasmLoader'
import { EXAMPLES } from './examples'
import './App.css'

const DEBOUNCE_MS = 500

function App() {
  const [source, setSource] = useState<string>(EXAMPLES[0].source)
  const [svgContent, setSvgContent] = useState<string>('')
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [wasmReady, setWasmReady] = useState(false)
  const [wasmError, setWasmError] = useState<string | null>(null)
  const [selectedExample, setSelectedExample] = useState<number>(0)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [tooltip, setTooltip] = useState<{ text: string; x: number; y: number } | null>(null)

  // Initialize WASM on mount
  useEffect(() => {
    initWasm()
      .then(() => setWasmReady(true))
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err)
        setWasmError(msg)
      })
  }, [])

  // Compile + check on source change (debounced)
  const compileAndCheck = useCallback(
    (src: string) => {
      if (!wasmReady) return
      const diags = checkSource(src)
      setDiagnostics(diags)

      const hasErrors = diags.some((d) => d.severity === 'error')
      if (!hasErrors) {
        try {
          const svg = renderSvg(src)
          setSvgContent(svg)
        } catch (e: unknown) {
          // SVG rendering failed — keep previous preview
          const msg = e instanceof Error ? e.message : String(e)
          setDiagnostics((prev) => [
            ...prev,
            { severity: 'error', message: msg, line: 0, col: 0 },
          ])
        }
      }
    },
    [wasmReady]
  )

  useEffect(() => {
    if (!wasmReady) return
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(() => {
      compileAndCheck(source)
    }, DEBOUNCE_MS)
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
    }
  }, [source, wasmReady, compileAndCheck])

  // Initial compile when WASM becomes ready
  useEffect(() => {
    if (wasmReady) {
      compileAndCheck(source)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wasmReady])

  function handleEditorChange(value: string) {
    setSource(value)
  }

  function handleExampleChange(e: React.ChangeEvent<HTMLSelectElement>) {
    const idx = parseInt(e.target.value, 10)
    setSelectedExample(idx)
    setSource(EXAMPLES[idx].source)
  }

  function downloadTdsl() {
    const blob = new Blob([source], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'timeline.tdsl'
    a.click()
    URL.revokeObjectURL(url)
  }

  function downloadSvg() {
    if (!svgContent) return
    const blob = new Blob([svgContent], { type: 'image/svg+xml' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'timeline.svg'
    a.click()
    URL.revokeObjectURL(url)
  }

  function downloadHtml() {
    if (!svgContent) return
    try {
      const html = renderHtml(source)
      const blob = new Blob([html], { type: 'text/html' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'timeline.html'
      a.click()
      URL.revokeObjectURL(url)
    } catch {
      // keep silent — errors are already shown in diagnostics
    }
  }

  function openFile() {
    fileInputRef.current?.click()
  }

  function handlePreviewMouseMove(e: MouseEvent<HTMLDivElement>) {
    const target = (e.target as Element).closest<HTMLElement>('[data-tdsl-tooltip]')
    if (target) {
      const text = target.dataset.tdslTooltip ?? ''
      setTooltip({ text, x: e.clientX, y: e.clientY })
    } else {
      setTooltip(null)
    }
  }

  function handlePreviewMouseLeave() {
    setTooltip(null)
  }

  function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = (ev) => {
      const text = ev.target?.result as string
      setSource(text)
    }
    reader.readAsText(file)
    // Reset so same file can be re-opened
    e.target.value = ''
  }

  const errorCount = diagnostics.filter((d) => d.severity === 'error').length
  const warnCount = diagnostics.filter((d) => d.severity === 'warning').length

  return (
    <div className="app">
      {/* Header / Toolbar */}
      <header className="toolbar">
        <div className="toolbar-left">
          <span className="app-title">Timeline DSL Editor</span>
          <select
            className="example-select"
            value={selectedExample}
            onChange={handleExampleChange}
          >
            {EXAMPLES.map((ex, i) => (
              <option key={i} value={i}>
                {ex.label}
              </option>
            ))}
          </select>
        </div>
        <div className="toolbar-right">
          <button className="btn" onClick={openFile} title=".tdsl ファイルを開く">
            ファイルを開く
          </button>
          <button className="btn" onClick={downloadTdsl} title=".tdsl をダウンロード">
            .tdsl 保存
          </button>
          <button
            className="btn"
            onClick={downloadSvg}
            disabled={!svgContent}
            title="SVG をダウンロード"
          >
            SVG 保存
          </button>
          <button
            className="btn"
            onClick={downloadHtml}
            disabled={!svgContent}
            title="スタンドアロン HTML をダウンロード"
          >
            HTML 保存
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".tdsl,text/plain"
            style={{ display: 'none' }}
            onChange={handleFileChange}
          />
        </div>
      </header>

      {/* WASM loading indicator */}
      {!wasmReady && !wasmError && (
        <div className="status-bar loading">WASM を初期化中...</div>
      )}
      {wasmError && (
        <div className="status-bar status-error">WASM 初期化エラー: {wasmError}</div>
      )}
      {wasmReady && (
        <div className="status-bar ready">
          {errorCount > 0 && <span className="badge badge-error">{errorCount} エラー</span>}
          {warnCount > 0 && <span className="badge badge-warn">{warnCount} 警告</span>}
          {errorCount === 0 && warnCount === 0 && (
            <span className="badge badge-ok">問題なし</span>
          )}
        </div>
      )}

      {/* Main: Editor + Preview */}
      <main className="main">
        <div className="editor-pane">
          <CodeMirror
            value={source}
            height="100%"
            theme={oneDark}
            extensions={[markdown()]}
            onChange={handleEditorChange}
            basicSetup={{
              lineNumbers: true,
              foldGutter: false,
              dropCursor: false,
              allowMultipleSelections: false,
              indentOnInput: true,
            }}
          />
        </div>
        <div
          className="preview-pane"
          onMouseMove={handlePreviewMouseMove}
          onMouseLeave={handlePreviewMouseLeave}
        >
          {svgContent ? (
            <div
              className="svg-container"
              dangerouslySetInnerHTML={{ __html: svgContent }}
            />
          ) : (
            <div className="preview-placeholder">
              {wasmReady ? 'プレビューなし（エラーを確認してください）' : '読み込み中...'}
            </div>
          )}
        </div>
      </main>

      {/* Diagnostics panel */}
      {diagnostics.length > 0 && (
        <aside className="diagnostics-panel">
          <div className="diagnostics-header">診断結果</div>
          <ul className="diagnostics-list">
            {diagnostics.map((d, i) => (
              <li key={i} className={`diagnostic-item ${d.severity}`}>
                <span className="diag-severity">
                  {d.severity === 'error' ? 'ERROR' : 'WARN'}
                </span>
                {d.line > 0 && (
                  <span className="diag-location">
                    {d.line}:{d.col}
                  </span>
                )}
                <span className="diag-message">{d.message}</span>
              </li>
            ))}
          </ul>
        </aside>
      )}
      {tooltip && (
        <div
          className="tdsl-tooltip"
          style={{ left: tooltip.x + 12, top: tooltip.y + 12 }}
        >
          {tooltip.text.split('\n').map((line, i) => (
            <div key={i}>{line}</div>
          ))}
        </div>
      )}
    </div>
  )
}

export default App
