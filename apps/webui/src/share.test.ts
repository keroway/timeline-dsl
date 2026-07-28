import { describe, expect, it } from 'vitest'
import { buildShareUrl, decodeSource, encodeSource } from './share'

describe('encodeSource / decodeSource', () => {
  it('ASCII テキストのラウンドトリップ', () => {
    const src = 'timeline "test" { span 0..100 "foo" }'
    expect(decodeSource(encodeSource(src))).toBe(src)
  })

  it('複数行テキストのラウンドトリップ', () => {
    const src = 'timeline "test" {\n  lane a "A"\n  span 0..100 "foo" on a\n}'
    expect(decodeSource(encodeSource(src))).toBe(src)
  })

  it('Unicode テキストのラウンドトリップ', () => {
    const src = '日本語テスト：タイムライン定義\n漢字・ひらがな・カタカナ'
    expect(decodeSource(encodeSource(src))).toBe(src)
  })

  it('空文字列のラウンドトリップ', () => {
    expect(decodeSource(encodeSource(''))).toBe('')
  })

  it('エンコード結果が URL-safe 文字のみ（+/= なし）', () => {
    const src = 'timeline "hello" { span 0..10 "world" }'
    const encoded = encodeSource(src)
    expect(encoded).not.toMatch(/[+/=]/)
  })
})

describe('buildShareUrl', () => {
  it('指定した origin を含む URL を生成する', () => {
    const src = 'timeline "t" {}'
    const url = buildShareUrl(src, 'https://example.com/')
    expect(url.startsWith('https://example.com/')).toBe(true)
    expect(url).toContain('#src=')
  })

  it('生成した URL のハッシュからソースを復元できる', () => {
    const src = 'timeline "test" { span 0..5 "e" }'
    const url = buildShareUrl(src, 'https://example.com/')
    const hash = url.split('#')[1]
    const params = new URLSearchParams(hash)
    const encoded = params.get('src')!
    expect(decodeSource(encoded)).toBe(src)
  })
})
