import { useEffect } from 'react'
import type { Translator } from '../lib/i18n'
import type { Locale } from '../lib/settings'

// <html lang> と document.title を選択中ロケールへ同期する。
// SPA は常にブラウザ環境で動くため document は必ず存在するが、
// useSettings.ts の防御スタイル（typeof window === 'undefined' ガード）に合わせる。
export function useDocumentMeta(locale: Locale, t: Translator): void {
  useEffect(() => {
    if (typeof document === 'undefined') return
    document.documentElement.lang = locale
    document.title = t('documentTitle')
  }, [locale, t])
}
