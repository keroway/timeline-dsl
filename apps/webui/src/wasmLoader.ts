// The WASM glue module (and its ~581KB .wasm payload) is loaded lazily via a
// dynamic import so it is split into its own chunk instead of being bundled into
// the main entry. All exported wrappers below are only ever called after
// `initWasm()` has resolved (the app gates every call behind `wasmReady`), so
// the cached module is guaranteed to be present when they run.
//
// The module itself comes from the published `@keroway/tdsl-wasm` npm package
// (see ADR 0001) rather than a committed build artifact under `src/wasm/`.
type WasmModule = typeof import('@keroway/tdsl-wasm')

let wasm: WasmModule | null = null
let initialized = false
let loading: Promise<void> | null = null

/** Return the initialized WASM module or throw if `initWasm()` has not resolved. */
function mod(): WasmModule {
  if (!wasm) {
    throw new Error('WASM module accessed before initWasm() resolved')
  }
  return wasm
}

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
  // De-duplicate concurrent callers so the chunk is fetched and instantiated once.
  if (!loading) {
    loading = (async () => {
      const m = await import('@keroway/tdsl-wasm')
      await m.default()
      wasm = m
      initialized = true
    })()
  }
  await loading
}

export function compileToIr(source: string): string {
  return mod().compile_to_ir(source)
}

export function renderSvg(source: string, scale: number = 0): string {
  return mod().render_svg_from_source(source, scale)
}

export function renderSvgWithOptions(source: string, scale: number = 0, opts: RenderOptions = {}): string {
  const jsOpts = new (mod().JsRenderOptions)()
  if (opts.orientation !== undefined) jsOpts.orientation = opts.orientation
  if (opts.grid !== undefined) jsOpts.grid = opts.grid
  if (opts.theme !== undefined) jsOpts.theme = opts.theme
  if (opts.showTable !== undefined) jsOpts.show_table = opts.showTable
  if (opts.showEventLabels !== undefined) jsOpts.show_event_labels = opts.showEventLabels
  if (opts.laneHeight !== undefined) jsOpts.lane_height = opts.laneHeight
  return mod().render_svg_from_source_with_options(source, scale, jsOpts)
}

export function renderHtml(source: string): string {
  return mod().render_html_from_source(source)
}

export function renderHtmlWithOptions(source: string, opts: RenderOptions = {}): string {
  const jsOpts = new (mod().JsRenderOptions)()
  if (opts.orientation !== undefined) jsOpts.orientation = opts.orientation
  if (opts.grid !== undefined) jsOpts.grid = opts.grid
  if (opts.theme !== undefined) jsOpts.theme = opts.theme
  if (opts.showTable !== undefined) jsOpts.show_table = opts.showTable
  if (opts.showEventLabels !== undefined) jsOpts.show_event_labels = opts.showEventLabels
  if (opts.laneHeight !== undefined) jsOpts.lane_height = opts.laneHeight
  return mod().render_html_from_source_with_options(source, jsOpts)
}

export function checkSource(source: string): Diagnostic[] {
  const result = mod().check_source(source)
  try {
    return JSON.parse(result) as Diagnostic[]
  } catch {
    return []
  }
}

export function formatSource(source: string): string {
  return mod().format_source(source)
}

export function lintSource(source: string): LintIssue[] {
  try {
    return JSON.parse(mod().lint_source(source)) as LintIssue[]
  } catch {
    return []
  }
}

export function lintFixSource(source: string): string {
  return mod().lint_fix_source(source)
}
