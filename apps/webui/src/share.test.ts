import { describe, it, expect } from 'vitest'
import { encodeSource, decodeSource, buildShareUrl } from './share.ts'

describe('encodeSource / decodeSource', () => {
  it('round-trips an ASCII string', () => {
    const src = 'timeline "test" { }'
    expect(decodeSource(encodeSource(src))).toBe(src)
  })

  it('round-trips a multi-line DSL source', () => {
    const src = `timeline "Dynasty" {
  unit year
  lane rulers as "Rulers"
  span 618 907 { label "Tang" lane rulers }
}`
    expect(decodeSource(encodeSource(src))).toBe(src)
  })

  it('round-trips a string containing Unicode characters', () => {
    const src = 'timeline "日本史" { lane 時代 as "jidai" }'
    expect(decodeSource(encodeSource(src))).toBe(src)
  })

  it('produces a URL-safe encoded string (no +, /, or = characters)', () => {
    const src = 'timeline "test" { unit year }'
    const encoded = encodeSource(src)
    expect(encoded).not.toMatch(/[+/=]/)
  })

  it('round-trips an empty string', () => {
    const src = ''
    expect(decodeSource(encodeSource(src))).toBe(src)
  })
})

describe('buildShareUrl', () => {
  it('builds a URL with the src hash parameter', () => {
    const src = 'timeline "t" { }'
    const url = buildShareUrl(src, 'https://example.com/')
    expect(url).toMatch(/^https:\/\/example\.com\/#src=/)
  })

  it('uses the provided origin instead of location.origin', () => {
    const src = 'hello'
    const url = buildShareUrl(src, 'https://my-app.example.com/index.html')
    expect(url.startsWith('https://my-app.example.com/index.html#src=')).toBe(true)
  })

  it('encodes and can be decoded back to the original source', () => {
    const src = 'timeline "round-trip" { unit year }'
    const url = buildShareUrl(src, 'https://example.com/')
    const hash = url.slice(url.indexOf('#') + 1)
    const params = new URLSearchParams(hash)
    const encoded = params.get('src')!
    expect(decodeSource(encoded)).toBe(src)
  })
})
