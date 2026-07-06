import { useMemo } from 'react'
import type { Diagnostic } from '../wasmLoader'
import { createTranslator, type Locale } from '../lib/i18n'

type DiagnosticsPanelProps = {
  diagnostics: Diagnostic[]
  onDiagClick: (diag: Diagnostic) => void
  locale: Locale
}

export function DiagnosticsPanel({ diagnostics, onDiagClick, locale }: DiagnosticsPanelProps) {
  const t = useMemo(() => createTranslator(locale), [locale])
  if (diagnostics.length === 0) return null
  return (
    <aside className="diagnostics-panel">
      <div className="diagnostics-header">{t('diagnosticsHeader')}</div>
      <ul className="diagnostics-list">
        {diagnostics.map((d, i) => (
          <li
            key={i}
            className={`diagnostic-item ${d.severity}${d.line > 0 ? ' clickable' : ''}`}
            onClick={() => onDiagClick(d)}
            role={d.line > 0 ? 'button' : undefined}
            tabIndex={d.line > 0 ? 0 : undefined}
            onKeyDown={d.line > 0 ? (e) => e.key === 'Enter' && onDiagClick(d) : undefined}
          >
            <span className="diag-severity">
              {d.severity === 'error' ? 'ERROR' : d.severity === 'warning' ? 'WARN' : 'INFO'}
            </span>
            {d.line > 0 && (
              <span className="diag-location">
                {d.line}:{d.col}
              </span>
            )}
            <span className="diag-message">{d.message}</span>
          </li>
        ))}
      </ul>
    </aside>
  )
}
