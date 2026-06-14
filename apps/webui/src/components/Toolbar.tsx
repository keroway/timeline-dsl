import { type ChangeEvent, type Dispatch, type RefObject, type SetStateAction } from 'react'
import type { ExportApi } from '../hooks/useExport'

type ToolbarProps = {
  fileMenuRef: RefObject<HTMLDivElement | null>
  fileMenuOpen: boolean
  setFileMenuOpen: Dispatch<SetStateAction<boolean>>
  onOpenFile: () => void
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
}

export function Toolbar(props: ToolbarProps) {
  const {
    fileMenuRef,
    fileMenuOpen,
    setFileMenuOpen,
    onOpenFile,
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
  } = props

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
            title="ファイル操作"
          >
            ファイル ▾
          </button>
          {fileMenuOpen && (
            <div className="export-menu export-menu-left">
              <button className="export-menu-item" onClick={() => { onOpenFile(); setFileMenuOpen(false) }}>
                .tdsl を開く
              </button>
            </div>
          )}
        </div>
        {/* テンプレートギャラリー */}
        <button
          className="btn"
          onClick={onShowGallery}
          title="テンプレートギャラリーを開く"
        >
          テンプレート
        </button>
        {historyEnabled && (
          <>
            <button
              className="btn"
              onClick={onSaveToHistory}
              title="現在の DSL を履歴に手動保存"
            >
              履歴に保存
            </button>
            <button
              className={`btn${historyCount > 0 ? ' btn-history-badge' : ''}`}
              onClick={onShowHistory}
              title="履歴パネルを開く"
            >
              履歴 {historyCount > 0 ? `(${historyCount})` : ''}
            </button>
          </>
        )}
        <button
          className="btn"
          onClick={onFormat}
          disabled={!wasmReady}
          title="エディタ内容を整形 (Ctrl/Cmd+Shift+F)"
        >
          Format
        </button>
        <button
          className="btn"
          onClick={onLintFix}
          disabled={!wasmReady}
          title="lint の自動修正可能な問題を一括修正"
        >
          Lint Fix
        </button>
      </div>
      <div className="toolbar-right">
        {/* エクスポートメニュー */}
        <div className="export-menu-wrapper" ref={exportMenuRef}>
          <button
            className="btn"
            onClick={() => setExportMenuOpen((v) => !v)}
            title="エクスポート"
          >
            エクスポート ▾
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
                SVG 保存
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.downloadHtml(); setExportMenuOpen(false) }} disabled={!svgContent}>
                HTML 保存
              </button>
              <button className="export-menu-item" onClick={() => { exportApi.exportPdf(); setExportMenuOpen(false) }} disabled={!svgContent}>
                PDF 保存（印刷）
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
          title="設定 (?)"
        >
          設定
        </button>
        {/* About */}
        <a
          className="btn"
          href="https://timeline-dsl-lp.pages.dev/"
          target="_blank"
          rel="noopener noreferrer"
          title="ランディングページ・ドキュメント"
        >
          About
        </a>
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
