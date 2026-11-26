import { afterEach, expect, it, vi } from 'vitest'
import { svgToPngBlob } from './svgExport'

const SVG = '<svg xmlns="http://www.w3.org/2000/svg"><style></style></svg>'

class StubImage {
  static last: StubImage | null = null
  onload: (() => void) | null = null
  onerror: (() => void) | null = null
  naturalWidth = 10
  naturalHeight = 10
  width = 10
  height = 10
  set src(_v: string) {
    StubImage.last = this
  }
}

function stubEnv(ctx: unknown): { revoke: ReturnType<typeof vi.fn> } {
  const revoke = vi.fn()
  vi.stubGlobal('Image', StubImage)
  vi.stubGlobal('URL', {
    createObjectURL: () => 'blob:x',
    revokeObjectURL: revoke,
  })
  vi.stubGlobal('document', {
    createElement: () => ({
      width: 0,
      height: 0,
      getContext: () => ctx,
      toBlob: vi.fn(),
    }),
  })
  return { revoke }
}

afterEach(() => {
  vi.unstubAllGlobals()
  StubImage.last = null
})

it('svgToPngBlob rejects and revokes the URL when getContext returns null (#1057)', async () => {
  const { revoke } = stubEnv(null)
  const p = svgToPngBlob(SVG, true)
  StubImage.last?.onload?.()
  await expect(p).rejects.toThrow('canvas 2d context unavailable')
  expect(revoke).toHaveBeenCalledTimes(1)
})

it('svgToPngBlob rejects and revokes the URL when drawing throws (#1057)', async () => {
  const { revoke } = stubEnv({
    fillRect: vi.fn(),
    drawImage: () => {
      throw new Error('draw failed')
    },
  })
  const p = svgToPngBlob(SVG, false)
  StubImage.last?.onload?.()
  await expect(p).rejects.toThrow('draw failed')
  expect(revoke).toHaveBeenCalledTimes(1)
})
