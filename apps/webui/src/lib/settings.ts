// ─── Settings & LocalStorage persistence ─────────────────────────────────────

import { DEFAULT_LOCALE, type Locale, SUPPORTED_LOCALES } from './i18n'

export type { Locale }
export type ColorScheme = 'dark' | 'light'
export type ThemePreference = 'auto' | 'light' | 'dark'
export type SvgOrientation = 'horizontal' | 'vertical'
export type SvgGrid = 'none' | 'decade' | 'year' | 'month'
export type SvgTheme = 'default' | 'dark' | 'print' | 'pastel'

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
  locale: DEFAULT_LOCALE,
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
    if (!SUPPORTED_LOCALES.includes(merged.locale as Locale)) {
      merged.locale = SETTINGS_DEFAULTS.locale
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
