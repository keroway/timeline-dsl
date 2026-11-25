import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { DEBOUNCE_MS } from '../lib/constants'
import { EDITOR_SOURCE_KEY } from '../lib/initialSource'
import { useSourcePersistence } from './useSourcePersistence'

type Props = {
  source: string
  enabled?: boolean
  skip?: boolean
  onError?: () => void
}

function Harness({ source, enabled = true, skip = false, onError }: Props) {
  useSourcePersistence(source, enabled, { current: skip }, onError)
  return null
}

function ClearHarness({
  source,
  enabled,
  onClearError,
}: {
  source: string
  enabled: boolean
  onClearError: () => void
}) {
  useSourcePersistence(
    source,
    enabled,
    { current: false },
    undefined,
    onClearError
  )
  return null
}

describe('useSourcePersistence', () => {
  let container: HTMLDivElement
  let root: Root

  beforeEach(() => {
    ;(
      globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true
    vi.useFakeTimers()
    localStorage.clear()
    container = document.createElement('div')
    root = createRoot(container)
  })

  afterEach(() => {
    act(() => root.unmount())
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  function render(props: Props) {
    act(() => root.render(<Harness {...props} />))
    act(() => {
      vi.advanceTimersByTime(DEBOUNCE_MS + 1)
    })
  }

  function mockQuotaExceeded() {
    return vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('full', 'QuotaExceededError')
    })
  }

  it('正常に保存でき、通知しない', () => {
    const onError = vi.fn()
    render({ source: 'new source', onError })
    expect(localStorage.getItem(EDITOR_SOURCE_KEY)).toBe('new source')
    expect(onError).not.toHaveBeenCalled()
  })

  it('容量超過で保存に失敗したら通知する', () => {
    mockQuotaExceeded()
    const onError = vi.fn()
    render({ source: 'new source', onError })
    expect(onError).toHaveBeenCalledTimes(1)
  })

  it('連続失敗中は 1 回だけ通知し、成功後の失敗では再通知する', () => {
    const spy = mockQuotaExceeded()
    const onError = vi.fn()
    render({ source: 'a', onError })
    render({ source: 'ab', onError })
    expect(onError).toHaveBeenCalledTimes(1)

    spy.mockRestore()
    render({ source: 'abc', onError })
    expect(localStorage.getItem(EDITOR_SOURCE_KEY)).toBe('abc')

    mockQuotaExceeded()
    render({ source: 'abcd', onError })
    expect(onError).toHaveBeenCalledTimes(2)
  })

  it('自動保存 OFF のときは保存も通知もしない', () => {
    const spy = mockQuotaExceeded()
    const onError = vi.fn()
    render({ source: 'x', enabled: false, onError })
    expect(spy).not.toHaveBeenCalled()
    expect(onError).not.toHaveBeenCalled()
  })

  it('読み込み直後のスキップ時は保存しない', () => {
    const spy = mockQuotaExceeded()
    const onError = vi.fn()
    render({ source: 'x', skip: true, onError })
    expect(spy).not.toHaveBeenCalled()
    expect(onError).not.toHaveBeenCalled()
  })

  describe('自動保存 OFF 時の削除', () => {
    function mockRemoveFails() {
      return vi
        .spyOn(Storage.prototype, 'removeItem')
        .mockImplementation(() => {
          throw new DOMException('denied', 'SecurityError')
        })
    }

    it('正常に削除でき、通知しない', () => {
      localStorage.setItem(EDITOR_SOURCE_KEY, 'old')
      const onClearError = vi.fn()
      act(() =>
        root.render(
          <ClearHarness
            source="x"
            enabled={false}
            onClearError={onClearError}
          />
        )
      )
      expect(localStorage.getItem(EDITOR_SOURCE_KEY)).toBeNull()
      expect(onClearError).not.toHaveBeenCalled()
    })

    it('削除に失敗したら通知する', () => {
      mockRemoveFails()
      const onClearError = vi.fn()
      act(() =>
        root.render(
          <ClearHarness
            source="x"
            enabled={false}
            onClearError={onClearError}
          />
        )
      )
      expect(onClearError).toHaveBeenCalledTimes(1)
    })

    it('失敗後に OFF を再度切り替えて成功すれば通知しない', () => {
      const spy = mockRemoveFails()
      const onClearError = vi.fn()
      act(() =>
        root.render(
          <ClearHarness
            source="x"
            enabled={false}
            onClearError={onClearError}
          />
        )
      )
      spy.mockRestore()
      localStorage.setItem(EDITOR_SOURCE_KEY, 'old')
      act(() =>
        root.render(
          <ClearHarness source="x" enabled={true} onClearError={onClearError} />
        )
      )
      act(() =>
        root.render(
          <ClearHarness
            source="x"
            enabled={false}
            onClearError={onClearError}
          />
        )
      )
      expect(localStorage.getItem(EDITOR_SOURCE_KEY)).toBeNull()
      expect(onClearError).toHaveBeenCalledTimes(1)
    })

    it('onClearError 未指定でも例外を投げない', () => {
      mockRemoveFails()
      expect(() =>
        act(() => root.render(<Harness source="x" enabled={false} />))
      ).not.toThrow()
    })
  })

  it('onSaveError 未指定でも例外を投げない', () => {
    mockQuotaExceeded()
    expect(() => render({ source: 'x' })).not.toThrow()
  })
})
