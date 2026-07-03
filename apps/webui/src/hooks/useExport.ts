import { checkSource, compileToIr, renderHtmlWithOptions } from '../wasmLoader'
import { svgToPngBlob, triggerDownload } from '../lib/svgExport'
import { buildShareUrl } from '../share'
import type { ToastVariant } from '../components/Toast'
import type { RenderOptions } from '../wasmLoader'
import type { FileHandleApi } from './useFileHandle'

export type ExportApi = {
  downloadTdsl: () => Promise<void>
  downloadJsonIr: () => void
  downloadSvg: () => void
  downloadHtml: () => void
  downloadPng: (whiteBg?: boolean) => void
  exportPdf: () => void
  copySvg: () => void
  copyPng: () => void
  copyMarkdown: () => void
  copyShareLink: () => void
}

// エクスポート系（ダウンロード/コピー/PDF 印刷）のハンドラ群をまとめて提供する。
export function useExport(
  source: string,
  svgContent: string,
  pngWhiteBg: boolean,
  renderOpts: RenderOptions,
  showToast: (message: string, variant?: ToastVariant) => void,
  fileHandle: FileHandleApi,
): ExportApi {
  async function downloadTdsl() {
    await fileHandle.saveSource(source)
  }

  function downloadJsonIr() {
    let json: string
    try {
      json = compileToIr(source)
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      showToast(`JSON IR の生成に失敗しました: ${msg}`, 'error')
      return
    }
    // WASM では Wikidata fetch が行われず import/map 由来のアイテムが IR に
    // 含まれない。不完全な IR を黙って保存しないよう、check_source の Info 診断
    // （未解決 import/map の通知）がある場合は明示的な確認を挟み、
    // 同意がなければ保存しない。
    if (checkSource(source).some((d) => d.severity === 'info')) {
      const proceed = window.confirm(
        'import / map ブロックは WebUI では解決されないため、この JSON IR にインポート由来のアイテムは含まれません。完全な IR は CLI の tdsl build で取得できます。\n\n静的アイテムのみの JSON IR を保存しますか？',
      )
      if (!proceed) return
    }
    triggerDownload(new Blob([json], { type: 'application/json' }), 'timeline.json')
  }

  function downloadSvg() {
    if (!svgContent) return
    triggerDownload(new Blob([svgContent], { type: 'image/svg+xml' }), 'timeline.svg')
  }

  function downloadPng(whiteBg: boolean = true) {
    if (!svgContent) return
    svgToPngBlob(svgContent, whiteBg)
      .then((blob) => triggerDownload(blob, 'timeline.png'))
      .catch(() => showToast('PNG の生成に失敗しました', 'error'))
  }

  function copySvg() {
    if (!svgContent) return
    navigator.clipboard.writeText(svgContent)
      .then(() => showToast('SVG をコピーしました', 'success'))
      .catch(() => showToast('SVG のコピーに失敗しました', 'error'))
  }

  function copyPng() {
    if (!svgContent) return
    svgToPngBlob(svgContent, pngWhiteBg)
      .then((blob) => navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]))
      .then(() => showToast('PNG をコピーしました', 'success'))
      .catch(() => showToast('PNG のコピーに失敗しました', 'error'))
  }

  function copyMarkdown() {
    const md = '```tdsl\n' + source + '\n```'
    navigator.clipboard.writeText(md)
      .then(() => showToast('Markdown をコピーしました', 'success'))
      .catch(() => showToast('Markdown のコピーに失敗しました', 'error'))
  }

  function copyShareLink() {
    try {
      const url = buildShareUrl(source)
      navigator.clipboard.writeText(url)
        .then(() => showToast('Share link をコピーしました', 'success'))
        .catch(() => showToast('Share link のコピーに失敗しました', 'error'))
    } catch {
      showToast('Share link の生成に失敗しました', 'error')
    }
  }

  function downloadHtml() {
    if (!svgContent) return
    try {
      const html = renderHtmlWithOptions(source, renderOpts)
      triggerDownload(new Blob([html], { type: 'text/html' }), 'timeline.html')
    } catch {
      // keep silent — errors are already shown in diagnostics
    }
  }

  // Export to PDF via the browser's native print-to-PDF. The CLI emits a
  // vector PDF through tdsl-render's `pdf` feature, but that path relies on
  // fontdb's system-font loading (ADR-0002 D5) which is unavailable in a
  // browser WASM sandbox — CJK labels would not shape. Printing the HTML
  // render instead lets the browser resolve fonts natively. We render into a
  // hidden iframe (no popup-blocker, prints only the iframe content) and let
  // the user pick "Save as PDF" in the print dialog.
  function exportPdf() {
    if (!svgContent) return
    let html: string
    try {
      html = renderHtmlWithOptions(source, renderOpts)
    } catch {
      showToast('PDF の生成に失敗しました', 'error')
      return
    }
    showToast('印刷ダイアログで「PDF に保存」を選択してください', 'info')
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
        showToast('PDF の生成に失敗しました', 'error')
        cleanup()
        return
      }
      cw.focus()
      cw.print()
      // Give the print dialog time to open before tearing down the iframe.
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
