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
    getWorkerClient().ready()
      .then(() => setWasmReady(true))
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err)
        setWasmError(msg)
      })
  }, [])

  return { wasmReady, wasmError }
}
