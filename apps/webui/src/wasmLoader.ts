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
  laneHeight?: number
  /** レイアウト方式。WASM 側は以前から対応していたが WebUI に露出していなかった（#752）。 */
  layoutStyle?: 'timeline' | 'group-bands' | 'gantt' | 'zigzag'
  /** lane / タグの色を示す静的な凡例パネルを描画する。 */
  showLegend?: boolean
}

export {
  createWorkerClient,
  getWorkerClient,
  type WorkerClient,
  type WorkerLike,
} from './worker/client'
