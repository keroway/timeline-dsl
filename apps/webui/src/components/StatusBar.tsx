import { useMemo } from 'react'
import { createTranslator, type Locale } from '../lib/i18n'

type StatusBarProps = {
  wasmReady: boolean
  wasmError: string | null
  errorCount: number
  warnCount: number
  locale: Locale
}

export function StatusBar({ wasmReady, wasmError, errorCount, warnCount, locale }: StatusBarProps) {
  const t = useMemo(() => createTranslator(locale), [locale])

  if (!wasmReady && !wasmError) {
    return <div className="status-bar loading">WASM を初期化中...</div>
  }
  if (wasmError) {
    return <div className="status-bar status-error">WASM 初期化エラー: {wasmError}</div>
  }
  return (
    <div className="status-bar ready">
      {errorCount > 0 && <span className="badge badge-error">{t.fmt('statusErrors', { count: errorCount })}</span>}
      {warnCount > 0 && <span className="badge badge-warn">{t.fmt('statusWarnings', { count: warnCount })}</span>}
      {errorCount === 0 && warnCount === 0 && (
        <span className="badge badge-ok">{t('statusOk')}</span>
      )}
    </div>
  )
}
