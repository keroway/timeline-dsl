import init, { compile_to_ir, render_svg_from_source, render_html_from_source, check_source, format_source } from './wasm/tdsl_wasm.js'

let initialized = false

export interface Diagnostic {
  severity: 'error' | 'warning'
  message: string
  line: number
  col: number
}

export async function initWasm(): Promise<void> {
  if (initialized) return
  await init()
  initialized = true
}

export function compileToIr(source: string): string {
  return compile_to_ir(source)
}

export function renderSvg(source: string, scale: number = 0): string {
  return render_svg_from_source(source, scale)
}

export function renderHtml(source: string): string {
  return render_html_from_source(source)
}

export function checkSource(source: string): Diagnostic[] {
  const result = check_source(source)
  try {
    return JSON.parse(result) as Diagnostic[]
  } catch {
    return []
  }
}

export function formatSource(source: string): string {
  return format_source(source)
}
