import { useEffect, useState } from 'react'
import { initWasm } from '../wasmLoader'

export type WasmStatus = {
  wasmReady: boolean
  wasmError: string | null
}

// WASM モジュールをマウント時に初期化し、準備状態とエラーを公開する。
export function useWasm(): WasmStatus {
  const [wasmReady, setWasmReady] = useState(false)
  const [wasmError, setWasmError] = useState<string | null>(null)

  useEffect(() => {
    initWasm()
      .then(() => setWasmReady(true))
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err)
        setWasmError(msg)
      })
  }, [])

  return { wasmReady, wasmError }
}
