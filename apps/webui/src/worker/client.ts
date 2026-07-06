import type { Diagnostic, LintIssue, RenderOptions } from '../wasmLoader'
import type { WorkerMessage, WorkerRequest } from './protocol'

type PendingRequest = {
  resolve: (value: unknown) => void
  reject: (reason?: unknown) => void
}

export type WorkerClient = {
  ready(): Promise<void>
  terminate(): void
  compileToIrAsync(source: string): Promise<string>
  renderSvgAsync(source: string, scale?: number): Promise<string>
  renderSvgWithOptionsAsync(source: string, scale?: number, opts?: RenderOptions): Promise<string>
  renderHtmlAsync(source: string): Promise<string>
  renderHtmlWithOptionsAsync(source: string, opts?: RenderOptions): Promise<string>
  checkSourceAsync(source: string): Promise<Diagnostic[]>
  formatSourceAsync(source: string): Promise<string>
  lintSourceAsync(source: string): Promise<LintIssue[]>
  lintFixSourceAsync(source: string): Promise<string>
}

export type WorkerLike = Pick<Worker, 'postMessage' | 'terminate'> & {
  onmessage: ((event: MessageEvent<WorkerMessage>) => void) | null
  onerror: ((event: ErrorEvent) => void) | null
}

export function createWorkerClient(workerFactory?: () => WorkerLike): WorkerClient {
  const worker = workerFactory
    ? workerFactory()
    : new Worker(new URL('./tdsl.worker.ts', import.meta.url), { type: 'module' })

  let requestId = 0
  const pending = new Map<number, PendingRequest>()
  let readyState: 'pending' | 'ready' | 'error' = 'pending'
  let readyError: string | null = null

  let readyResolve: (() => void) | null = null
  let readyReject: ((reason?: unknown) => void) | null = null
  const readyPromise = new Promise<void>((resolve, reject) => {
    readyResolve = resolve
    readyReject = reject
  })

  function rejectAll(reason: unknown) {
    for (const entry of pending.values()) {
      entry.reject(reason)
    }
    pending.clear()
  }

  worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
    const data = event.data
    if ('type' in data) {
      if (data.type === 'ready') {
        readyState = 'ready'
        readyResolve?.()
      } else {
        readyState = 'error'
        readyError = data.error
        readyReject?.(new Error(data.error))
        rejectAll(new Error(data.error))
      }
      return
    }

    const entry = pending.get(data.id)
    if (!entry) return
    pending.delete(data.id)
    if (data.ok) {
      entry.resolve(data.result)
    } else {
      entry.reject(new Error(data.error))
    }
  }

  worker.onerror = (event: ErrorEvent) => {
    const error = event.message || 'Worker error'
    readyState = 'error'
    readyError = error
    readyReject?.(new Error(error))
    rejectAll(new Error(error))
  }

  async function request<T extends WorkerRequest>(message: T): Promise<unknown> {
    if (readyState === 'error') {
      throw new Error(readyError ?? 'Worker initialization failed')
    }
    await readyPromise
    return await new Promise<unknown>((resolve, reject) => {
      pending.set(message.id, { resolve, reject })
      worker.postMessage(message)
    })
  }

  function nextId(): number {
    requestId += 1
    return requestId
  }

  return {
    ready: () => readyPromise,
    terminate() {
      rejectAll(new Error('Worker client terminated'))
      worker.terminate()
    },
    async compileToIrAsync(source) {
      return await request({ id: nextId(), op: 'compileToIr', args: [source] }) as string
    },
    async renderSvgAsync(source, scale = 0) {
      return await request({ id: nextId(), op: 'renderSvg', args: [source, scale] }) as string
    },
    async renderSvgWithOptionsAsync(source, scale = 0, opts = {}) {
      return await request({ id: nextId(), op: 'renderSvgWithOptions', args: [source, scale, opts] }) as string
    },
    async renderHtmlAsync(source) {
      return await request({ id: nextId(), op: 'renderHtml', args: [source] }) as string
    },
    async renderHtmlWithOptionsAsync(source, opts = {}) {
      return await request({ id: nextId(), op: 'renderHtmlWithOptions', args: [source, opts] }) as string
    },
    async checkSourceAsync(source) {
      return await request({ id: nextId(), op: 'checkSource', args: [source] }) as Diagnostic[]
    },
    async formatSourceAsync(source) {
      return await request({ id: nextId(), op: 'formatSource', args: [source] }) as string
    },
    async lintSourceAsync(source) {
      return await request({ id: nextId(), op: 'lintSource', args: [source] }) as LintIssue[]
    },
    async lintFixSourceAsync(source) {
      return await request({ id: nextId(), op: 'lintFixSource', args: [source] }) as string
    },
  }
}

let sharedClient: WorkerClient | null = null

export function getWorkerClient(): WorkerClient {
  if (!sharedClient) {
    sharedClient = createWorkerClient()
  }
  return sharedClient
}
