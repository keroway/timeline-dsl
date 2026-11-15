import { Inflate } from 'pako'
import { checkEncodedSize, MAX_SOURCE_BYTES, ShareError } from '../shareLimits'

// Only the dedicated share worker calls this in production.
export function inflateSource(encoded: string): string {
  checkEncodedSize(encoded)
  if (!/^[A-Za-z0-9_-]+$/.test(encoded) || encoded.length % 4 === 1)
    throw new ShareError('invalid')
  try {
    const binary = atob(encoded.replace(/-/g, '+').replace(/_/g, '/'))
    const input = Uint8Array.from(binary, (c) => c.charCodeAt(0))
    const inflate = new Inflate({ chunkSize: 16384 })
    const chunks: Uint8Array[] = []
    let length = 0
    inflate.onData = (chunk) => {
      length += chunk.length
      if (length > MAX_SOURCE_BYTES) throw new ShareError('tooLarge')
      chunks.push(chunk)
    }
    if (!inflate.push(input, true) || inflate.err)
      throw new ShareError('invalid')
    const output = new Uint8Array(length)
    let offset = 0
    for (const chunk of chunks) {
      output.set(chunk, offset)
      offset += chunk.length
    }
    return new TextDecoder('utf-8', { fatal: true }).decode(output)
  } catch (error) {
    if (error instanceof ShareError) throw error
    throw new ShareError('invalid')
  }
}
