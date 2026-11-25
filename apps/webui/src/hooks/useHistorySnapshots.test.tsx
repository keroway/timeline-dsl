import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { readAutoSnapshots } from '../history'
import { type HistoryApi, useHistorySnapshots } from './useHistorySnapshots'

const t = Object.assign((k: string) => k, {
  fmt: (k: string) => k,
}) as unknown as Parameters<typeof useHistorySnapshots>[0]['t']

let latest: HistoryApi | null
let toasts: { message: string; variant?: string }[] = []

function Harness({ source, enabled }: { source: string; enabled: boolean }) {
  latest = useHistorySnapshots({
    source,
    historyEnabled: enabled,
    showToast: (message, variant) => toasts.push({ message, variant }),
    setSource: () => {},
    setShowHistory: () => {},
    skipAutoSaveRef: { current: false },
    t,
  })
  return null
}

describe('useHistorySnapshots 自動スナップショット', () => {
  let container: HTMLDivElement
  let root: Root
  beforeEach(() => {
    ;(
      globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true
    vi.useFakeTimers()
    vi.setSystemTime(1_000_000)
    localStorage.clear()
    latest = null
    toasts = []
    container = document.createElement('div')
    root = createRoot(container)
  })
  afterEach(() => {
    act(() => root.unmount())
    vi.useRealTimers()
  })

  function render(source: string, enabled = true) {
    act(() => root.render(<Harness source={source} enabled={enabled} />))
  }

  it('5分未満で見送った編集も、5分経過後に最新ソースで保存される', () => {
    render('A')
    act(() => vi.advanceTimersByTime(600))
    expect(readAutoSnapshots().map((s) => s.source)).toEqual(['A'])

    render('B')
    act(() => vi.advanceTimersByTime(600))
    // 5分未満なので未保存
    expect(readAutoSnapshots().map((s) => s.source)).toEqual(['A'])

    act(() => vi.advanceTimersByTime(5 * 60 * 1000))
    expect(readAutoSnapshots().map((s) => s.source)).toEqual(['B', 'A'])
    expect(latest?.autoSnaps).toHaveLength(2)
  })

  it('同一内容は重複保存しない', () => {
    render('A')
    act(() => vi.advanceTimersByTime(600))
    render('A2')
    render('A')
    act(() => vi.advanceTimersByTime(6 * 60 * 1000))
    expect(readAutoSnapshots()).toHaveLength(1)
  })

  it('履歴無効化で保留中の再判定が解除される', () => {
    render('A')
    act(() => vi.advanceTimersByTime(600))
    render('B')
    act(() => vi.advanceTimersByTime(600))
    render('B', false)
    act(() => vi.advanceTimersByTime(6 * 60 * 1000))
    expect(readAutoSnapshots().map((s) => s.source)).toEqual(['A'])
  })

  it('全件削除に成功すると一覧が空になり成功通知を出す', () => {
    render('A', false)
    act(() => latest?.handleSaveToHistory())
    toasts = []
    act(() => latest?.handleClearAllHistory())
    expect(latest?.manualSnaps).toEqual([])
    expect(toasts).toEqual([
      { message: 'historyClearedAll', variant: 'success' },
    ])
  })

  it('全件削除が失敗したら失敗通知を出し、残った履歴を表示し続ける', () => {
    render('A', false)
    act(() => latest?.handleSaveToHistory())
    toasts = []
    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation(() => {
      throw new DOMException('denied', 'SecurityError')
    })
    act(() => latest?.handleClearAllHistory())
    vi.restoreAllMocks()
    expect(latest?.manualSnaps).toHaveLength(1)
    expect(toasts).toEqual([
      { message: 'historyClearFailed', variant: 'error' },
    ])
  })
})
