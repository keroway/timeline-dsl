import { act, createElement } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import { EDITOR_SOURCE_KEY, readInitialSource } from '../lib/initialSource'
import { ShareError } from '../shareLimits'
import { InitialSourceGate } from './InitialSourceGate'

vi.mock('../lib/initialSource', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/initialSource')>()),
  readInitialSource: vi.fn(),
}))
vi.mock('../hooks/useSettings', () => ({
  useSettings: () => ({ settings: { locale: 'en' } }),
}))
;(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true
afterEach(() => vi.clearAllMocks())
it('does not mount editor/persistence during loading or failure, and requires explicit recovery', async () => {
  localStorage.setItem(EDITOR_SOURCE_KEY, 'saved')
  localStorage.setItem('tdsl:history:auto', 'unchanged')
  const before = { ...localStorage }
  let reject!: (error: Error) => void
  vi.mocked(readInitialSource).mockImplementation(
    () =>
      new Promise((_resolve, fail) => {
        reject = fail
      })
  )
  const child = vi.fn((source: string) => createElement('div', null, source))
  const container = document.createElement('div')
  const root = createRoot(container)
  await act(async () => {
    root.render(<InitialSourceGate>{child}</InitialSourceGate>)
  })
  expect(child).not.toHaveBeenCalled()
  expect(container.textContent).toContain('Loading')
  await act(async () => reject(new ShareError('invalid')))
  expect(child).not.toHaveBeenCalled()
  expect({ ...localStorage }).toEqual(before)
  await act(async () => {
    container.querySelector('button')!.click()
  })
  expect(child).toHaveBeenCalledWith('saved')
  await act(async () => root.unmount())
})
