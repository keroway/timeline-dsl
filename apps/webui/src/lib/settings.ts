// ─── Settings & LocalStorage persistence ─────────────────────────────────────

import { DEFAULT_LOCALE, type Locale, SUPPORTED_LOCALES } from './i18n'

export type { Locale }
export type ColorScheme = 'dark' | 'light'
export type ThemePreference = 'auto' | 'light' | 'dark'
export type SvgOrientation = 'horizontal' | 'vertical'
export type SvgGrid = 'none' | 'decade' | 'year' | 'month'
export type SvgTheme = 'default' | 'dark' | 'print' | 'pastel'
export type SvgLayoutStyle = 'timeline' | 'group-bands' | 'gantt' | 'zigzag'

export type Settings = {
  theme: ThemePreference
  fontSize: number
  lineWrap: boolean
  scale: number
  pngWhiteBg: boolean
  historyEnabled: boolean
  autoSaveEnabled: boolean
  svgOrientation: SvgOrientation
  svgGrid: SvgGrid
  svgTheme: SvgTheme
  svgShowEventLabels: boolean
  svgLayoutStyle: SvgLayoutStyle
  svgShowLegend: boolean
  locale: Locale
}

export const SETTINGS_KEY = 'tdsl:settings'
export const SPLIT_RATIO_KEY = 'tdsl:split-ratio'

export const SPLIT_RATIO_DEFAULT = 0.4
export const SPLIT_RATIO_MIN = 0.15
export const SPLIT_RATIO_MAX = 0.85

export const SETTINGS_DEFAULTS: Settings = {
  theme: 'auto',
  fontSize: 14,
  lineWrap: false,
  scale: 0,
  pngWhiteBg: true,
  historyEnabled: true,
  autoSaveEnabled: true,
  svgOrientation: 'horizontal',
  svgGrid: 'none',
  svgTheme: 'default',
  svgShowEventLabels: false,
  svgLayoutStyle: 'timeline',
  svgShowLegend: false,
  locale: DEFAULT_LOCALE,
}

// SettingsModal の選択肢（フォント 12〜18px、スケール 0=自動 / 0.5〜8×）に合わせた範囲。
const FONT_SIZE_MIN = 12
const FONT_SIZE_MAX = 18
const SCALE_MIN = 0
const SCALE_MAX = 8

const BOOLEAN_KEYS = [
  'lineWrap',
  'pngWhiteBg',
  'historyEnabled',
  'autoSaveEnabled',
] as const

function isNumberInRange(value: unknown, min: number, max: number): boolean {
  return (
    typeof value === 'number' &&
    Number.isFinite(value) &&
    value >= min &&
    value <= max
  )
}

export function readSplitRatio(): number {
  try {
    const raw = localStorage.getItem(SPLIT_RATIO_KEY)
    if (raw === null) return SPLIT_RATIO_DEFAULT
    const n = parseFloat(raw)
    if (!Number.isFinite(n)) return SPLIT_RATIO_DEFAULT
    return Math.max(SPLIT_RATIO_MIN, Math.min(SPLIT_RATIO_MAX, n))
  } catch {
    return SPLIT_RATIO_DEFAULT
  }
}

export function readSettings(): Settings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY)
    if (!raw) return SETTINGS_DEFAULTS
    const parsed = JSON.parse(raw) as Partial<Settings>
    const merged: Settings = { ...SETTINGS_DEFAULTS, ...parsed }
    if (
      merged.theme !== 'auto' &&
      merged.theme !== 'light' &&
      merged.theme !== 'dark'
    ) {
      merged.theme = SETTINGS_DEFAULTS.theme
    }
    if (!['horizontal', 'vertical'].includes(merged.svgOrientation)) {
      merged.svgOrientation = SETTINGS_DEFAULTS.svgOrientation
    }
    if (!['none', 'decade', 'year', 'month'].includes(merged.svgGrid)) {
      merged.svgGrid = SETTINGS_DEFAULTS.svgGrid
    }
    if (!['default', 'dark', 'print', 'pastel'].includes(merged.svgTheme)) {
      merged.svgTheme = SETTINGS_DEFAULTS.svgTheme
    }
    if (typeof merged.svgShowEventLabels !== 'boolean') {
      merged.svgShowEventLabels = SETTINGS_DEFAULTS.svgShowEventLabels
    }
    // 保存済み設定に未知の値が入っていても既定へ戻す（他の svg* と同じ扱い）。
    if (
      !['timeline', 'group-bands', 'gantt', 'zigzag'].includes(
        merged.svgLayoutStyle
      )
    ) {
      merged.svgLayoutStyle = SETTINGS_DEFAULTS.svgLayoutStyle
    }
    if (typeof merged.svgShowLegend !== 'boolean') {
      merged.svgShowLegend = SETTINGS_DEFAULTS.svgShowLegend
    }
    if (!SUPPORTED_LOCALES.includes(merged.locale as Locale)) {
      merged.locale = SETTINGS_DEFAULTS.locale
    }
    // 数値項目は型・有限性・UI が提示する範囲で検証し、不正なら項目単位で既定へ戻す。
    if (!isNumberInRange(merged.fontSize, FONT_SIZE_MIN, FONT_SIZE_MAX)) {
      merged.fontSize = SETTINGS_DEFAULTS.fontSize
    }
    if (!isNumberInRange(merged.scale, SCALE_MIN, SCALE_MAX)) {
      merged.scale = SETTINGS_DEFAULTS.scale
    }
    // boolean 項目は文字列 "false" などを素通しすると真偽値として誤評価される。
    for (const key of BOOLEAN_KEYS) {
      if (typeof merged[key] !== 'boolean') {
        merged[key] = SETTINGS_DEFAULTS[key]
      }
    }
    return merged
  } catch {
    return SETTINGS_DEFAULTS
  }
}

export function detectSystemScheme(): ColorScheme {
  if (typeof window === 'undefined' || !window.matchMedia) return 'dark'
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light'
}

export function resolveColorScheme(
  pref: ThemePreference,
  systemScheme: ColorScheme
): ColorScheme {
  return pref === 'auto' ? systemScheme : pref
}
