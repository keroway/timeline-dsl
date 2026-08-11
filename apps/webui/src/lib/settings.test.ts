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

// ─── layout_style / show_legend（#752）────────────────────────────────────────
// WASM 側は以前から対応していたが WebUI に露出しておらず、
// `--layout-style group-bands / gantt / zigzag` と静的凡例が WebUI から
// 到達不能だった。設定として保存・復元できることを固定する。

describe('readSettings svgLayoutStyle / svgShowLegend', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.restoreAllMocks()
  })

  it('defaults to timeline layout with the legend hidden', () => {
    const s = readSettings()
    expect(s.svgLayoutStyle).toBe('timeline')
    expect(s.svgShowLegend).toBe(false)
  })

  it('preserves supported layout styles', () => {
    for (const style of ['timeline', 'group-bands', 'gantt', 'zigzag']) {
      localStorage.setItem(
        SETTINGS_KEY,
        JSON.stringify({ svgLayoutStyle: style })
      )
      expect(readSettings().svgLayoutStyle).toBe(style)
    }
  })

  // 保存済み設定に未知の値が入っていても既定へ戻す（他の svg* と同じ扱い）。
  // ここを素通しすると WASM 側でエラーになり、原因が設定だと分かりにくい。
  it('falls back to the default for an unknown layout style', () => {
    localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({ svgLayoutStyle: 'spiral' })
    )
    expect(readSettings().svgLayoutStyle).toBe('timeline')
  })

  it('falls back to the default when showLegend is not a boolean', () => {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify({ svgShowLegend: 'yes' }))
    expect(readSettings().svgShowLegend).toBe(false)
  })

  it('preserves an explicit showLegend', () => {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify({ svgShowLegend: true }))
    expect(readSettings().svgShowLegend).toBe(true)
  })
})
