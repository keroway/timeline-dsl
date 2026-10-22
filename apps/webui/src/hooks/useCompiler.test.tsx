import { act, createElement } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { RenderOptions } from '../wasmLoader'
import { useCompiler } from './useCompiler'

const renderSvgWithOptionsAsync = vi.fn(async () => '<svg/>')

vi.mock('../wasmLoader', () => ({
  getWorkerClient: () => ({
    checkSourceAsync: async () => [],
    lintSourceAsync: async () => [],
    renderSvgWithOptionsAsync,
  }),
}))

// React reads this flag to allow act() outside of a test renderer.
;(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true

const SOURCE = 'timeline "t"'
const BASE: RenderOptions = {
  orientation: 'horizontal',
  grid: true,
  theme: 'light',
  showEventLabels: true,
  layoutStyle: 'timeline',
  showLegend: true,
}

function Probe({ opts }: { opts: RenderOptions }) {
  useCompiler(SOURCE, true, 1, opts)
  return null
}

describe('useCompiler 設定変更時の再描画', () => {
  let container: HTMLDivElement
  let root: Root

  const mount = async (opts: RenderOptions) => {
    await act(async () => {
      root.render(createElement(Probe, { opts }))
    })
    await act(async () => {})
  }

  beforeEach(() => {
    renderSvgWithOptionsAsync.mockClear()
    container = document.createElement('div')
    root = createRoot(container)
  })

  afterEach(async () => {
    await act(async () => root.unmount())
  })

  it.each([
    ['layoutStyle', { layoutStyle: 'gantt' }],
    ['showLegend', { showLegend: false }],
  ] as const)(
    '%s の単独変更で新しい設定を使って再描画する',
    async (_name, patch) => {
      await mount(BASE)
      renderSvgWithOptionsAsync.mockClear()

      await mount({ ...BASE, ...patch })

      expect(renderSvgWithOptionsAsync).toHaveBeenCalled()
      const lastCall = renderSvgWithOptionsAsync.mock.calls.at(-1) as unknown[]
      expect(lastCall[2]).toMatchObject(patch)
    }
  )
})
