import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { DEBOUNCE_MS } from '../lib/constants'
import { type CompilerState, useCompiler } from './useCompiler'
const check = vi.fn(
  async (_source: string) =>
    [] as { severity: string; message: string; line: number; col: number }[]
)
const render = vi.fn(async (source: string) => `<svg>${source}</svg>`)
vi.mock('../wasmLoader', () => ({
  getWorkerClient: () => ({
    checkSourceAsync: check,
    lintSourceAsync: async () => [],
    renderSvgWithOptionsAsync: render,
  }),
}))
;(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true
let state: CompilerState
let root: ReturnType<typeof createRoot>
function Probe({ source, scale = 1 }: { source: string; scale?: number }) {
  state = useCompiler(source, true, scale, { showLegend: true })
  return null
}
async function update(source: string, scale = 1) {
  await act(async () => root.render(<Probe source={source} scale={scale} />))
}
async function debounce() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS)
  })
}
beforeEach(() => {
  vi.useFakeTimers()
  root = createRoot(document.createElement('div'))
  vi.clearAllMocks()
})
afterEach(async () => {
  await act(async () => root.unmount())
  vi.useRealTimers()
})
it('invalidates success immediately on edits and scale changes, retaining the last preview', async () => {
  await update('A')
  await debounce()
  const exportA = state.isCurrent
  expect(exportA()).toBe(true)
  await update('B')
  expect(state.canExport).toBe(false)
  expect(state.svgContent).toBe('<svg>A</svg>')
  expect(exportA()).toBe(false)
  expect(state.phase).toBe('checking')
  await debounce()
  expect(state.canExport).toBe(true)
  await update('B', 2)
  expect(state.canExport).toBe(false)
  await debounce()
  expect(render).toHaveBeenLastCalledWith('B', 2, { showLegend: true })
})
it.each(['resolve', 'reject'] as const)(
  'ignores an old render %s during the debounce window',
  async (kind) => {
    let resolve!: (value: string) => void
    let reject!: (error: Error) => void
    render.mockImplementationOnce(
      () =>
        new Promise((ok, fail) => {
          resolve = ok
          reject = fail
        })
    )
    await update('A')
    await debounce()
    await update('B')
    await act(async () => {
      if (kind === 'resolve') resolve('<svg>old</svg>')
      else reject(new Error('old error'))
    })
    expect(state.svgContent).toBe('')
    expect(state.diagnostics).toEqual([])
    expect(state.canExport).toBe(false)
    await debounce()
    expect(state.svgContent).toBe('<svg>B</svg>')
  }
)
it('never exports on syntax errors and invalidates old tickets through A -> B -> A', async () => {
  await update('A')
  await debounce()
  const ticket = state.isCurrent
  check.mockResolvedValueOnce([
    { severity: 'error', message: 'syntax', line: 1, col: 1 },
  ])
  await update('invalid')
  await debounce()
  expect(state.phase).toBe('error')
  expect(state.svgContent).toBe('<svg>A</svg>')
  expect(state.canExport).toBe(false)
  await update('A')
  await debounce()
  expect(state.canExport).toBe(true)
  expect(ticket()).toBe(false)
  const current = state.isCurrent
  await act(async () => root.unmount())
  expect(current()).toBe(false)
  root = createRoot(document.createElement('div'))
})
