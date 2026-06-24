import init, { JsRenderOptions, compile_to_ir, render_svg_from_source, render_html_from_source, render_svg_from_source_with_options, render_html_from_source_with_options, check_source, format_source, lint_source, lint_fix_source } from './wasm/tdsl_wasm.js'

let initialized = false

export interface Diagnostic {
  severity: 'error' | 'warning' | 'info'
  message: string
  line: number
  col: number
}

export interface LintIssue {
  code: string
  severity: 'error' | 'warning'
  line: number
  message: string
  fixable: boolean
}

export interface RenderOptions {
  orientation?: 'horizontal' | 'vertical'
  grid?: 'none' | 'decade' | 'year' | 'month'
  theme?: 'default' | 'dark' | 'print' | 'pastel'
  showTable?: boolean
  showEventLabels?: boolean
  /** Height of each lane in pixels. `0`/undefined uses the renderer default (60). */
  laneHeight?: number
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

export function renderSvgWithOptions(source: string, scale: number = 0, opts: RenderOptions = {}): string {
  const jsOpts = new JsRenderOptions()
  if (opts.orientation !== undefined) jsOpts.orientation = opts.orientation
  if (opts.grid !== undefined) jsOpts.grid = opts.grid
  if (opts.theme !== undefined) jsOpts.theme = opts.theme
  if (opts.showTable !== undefined) jsOpts.show_table = opts.showTable
  if (opts.showEventLabels !== undefined) jsOpts.show_event_labels = opts.showEventLabels
  if (opts.laneHeight !== undefined) jsOpts.lane_height = opts.laneHeight
  return render_svg_from_source_with_options(source, scale, jsOpts)
}

export function renderHtml(source: string): string {
  return render_html_from_source(source)
}

export function renderHtmlWithOptions(source: string, opts: RenderOptions = {}): string {
  const jsOpts = new JsRenderOptions()
  if (opts.orientation !== undefined) jsOpts.orientation = opts.orientation
  if (opts.grid !== undefined) jsOpts.grid = opts.grid
  if (opts.theme !== undefined) jsOpts.theme = opts.theme
  if (opts.showTable !== undefined) jsOpts.show_table = opts.showTable
  if (opts.showEventLabels !== undefined) jsOpts.show_event_labels = opts.showEventLabels
  if (opts.laneHeight !== undefined) jsOpts.lane_height = opts.laneHeight
  return render_html_from_source_with_options(source, jsOpts)
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

export function lintSource(source: string): LintIssue[] {
  try {
    return JSON.parse(lint_source(source)) as LintIssue[]
  } catch {
    return []
  }
}

export function lintFixSource(source: string): string {
  return lint_fix_source(source)
}
