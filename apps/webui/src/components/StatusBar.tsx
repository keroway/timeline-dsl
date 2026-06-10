type StatusBarProps = {
  wasmReady: boolean
  wasmError: string | null
  errorCount: number
  warnCount: number
}

export function StatusBar({ wasmReady, wasmError, errorCount, warnCount }: StatusBarProps) {
  if (!wasmReady && !wasmError) {
    return <div className="status-bar loading">WASM を初期化中...</div>
  }
  if (wasmError) {
    return <div className="status-bar status-error">WASM 初期化エラー: {wasmError}</div>
  }
  return (
    <div className="status-bar ready">
      {errorCount > 0 && <span className="badge badge-error">{errorCount} エラー</span>}
      {warnCount > 0 && <span className="badge badge-warn">{warnCount} 警告</span>}
      {errorCount === 0 && warnCount === 0 && (
        <span className="badge badge-ok">問題なし</span>
      )}
    </div>
  )
}
