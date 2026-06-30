import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type RefObject } from 'react'
import {
  SPLIT_RATIO_KEY,
  SPLIT_RATIO_MAX,
  SPLIT_RATIO_MIN,
  readSplitRatio,
} from '../lib/settings'

export type SplitPaneApi = {
  splitRatio: number
  splitRatioMin: number
  splitRatioMax: number
  mainRef: RefObject<HTMLElement | null>
  handleDividerMouseDown: (e: MouseEvent<HTMLDivElement>) => void
  handleDividerKeyDown: (e: KeyboardEvent<HTMLDivElement>) => void
}

const SPLIT_RATIO_KEYBOARD_STEP = 0.02

function clampSplitRatio(value: number): number {
  return Math.max(SPLIT_RATIO_MIN, Math.min(SPLIT_RATIO_MAX, value))
}

export function splitRatioForKey(current: number, key: string): number | null {
  switch (key) {
    case 'ArrowLeft':
      return clampSplitRatio(current - SPLIT_RATIO_KEYBOARD_STEP)
    case 'ArrowRight':
      return clampSplitRatio(current + SPLIT_RATIO_KEYBOARD_STEP)
    case 'Home':
      return SPLIT_RATIO_MIN
    case 'End':
      return SPLIT_RATIO_MAX
    default:
      return null
  }
}

// エディタ/プレビューの分割比をドラッグで調整し、localStorage に永続化する。
export function useSplitPane(): SplitPaneApi {
  const [splitRatio, setSplitRatio] = useState<number>(readSplitRatio)
  // mouseup で永続化する最新 ratio。ドラッグ中の onMouseMove（render 外のイベント
  // ハンドラ）で同期するため、effect の実行タイミングに依存せず常に最新値を保持する。
  const splitRatioRef = useRef<number>(splitRatio)
  const splitDragRef = useRef<{ startX: number; startRatio: number; containerWidth: number } | null>(null)
  const mainRef = useRef<HTMLElement>(null)

  // Split pane drag (document-level to prevent losing track when cursor leaves divider)
  useEffect(() => {
    function onMouseMove(e: globalThis.MouseEvent) {
      if (!splitDragRef.current) return
      const dx = e.clientX - splitDragRef.current.startX
      const newRatio = clampSplitRatio(splitDragRef.current.startRatio + dx / splitDragRef.current.containerWidth)
      splitRatioRef.current = newRatio
      setSplitRatio(newRatio)
    }
    function onMouseUp() {
      if (!splitDragRef.current) return
      splitDragRef.current = null
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
      try {
        localStorage.setItem(SPLIT_RATIO_KEY, String(splitRatioRef.current))
      } catch {/* quota or private browsing — ignore */}
    }
    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
    return () => {
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }
  }, [])

  function persistSplitRatio(value: number) {
    splitRatioRef.current = value
    setSplitRatio(value)
    try {
      localStorage.setItem(SPLIT_RATIO_KEY, String(value))
    } catch {/* quota or private browsing — ignore */}
  }

  function handleDividerMouseDown(e: MouseEvent<HTMLDivElement>) {
    e.preventDefault()
    const containerWidth = mainRef.current?.clientWidth ?? document.documentElement.clientWidth
    splitDragRef.current = { startX: e.clientX, startRatio: splitRatio, containerWidth }
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
  }

  function handleDividerKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    const next = splitRatioForKey(splitRatioRef.current, e.key)
    if (next === null) return
    e.preventDefault()
    persistSplitRatio(next)
  }

  return {
    splitRatio,
    splitRatioMin: SPLIT_RATIO_MIN,
    splitRatioMax: SPLIT_RATIO_MAX,
    mainRef,
    handleDividerMouseDown,
    handleDividerKeyDown,
  }
}
