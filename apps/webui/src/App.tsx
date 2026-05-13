import { useState, useEffect, useRef, useCallback, type CSSProperties, type MouseEvent } from 'react'
import CodeMirror from '@uiw/react-codemirror'
import { tdsl } from './lang-tdsl'
import { oneDark } from '@codemirror/theme-one-dark'
import { EditorView } from '@codemirror/view'
import { autocompletion, snippetCompletion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete'
import { bracketMatching } from '@codemirror/language'
import { search } from '@codemirror/search'
import { initWasm, renderSvg, renderHtml, checkSource } from './wasmLoader'
import type { Diagnostic } from './wasmLoader'
import { EXAMPLES } from './examples'
import './App.css'

const SVG_EMBEDDED_CSS = `
  .tdsl-lane-band-even { fill: #ffffff; }
  .tdsl-lane-band-odd  { fill: #f5f5f7; }
  .tdsl-axis-baseline  { stroke: #888888; stroke-width: 1; }
  .tdsl-axis-tick      { stroke: #e0e0e0; stroke-width: 1; }
  .tdsl-axis-text      { font-size: 11px; fill: #666666; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  .tdsl-lane-label     { font-size: 13px; fill: #333333; font-weight: 500; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  .tdsl-item-label     { font-size: 11px; fill: #ffffff; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  .tdsl-event-stem     { stroke: #666666; stroke-width: 1.5; }
  .tdsl-event-hit      { fill: transparent; }
  .tdsl-span           { fill-opacity: 0.78; }
  .tdsl-event-range    { fill-opacity: 0.75; }
  .tdsl-event-dot      { stroke: #ffffff; stroke-width: 1; }
`

function svgWithEmbeddedStyles(svg: string): string {
  return svg.replace('</style>', SVG_EMBEDDED_CSS + '</style>')
}

function svgToPngBlob(svg: string, whiteBg: boolean): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const enriched = svgWithEmbeddedStyles(svg)
    const blob = new Blob([enriched], { type: 'image/svg+xml;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const img = new Image()
    img.onload = () => {
      const canvas = document.createElement('canvas')
      canvas.width = img.naturalWidth || img.width || 800
      canvas.height = img.naturalHeight || img.height || 400
      const ctx = canvas.getContext('2d')!
      if (whiteBg) {
        ctx.fillStyle = '#ffffff'
        ctx.fillRect(0, 0, canvas.width, canvas.height)
      }
      ctx.drawImage(img, 0, 0)
      URL.revokeObjectURL(url)
      canvas.toBlob((b) => {
        if (b) resolve(b)
        else reject(new Error('canvas.toBlob failed'))
      }, 'image/png')
    }
    img.onerror = () => { URL.revokeObjectURL(url); reject(new Error('SVG load failed')) }
    img.src = url
  })
}

const DEBOUNCE_MS = 500

type ColorScheme = 'dark' | 'light'

type LegendItem = { lane: string; label: string; color: string }

type SelectedItem = {
  label: string
  type: string
  lane: string
  source: string
  tooltip: string
}

function extractLegend(container: Element): LegendItem[] {
  const colorMap = new Map<string, string>()
  container.querySelectorAll<Element>('[data-lane]').forEach((el) => {
    const lane = el.getAttribute('data-lane') || ''
    if (!colorMap.has(lane)) {
      const fillEl = el.querySelector('.tdsl-span, .tdsl-event-range, .tdsl-event-dot')
      const style = fillEl?.getAttribute('style') || ''
      const m = style.match(/fill:([^;]+)/)
      if (m) colorMap.set(lane, m[1].trim())
    }
  })
  const result: LegendItem[] = []
  container.querySelectorAll<Element>('.tdsl-lane-label[data-lane]').forEach((el) => {
    const lane = el.getAttribute('data-lane') || ''
    const label = el.textContent || lane
    result.push({ lane, label, color: colorMap.get(lane) || '#888' })
  })
  return result
}

// ─── TDSL keyword completions & snippets ─────────────────────────────────────

const TDSL_SNIPPETS = [
  snippetCompletion('timeline "${1:タイトル}" {\n  unit: year\n  range: ${2:1900} to ${3:2000}\n\n  ${0}\n}', {
    label: 'timeline', detail: '年表ブロック', type: 'keyword', boost: 10,
  }),
  snippetCompletion('lane "${1:レーン名}" as ${2:id}', {
    label: 'lane', detail: 'レーン定義', type: 'keyword', boost: 9,
  }),
  snippetCompletion('span "${1:名前}" {\n  lane: ${2:id}\n  start: ${3:1900}\n  end: ${4:1950}\n}', {
    label: 'span', detail: 'スパン', type: 'keyword', boost: 8,
  }),
  snippetCompletion('event "${1:名前}" {\n  lane: ${2:id}\n  at: ${3:1900}\n}', {
    label: 'event', detail: 'イベント', type: 'keyword', boost: 8,
  }),
  snippetCompletion('event_range "${1:名前}" {\n  lane: ${2:id}\n  start: ${3:1900}\n  end: ${4:1950}\n}', {
    label: 'event_range', detail: 'イベント範囲', type: 'keyword', boost: 7,
  }),
  snippetCompletion('import "${1:Q12345}" as ${2:alias} {\n  lang: ja, en\n}', {
    label: 'import', detail: 'Wikidataインポート', type: 'keyword', boost: 7,
  }),
  snippetCompletion('map ${1:alias} {\n  target_type: span\n  lane: ${2:id}\n  label: wd.${3:label}\n  start: wd.${4:start}\n  end: wd.${5:end}\n}', {
    label: 'map', detail: 'マッピング', type: 'keyword', boost: 6,
  }),
  snippetCompletion('query "${1:SPARQL}" as ${2:alias}', {
    label: 'query', detail: 'SPARQLクエリ', type: 'keyword', boost: 5,
  }),
  snippetCompletion('color_map {\n  "${1:タグ}": "${2:#4682B4}"\n}', {
    label: 'color_map', detail: 'タグ→色マッピング', type: 'keyword', boost: 5,
  }),
]

const STATIC_KEYWORDS = [
  { label: 'unit', type: 'keyword' as const },
  { label: 'range', type: 'keyword' as const },
  { label: 'calendar', type: 'keyword' as const },
  { label: 'meta', type: 'keyword' as const },
  { label: 'target_type', type: 'keyword' as const },
  { label: 'year', type: 'keyword' as const },
  { label: 'lang', type: 'keyword' as const },
  { label: 'label', type: 'property' as const },
  { label: 'start', type: 'property' as const },
  { label: 'end', type: 'property' as const },
  { label: 'at', type: 'property' as const },
  { label: 'source', type: 'property' as const },
  { label: 'tags', type: 'property' as const },
  { label: 'order', type: 'property' as const },
]

function makeTdslCompletionSource(getSource: () => string) {
  return function tdslCompletions(context: CompletionContext): CompletionResult | null {
    const word = context.matchBefore(/[\w.]+/)
    if (!word || (word.from === word.to && !context.explicit)) return null
    const src = getSource()
    const laneIds = [...src.matchAll(/\blane\s+"[^"]*"\s+as\s+(\w+)/g)].map((m) => ({
      label: m[1], type: 'variable' as const, detail: 'lane id',
    }))
    const importAliases = [...src.matchAll(/\bimport\s+"[^"]*"\s+as\s+(\w+)/g)].map((m) => ({
      label: m[1], type: 'variable' as const, detail: 'import alias',
    }))
    return {
      from: word.from,
      options: [...TDSL_SNIPPETS, ...STATIC_KEYWORDS, ...laneIds, ...importAliases],
    }
  }
}

// ─── Keyboard shortcut reference ─────────────────────────────────────────────

const SHORTCUTS = [
  { key: 'Ctrl/Cmd + F', desc: '検索・置換パネルを開く' },
  { key: 'Escape', desc: '検索パネルを閉じる' },
  { key: 'Ctrl/Cmd + Enter', desc: '次の候補へ' },
  { key: 'Ctrl/Cmd + G', desc: '次の一致へ' },
  { key: 'Ctrl/Cmd + Z', desc: '元に戻す' },
  { key: 'Ctrl/Cmd + Shift + Z', desc: 'やり直す' },
  { key: 'Tab / Space', desc: 'スニペット候補を選択' },
  { key: 'Ctrl/Cmd + Space', desc: '補完候補を表示' },
  { key: '? (エディタ外)', desc: 'ショートカット一覧を開く' },
]

type MobileTab = 'editor' | 'preview'

function App() {
  const [source, setSource] = useState<string>(EXAMPLES[0].source)
  const [svgContent, setSvgContent] = useState<string>('')
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [wasmReady, setWasmReady] = useState(false)
  const [wasmError, setWasmError] = useState<string | null>(null)
  const [selectedExample, setSelectedExample] = useState<number>(0)
  const [fontSize, setFontSize] = useState<number>(14)
  const [colorScheme, setColorScheme] = useState<ColorScheme>('dark')
  const [mobileTab, setMobileTab] = useState<MobileTab>('editor')
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const editorViewRef = useRef<EditorView | null>(null)
  const [tooltip, setTooltip] = useState<{ text: string; x: number; y: number } | null>(null)
  const [scale, setScale] = useState<number>(0) // 0 = Auto
  const [exportMenuOpen, setExportMenuOpen] = useState(false)
  const [copyFeedback, setCopyFeedback] = useState<string | null>(null)
  const exportMenuRef = useRef<HTMLDivElement>(null)
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [lineWrap, setLineWrap] = useState(false)
  const [isStalePreview, setIsStalePreview] = useState(false)

  // Split pane ratio (editor / preview)
  const [splitRatio, setSplitRatio] = useState(0.4)
  const splitDragRef = useRef<{ startX: number; startRatio: number; containerWidth: number } | null>(null)
  const mainRef = useRef<HTMLElement>(null)

  // Pan/zoom (direct DOM manipulation avoids React re-renders during drag)
  const panZoomRef = useRef({ x: 0, y: 0, s: 1 })
  const [cursorGrab, setCursorGrab] = useState(false)
  const previewRef = useRef<HTMLDivElement>(null)
  const svgContainerRef = useRef<HTMLDivElement>(null)
  const dragRef = useRef<{ mx: number; my: number; px: number; py: number } | null>(null)
  const didDragRef = useRef(false)

  // Legend & detail panel
  const [showLegend, setShowLegend] = useState(false)
  const [legendItems, setLegendItems] = useState<LegendItem[]>([])
  const [selectedItem, setSelectedItem] = useState<SelectedItem | null>(null)

  function applyTransform(t: { x: number; y: number; s: number }) {
    panZoomRef.current = t
    if (svgContainerRef.current) {
      svgContainerRef.current.style.transform = `translate(${t.x}px, ${t.y}px) scale(${t.s})`
    }
  }

  function resetPanZoom() {
    applyTransform({ x: 0, y: 0, s: 1 })
  }

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
          const svg = renderSvg(src, scale)
          setSvgContent(svg)
          setIsStalePreview(false)
        } catch (e: unknown) {
          const msg = e instanceof Error ? e.message : String(e)
          setDiagnostics((prev) => [
            ...prev,
            { severity: 'error', message: msg, line: 0, col: 0 },
          ])
          setIsStalePreview(true)
        }
      } else {
        setIsStalePreview(true)
      }
    },
    [wasmReady, scale]
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
  }, [source, wasmReady, scale, compileAndCheck])

  // Initial compile when WASM becomes ready
  useEffect(() => {
    if (wasmReady) {
      compileAndCheck(source)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wasmReady])

  // Close export menu on outside click
  useEffect(() => {
    if (!exportMenuOpen) return
    function onOutside(e: globalThis.MouseEvent) {
      if (exportMenuRef.current && !exportMenuRef.current.contains(e.target as Node)) {
        setExportMenuOpen(false)
      }
    }
    document.addEventListener('mousedown', onOutside)
    return () => document.removeEventListener('mousedown', onOutside)
  }, [exportMenuOpen])

  function showCopyFeedback(msg: string) {
    setCopyFeedback(msg)
    setTimeout(() => setCopyFeedback(null), 2000)
  }

  // Wheel zoom (passive:false required to call preventDefault)
  useEffect(() => {
    const preview = previewRef.current
    if (!preview) return
    function onWheel(e: WheelEvent) {
      e.preventDefault()
      const rect = preview!.getBoundingClientRect()
      const cx = e.clientX - rect.left
      const cy = e.clientY - rect.top
      const factor = e.deltaY < 0 ? 1.1 : 0.9
      const pz = panZoomRef.current
      const newS = Math.max(0.1, Math.min(10, pz.s * factor))
      const ratio = newS / pz.s
      applyTransform({ s: newS, x: cx - (cx - pz.x) * ratio, y: cy - (cy - pz.y) * ratio })
    }
    preview.addEventListener('wheel', onWheel, { passive: false })
    return () => preview.removeEventListener('wheel', onWheel)
  }, [])

  // Extract legend after SVG renders into DOM
  useEffect(() => {
    if (!svgContent) { setLegendItems([]); return }
    requestAnimationFrame(() => {
      if (svgContainerRef.current) {
        setLegendItems(extractLegend(svgContainerRef.current))
      }
    })
  }, [svgContent])

  // Split pane drag (document-level to prevent losing track when cursor leaves divider)
  useEffect(() => {
    function onMouseMove(e: globalThis.MouseEvent) {
      if (!splitDragRef.current) return
      const dx = e.clientX - splitDragRef.current.startX
      const newRatio = Math.max(0.15, Math.min(0.85, splitDragRef.current.startRatio + dx / splitDragRef.current.containerWidth))
      setSplitRatio(newRatio)
    }
    function onMouseUp() {
      if (!splitDragRef.current) return
      splitDragRef.current = null
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
    }
    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
    return () => {
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }
  }, [])

  function handleDividerMouseDown(e: MouseEvent<HTMLDivElement>) {
    e.preventDefault()
    const containerWidth = mainRef.current?.clientWidth ?? document.documentElement.clientWidth
    splitDragRef.current = { startX: e.clientX, startRatio: splitRatio, containerWidth }
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
  }

  // Global `?` key to toggle shortcut modal (only when editor doesn't have focus)
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === '?' && !(e.target instanceof HTMLTextAreaElement) && !editorViewRef.current?.hasFocus) {
        e.preventDefault()
        setShowShortcuts((v) => !v)
      }
      if (e.key === 'Escape') setShowShortcuts(false)
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [])
  function handleEditorChange(value: string) {
    setSource(value)
  }

  function handleExampleChange(e: React.ChangeEvent<HTMLSelectElement>) {
    const idx = parseInt(e.target.value, 10)
    setSelectedExample(idx)
    setSource(EXAMPLES[idx].source)
  }

  function handleFontSizeChange(e: React.ChangeEvent<HTMLSelectElement>) {
    setFontSize(parseInt(e.target.value, 10))
  }

  function toggleColorScheme() {
    setColorScheme((prev) => (prev === 'dark' ? 'light' : 'dark'))
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

  function downloadPng(whiteBg: boolean = true) {
    if (!svgContent) return
    svgToPngBlob(svgContent, whiteBg).then((blob) => {
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'timeline.png'
      a.click()
      URL.revokeObjectURL(url)
    }).catch(() => {/* silently ignore */})
  }

  function copySvg() {
    if (!svgContent) return
    navigator.clipboard.writeText(svgContent)
      .then(() => showCopyFeedback('SVG をコピーしました'))
      .catch(() => {/* silently ignore */})
  }

  function copyPng() {
    if (!svgContent) return
    svgToPngBlob(svgContent, true).then((blob) => {
      return navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })])
    }).then(() => showCopyFeedback('PNG をコピーしました'))
      .catch(() => {/* silently ignore */})
  }

  function copyMarkdown() {
    const md = '```tdsl\n' + source + '\n```'
    navigator.clipboard.writeText(md)
      .then(() => showCopyFeedback('Markdown をコピーしました'))
      .catch(() => {/* silently ignore */})
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

  // Preview mouse handlers (drag pan + tooltip + item selection)
  function handlePreviewMouseDown(e: MouseEvent<HTMLDivElement>) {
    if (e.button !== 0) return
    const pz = panZoomRef.current
    dragRef.current = { mx: e.clientX, my: e.clientY, px: pz.x, py: pz.y }
    didDragRef.current = false
    setCursorGrab(true)
  }

  function handlePreviewMouseMove(e: MouseEvent<HTMLDivElement>) {
    if (dragRef.current) {
      const dx = e.clientX - dragRef.current.mx
      const dy = e.clientY - dragRef.current.my
      if (Math.abs(dx) > 3 || Math.abs(dy) > 3) didDragRef.current = true
      const pz = panZoomRef.current
      applyTransform({ s: pz.s, x: dragRef.current.px + dx, y: dragRef.current.py + dy })
      setTooltip(null)
      return
    }
    const target = (e.target as Element).closest<HTMLElement>('[data-tdsl-tooltip]')
    if (target) {
      const text = target.dataset.tdslTooltip ?? ''
      setTooltip({ text, x: e.clientX, y: e.clientY })
    } else {
      setTooltip(null)
    }
  }

  function handlePreviewMouseUp() {
    dragRef.current = null
    setCursorGrab(false)
  }

  function handlePreviewMouseLeave() {
    dragRef.current = null
    setCursorGrab(false)
    setTooltip(null)
  }

  function handlePreviewDblClick() {
    resetPanZoom()
  }

  function handlePreviewClick(e: MouseEvent<HTMLDivElement>) {
    if (didDragRef.current) { didDragRef.current = false; return }
    const target = (e.target as Element).closest<HTMLElement>('[data-label]')
    if (target) {
      setSelectedItem({
        label: target.dataset.label || '',
        type: target.dataset.type || '',
        lane: target.dataset.lane || '',
        source: target.dataset.source || '',
        tooltip: target.dataset.tdslTooltip || '',
      })
    } else {
      setSelectedItem(null)
    }
  }

  function handleDiagClick(diag: Diagnostic) {
    const view = editorViewRef.current
    if (!view || diag.line <= 0) return
    try {
      const lineInfo = view.state.doc.line(diag.line)
      const pos = lineInfo.from + Math.max(0, diag.col - 1)
      view.dispatch({ selection: { anchor: pos }, scrollIntoView: true })
      view.focus()
    } catch {
      // line out of range — ignore
    }
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

  const appStyle: CSSProperties = {
    '--editor-font-size': `${fontSize}px`,
  } as CSSProperties

  return (
    <div className="app" data-theme={colorScheme} style={appStyle}>
      {/* Header / Toolbar */}
      <header className="toolbar">
        <div className="toolbar-left">
          <span className="app-title">Timeline DSL Editor</span>
          <select
            className="toolbar-select"
            value={selectedExample}
            onChange={handleExampleChange}
          >
            {EXAMPLES.map((ex, i) => (
              <option key={i} value={i}>
                {ex.label}
              </option>
            ))}
          </select>
          <div className="toolbar-divider" />
          <select
            className="toolbar-select"
            value={fontSize}
            onChange={handleFontSizeChange}
            title="フォントサイズ"
          >
            <option value={12}>12px</option>
            <option value={13}>13px</option>
            <option value={14}>14px</option>
            <option value={16}>16px</option>
            <option value={18}>18px</option>
          </select>
          <button
            className="btn btn-theme"
            onClick={toggleColorScheme}
            title={colorScheme === 'dark' ? 'ライトモードに切替' : 'ダークモードに切替'}
          >
            {colorScheme === 'dark' ? 'ライト' : 'ダーク'}
          </button>
          <button
            className={`btn${lineWrap ? ' btn-active' : ''}`}
            onClick={() => setLineWrap((v) => !v)}
            title="行折り返しの切替"
          >
            折り返し
          </button>
          <button
            className="btn"
            onClick={() => setShowShortcuts(true)}
            title="キーボードショートカット一覧 (?)"
          >
            ?
          </button>
        </div>
        <div className="toolbar-right">
          <a
            className="btn"
            href="https://timeline-dsl-lp.pages.dev/"
            target="_blank"
            rel="noopener noreferrer"
            title="ランディングページ・ドキュメント"
          >
            About
          </a>
          <a
            className="btn"
            href="https://github.com/keroway/timeline-dsl"
            target="_blank"
            rel="noopener noreferrer"
            title="GitHub リポジトリ"
          >
            GitHub
          </a>
          <select
            className="scale-select"
            value={scale}
            onChange={(e) => setScale(Number(e.target.value))}
            title="プレビューのスケール（ピクセル/年）"
          >
            <option value={0}>スケール: Auto</option>
            <option value={0.5}>0.5×</option>
            <option value={1}>1×</option>
            <option value={2}>2×</option>
            <option value={4}>4×</option>
            <option value={8}>8×</option>
          </select>
          <button className="btn" onClick={openFile} title=".tdsl ファイルを開く">
            ファイルを開く
          </button>
          <div className="export-menu-wrapper" ref={exportMenuRef}>
            <button
              className="btn"
              onClick={() => setExportMenuOpen((v) => !v)}
              title="エクスポート"
            >
              エクスポート ▾
            </button>
            {exportMenuOpen && (
              <div className="export-menu">
                <div className="export-menu-section">ダウンロード</div>
                <button className="export-menu-item" onClick={() => { downloadTdsl(); setExportMenuOpen(false) }}>
                  .tdsl 保存
                </button>
                <button className="export-menu-item" onClick={() => { downloadSvg(); setExportMenuOpen(false) }} disabled={!svgContent}>
                  SVG 保存
                </button>
                <button className="export-menu-item" onClick={() => { downloadHtml(); setExportMenuOpen(false) }} disabled={!svgContent}>
                  HTML 保存
                </button>
                <button className="export-menu-item" onClick={() => { downloadPng(true); setExportMenuOpen(false) }} disabled={!svgContent}>
                  PNG 保存（白背景）
                </button>
                <button className="export-menu-item" onClick={() => { downloadPng(false); setExportMenuOpen(false) }} disabled={!svgContent}>
                  PNG 保存（透過）
                </button>
                <div className="export-menu-section">クリップボードへコピー</div>
                <button className="export-menu-item" onClick={() => { copySvg(); setExportMenuOpen(false) }} disabled={!svgContent}>
                  SVG をコピー
                </button>
                <button className="export-menu-item" onClick={() => { copyPng(); setExportMenuOpen(false) }} disabled={!svgContent}>
                  PNG をコピー
                </button>
                <button className="export-menu-item" onClick={() => { copyMarkdown(); setExportMenuOpen(false) }}>
                  Markdown をコピー
                </button>
              </div>
            )}
          </div>
          {copyFeedback && (
            <span className="copy-feedback">{copyFeedback}</span>
          )}
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

      {/* Mobile tab bar */}
      <div className="mobile-tab-bar">
        <button
          className={`mobile-tab${mobileTab === 'editor' ? ' mobile-tab-active' : ''}`}
          onClick={() => setMobileTab('editor')}
        >
          エディタ
        </button>
        <button
          className={`mobile-tab${mobileTab === 'preview' ? ' mobile-tab-active' : ''}`}
          onClick={() => setMobileTab('preview')}
        >
          プレビュー
        </button>
      </div>

      {/* Main: Editor + Preview */}
      <main className="main" ref={mainRef}>
        <div
          className={`editor-pane${mobileTab !== 'editor' ? ' mobile-hidden' : ''}`}
          style={{ flex: `0 0 ${splitRatio * 100}%` }}
        >
          <CodeMirror
            value={source}
            height="100%"
            theme={colorScheme === 'dark' ? oneDark : 'light'}
            extensions={[
              tdsl(),
              search({ top: true }),
              bracketMatching(),
              autocompletion({ override: [makeTdslCompletionSource(() => source)] }),
              ...(lineWrap ? [EditorView.lineWrapping] : []),
            ]}
            onChange={handleEditorChange}
            onCreateEditor={(view) => { editorViewRef.current = view }}
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
          className="split-divider"
          onMouseDown={handleDividerMouseDown}
          title="ドラッグして分割幅を調整"
        />
        <div className={`preview-area${mobileTab !== 'preview' ? ' mobile-hidden' : ''}`}>
          {/* Preview controls overlay */}
          {svgContent && (
            <div className="preview-controls">
              <button
                className="btn btn-preview-ctrl"
                onClick={resetPanZoom}
                title="ビューをリセット（ダブルクリックでも可）"
              >
                リセット
              </button>
              <button
                className="btn btn-preview-ctrl"
                onClick={() => setShowLegend((v) => !v)}
                title="凡例を表示/非表示"
              >
                {showLegend ? '凡例 ✕' : '凡例'}
              </button>
            </div>
          )}
          {/* Legend panel */}
          {showLegend && legendItems.length > 0 && (
            <div className="legend-panel">
              <div className="legend-header">凡例</div>
              {legendItems.map((item) => (
                <div key={item.lane} className="legend-item">
                  <span className="legend-swatch" style={{ background: item.color }} />
                  <span className="legend-label">{item.label}</span>
                </div>
              ))}
            </div>
          )}
          {/* Selected item detail panel */}
          {selectedItem && (
            <div className="detail-panel">
              <div className="detail-header">
                <span>詳細</span>
                <button className="detail-close" onClick={() => setSelectedItem(null)}>✕</button>
              </div>
              <dl className="detail-list">
                <dt>名前</dt><dd>{selectedItem.label || '—'}</dd>
                <dt>種類</dt><dd>{selectedItem.type || '—'}</dd>
                <dt>レーン</dt><dd>{selectedItem.lane || '—'}</dd>
                {selectedItem.source && <><dt>出典</dt><dd>{selectedItem.source}</dd></>}
                {selectedItem.tooltip && (
                  <><dt>情報</dt><dd className="detail-tooltip">{selectedItem.tooltip}</dd></>
                )}
              </dl>
            </div>
          )}
          <div
            ref={previewRef}
            className={`preview-pane${cursorGrab ? ' grabbing' : ''}`}
            onMouseDown={handlePreviewMouseDown}
            onMouseMove={handlePreviewMouseMove}
            onMouseUp={handlePreviewMouseUp}
            onMouseLeave={handlePreviewMouseLeave}
            onDoubleClick={handlePreviewDblClick}
            onClick={handlePreviewClick}
          >
            {svgContent ? (
              <>
                {isStalePreview && (
                  <div className="stale-preview-badge">直前の成功時プレビューを表示中</div>
                )}
                <div
                  ref={svgContainerRef}
                  className="svg-container"
                  dangerouslySetInnerHTML={{ __html: svgContent }}
                />
              </>
            ) : (
              <div className="preview-placeholder">
                {wasmReady ? 'プレビューなし（エラーを確認してください）' : '読み込み中...'}
              </div>
            )}
          </div>
        </div>
      </main>

      {/* Diagnostics panel */}
      {diagnostics.length > 0 && (
        <aside className="diagnostics-panel">
          <div className="diagnostics-header">診断結果</div>
          <ul className="diagnostics-list">
            {diagnostics.map((d, i) => (
              <li
                key={i}
                className={`diagnostic-item ${d.severity}${d.line > 0 ? ' clickable' : ''}`}
                onClick={() => handleDiagClick(d)}
                role={d.line > 0 ? 'button' : undefined}
                tabIndex={d.line > 0 ? 0 : undefined}
                onKeyDown={d.line > 0 ? (e) => e.key === 'Enter' && handleDiagClick(d) : undefined}
              >
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
      {/* Keyboard shortcuts modal */}
      {showShortcuts && (
        <div className="modal-overlay" onClick={() => setShowShortcuts(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <span>キーボードショートカット</span>
              <button className="modal-close" onClick={() => setShowShortcuts(false)}>✕</button>
            </div>
            <table className="shortcuts-table">
              <tbody>
                {SHORTCUTS.map(({ key, desc }) => (
                  <tr key={key}>
                    <td><kbd className="kbd">{key}</kbd></td>
                    <td>{desc}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  )
}

export default App
