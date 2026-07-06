import { useCallback, useEffect, useRef, useState } from 'react'
import { DEBOUNCE_MS } from '../lib/constants'
import { getWorkerClient, type Diagnostic, type LintIssue, type RenderOptions } from '../wasmLoader'

function lintIssueToDiagnostic(issue: LintIssue): Diagnostic {
  return {
    severity: issue.severity,
    message: `[lint:${issue.code}]${issue.fixable ? ' (fixable)' : ''} ${issue.message}`,
    line: issue.line,
    col: issue.line > 0 ? 1 : 0,
  }
}

export type CompilerState = {
  svgContent: string
  diagnostics: Diagnostic[]
  diagnosticsRef: React.RefObject<Diagnostic[]>
  isStalePreview: boolean
}

export function useCompiler(source: string, wasmReady: boolean, scale: number, renderOpts: RenderOptions = {}): CompilerState {
  const [svgContent, setSvgContent] = useState<string>('')
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [isStalePreview, setIsStalePreview] = useState(false)
  const diagnosticsRef = useRef<Diagnostic[]>(diagnostics)
  const latestRequestIdRef = useRef(0)
  const clientRef = useRef(getWorkerClient())

  useEffect(() => {
    diagnosticsRef.current = diagnostics
  }, [diagnostics])

  const renderOptsRef = useRef<RenderOptions>(renderOpts)
  useEffect(() => {
    renderOptsRef.current = renderOpts
  }, [renderOpts])

  const compileAndCheck = useCallback(
    async (src: string) => {
      if (!wasmReady) return
      latestRequestIdRef.current += 1
      const requestId = latestRequestIdRef.current
      const client = clientRef.current

      const checkDiags = await client.checkSourceAsync(src)
      if (requestId !== latestRequestIdRef.current) return

      const lintDiags = (await client.lintSourceAsync(src))
        .filter((i) => i.code !== 'parse_error')
        .map(lintIssueToDiagnostic)
      if (requestId !== latestRequestIdRef.current) return

      const diags = [...checkDiags, ...lintDiags]
      setDiagnostics(diags)

      const hasErrors = diags.some((d) => d.severity === 'error')
      if (!hasErrors) {
        try {
          const svg = await client.renderSvgWithOptionsAsync(src, scale, renderOptsRef.current)
          if (requestId !== latestRequestIdRef.current) return
          setSvgContent(svg)
          setIsStalePreview(false)
        } catch (e: unknown) {
          if (requestId !== latestRequestIdRef.current) return
          const msg = e instanceof Error ? e.message : String(e)
          setDiagnostics((prev) => [
            ...prev,
            { severity: 'error', message: msg, line: 0, col: 0 },
          ])
          setIsStalePreview(true)
        }
      } else {
        setIsStalePreview(true)
      }
    },
    [wasmReady, scale]
  )

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  useEffect(() => {
    if (!wasmReady) return
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(() => {
      void compileAndCheck(source)
    }, DEBOUNCE_MS)
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
    }
  }, [source, wasmReady, scale, compileAndCheck])

  useEffect(() => {
    if (!wasmReady) return
    queueMicrotask(() => {
      void compileAndCheck(source)
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [renderOpts.orientation, renderOpts.grid, renderOpts.theme, wasmReady, renderOpts.showEventLabels])

  useEffect(() => {
    if (wasmReady) {
      queueMicrotask(() => {
        void compileAndCheck(source)
      })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wasmReady])

  return { svgContent, diagnostics, diagnosticsRef, isStalePreview }
}
