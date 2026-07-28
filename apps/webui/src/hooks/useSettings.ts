import { useEffect, useState } from 'react'
import {
  type ColorScheme,
  detectSystemScheme,
  readSettings,
  resolveColorScheme,
  SETTINGS_KEY,
  type Settings,
} from '../lib/settings'

export type SettingsApi = {
  settings: Settings
  updateSetting: <K extends keyof Settings>(key: K, value: Settings[K]) => void
  systemScheme: ColorScheme
  colorScheme: ColorScheme
}

// 設定 state を所有し、localStorage への永続化と OS カラースキームの追従を行う。
export function useSettings(): SettingsApi {
  const [settings, setSettings] = useState<Settings>(readSettings)
  const [systemScheme, setSystemScheme] =
    useState<ColorScheme>(detectSystemScheme)
  const colorScheme = resolveColorScheme(settings.theme, systemScheme)

  function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]) {
    setSettings((prev) => ({ ...prev, [key]: value }))
  }

  // Persist settings to localStorage whenever they change
  useEffect(() => {
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings))
    } catch {
      /* quota exceeded or private browsing — ignore */
    }
  }, [settings])

  // Track OS color-scheme preference so `auto` follows it live
  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return
    const mql = window.matchMedia('(prefers-color-scheme: dark)')
    function handleChange(e: MediaQueryListEvent) {
      setSystemScheme(e.matches ? 'dark' : 'light')
    }
    if (mql.addEventListener) {
      mql.addEventListener('change', handleChange)
      return () => mql.removeEventListener('change', handleChange)
    }
    // Safari < 14 fallback
    mql.addListener(handleChange)
    return () => mql.removeListener(handleChange)
  }, [])

  return { settings, updateSetting, systemScheme, colorScheme }
}
