import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Translator } from '../lib/i18n'
import type { RenderOptions } from '../wasmLoader'
import type { ConfirmOptions } from './useConfirm'
import { useExport } from './useExport'
import type { FileHandleApi } from './useFileHandle'

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

function callUseExport(
  showToast: (message: string, variant?: string) => void,
  isCurrent = () => true
) {
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
    confirm,
    isCurrent
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

describe('image export freshness', () => {
  beforeEach(() => vi.clearAllMocks())
  it('guards all image action entry points', async () => {
    const toast = vi.fn()
    const api = callUseExport(toast, () => false)
    api.downloadSvg()
    api.downloadPng()
    api.copySvg()
    api.copyPng()
    await api.downloadHtml()
    await api.exportPdf()
    expect(triggerDownload).not.toHaveBeenCalled()
    expect(renderHtmlWithOptionsAsync).not.toHaveBeenCalled()
    expect(toast).toHaveBeenCalledTimes(6)
  })
  it.each(['downloadHtml', 'exportPdf'] as const)(
    'cancels %s when HTML resolves after an edit',
    async (action) => {
      let current = true
      let finish!: (value: string) => void
      renderHtmlWithOptionsAsync.mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finish = resolve
          })
      )
      const toast = vi.fn()
      const task = callUseExport(toast, () => current)[action]()
      current = false
      finish('<html/>')
      await task
      expect(triggerDownload).not.toHaveBeenCalled()
      expect(document.querySelector('iframe')).toBeNull()
      expect(toast).toHaveBeenCalledWith('exportStaleCancelled', 'info')
    }
  )
  it.each(['downloadPng', 'copyPng'] as const)(
    'cancels %s during rasterization',
    async (action) => {
      const { svgToPngBlob } = await import('../lib/svgExport')
      let current = true
      let finish!: (value: Blob) => void
      vi.mocked(svgToPngBlob).mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finish = resolve
          })
      )
      const toast = vi.fn()
      callUseExport(toast, () => current)[action]()
      current = false
      finish(new Blob())
      await Promise.resolve()
      await Promise.resolve()
      expect(triggerDownload).not.toHaveBeenCalled()
      expect(toast).toHaveBeenCalledWith('exportStaleCancelled', 'info')
      expect(toast).not.toHaveBeenCalledWith('exportPngCopied', 'success')
    }
  )
  it('cancels printing if input changes while the iframe loads', async () => {
    let current = true
    renderHtmlWithOptionsAsync.mockResolvedValueOnce('<html/>')
    const createUrl = vi
      .spyOn(URL, 'createObjectURL')
      .mockReturnValue('blob:test')
    const revokeUrl = vi
      .spyOn(URL, 'revokeObjectURL')
      .mockImplementation(() => {})
    const toast = vi.fn()
    await callUseExport(toast, () => current).exportPdf()
    const frame = document.querySelector('iframe')!
    current = false
    frame.dispatchEvent(new Event('load'))
    expect(document.querySelector('iframe')).toBeNull()
    expect(revokeUrl).toHaveBeenCalledWith('blob:test')
    expect(toast).toHaveBeenCalledWith('exportStaleCancelled', 'info')
    createUrl.mockRestore()
    revokeUrl.mockRestore()
  })
})
