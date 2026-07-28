import { useEffect, useState } from 'react'
import { getWorkerClient } from '../wasmLoader'

export type WasmStatus = {
  wasmReady: boolean
  wasmError: string | null
}

export function useWasm(): WasmStatus {
  const [wasmReady, setWasmReady] = useState(false)
  const [wasmError, setWasmError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    async function initializeWorker() {
      let lastError: unknown
      // A transient Worker/WASM startup failure gets one fresh-client retry.
      for (let attempt = 0; attempt < 2; attempt += 1) {
        try {
          await getWorkerClient().ready()
          if (!cancelled) {
            setWasmReady(true)
            setWasmError(null)
          }
          return
        } catch (err: unknown) {
          lastError = err
        }
      }

      if (!cancelled) {
        const msg =
          lastError instanceof Error ? lastError.message : String(lastError)
        setWasmError(msg)
      }
    }

    void initializeWorker()
    return () => {
      cancelled = true
    }
  }, [])

  return { wasmReady, wasmError }
}
