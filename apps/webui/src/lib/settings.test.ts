import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DEFAULT_LOCALE } from './i18n'
import { readSettings, SETTINGS_KEY } from './settings'

describe('readSettings locale', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.restoreAllMocks()
  })

  it('defaults locale when settings are missing', () => {
    expect(readSettings().locale).toBe(DEFAULT_LOCALE)
  })

  it('preserves a supported locale', () => {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify({ locale: 'en' }))
    expect(readSettings().locale).toBe('en')
  })

  it('falls back to default locale for an invalid locale', () => {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify({ locale: 'zz' }))
    expect(readSettings().locale).toBe(DEFAULT_LOCALE)
  })
})
