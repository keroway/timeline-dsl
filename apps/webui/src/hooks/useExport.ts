import { getWorkerClient, type RenderOptions } from '../wasmLoader'
import { svgToPngBlob, triggerDownload } from '../lib/svgExport'
import { buildShareUrl } from '../share'
import type { ToastVariant } from '../components/Toast'
import type { FileHandleApi } from './useFileHandle'
import type { Translator } from '../lib/i18n'
import type { ConfirmOptions } from './useConfirm'

export type ExportApi = {
  downloadTdsl: () => Promise<void>
  downloadJsonIr: () => Promise<void>
  downloadSvg: () => void
  downloadHtml: () => Promise<void>
  downloadPng: (whiteBg?: boolean) => void
  exportPdf: () => Promise<void>
  copySvg: () => void
  copyPng: () => void
  copyMarkdown: () => void
  copyShareLink: () => void
}

export function useExport(
  source: string,
  svgContent: string,
  pngWhiteBg: boolean,
  renderOpts: RenderOptions,
  showToast: (message: string, variant?: ToastVariant) => void,
  fileHandle: FileHandleApi,
  t: Translator,
  confirm: (options: ConfirmOptions) => Promise<boolean>,
): ExportApi {
  async function downloadTdsl() {
    await fileHandle.saveSource(source)
  }

  async function downloadJsonIr() {
    try {
      // Resolve per action so a prior Worker failure does not poison exports.
      const client = getWorkerClient()
      const json = await client.compileToIrAsync(source)
      if ((await client.checkSourceAsync(source)).some((d) => d.severity === 'info')) {
        const proceed = await confirm({
          title: t('confirmJsonIrIncompleteTitle'),
          body: t('exportJsonIrIncompleteConfirm'),
          confirmLabel: t('confirmProceed'),
          cancelLabel: t('confirmCancel'),
          tone: 'warn',
        })
        if (!proceed) return
      }
      triggerDownload(new Blob([json], { type: 'application/json' }), 'timeline.json')
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      showToast(t.fmt('exportJsonIrFailed', { msg }), 'error')
    }
  }

  function downloadSvg() {
    if (!svgContent) return
    triggerDownload(new Blob([svgContent], { type: 'image/svg+xml' }), 'timeline.svg')
  }

  async function downloadHtml() {
    if (!svgContent) return
    try {
      const html = await getWorkerClient().renderHtmlWithOptionsAsync(source, renderOpts)
      triggerDownload(new Blob([html], { type: 'text/html' }), 'timeline.html')
    } catch {
      // keep silent — errors are already shown in diagnostics
    }
  }

  function downloadPng(whiteBg: boolean = true) {
    if (!svgContent) return
    svgToPngBlob(svgContent, whiteBg)
      .then((blob) => triggerDownload(blob, 'timeline.png'))
      .catch(() => showToast(t('exportPngGenerateFailed'), 'error'))
  }

  function copySvg() {
    if (!svgContent) return
    navigator.clipboard.writeText(svgContent)
      .then(() => showToast(t('exportSvgCopied'), 'success'))
      .catch(() => showToast(t('exportSvgCopyFailed'), 'error'))
  }

  function copyPng() {
    if (!svgContent) return
    svgToPngBlob(svgContent, pngWhiteBg)
      .then((blob) => navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]))
      .then(() => showToast(t('exportPngCopied'), 'success'))
      .catch(() => showToast(t('exportPngCopyFailed'), 'error'))
  }

  function copyMarkdown() {
    const md = '```tdsl\n' + source + '\n```'
    navigator.clipboard.writeText(md)
      .then(() => showToast(t('exportMarkdownCopied'), 'success'))
      .catch(() => showToast(t('exportMarkdownCopyFailed'), 'error'))
  }

  function copyShareLink() {
    try {
      const url = buildShareUrl(source)
      navigator.clipboard.writeText(url)
        .then(() => showToast(t('exportShareLinkCopied'), 'success'))
        .catch(() => showToast(t('exportShareLinkCopyFailed'), 'error'))
    } catch {
      showToast(t('exportShareLinkFailed'), 'error')
    }
  }

  async function exportPdf() {
    if (!svgContent) return
    let html: string
    try {
      html = await getWorkerClient().renderHtmlWithOptionsAsync(source, renderOpts)
    } catch {
      showToast(t('exportPdfFailed'), 'error')
      return
    }
    showToast(t('exportPdfPrintHint'), 'info')
    const blob = new Blob([html], { type: 'text/html' })
    const url = URL.createObjectURL(blob)
    const iframe = document.createElement('iframe')
    iframe.setAttribute('aria-hidden', 'true')
    iframe.style.cssText = 'position:fixed;right:0;bottom:0;width:0;height:0;border:0'
    const cleanup = () => {
      URL.revokeObjectURL(url)
      iframe.remove()
    }
    iframe.onload = () => {
      const cw = iframe.contentWindow
      if (!cw) {
        showToast(t('exportPdfFailed'), 'error')
        cleanup()
        return
      }
      cw.focus()
      cw.print()
      setTimeout(cleanup, 1000)
    }
    iframe.src = url
    document.body.appendChild(iframe)
  }

  return {
    downloadTdsl,
    downloadJsonIr,
    downloadSvg,
    downloadHtml,
    downloadPng,
    exportPdf,
    copySvg,
    copyPng,
    copyMarkdown,
    copyShareLink,
  }
}
