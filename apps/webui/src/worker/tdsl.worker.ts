/// <reference lib="webworker" />
import initWasm, {
  check_source,
  compile_to_ir,
  format_source,
  JsRenderOptions,
  lint_fix_source,
  lint_source,
  render_html_from_source,
  render_html_from_source_with_options,
  render_svg_from_source,
  render_svg_from_source_with_options,
} from '@keroway/tdsl-wasm'
import type { Diagnostic, LintIssue, RenderOptions } from '../wasmLoader'
import type {
  WorkerMessage,
  WorkerRequest,
  WorkerSuccessResultMap,
} from './protocol'

declare const self: DedicatedWorkerGlobalScope

function toJsRenderOptions(opts: RenderOptions = {}): JsRenderOptions {
  const jsOpts = new JsRenderOptions()
  if (opts.orientation !== undefined) jsOpts.orientation = opts.orientation
  if (opts.grid !== undefined) jsOpts.grid = opts.grid
  if (opts.theme !== undefined) jsOpts.theme = opts.theme
  if (opts.showTable !== undefined) jsOpts.show_table = opts.showTable
  if (opts.showEventLabels !== undefined)
    jsOpts.show_event_labels = opts.showEventLabels
  if (opts.laneHeight !== undefined) jsOpts.lane_height = opts.laneHeight
  return jsOpts
}

function parseDiagnostics(result: string): Diagnostic[] {
  try {
    return JSON.parse(result) as Diagnostic[]
  } catch {
    return []
  }
}

function parseLintIssues(result: string): LintIssue[] {
  try {
    return JSON.parse(result) as LintIssue[]
  } catch {
    return []
  }
}

function dispatchRequest(
  request: WorkerRequest
): WorkerSuccessResultMap[typeof request.op] {
  switch (request.op) {
    case 'compileToIr':
      return compile_to_ir(...request.args)
    case 'renderSvg':
      return render_svg_from_source(request.args[0], request.args[1] ?? 0)
    case 'renderSvgWithOptions': {
      const [source, scale = 0, opts = {}] = request.args
      return render_svg_from_source_with_options(
        source,
        scale,
        toJsRenderOptions(opts)
      )
    }
    case 'renderHtml':
      return render_html_from_source(...request.args)
    case 'renderHtmlWithOptions': {
      const [source, opts = {}] = request.args
      return render_html_from_source_with_options(
        source,
        toJsRenderOptions(opts)
      )
    }
    case 'checkSource':
      return parseDiagnostics(check_source(...request.args))
    case 'formatSource':
      return format_source(...request.args)
    case 'lintSource':
      return parseLintIssues(lint_source(...request.args))
    case 'lintFixSource':
      return lint_fix_source(...request.args)
  }
}

async function main() {
  try {
    await initWasm()
    self.postMessage({ type: 'ready' } satisfies WorkerMessage)
  } catch (err: unknown) {
    const error = err instanceof Error ? err.message : String(err)
    self.postMessage({ type: 'error', error } satisfies WorkerMessage)
    return
  }

  self.onmessage = (event: MessageEvent<WorkerRequest>) => {
    const request = event.data
    try {
      const result = dispatchRequest(request)
      self.postMessage({
        id: request.id,
        ok: true,
        result,
      } satisfies WorkerMessage)
    } catch (err: unknown) {
      const error = err instanceof Error ? err.message : String(err)
      self.postMessage({
        id: request.id,
        ok: false,
        error,
      } satisfies WorkerMessage)
    }
  }
}

void main()
