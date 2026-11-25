import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DEFAULT_LOCALE } from './i18n'
import { readSettings, SETTINGS_DEFAULTS, SETTINGS_KEY } from './settings'

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

// ─── 数値・boolean 項目の型検証（#1045）──────────────────────────────────────

describe('readSettings numeric / boolean validation', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.restoreAllMocks()
  })

  it('falls back to defaults per field for invalid values', () => {
    localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({
        fontSize: -999,
        scale: 'broken',
        lineWrap: 'false',
        pngWhiteBg: null,
        historyEnabled: 'false',
        autoSaveEnabled: 'false',
      })
    )
    const s = readSettings()
    expect(s.fontSize).toBe(SETTINGS_DEFAULTS.fontSize)
    expect(s.scale).toBe(SETTINGS_DEFAULTS.scale)
    expect(s.lineWrap).toBe(SETTINGS_DEFAULTS.lineWrap)
    expect(s.pngWhiteBg).toBe(SETTINGS_DEFAULTS.pngWhiteBg)
    expect(s.historyEnabled).toBe(SETTINGS_DEFAULTS.historyEnabled)
    expect(s.autoSaveEnabled).toBe(SETTINGS_DEFAULTS.autoSaveEnabled)
  })

  it('preserves valid values including false and scale=0', () => {
    localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({
        fontSize: 16,
        scale: 0,
        lineWrap: true,
        pngWhiteBg: false,
        historyEnabled: false,
        autoSaveEnabled: false,
      })
    )
    const s = readSettings()
    expect(s.fontSize).toBe(16)
    expect(s.scale).toBe(0)
    expect(s.lineWrap).toBe(true)
    expect(s.pngWhiteBg).toBe(false)
    expect(s.historyEnabled).toBe(false)
    expect(s.autoSaveEnabled).toBe(false)
  })

  it('rejects out-of-range numbers', () => {
    localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({ fontSize: 9999, scale: -1 })
    )
    const s = readSettings()
    expect(s.fontSize).toBe(SETTINGS_DEFAULTS.fontSize)
    expect(s.scale).toBe(SETTINGS_DEFAULTS.scale)
  })

  it('keeps valid fields when only one field is invalid', () => {
    localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({ fontSize: 18, autoSaveEnabled: 'x' })
    )
    const s = readSettings()
    expect(s.fontSize).toBe(18)
    expect(s.autoSaveEnabled).toBe(true)
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
