import type { Diagnostic, LintIssue, RenderOptions } from '../wasmLoader'

export type WorkerOp =
  | 'compileToIr'
  | 'renderSvg'
  | 'renderSvgWithOptions'
  | 'renderHtml'
  | 'renderHtmlWithOptions'
  | 'checkSource'
  | 'formatSource'
  | 'lintSource'
  | 'lintFixSource'

export type WorkerRequest =
  | { id: number; op: 'compileToIr'; args: [source: string] }
  | { id: number; op: 'renderSvg'; args: [source: string, scale?: number] }
  | {
      id: number
      op: 'renderSvgWithOptions'
      args: [source: string, scale?: number, opts?: RenderOptions]
    }
  | { id: number; op: 'renderHtml'; args: [source: string] }
  | {
      id: number
      op: 'renderHtmlWithOptions'
      args: [source: string, opts?: RenderOptions]
    }
  | { id: number; op: 'checkSource'; args: [source: string] }
  | { id: number; op: 'formatSource'; args: [source: string] }
  | { id: number; op: 'lintSource'; args: [source: string] }
  | { id: number; op: 'lintFixSource'; args: [source: string] }

export type WorkerSuccessResultMap = {
  compileToIr: string
  renderSvg: string
  renderSvgWithOptions: string
  renderHtml: string
  renderHtmlWithOptions: string
  checkSource: Diagnostic[]
  formatSource: string
  lintSource: LintIssue[]
  lintFixSource: string
}

export type WorkerResponse =
  | { id: number; ok: true; result: WorkerSuccessResultMap[WorkerOp] }
  | { id: number; ok: false; error: string }

export type WorkerReadyMessage =
  | { type: 'ready' }
  | { type: 'error'; error: string }

export type WorkerMessage = WorkerReadyMessage | WorkerResponse
