import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { type ConfirmState, useConfirm } from './useConfirm'

// useConfirm is a plain hook (no JSX needed by the test itself), but hooks
// must run inside a component. We mount a tiny harness with react-dom/client
// and drive it via `act`, without pulling in a new testing-library dependency.
describe('useConfirm', () => {
  let container: HTMLDivElement
  let root: Root
  let latestConfirm:
    | ((
        opts: Parameters<ReturnType<typeof useConfirm>['confirm']>[0]
      ) => Promise<boolean>)
    | null
  let latestState: ConfirmState | null

  function Harness() {
    const { confirm, confirmState } = useConfirm()
    latestConfirm = confirm
    latestState = confirmState
    return null
  }

  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
    latestConfirm = null
    latestState = null
    act(() => {
      root = createRoot(container)
      root.render(<Harness />)
    })
  })

  afterEach(() => {
    act(() => {
      root.unmount()
    })
    container.remove()
  })

  const baseOptions = {
    title: 'Title',
    body: 'Body',
    confirmLabel: 'Proceed',
    cancelLabel: 'Cancel',
  }

  it('resolves true when the confirm state is resolved with true', async () => {
    let resultPromise: Promise<boolean> | null = null
    act(() => {
      resultPromise = latestConfirm!(baseOptions)
    })
    expect(latestState).not.toBeNull()
    act(() => {
      latestState!.resolve(true)
    })
    await expect(resultPromise!).resolves.toBe(true)
  })

  it('resolves false when the confirm state is resolved with false (cancel/Esc)', async () => {
    let resultPromise: Promise<boolean> | null = null
    act(() => {
      resultPromise = latestConfirm!(baseOptions)
    })
    expect(latestState).not.toBeNull()
    act(() => {
      latestState!.resolve(false)
    })
    await expect(resultPromise!).resolves.toBe(false)
  })

  it('resolves a superseded prior confirm() with false when a new one is requested', async () => {
    let firstPromise: Promise<boolean> | null = null
    act(() => {
      firstPromise = latestConfirm!(baseOptions)
    })
    let secondPromise: Promise<boolean> | null = null
    act(() => {
      secondPromise = latestConfirm!({ ...baseOptions, title: 'Second' })
    })
    await expect(firstPromise!).resolves.toBe(false)
    act(() => {
      latestState!.resolve(true)
    })
    await expect(secondPromise!).resolves.toBe(true)
  })
})
