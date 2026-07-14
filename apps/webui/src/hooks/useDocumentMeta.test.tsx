import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { createTranslator } from '../lib/i18n'
import type { Locale } from '../lib/settings'
import { useDocumentMeta } from './useDocumentMeta'

// useDocumentMeta is a plain hook, but hooks must run inside a component.
// We mount a tiny harness with react-dom/client and drive it via `act`,
// matching the pattern already used by useConfirm.test.tsx (no testing-library dependency).
describe('useDocumentMeta', () => {
  let container: HTMLDivElement
  let root: Root

  function Harness({ locale }: { locale: Locale }) {
    const t = createTranslator(locale)
    useDocumentMeta(locale, t)
    return null
  }

  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
  })

  afterEach(() => {
    act(() => {
      root.unmount()
    })
    container.remove()
  })

  it('sets <html lang> and document.title from the ja dictionary on mount', () => {
    act(() => {
      root = createRoot(container)
      root.render(<Harness locale="ja" />)
    })
    expect(document.documentElement.lang).toBe('ja')
    expect(document.title).toBe(createTranslator('ja')('documentTitle'))
  })

  it('updates <html lang> and document.title when the locale changes', () => {
    act(() => {
      root = createRoot(container)
      root.render(<Harness locale="ja" />)
    })
    act(() => {
      root.render(<Harness locale="en" />)
    })
    expect(document.documentElement.lang).toBe('en')
    expect(document.title).toBe(createTranslator('en')('documentTitle'))
  })
})
