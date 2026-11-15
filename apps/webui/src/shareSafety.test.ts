import { deflate } from 'pako'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { decodeSource, encodeSource } from './share'
import {
  MAX_ENCODED_CHARS,
  MAX_SOURCE_BYTES,
  SHARE_TIMEOUT_MS,
} from './shareLimits'
import { inflateSource } from './worker/shareDecode'

function rawEncode(source: string) {
  return btoa(String.fromCharCode(...deflate(source)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '')
}

describe('bounded shared sources', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })
  it('accepts the exact UTF-8 boundary and rejects one byte more', () => {
    const source = 'a'.repeat(MAX_SOURCE_BYTES)
    expect(inflateSource(encodeSource(source))).toBe(source)
    expect(() => encodeSource(`${source}a`)).toThrow('tooLarge')
    expect(() =>
      encodeSource('あ'.repeat(Math.ceil(MAX_SOURCE_BYTES / 3)))
    ).toThrow('tooLarge')
  })
  it('interrupts highly compressed oversized output', () => {
    expect(() =>
      inflateSource(rawEncode('a'.repeat(MAX_SOURCE_BYTES * 4)))
    ).toThrow('tooLarge')
  })
  it('rejects encoded overflow before creating a worker', async () => {
    const worker = vi.fn()
    vi.stubGlobal('Worker', worker)
    await expect(
      decodeSource('a'.repeat(MAX_ENCODED_CHARS + 1))
    ).rejects.toThrow('tooLarge')
    expect(worker).not.toHaveBeenCalled()
  })
  it.each(['', '@@@', 'a', 'abcd', encodeSource('test').slice(0, -2)])(
    'rejects malformed/truncated input %s',
    (value) => {
      expect(() => inflateSource(value)).toThrow('invalid')
    }
  )
  it.each(['timeout', 'cancelled'] as const)(
    'terminates on %s and ignores late results',
    async (reason) => {
      vi.useFakeTimers()
      let instance: FakeWorker
      class FakeWorker {
        onmessage?: (event: { data: { source: string } }) => void
        onerror?: () => void
        terminate = vi.fn()
        postMessage = vi.fn()
        constructor() {
          instance = this
        }
      }
      vi.stubGlobal('Worker', FakeWorker)
      const abort = new AbortController()
      const result = decodeSource(encodeSource('hello'), abort.signal)
      const rejected = expect(result).rejects.toThrow(reason)
      if (reason === 'timeout')
        await vi.advanceTimersByTimeAsync(SHARE_TIMEOUT_MS)
      else abort.abort()
      await rejected
      expect(instance!.terminate).toHaveBeenCalledOnce()
      instance!.onmessage?.({ data: { source: 'late' } })
      expect(instance!.terminate).toHaveBeenCalledOnce()
    }
  )
  it('does not create a link after the processing deadline', () => {
    vi.spyOn(performance, 'now')
      .mockReturnValueOnce(0)
      .mockReturnValueOnce(SHARE_TIMEOUT_MS + 1)
    expect(() => encodeSource('source')).toThrow('timeout')
  })
  it('rejects encoded overflow even when source bytes fit', () => {
    let seed = 12345
    const source = Array.from({ length: MAX_SOURCE_BYTES / 2 }, () => {
      seed = (Math.imul(seed, 1664525) + 1013904223) | 0
      return String.fromCharCode((seed >>> 24) & 127)
    }).join('')
    expect(new TextEncoder().encode(source).length).toBe(MAX_SOURCE_BYTES / 2)
    expect(() => encodeSource(source)).toThrow('tooLarge')
  })
  it('rejects non-UTF-8 decompressed bytes', () => {
    const encoded = btoa(
      String.fromCharCode(...deflate(new Uint8Array([255, 254])))
    )
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=+$/, '')
    expect(() => inflateSource(encoded)).toThrow('invalid')
  })
})
