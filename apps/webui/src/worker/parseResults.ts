import type { Diagnostic, LintIssue } from '../wasmLoader'

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function parseJsonArray(result: string, what: string): unknown[] {
  let parsed: unknown
  try {
    parsed = JSON.parse(result)
  } catch (err: unknown) {
    const reason = err instanceof Error ? err.message : String(err)
    throw new Error(`${what}: invalid JSON from WASM (${reason})`)
  }
  if (!Array.isArray(parsed)) {
    throw new Error(`${what}: expected a JSON array from WASM`)
  }
  return parsed
}

function isDiagnostic(value: unknown): value is Diagnostic {
  return (
    isRecord(value) &&
    (value.severity === 'error' ||
      value.severity === 'warning' ||
      value.severity === 'info') &&
    typeof value.message === 'string' &&
    typeof value.line === 'number' &&
    typeof value.col === 'number'
  )
}

function isLintIssue(value: unknown): value is LintIssue {
  return (
    isRecord(value) &&
    typeof value.code === 'string' &&
    (value.severity === 'error' || value.severity === 'warning') &&
    typeof value.line === 'number' &&
    typeof value.message === 'string' &&
    typeof value.fixable === 'boolean'
  )
}

/** WASM の診断 JSON を検証して返す。不正なら例外（Worker が ok:false に変換する）。 */
export function parseDiagnostics(result: string): Diagnostic[] {
  const items = parseJsonArray(result, 'checkSource')
  if (!items.every(isDiagnostic)) {
    throw new Error('checkSource: malformed diagnostic entry from WASM')
  }
  return items
}

/** WASM の lint JSON を検証して返す。不正なら例外（Worker が ok:false に変換する）。 */
export function parseLintIssues(result: string): LintIssue[] {
  const items = parseJsonArray(result, 'lintSource')
  if (!items.every(isLintIssue)) {
    throw new Error('lintSource: malformed lint issue entry from WASM')
  }
  return items
}
