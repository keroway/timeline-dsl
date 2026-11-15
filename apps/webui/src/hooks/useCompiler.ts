import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { DEBOUNCE_MS } from '../lib/constants'
import {
  type Diagnostic,
  getWorkerClient,
  type LintIssue,
  type RenderOptions,
} from '../wasmLoader'

function lintIssueToDiagnostic(issue: LintIssue): Diagnostic {
  return {
    severity: issue.severity,
    message: `[lint:${issue.code}]${issue.fixable ? ' (fixable)' : ''} ${issue.message}`,
    line: issue.line,
    col: issue.line > 0 ? 1 : 0,
  }
}

export type CompilePhase = 'preparing' | 'checking' | 'success' | 'error'
type Input = {
  source: string
  scale: number
  options: RenderOptions
  ready: boolean
}
type Result = { input: Input; phase: CompilePhase; diagnostics: Diagnostic[] }
type Success = { input: Input; svg: string }
export type CompilerState = {
  svgContent: string
  diagnostics: Diagnostic[]
  diagnosticsRef: React.RefObject<Diagnostic[]>
  isStalePreview: boolean
  phase: CompilePhase
  canExport: boolean
  isCurrent: () => boolean
}

export function useCompiler(
  source: string,
  wasmReady: boolean,
  scale: number,
  renderOpts: RenderOptions = {}
): CompilerState {
  // Include every option, including future additions. A new input lifetime also
  // invalidates A -> B -> A exports even though the final text is identical.
  const optionsKey = JSON.stringify(renderOpts)
  const input = useMemo<Input>(
    () => ({
      source,
      scale,
      options: JSON.parse(optionsKey),
      ready: wasmReady,
    }),
    [source, scale, optionsKey, wasmReady]
  )
  const latest = useRef<Input | null>(input)
  const [result, setResult] = useState<Result | null>(null)
  const [success, setSuccess] = useState<Success | null>(null)
  const diagnostics = result?.input === input ? result.diagnostics : []
  const diagnosticsRef = useRef<Diagnostic[]>(diagnostics)
  // Invalidate at commit, BEFORE debounce/async effects or user events.
  useLayoutEffect(() => {
    latest.current = input
    return () => {
      latest.current = null
    }
  }, [input])
  useLayoutEffect(() => {
    diagnosticsRef.current = diagnostics
  }, [diagnostics])

  useEffect(() => {
    if (!input.ready) return
    let active = true
    const current = () => active && latest.current === input
    const timer = setTimeout(async () => {
      if (!current()) return
      setResult({ input, phase: 'checking', diagnostics: [] })
      let diags: Diagnostic[] = []
      try {
        const client = getWorkerClient()
        const checked = await client.checkSourceAsync(input.source)
        if (!current()) return
        const lint = await client.lintSourceAsync(input.source)
        if (!current()) return
        diags = [
          ...checked,
          ...lint
            .filter((i) => i.code !== 'parse_error')
            .map(lintIssueToDiagnostic),
        ]
        if (diags.some((d) => d.severity === 'error')) {
          setResult({ input, phase: 'error', diagnostics: diags })
          return
        }
        const svg = await client.renderSvgWithOptionsAsync(
          input.source,
          input.scale,
          input.options
        )
        if (!current()) return
        setSuccess({ input, svg })
        setResult({ input, phase: 'success', diagnostics: diags })
      } catch (error) {
        if (!current()) return
        setResult({
          input,
          phase: 'error',
          diagnostics: [
            ...diags,
            {
              severity: 'error',
              message: error instanceof Error ? error.message : String(error),
              line: 0,
              col: 0,
            },
          ],
        })
      }
    }, DEBOUNCE_MS)
    return () => {
      active = false
      clearTimeout(timer)
    }
  }, [input])

  const phase = !input.ready
    ? 'preparing'
    : result?.input === input
      ? result.phase
      : 'checking'
  const canExport = phase === 'success' && success?.input === input
  const isCurrent = () => canExport && latest.current === input
  return {
    svgContent: success?.svg ?? '',
    diagnostics,
    diagnosticsRef,
    isStalePreview: !!success && !canExport,
    phase,
    canExport,
    isCurrent,
  }
}
