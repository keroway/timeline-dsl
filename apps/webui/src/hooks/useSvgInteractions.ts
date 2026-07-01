import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type KeyboardEvent,
  type MouseEvent,
  type RefObject,
  type SetStateAction,
} from 'react'
import { EditorView } from '@codemirror/view'
import {
  type FilterState,
  type LegendItem,
  type SelectedItem,
  FILTER_STATE_KEY,
  extractLegend,
  extractTags,
  loadFilterState,
} from '../lib/svgDom'
import { makeCursorLineExtension, setLineHighlight } from '../editor/extensions'

export type SvgInteractionsApi = {
  previewRef: RefObject<HTMLDivElement | null>
  svgContainerRef: RefObject<HTMLDivElement | null>
  cursorGrab: boolean
  resetPanZoom: () => void
  handlePreviewMouseDown: (e: MouseEvent<HTMLDivElement>) => void
  handlePreviewMouseMove: (e: MouseEvent<HTMLDivElement>) => void
  handlePreviewMouseUp: () => void
  handlePreviewMouseLeave: () => void
  handlePreviewDblClick: () => void
  handlePreviewClick: (e: MouseEvent<HTMLDivElement>) => void
  handlePreviewKeyDown: (e: KeyboardEvent<HTMLDivElement>) => void
  tooltip: { text: string; x: number; y: number } | null
  legendItems: LegendItem[]
  allTags: string[]
  showLegend: boolean
  setShowLegend: Dispatch<SetStateAction<boolean>>
  showFilterPanel: boolean
  setShowFilterPanel: Dispatch<SetStateAction<boolean>>
  filterState: FilterState
  setFilterState: Dispatch<SetStateAction<FilterState>>
  selectedItem: SelectedItem | null
  setSelectedItem: Dispatch<SetStateAction<SelectedItem | null>>
  // CodeMirror 用: カーソル行に対応する SVG 要素を強調する extension
  cursorLineExtension: ReturnType<typeof makeCursorLineExtension>
}

// プレビュー SVG に対する全インタラクションを所有する:
// パン/ズーム・ツールチップ・凡例/タグ抽出・レーン/タグフィルタ・
// アイテム選択・双方向カーソルジャンプ（プレビュー↔エディタ）。
export function useSvgInteractions(
  svgContent: string,
  editorViewRef: RefObject<EditorView | null>,
): SvgInteractionsApi {
  const previewRef = useRef<HTMLDivElement>(null)
  const svgContainerRef = useRef<HTMLDivElement>(null)
  const [tooltip, setTooltip] = useState<{ text: string; x: number; y: number } | null>(null)

  // Pan/zoom (direct DOM manipulation avoids React re-renders during drag)
  const panZoomRef = useRef({ x: 0, y: 0, s: 1 })
  const [cursorGrab, setCursorGrab] = useState(false)
  const dragRef = useRef<{ mx: number; my: number; px: number; py: number } | null>(null)
  const didDragRef = useRef(false)

  // Legend, filter & detail panel
  const [showLegend, setShowLegend] = useState(false)
  const [legendItems, setLegendItems] = useState<LegendItem[]>([])
  const [selectedItem, setSelectedItem] = useState<SelectedItem | null>(null)
  const [showFilterPanel, setShowFilterPanel] = useState(false)
  const [filterState, setFilterState] = useState<FilterState>(loadFilterState)
  const [allTags, setAllTags] = useState<string[]>([])

  // 双方向ジャンプ: エディタ→プレビュー方向のカーソル行
  const cursorLineRef = useRef<number>(0)
  // ハイライトタイマー（プレビュー→エディタ方向の 500ms フェード用）
  const highlightTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  function applyTransform(t: { x: number; y: number; s: number }) {
    panZoomRef.current = t
    if (svgContainerRef.current) {
      svgContainerRef.current.style.transform = `translate(${t.x}px, ${t.y}px) scale(${t.s})`
    }
  }

  function resetPanZoom() {
    applyTransform({ x: 0, y: 0, s: 1 })
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

  // Extract legend and tags after SVG renders into DOM; also restore cursor highlight
  useEffect(() => {
    if (!svgContent) {
      requestAnimationFrame(() => { setLegendItems([]); setAllTags([]) })
      return
    }
    requestAnimationFrame(() => {
      const container = svgContainerRef.current
      if (container) {
        setLegendItems(extractLegend(container))
        setAllTags(extractTags(container))
        // SVG再描画後にカーソル行ハイライトを復元（直接DOM操作）
        const currentLine = cursorLineRef.current
        if (currentLine > 0) {
          container.querySelectorAll<Element>('.tdsl-item-cursor-highlight').forEach((el) => {
            el.classList.remove('tdsl-item-cursor-highlight')
          })
          container.querySelectorAll<HTMLElement>('[data-line]').forEach((el) => {
            if (parseInt(el.dataset.line || '0', 10) === currentLine) {
              el.classList.add('tdsl-item-cursor-highlight')
            }
          })
        }
      }
    })
  }, [svgContent])

  // Apply filter state to SVG DOM (opacity control)
  useEffect(() => {
    const container = svgContainerRef.current
    if (!container) return
    const tagFilter = filterState.tagSearch.trim().toLowerCase()
    container.querySelectorAll<HTMLElement>('.tdsl-item').forEach((el) => {
      const lane = el.getAttribute('data-lane') ?? ''
      const rawTags = el.getAttribute('data-tags') ?? ''
      const tags = rawTags ? rawTags.split(',').map((t) => t.trim()) : []
      const laneHidden = filterState.hiddenLanes.has(lane)
      const tagNoMatch = tagFilter !== '' && !tags.some((t) => t.toLowerCase().includes(tagFilter))
      el.style.opacity = laneHidden || tagNoMatch ? '0.12' : ''
    })
    container.querySelectorAll<HTMLElement>('.tdsl-lane-label[data-lane]').forEach((el) => {
      const lane = el.getAttribute('data-lane') ?? ''
      el.style.opacity = filterState.hiddenLanes.has(lane) ? '0.3' : ''
    })
  }, [filterState, svgContent])

  // Persist filter state to sessionStorage
  useEffect(() => {
    try {
      sessionStorage.setItem(FILTER_STATE_KEY, JSON.stringify({
        hiddenLanes: [...filterState.hiddenLanes],
        tagSearch: filterState.tagSearch,
      }))
    } catch { /* ignore */ }
  }, [filterState])

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

  function activatePreviewTarget(target: HTMLElement | null) {
    if (!target) {
      setSelectedItem(null)
      return
    }
    setSelectedItem({
      label: target.dataset.label || '',
      type: target.dataset.type || '',
      lane: target.dataset.lane || '',
      source: target.dataset.source || '',
      tooltip: target.dataset.tdslTooltip || '',
    })

    // プレビュー → エディタ方向ジャンプ
    const lineStr = target.dataset.line
    if (lineStr) {
      const lineNum = parseInt(lineStr, 10)
      const view = editorViewRef.current
      if (view && lineNum > 0) {
        try {
          const lineInfo = view.state.doc.line(lineNum)
          view.dispatch({
            selection: { anchor: lineInfo.from },
            scrollIntoView: true,
            effects: [
              EditorView.scrollIntoView(lineInfo.from, { y: 'center' }),
              setLineHighlight.of(lineNum),
            ],
          })
          view.focus()
          // 500ms 後にハイライトをフェードアウト
          if (highlightTimerRef.current !== null) clearTimeout(highlightTimerRef.current)
          highlightTimerRef.current = setTimeout(() => {
            view.dispatch({ effects: setLineHighlight.of(null) })
            highlightTimerRef.current = null
          }, 500)
        } catch {
          // 行範囲外は無視
        }
      }
    }
  }

  function handlePreviewClick(e: MouseEvent<HTMLDivElement>) {
    if (didDragRef.current) { didDragRef.current = false; return }
    const target = (e.target as Element).closest<HTMLElement>('[data-label]')
    activatePreviewTarget(target)
  }

  function handlePreviewKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === 'Enter' || e.key === ' ') {
      const target = (e.target as Element).closest<HTMLElement>('[data-label]')
      if (target) {
        e.preventDefault()
        activatePreviewTarget(target)
      }
    } else if (e.key === 'Escape') {
      setSelectedItem(null)
    }
  }

  // エディタ→プレビュー方向: カーソル行に対応するSVG要素を強調
  const handleCursorLine = useCallback((line: number) => {
    cursorLineRef.current = line
    const container = svgContainerRef.current
    if (!container) return
    // 既存の強調をすべて解除
    container.querySelectorAll<Element>('.tdsl-item-cursor-highlight').forEach((el) => {
      el.classList.remove('tdsl-item-cursor-highlight')
    })
    // カーソル行に対応するアイテムを強調
    container.querySelectorAll<HTMLElement>('[data-line]').forEach((el) => {
      const elLine = parseInt(el.dataset.line || '0', 10)
      if (elLine === line) {
        el.classList.add('tdsl-item-cursor-highlight')
      }
    })
  }, [])

  // カーソル行監視 extension（handleCursorLine は useCallback で安定しているため再生成しない）。
  // makeCursorLineExtension は handleCursorLine を ViewPlugin の update 内でのみ呼び出し、
  // 生成（render）中には呼ばないため、ref 読み取りの警告は誤検知。
  // eslint-disable-next-line react-hooks/refs
  const cursorLineExtension = useMemo(() => makeCursorLineExtension(handleCursorLine), [handleCursorLine])

  return {
    previewRef,
    svgContainerRef,
    cursorGrab,
    resetPanZoom,
    handlePreviewMouseDown,
    handlePreviewMouseMove,
    handlePreviewMouseUp,
    handlePreviewMouseLeave,
    handlePreviewDblClick,
    handlePreviewClick,
    handlePreviewKeyDown,
    tooltip,
    legendItems,
    allTags,
    showLegend,
    setShowLegend,
    showFilterPanel,
    setShowFilterPanel,
    filterState,
    setFilterState,
    selectedItem,
    setSelectedItem,
    cursorLineExtension,
  }
}
