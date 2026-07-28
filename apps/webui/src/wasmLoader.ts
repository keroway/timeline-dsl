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
}

export {
  createWorkerClient,
  getWorkerClient,
  type WorkerClient,
  type WorkerLike,
} from './worker/client'
