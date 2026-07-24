import * as pako from 'pako'

const HASH_KEY = 'src'

function bytesToBase64(bytes: Uint8Array): string {
  let bin = ''
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i])
  return btoa(bin)
}

function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64)
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
  return out
}

function base64ToUrl(b64: string): string {
  return b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

function urlToBase64(b64url: string): string {
  let s = b64url.replace(/-/g, '+').replace(/_/g, '/')
  const pad = s.length % 4
  if (pad === 2) s += '=='
  else if (pad === 3) s += '='
  else if (pad !== 0) throw new Error('invalid base64url length')
  return s
}

export function encodeSource(source: string): string {
  const deflated = pako.deflate(source)
  return base64ToUrl(bytesToBase64(deflated))
}

export function decodeSource(encoded: string): string {
  const bytes = base64ToBytes(urlToBase64(encoded))
  return pako.inflate(bytes, { toText: true })
}

export function buildShareUrl(source: string, origin: string = location.origin + location.pathname): string {
  return `${origin}#${HASH_KEY}=${encodeSource(source)}`
}

export function readSourceFromHash(): string | null {
  const hash = location.hash.startsWith('#') ? location.hash.slice(1) : location.hash
  if (!hash) return null
  const params = new URLSearchParams(hash)
  const encoded = params.get(HASH_KEY)
  if (!encoded) return null
  return decodeSource(encoded)
}
