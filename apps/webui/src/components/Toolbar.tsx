import { useMemo, type ChangeEvent, type Dispatch, type RefObject, type SetStateAction } from 'react'
import type { ExportApi } from '../hooks/useExport'
import type { Settings } from '../lib/settings'
import { createTranslator } from '../lib/i18n'

type ToolbarProps = {
  fileMenuRef: RefObject<HTMLDivElement | null>
  fileMenuOpen: boolean
  setFileMenuOpen: Dispatch<SetStateAction<boolean>>
  onOpenFile: () => void
  fileAccessSupported: boolean
  currentFileName: string | null
  hasWritableFile: boolean
  onShowGallery: () => void
  historyEnabled: boolean
  historyCount: number
  onSaveToHistory: () => void
  onShowHistory: () => void
  onFormat: () => void
  onLintFix: () => void
  wasmReady: boolean
  exportMenuRef: RefObject<HTMLDivElement | null>
  exportMenuOpen: boolean
  setExportMenuOpen: Dispatch<SetStateAction<boolean>>
  exportApi: ExportApi
  svgContent: string
  onShowSettings: () => void
  fileInputRef: RefObject<HTMLInputElement | null>
  onFileChange: (e: ChangeEvent<HTMLInputElement>) => void
  locale: Settings['locale']
}

export function Toolbar(props: ToolbarProps) {
  const {
    fileMenuRef,
    fileMenuOpen,
    setFileMenuOpen,
    onOpenFile,
    fileAccessSupported,
    currentFileName,
    hasWritableFile,
    onShowGallery,
    historyEnabled,
    historyCount,
    onSaveToHistory,
    onShowHistory,
    onFormat,
    onLintFix,
    wasmReady,
    exportMenuRef,
    exportMenuOpen,
    setExportMenuOpen,
    exportApi,
    svgContent,
    onShowSettings,
    fileInputRef,
    onFileChange,
    locale,
  } = props

  const t = useMemo(() => createTranslator(locale), [locale])

  return (
    <header className="toolbar">
      <div className="toolbar-left">
        <span className="app-title">Timeline DSL</span>
        <div className="toolbar-divider" />
        {/* ファイルメニュー */}
        <div className="export-menu-wrapper" ref={fileMenuRef}>
          <button
            className="btn"
            onClick={() => setFileMenuOpen((v) => !v)}
            title={t('toolbarFileMenu')}
          >
            {t('toolbarFileMenu')} ▾
          </button>
          {fileMenuOpen && (
            <div className="export-menu export-menu-left">
              <button className="export-menu-item" onClick={() => { onOpenFile(); setFileMenuOpen(false) }}>
                {t('toolbarOpen')}
              </button>
              <div className="export-menu-section">
                {currentFileName ? t.fmt('toolbarCurrentFile', { name: currentFileName }) : t('toolbarNoWritableFile')}
              </div>
              {!fileAccessSupported && (
                <div className="export-menu-section" role="note">
                  {t('toolbarFileUnsupported')}
                </div>
              )}
              {fileAccessSupported && currentFileName && !hasWritableFile && (
                <div className="export-menu-section" role="note">
                  {t('toolbarNoWritableFile')}
                </div>
              )}
            </div>
          )}
        </div>
        {/* テンプレートギャラリー */}
        <button
          className="btn"
          onClick={onShowGallery}
          title={t('toolbarGallery')}
        >
          {t('toolbarGallery')}
        </button>
        {historyEnabled && (
          <>
            <button
              className="btn"
              onClick={onSaveToHistory}
              title={t('toolbarSaveHistory')}
            >
              {t('toolbarSaveHistory')}
            </button>
            <button
              className={`btn${historyCount > 0 ? ' btn-history-badge' : ''}`}
              onClick={onShowHistory}
              title={t('toolbarHistory')}
            >
              {t('toolbarHistory')} {historyCount > 0 ? `(${historyCount})` : ''}
            </button>
          </>
        )}
        <button
          className="btn"
          onClick={onFormat}
          disabled={!wasmReady}
          title={t('toolbarFormat')}
        >
          {t('toolbarFormat')}
        </button>
        <button
          className="btn"
          onClick={onLintFix}
          disabled={!wasmReady}
          title={t('toolbarLintFix')}
        >
          {t('toolbarLintFix')}
        </button>
      </div>
      <div className="toolbar-right">
        {/* エクスポートメニュー */}
        <div className="export-menu-wrapper" ref={exportMenuRef}>
          <button
            className="btn"
            onClick={() => setExportMenuOpen((v) => !v)}
            title={t('toolbarExportMenu')}
          >
            {t('toolbarExportMenu')} ▾
          </button>
          {exportMenuOpen && (
            <div className="export-menu">
              <div className="export-menu-section">ダウンロード</div>
              <button className="export-menu-item" onClick={() => { exportApi.downloadTdsl(); setExportMenuOpen(false) }}>
                .tdsl 保存
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.downloadJsonIr(); setExportMenuOpen(false) }}>
                JSON IR 保存
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.downloadSvg(); setExportMenuOpen(false) }} disabled={!svgContent}>
                {t('toolbarExportSvg')}
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.downloadHtml(); setExportMenuOpen(false) }} disabled={!svgContent}>
                {t('toolbarExportHtml')}
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.exportPdf(); setExportMenuOpen(false) }} disabled={!svgContent}>
                {t('toolbarExportPdf')}
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.downloadPng(true); setExportMenuOpen(false) }} disabled={!svgContent}>
                PNG 保存（白背景）
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.downloadPng(false); setExportMenuOpen(false) }} disabled={!svgContent}>
                PNG 保存（透過）
              </button>
              <div className="export-menu-section">クリップボードへコピー</div>
              <button className="export-menu-item" onClick={() => { exportApi.copySvg(); setExportMenuOpen(false) }} disabled={!svgContent}>
                SVG をコピー
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.copyPng(); setExportMenuOpen(false) }} disabled={!svgContent}>
                PNG をコピー
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.copyMarkdown(); setExportMenuOpen(false) }}>
                Markdown をコピー
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.copyShareLink(); setExportMenuOpen(false) }}>
                Share link をコピー
              </button>
            </div>
          )}
        </div>
        {/* 設定 */}
        <button
          className="btn"
          onClick={onShowSettings}
          title={t('toolbarSettings')}
        >
          {t('toolbarSettings')}
        </button>
        {/* About */}
        <button
          type="button"
          className="btn"
          onClick={() => window.open('https://timeline-dsl-lp.pages.dev/', '_blank', 'noopener,noreferrer')}
          title="ランディングページ・ドキュメント"
        >
          About
        </button>
        <input
          ref={fileInputRef}
          type="file"
          accept=".tdsl,text/plain"
          style={{ display: 'none' }}
          onChange={onFileChange}
        />
      </div>
    </header>
  )
}
