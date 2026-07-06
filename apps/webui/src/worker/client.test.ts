import { describe, expect, it } from 'vitest'
import { createWorkerClient, type WorkerLike } from './client'
import type { WorkerRequest } from './protocol'

// A minimal in-memory stand-in for the real `Worker` that lets the test
// script the order in which responses are delivered, independent of the
// order requests were sent — this is what "latest-wins" from the caller's
// perspective needs to prove (the *client* must not assume request/response
// ordering matches the eventual UI-visible "freshest wins" contract; that
// policy lives in `useCompiler`, but the client itself must still deliver
// each response to the right caller regardless of resolution order).
function createFakeWorker(): { worker: WorkerLike; postMessages: WorkerRequest[] } {
  const postMessages: WorkerRequest[] = []
  const worker: WorkerLike = {
    onmessage: null,
    onerror: null,
    postMessage(message: unknown) {
      postMessages.push(message as WorkerRequest)
    },
    terminate() {},
  }
  return { worker, postMessages }
}

describe('createWorkerClient', () => {
  it('delivers only the newest result when requests resolve out of order', async () => {
    const { worker, postMessages } = createFakeWorker()
    const client = createWorkerClient(() => worker)

    worker.onmessage!({ data: { type: 'ready' } } as MessageEvent)
    await client.ready()

    const firstPromise = client.checkSourceAsync('source-a')
    const secondPromise = client.checkSourceAsync('source-b')

    // The client awaits the ready handshake before posting, so give both
    // pending `request()` calls a microtask turn to reach `postMessage`.
    await Promise.resolve()
    await Promise.resolve()

    expect(postMessages).toHaveLength(2)
    const [firstReq, secondReq] = postMessages

    // Resolve the *second* (newest) request first, then the first (oldest).
    worker.onmessage!({
      data: { id: secondReq.id, ok: true, result: [{ severity: 'warning', message: 'b', line: 2, col: 1 }] },
    } as MessageEvent)
    worker.onmessage!({
      data: { id: firstReq.id, ok: true, result: [{ severity: 'error', message: 'a', line: 1, col: 1 }] },
    } as MessageEvent)

    const [firstResult, secondResult] = await Promise.all([firstPromise, secondPromise])

    // Each caller gets its own matching response regardless of delivery order...
    expect(firstResult).toEqual([{ severity: 'error', message: 'a', line: 1, col: 1 }])
    expect(secondResult).toEqual([{ severity: 'warning', message: 'b', line: 2, col: 1 }])

    // ...which is what allows a "latest-wins" consumer (useCompiler) to safely
    // ignore `firstResult` once it knows `secondPromise` (the newer request)
    // was issued, even though `secondResult` was the one delivered first.
  })

  it('rejects a pending request when the worker reports an error response', async () => {
    const { worker, postMessages } = createFakeWorker()
    const client = createWorkerClient(() => worker)

    worker.onmessage!({ data: { type: 'ready' } } as MessageEvent)
    await client.ready()

    const pending = client.compileToIrAsync('bad source')
    await Promise.resolve()
    await Promise.resolve()
    const [{ id }] = postMessages

    worker.onmessage!({
      data: { id, ok: false, error: 'parse failed' },
    } as MessageEvent)

    await expect(pending).rejects.toThrow('parse failed')
  })

  it('rejects ready() and any in-flight requests when the worker posts an init error', async () => {
    const { worker } = createFakeWorker()
    const client = createWorkerClient(() => worker)

    const readyPromise = client.ready()
    worker.onmessage!({ data: { type: 'error', error: 'wasm init failed' } } as MessageEvent)

    await expect(readyPromise).rejects.toThrow('wasm init failed')
    await expect(client.checkSourceAsync('x')).rejects.toThrow('wasm init failed')
  })
})
