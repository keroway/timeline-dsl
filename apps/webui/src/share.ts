import * as pako from 'pako'

import {
  checkEncodedSize,
  checkSourceSize,
  ShareError,
  SHARE_TIMEOUT_MS,
} from './shareLimits'

const HASH_KEY = 'src'

function bytesToBase64(bytes: Uint8Array): string {
  let bin = ''
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i])
  return btoa(bin)
}

function base64ToUrl(b64: string): string {
  return b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

export function encodeSource(source: string): string {
  checkSourceSize(source)
  const deflated = pako.deflate(source)
  const encoded = base64ToUrl(bytesToBase64(deflated))
  checkEncodedSize(encoded)
  return encoded
}

// One worker per request: termination cancels CPU work as well as result delivery.
export function decodeSource(
  encoded: string,
  signal?: AbortSignal
): Promise<string> {
  return new Promise((resolve, reject) => {
    checkEncodedSize(encoded)
    if (signal?.aborted) throw new ShareError('cancelled')
    const worker = new Worker(
      new URL('./worker/share.worker.ts', import.meta.url),
      { type: 'module' }
    )
    let settled = false
    const finish = (value?: string, error?: ShareError) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      signal?.removeEventListener('abort', abort)
      worker.terminate()
      if (error) reject(error)
      else resolve(value ?? '')
    }
    const abort = () => finish(undefined, new ShareError('cancelled'))
    const timer = setTimeout(
      () => finish(undefined, new ShareError('timeout')),
      SHARE_TIMEOUT_MS
    )
    signal?.addEventListener('abort', abort, { once: true })
    worker.onmessage = ({ data }) => {
      if (data.error) finish(undefined, new ShareError(data.error))
      else finish(data.source)
    }
    worker.onerror = () => finish(undefined, new ShareError('invalid'))
    try {
      worker.postMessage(encoded)
    } catch {
      finish(undefined, new ShareError('invalid'))
    }
  })
}

export function buildShareUrl(
  source: string,
  origin: string = location.origin + location.pathname
): string {
  return `${origin}#${HASH_KEY}=${encodeSource(source)}`
}

export async function readSourceFromHash(
  signal?: AbortSignal
): Promise<string | null> {
  const hash = location.hash.startsWith('#')
    ? location.hash.slice(1)
    : location.hash
  if (!hash) return null
  const params = new URLSearchParams(hash)
  const encoded = params.get(HASH_KEY)
  if (encoded === null) return null
  return decodeSource(encoded, signal)
}
