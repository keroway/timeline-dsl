import type { Diagnostic } from '../wasmLoader'

type DiagnosticsPanelProps = {
  diagnostics: Diagnostic[]
  onDiagClick: (diag: Diagnostic) => void
}

export function DiagnosticsPanel({ diagnostics, onDiagClick }: DiagnosticsPanelProps) {
  if (diagnostics.length === 0) return null
  return (
    <aside className="diagnostics-panel">
      <div className="diagnostics-header">診断結果</div>
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
