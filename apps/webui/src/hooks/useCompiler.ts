import { useCallback, useEffect, useRef, useState } from 'react'
import { DEBOUNCE_MS } from '../lib/constants'
import { type Diagnostic, type RenderOptions, checkSource, renderSvgWithOptions } from '../wasmLoader'

export type CompilerState = {
  svgContent: string
  diagnostics: Diagnostic[]
  // CodeMirror linter が最新の diagnostics を参照するための ref
  diagnosticsRef: React.RefObject<Diagnostic[]>
  isStalePreview: boolean
}

// ソース変更を（デバウンス付きで）チェック＋SVG レンダリングし、
// 診断とプレビュー SVG を公開する。エラー時は直前の成功プレビューを保持する。
export function useCompiler(source: string, wasmReady: boolean, scale: number, renderOpts: RenderOptions = {}): CompilerState {
  const [svgContent, setSvgContent] = useState<string>('')
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [isStalePreview, setIsStalePreview] = useState(false)
  const diagnosticsRef = useRef<Diagnostic[]>(diagnostics)
  // CodeMirror linter が参照する ref を最新の diagnostics に同期する（render 中の
  // ref 書き込みは避け、effect で更新する）。
  useEffect(() => {
    diagnosticsRef.current = diagnostics
  }, [diagnostics])

  const renderOptsRef = useRef<RenderOptions>(renderOpts)
  useEffect(() => {
    renderOptsRef.current = renderOpts
  }, [renderOpts])

  const compileAndCheck = useCallback(
    (src: string) => {
      if (!wasmReady) return
      const diags = checkSource(src)
      setDiagnostics(diags)

      const hasErrors = diags.some((d) => d.severity === 'error')
      if (!hasErrors) {
        try {
          const svg = renderSvgWithOptions(src, scale, renderOptsRef.current)
          setSvgContent(svg)
          setIsStalePreview(false)
        } catch (e: unknown) {
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
      compileAndCheck(source)
    }, DEBOUNCE_MS)
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
    }
  }, [source, wasmReady, scale, compileAndCheck])

  // renderOpts が変化したら即座に再レンダリング（ソース変更なし）
  useEffect(() => {
    if (!wasmReady) return
    queueMicrotask(() => compileAndCheck(source))
    // renderOpts の変更時のみ発火させる
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [renderOpts.orientation, renderOpts.grid, renderOpts.theme, wasmReady])

  // Initial compile when WASM becomes ready
  useEffect(() => {
    if (wasmReady) {
      queueMicrotask(() => compileAndCheck(source))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wasmReady])

  return { svgContent, diagnostics, diagnosticsRef, isStalePreview }
}
