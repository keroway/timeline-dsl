import { useEffect, useMemo, useRef, useState, type ChangeEvent, type CSSProperties } from 'react'
import { EditorView } from '@codemirror/view'
import { forceLinting } from '@codemirror/lint'
import { getWorkerClient, type Diagnostic } from './wasmLoader'
import { useToast } from './components/useToast'
import { readInitialSource } from './lib/initialSource'
import { makeTdslLinter } from './editor/extensions'
import { useWasm } from './hooks/useWasm'
import { useSettings } from './hooks/useSettings'
import { useCompiler } from './hooks/useCompiler'
import { useSvgInteractions } from './hooks/useSvgInteractions'
import { useSplitPane } from './hooks/useSplitPane'
import { useExport } from './hooks/useExport'
import { useConfirm } from './hooks/useConfirm'
import { useHistorySnapshots } from './hooks/useHistorySnapshots'
import { useSourcePersistence } from './hooks/useSourcePersistence'
import { useFileHandle } from './hooks/useFileHandle'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import { useOutsideClick } from './hooks/useOutsideClick'
import { usePwaLifecycle } from './hooks/usePwaLifecycle'
import { useDocumentMeta } from './hooks/useDocumentMeta'
import { Toolbar } from './components/Toolbar'
import { StatusBar } from './components/StatusBar'
import { MobileTabBar, type MobileTab } from './components/MobileTabBar'
import { EditorPane } from './components/EditorPane'
import { PreviewPanel } from './components/PreviewPanel'
import { DiagnosticsPanel } from './components/DiagnosticsPanel'
import { Tooltip } from './components/Tooltip'
import { SettingsModal } from './components/SettingsModal'
import { GalleryModal } from './components/GalleryModal'
import { HistoryModal } from './components/HistoryModal'
import { ConfirmModal } from './components/ConfirmModal'
import { createTranslator } from './lib/i18n'
import './App.css'

function App() {
  const [initial] = useState(readInitialSource)
  const [source, setSource] = useState<string>(initial.source)
  const showToast = useToast()

  const editorViewRef = useRef<EditorView | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  // Tracks programmatic source loads (template/file/restore) that should not be auto-saved
  const skipAutoSaveRef = useRef(false)

  // UI toggles owned by the shell
  const [mobileTab, setMobileTab] = useState<MobileTab>('editor')
  const [exportMenuOpen, setExportMenuOpen] = useState(false)
  const exportMenuRef = useRef<HTMLDivElement>(null)
  const [fileMenuOpen, setFileMenuOpen] = useState(false)
  const fileMenuRef = useRef<HTMLDivElement>(null)
  const [showSettings, setShowSettings] = useState(false)
  const [showGallery, setShowGallery] = useState(false)
  const [showHistory, setShowHistory] = useState(false)
  const [previewFullscreen, setPreviewFullscreen] = useState<boolean>(() =>
    new URLSearchParams(location.search).get('preview') === '1'
  )

  const { wasmReady, wasmError } = useWasm()
  const { settings, updateSetting, systemScheme, colorScheme } = useSettings()
  const t = useMemo(() => createTranslator(settings.locale), [settings.locale])
  useDocumentMeta(settings.locale, t)
  const pwaUpdate = usePwaLifecycle(showToast, t)
  const renderOpts = useMemo(
    () => ({
      orientation: settings.svgOrientation,
      grid: settings.svgGrid,
      theme: settings.svgTheme,
      showEventLabels: settings.svgShowEventLabels,
    }),
    [settings.svgOrientation, settings.svgGrid, settings.svgTheme, settings.svgShowEventLabels]
  )
  const { svgContent, diagnostics, diagnosticsRef, isStalePreview } = useCompiler(source, wasmReady, settings.scale, renderOpts)
  const fileHandle = useFileHandle(showToast, t)
  const svg = useSvgInteractions(svgContent, editorViewRef)
  const {
    splitRatio,
    splitRatioMin,
    splitRatioMax,
    mainRef,
    handleDividerMouseDown,
    handleDividerKeyDown,
  } = useSplitPane()
  const { confirm, confirmState } = useConfirm()
  const exportApi = useExport(source, svgContent, settings.pngWhiteBg, renderOpts, showToast, fileHandle, t, confirm)
  const history = useHistorySnapshots({
    source,
    historyEnabled: settings.historyEnabled,
    showToast,
    setSource,
    setShowHistory,
    skipAutoSaveRef,
    t,
  })

  // インライン linter extension（ref 経由で最新 diagnostics を参照する）
  const tdslLinterExtension = useMemo(() => makeTdslLinter(diagnosticsRef), [diagnosticsRef])

  useSourcePersistence(source, settings.autoSaveEnabled, skipAutoSaveRef)
  useOutsideClick(exportMenuRef, exportMenuOpen, setExportMenuOpen)
  useOutsideClick(fileMenuRef, fileMenuOpen, setFileMenuOpen)

  function handleEditorChange(value: string) {
    setSource(value)
  }

  function handleGallerySelect(newSource: string) {
    history.snapshotBeforeLoad(t('historySnapshotBeforeTemplate'))
    skipAutoSaveRef.current = true
    setSource(newSource)
    setShowGallery(false)
  }

  async function handleFormat() {
    if (!wasmReady) return
    const view = editorViewRef.current
    if (!view) return
    const currentSource = view.state.doc.toString()
    let formatted: string
    try {
      formatted = await getWorkerClient().formatSourceAsync(currentSource)
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      showToast(t.fmt('appFormatFailed', { msg }), 'error')
      return
    }
    if (formatted === currentSource) {
      showToast(t('appAlreadyFormatted'), 'info')
      return
    }
    const hadComment = currentSource.includes('//') || currentSource.includes('/*')
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: formatted },
    })
    if (hadComment) {
      showToast(t('appFormattedCommentWarning'), 'info')
    } else {
      showToast(t('appFormatted'), 'success')
    }
  }

  function handlePwaReload() {
    if (!pwaUpdate.updateServiceWorker) return
    void pwaUpdate.updateServiceWorker(true)
  }

  async function handleLintFix() {
    if (!wasmReady) return
    const view = editorViewRef.current
    if (!view) return
    const currentSource = view.state.doc.toString()
    let fixed: string
    try {
      fixed = await getWorkerClient().lintFixSourceAsync(currentSource)
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      showToast(t.fmt('appLintFixFailed', { msg }), 'error')
      return
    }
    if (fixed === currentSource) {
      showToast(t('appLintFixNoIssues'), 'info')
      return
    }
    const hadComment = currentSource.includes('//') || currentSource.includes('/*')
    const warning = hadComment
      ? t('appLintFixCommentConfirm')
      : t('appLintFixConfirm')
    const proceed = await confirm({
      title: t('confirmLintFixTitle'),
      body: warning,
      confirmLabel: t('confirmProceed'),
      cancelLabel: t('confirmCancel'),
      tone: 'warn',
    })
    if (!proceed) return
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: fixed },
    })
    showToast(t('appLintFixed'), 'success')
  }

  async function openFile() {
    if (!fileHandle.supported) {
      showToast(t('fileAccessUnsupported'), 'info')
      fileInputRef.current?.click()
      return
    }
    try {
      const result = await fileHandle.openWithPicker()
      if (result.status !== 'opened') return
      history.snapshotBeforeLoad(t('historySnapshotBeforeFileOpen'))
      skipAutoSaveRef.current = true
      setSource(result.text)
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : String(error)
      showToast(t.fmt('appFileOpenFailed', { msg }), 'error')
    }
  }

  function handleFileChange(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = (ev) => {
      const text = ev.target?.result as string
      history.snapshotBeforeLoad(t('historySnapshotBeforeFileOpen'))
      fileHandle.markLegacyFileOpened(file.name)
      skipAutoSaveRef.current = true
      setSource(text)
    }
    reader.readAsText(file)
    // Reset so same file can be re-opened
    e.target.value = ''
  }

  function handleDiagClick(diag: Diagnostic) {
    const view = editorViewRef.current
    if (!view || diag.line <= 0) return
    try {
      const lineInfo = view.state.doc.line(diag.line)
      const pos = lineInfo.from + Math.max(0, diag.col - 1)
      view.dispatch({ selection: { anchor: pos }, scrollIntoView: true })
      view.focus()
    } catch {
      // line out of range — ignore
    }
  }

  useKeyboardShortcuts({
    editorViewRef,
    setShowSettings,
    setPreviewFullscreen,
    onSave: exportApi.downloadTdsl,
    onFormat: handleFormat,
    source,
    wasmReady,
  })

  // Surface a Toast if the initial Hash failed to decode
  useEffect(() => {
    if (initial.hashError) showToast(initial.hashError, 'error')
    // run once after mount; `initial` is from useState lazy initializer (stable)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // diagnostics が更新されたら linter を強制再実行してマーカーを最新化する
  useEffect(() => {
    const view = editorViewRef.current
    if (view) forceLinting(view)
  }, [diagnostics])

  const errorCount = diagnostics.filter((d) => d.severity === 'error').length
  const warnCount = diagnostics.filter((d) => d.severity === 'warning').length
  const historyCount = history.autoSnaps.length + history.manualSnaps.length

  const appStyle: CSSProperties = {
    '--editor-font-size': `${settings.fontSize}px`,
  } as CSSProperties

  return (
    <div className="app" data-theme={colorScheme} style={appStyle}>
      {pwaUpdate.needRefresh && (
        <div className="pwa-update-banner" role="status" aria-live="polite">
          <span>{t('pwaUpdateMessage')}</span>
          <button type="button" onClick={handlePwaReload}>{t('pwaReload')}</button>
        </div>
      )}

      <Toolbar
        fileMenuRef={fileMenuRef}
        fileMenuOpen={fileMenuOpen}
        setFileMenuOpen={setFileMenuOpen}
        onOpenFile={openFile}
        fileAccessSupported={fileHandle.supported}
        currentFileName={fileHandle.fileName}
        hasWritableFile={fileHandle.hasWritableHandle}
        onShowGallery={() => setShowGallery(true)}
        historyEnabled={settings.historyEnabled}
        historyCount={historyCount}
        onSaveToHistory={history.handleSaveToHistory}
        onShowHistory={() => setShowHistory(true)}
        onFormat={handleFormat}
        onLintFix={handleLintFix}
        wasmReady={wasmReady}
        exportMenuRef={exportMenuRef}
        exportMenuOpen={exportMenuOpen}
        setExportMenuOpen={setExportMenuOpen}
        exportApi={exportApi}
        svgContent={svgContent}
        onShowSettings={() => setShowSettings(true)}
        fileInputRef={fileInputRef}
        onFileChange={handleFileChange}
        locale={settings.locale}
      />

      <StatusBar wasmReady={wasmReady} wasmError={wasmError} errorCount={errorCount} warnCount={warnCount} locale={settings.locale} />

      <MobileTabBar mobileTab={mobileTab} setMobileTab={setMobileTab} locale={settings.locale} />

      <main className="main" ref={mainRef}>
        <EditorPane
          source={source}
          colorScheme={colorScheme}
          lineWrap={settings.lineWrap}
          splitRatio={splitRatio}
          hidden={mobileTab !== 'editor'}
          fullscreen={previewFullscreen}
          cursorLineExtension={svg.cursorLineExtension}
          tdslLinterExtension={tdslLinterExtension}
          onChange={handleEditorChange}
          onCreateEditor={(view) => { editorViewRef.current = view }}
          locale={settings.locale}
        />
        <div
          className="split-divider"
          role="separator"
          aria-orientation="vertical"
          aria-valuemin={Math.round(splitRatioMin * 100)}
          aria-valuemax={Math.round(splitRatioMax * 100)}
          aria-valuenow={Math.round(splitRatio * 100)}
          tabIndex={0}
          onMouseDown={handleDividerMouseDown}
          onKeyDown={handleDividerKeyDown}
          title={t('splitDividerTitle')}
          style={previewFullscreen ? { display: 'none' } : undefined}
        />
        <PreviewPanel
          hidden={mobileTab !== 'preview'}
          scale={settings.scale}
          onScaleChange={(value) => updateSetting('scale', value)}
          svgContent={svgContent}
          isStalePreview={isStalePreview}
          wasmReady={wasmReady}
          previewRef={svg.previewRef}
          svgContainerRef={svg.svgContainerRef}
          cursorGrab={svg.cursorGrab}
          resetPanZoom={svg.resetPanZoom}
          previewFullscreen={previewFullscreen}
          setPreviewFullscreen={setPreviewFullscreen}
          showLegend={svg.showLegend}
          setShowLegend={svg.setShowLegend}
          showFilterPanel={svg.showFilterPanel}
          setShowFilterPanel={svg.setShowFilterPanel}
          legendItems={svg.legendItems}
          allTags={svg.allTags}
          filterState={svg.filterState}
          setFilterState={svg.setFilterState}
          selectedItem={svg.selectedItem}
          setSelectedItem={svg.setSelectedItem}
          onMouseDown={svg.handlePreviewMouseDown}
          onMouseMove={svg.handlePreviewMouseMove}
          onMouseUp={svg.handlePreviewMouseUp}
          onMouseLeave={svg.handlePreviewMouseLeave}
          onDoubleClick={svg.handlePreviewDblClick}
          onClick={svg.handlePreviewClick}
          onKeyDown={svg.handlePreviewKeyDown}
          locale={settings.locale}
        />
      </main>

      <DiagnosticsPanel diagnostics={diagnostics} onDiagClick={handleDiagClick} locale={settings.locale} />

      <Tooltip tooltip={svg.tooltip} />

      {showSettings && (
        <SettingsModal
          onClose={() => setShowSettings(false)}
          settings={settings}
          updateSetting={updateSetting}
          systemScheme={systemScheme}
        />
      )}
      {showGallery && (
        <GalleryModal onClose={() => setShowGallery(false)} onSelect={handleGallerySelect} locale={settings.locale} />
      )}
      {showHistory && (
        <HistoryModal
          onClose={() => setShowHistory(false)}
          manualSnaps={history.manualSnaps}
          autoSnaps={history.autoSnaps}
          renamingId={history.renamingId}
          renameValue={history.renameValue}
          setRenameValue={history.setRenameValue}
          onRestore={history.handleRestoreSnapshot}
          onRenameStart={history.handleRenameStart}
          onRenameCommit={history.handleRenameCommit}
          onRenameCancel={history.cancelRename}
          onDeleteManual={history.handleDeleteManual}
          onClearAll={history.handleClearAllHistory}
          locale={settings.locale}
        />
      )}
      {confirmState && <ConfirmModal state={confirmState} />}
    </div>
  )
}

export default App
