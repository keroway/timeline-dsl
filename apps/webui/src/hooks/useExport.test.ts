import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Translator } from '../lib/i18n'
import type { RenderOptions } from '../wasmLoader'
import type { ConfirmOptions } from './useConfirm'
import type { FileHandleApi } from './useFileHandle'
import { useExport } from './useExport'

const renderHtmlWithOptionsAsync = vi.fn()

vi.mock('../wasmLoader', () => ({
  getWorkerClient: () => ({
    renderHtmlWithOptionsAsync,
  }),
}))

vi.mock('../lib/svgExport', () => ({
  svgToPngBlob: vi.fn(),
  triggerDownload: vi.fn(),
}))

const { triggerDownload } = await import('../lib/svgExport')

function createTranslator(): Translator {
  const t = ((key: string) => key) as Translator
  t.fmt = (key: string) => key
  return t
}

function callUseExport(showToast: (message: string, variant?: string) => void) {
  const fileHandle = {} as FileHandleApi
  const confirm = vi.fn(async (_options: ConfirmOptions) => true)
  const renderOpts = {} as RenderOptions
  // biome-ignore lint/correctness/useHookAtTopLevel: useExport has no React hook calls of its own; invoking it directly in a test is safe.
  return useExport(
    'timeline { title "T"; }',
    '<svg></svg>',
    true,
    renderOpts,
    showToast,
    fileHandle,
    createTranslator(),
    confirm
  )
}

describe('useExport downloadHtml', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('triggers a download when the Worker request succeeds', async () => {
    renderHtmlWithOptionsAsync.mockResolvedValueOnce('<html></html>')
    const showToast = vi.fn()
    const exportApi = callUseExport(showToast)

    await exportApi.downloadHtml()

    expect(triggerDownload).toHaveBeenCalledWith(
      expect.any(Blob),
      'timeline.html'
    )
    expect(showToast).not.toHaveBeenCalled()
  })

  it('shows an error toast instead of failing silently when the Worker rejects', async () => {
    renderHtmlWithOptionsAsync.mockRejectedValueOnce(new Error('worker down'))
    const showToast = vi.fn()
    const exportApi = callUseExport(showToast)

    await exportApi.downloadHtml()

    expect(triggerDownload).not.toHaveBeenCalled()
    expect(showToast).toHaveBeenCalledWith('exportHtmlFailed', 'error')
  })
})
