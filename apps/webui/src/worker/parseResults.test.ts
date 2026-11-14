import { describe, expect, it } from 'vitest'
import { parseDiagnostics, parseLintIssues } from './parseResults'

const diag = { severity: 'error', message: 'm', line: 1, col: 2 }
const lint = {
  code: 'L001',
  severity: 'warning',
  line: 3,
  message: 'm',
  fixable: true,
}

describe('parseDiagnostics', () => {
  it('正常な空配列は空配列として返す', () => {
    expect(parseDiagnostics('[]')).toEqual([])
  })
  it('正常な診断を返す', () => {
    expect(parseDiagnostics(JSON.stringify([diag]))).toEqual([diag])
  })
  it('不正 JSON は例外', () => {
    expect(() => parseDiagnostics('invalid')).toThrow(/invalid JSON/)
  })
  it('null / 非配列は例外', () => {
    expect(() => parseDiagnostics('null')).toThrow(/array/)
    expect(() => parseDiagnostics('{}')).toThrow(/array/)
  })
  it('要素のフィールド欠落・型違いは例外', () => {
    expect(() => parseDiagnostics('[{"message":"x"}]')).toThrow(/malformed/)
    expect(() => parseDiagnostics('[null]')).toThrow(/malformed/)
    expect(() =>
      parseDiagnostics(JSON.stringify([{ ...diag, severity: 'fatal' }]))
    ).toThrow(/malformed/)
  })
})

describe('parseLintIssues', () => {
  it('正常な空配列は空配列として返す', () => {
    expect(parseLintIssues('[]')).toEqual([])
  })
  it('正常な lint 結果を返す', () => {
    expect(parseLintIssues(JSON.stringify([lint]))).toEqual([lint])
  })
  it('不正 JSON は例外', () => {
    expect(() => parseLintIssues('invalid')).toThrow(/invalid JSON/)
  })
  it('null / 非配列は例外', () => {
    expect(() => parseLintIssues('null')).toThrow(/array/)
    expect(() => parseLintIssues('"x"')).toThrow(/array/)
  })
  it('要素のフィールド欠落は例外', () => {
    expect(() => parseLintIssues('[{"code":"L001"}]')).toThrow(/malformed/)
    expect(() =>
      parseLintIssues(JSON.stringify([{ ...lint, fixable: 'yes' }]))
    ).toThrow(/malformed/)
  })
})
