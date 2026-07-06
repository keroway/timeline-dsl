// ─── Typed i18n foundation ────────────────────────────────────────────────────
// Supported locales. Extend here when adding new languages.
export type Locale = 'ja' | 'en'

export const SUPPORTED_LOCALES: readonly Locale[] = ['ja', 'en']
export const DEFAULT_LOCALE: Locale = 'ja'

// ─── Dictionary shape ─────────────────────────────────────────────────────────
// Every key must be present in all locale dictionaries (enforced by tests).
// Values may contain `{name}` placeholders; use `t.fmt(key, { name })`.
export type Dictionary = {
  // App / toast
  appFormatFailed: string
  appAlreadyFormatted: string
  appFormatted: string
  appFormattedCommentWarning: string
  appLintFixFailed: string
  appLintFixNoIssues: string
  appLintFixCommentConfirm: string
  appLintFixConfirm: string
  appLintFixed: string
  appLintFixedCommentWarning: string
  fileAccessUnsupported: string
  fileAccessDownloadFallback: string
  fileAccessSaved: string
  fileAccessSavedAs: string
  fileAccessSaveFailed: string
  pwaOfflineReady: string
  pwaUpdateAvailable: string
  pwaUpdateMessage: string
  pwaReload: string
  pwaRegistrationFailed: string
  pwaNetworkOffline: string
  pwaNetworkOnline: string

  // Split divider
  splitDividerTitle: string

  // Toolbar — file menu
  toolbarFileMenu: string
  toolbarNew: string
  toolbarOpen: string
  toolbarFileUnsupported: string
  toolbarCurrentFile: string
  toolbarNoWritableFile: string
  toolbarGallery: string
  toolbarSaveHistory: string
  toolbarHistory: string
  toolbarExportMenu: string
  toolbarExportSvg: string
  toolbarExportPng: string
  toolbarExportHtml: string
  toolbarExportPdf: string
  toolbarFormat: string
  toolbarLintFix: string
  toolbarSettings: string

  // Settings modal
  settingsTitle: string
  settingsClose: string
  settingsTheme: string
  settingsThemeAuto: string
  settingsThemeLight: string
  settingsThemeDark: string
  settingsFontSize: string
  settingsLineWrap: string
  settingsLineWrapOn: string
  settingsLineWrapOff: string
  settingsScale: string
  settingsScaleAuto: string
  settingsOrientation: string
  settingsOrientationHorizontal: string
  settingsOrientationVertical: string
  settingsGrid: string
  settingsGridNone: string
  settingsGridDecade: string
  settingsGridYear: string
  settingsGridMonth: string
  settingsSvgTheme: string
  settingsSvgThemeDefault: string
  settingsSvgThemeDark: string
  settingsSvgThemePrint: string
  settingsSvgThemePastel: string
  settingsShowEventLabels: string
  settingsShowEventLabelsOn: string
  settingsShowEventLabelsOff: string
  settingsPngBg: string
  settingsPngBgWhite: string
  settingsPngBgTransparent: string
  settingsHistory: string
  settingsHistoryOn: string
  settingsHistoryOff: string
  settingsAutoSave: string
  settingsAutoSaveOn: string
  settingsAutoSaveOff: string
  settingsLanguage: string

  // Preview panel
  previewScaleTitle: string
  previewReset: string
  previewResetTitle: string
  previewLegend: string
  previewLegendClose: string
  previewLegendTitle: string
  previewFilter: string
  previewFilterClose: string
  previewFilterTitle: string
  previewFilterLaneSection: string
  previewFilterTagSection: string
  previewFilterTagPlaceholder: string
  previewFullscreen: string
  previewFullscreenExit: string
  previewFullscreenTitle: string
  previewFullscreenExitTitle: string
  previewDetailClose: string

  // History modal
  historyTitle: string
  historyClose: string
  historyManualSection: string
  historyAutoSection: string  // {count} and {max} placeholders
  historyRestoreTitle: string
  historyRenameTitle: string
  historyDeleteTitle: string
  historyRenameCommit: string
  historyRenameCancel: string

  // Gallery modal
  galleryTitle: string
  galleryClose: string
  galleryNetworkNote: string
  galleryCliOnly: string
  galleryCliNoteItem: string

  // Status bar
  statusErrors: string   // {count} placeholder
  statusWarnings: string // {count} placeholder
  statusOk: string
  statusStale: string

  // Diagnostics panel
  diagnosticsNoErrors: string
  diagnosticsHeader: string

  // Export (hooks/useExport.ts)
  exportJsonIrFailed: string       // {msg}
  exportJsonIrIncompleteConfirm: string
  exportPngGenerateFailed: string
  exportSvgCopied: string
  exportSvgCopyFailed: string
  exportPngCopied: string
  exportPngCopyFailed: string
  exportMarkdownCopied: string
  exportMarkdownCopyFailed: string
  exportShareLinkCopied: string
  exportShareLinkCopyFailed: string
  exportShareLinkFailed: string
  exportPdfFailed: string
  exportPdfPrintHint: string

  // History (hooks/useHistorySnapshots.ts, history.ts)
  historySnapshotBeforeTemplate: string
  historySnapshotBeforeFileOpen: string
  historyAutoSnapshotLabel: string
  historyManualSnapshotPrefix: string
  historyManualSnapshotLabel: string  // {datetime}
  historySavedToHistory: string       // {label}
  historyClearedAll: string
  historyEmpty: string
  historyClearAll: string

  // App shell
  appFileOpenFailed: string  // {msg}

  // Toolbar — export menu
  toolbarExportDownloadSection: string
  toolbarExportTdsl: string
  toolbarExportJsonIr: string
  toolbarExportPngWhite: string
  toolbarExportPngTransparent: string
  toolbarExportClipboardSection: string
  toolbarCopySvg: string
  toolbarCopyPng: string
  toolbarCopyMarkdown: string
  toolbarCopyShareLink: string
  toolbarFileMenuTitle: string
  toolbarGalleryTitle: string
  toolbarAbout: string
  toolbarAboutTitle: string

  // Preview panel — detail panel
  previewLabel: string
  previewDetailTitle: string
  previewDetailName: string
  previewDetailType: string
  previewDetailLane: string
  previewDetailSource: string
  previewDetailInfo: string
  previewStaleBadge: string
  previewPlaceholderNoPreview: string
  previewPlaceholderLoading: string
  previewEmptyValue: string

  // Settings modal — extra strings
  settingsThemeAutoTitle: string  // {scheme}
  settingsThemeDarkLabel: string
  settingsThemeLightLabel: string
  settingsSvgPreviewSection: string
  settingsShowEventLabelsTitle: string
  settingsAutoSaveTitle: string
  settingsAutoSaveOnHint: string
  settingsAutoSaveOffHint: string
  settingsHistoryTitle: string
  settingsHistoryOnHint: string
  settingsHistoryOffHint: string
  settingsLocaleJa: string
  settingsLocaleEn: string
  settingsShortcutsSection: string

  // Status bar
  statusInitializing: string
  statusWasmInitError: string  // {msg}

  // Mobile tab bar
  mobileTabEditor: string
  mobileTabPreview: string

  // Toast
  toastCloseLabel: string

  // Editor completions (editor/completions.ts)
  completionTimelineDetail: string
  completionLaneDetail: string
  completionSpanDetail: string
  completionEventDetail: string
  completionEventRangeDetail: string
  completionImportDetail: string
  completionMapDetail: string
  completionQueryDetail: string
  completionColorMapDetail: string

  // Editor shortcuts (editor/shortcuts.ts)
  shortcutSave: string
  shortcutFormat: string
  shortcutSearch: string
  shortcutEscape: string
  shortcutNextSuggestion: string
  shortcutNextMatch: string
  shortcutUndo: string
  shortcutRedo: string
  shortcutSelectSnippet: string
  shortcutShowCompletions: string
  shortcutOpenSettings: string
}

// ─── Japanese dictionary ──────────────────────────────────────────────────────
const ja: Dictionary = {
  // App / toast
  appFormatFailed: '整形に失敗しました: {msg}',
  appAlreadyFormatted: '既に整形済みです',
  appFormatted: '整形しました',
  appFormattedCommentWarning: '整形しました（コメントは保持されません）',
  appLintFixFailed: 'Lint fix に失敗しました: {msg}',
  appLintFixNoIssues: '自動修正可能な lint 問題はありません',
  appLintFixCommentConfirm:
    'lint --fix を適用します。コメントとフォーマットは保持されません。続行しますか？',
  appLintFixConfirm: 'lint --fix を適用します。フォーマットも再整形されます。続行しますか？',
  appLintFixed: 'lint --fix を適用しました',
  appLintFixedCommentWarning: 'lint --fix を適用しました（コメントは保持されません）',
  fileAccessUnsupported: 'このブラウザは直接オープン/上書き保存に非対応です。従来のファイル選択とダウンロード保存を使用します。',
  fileAccessDownloadFallback: '直接保存に非対応のため .tdsl をダウンロードしました',
  fileAccessSaved: '{name} に保存しました',
  fileAccessSavedAs: '{name} として保存しました',
  fileAccessSaveFailed: '保存に失敗しました: {msg}',
  pwaOfflineReady: 'オフライン起動の準備が完了しました',
  pwaUpdateAvailable: '新しいバージョンを利用できます',
  pwaUpdateMessage: '新しいバージョンを利用できます。再読み込みして更新してください。',
  pwaReload: '再読み込み',
  pwaRegistrationFailed: 'Service Worker の登録に失敗しました: {msg}',
  pwaNetworkOffline:
    'オフラインです。静的 DSL の編集・プレビューは継続できますが、Wikidata インポートは利用できません。',
  pwaNetworkOnline: 'オンラインに復帰しました',

  // Split divider
  splitDividerTitle: 'ドラッグまたは矢印キーで分割幅を調整',

  // Toolbar
  toolbarFileMenu: 'ファイル',
  toolbarNew: '新規',
  toolbarOpen: '開く',
  toolbarFileUnsupported: '直接オープン/上書き保存はこのブラウザでは非対応です',
  toolbarCurrentFile: '現在のファイル: {name}',
  toolbarNoWritableFile: '上書き先ファイルは未選択です',
  toolbarGallery: 'ギャラリー',
  toolbarSaveHistory: '履歴に保存',
  toolbarHistory: '履歴',
  toolbarExportMenu: 'エクスポート',
  toolbarExportSvg: 'SVG をダウンロード',
  toolbarExportPng: 'PNG をダウンロード',
  toolbarExportHtml: 'HTML をダウンロード',
  toolbarExportPdf: 'PDF として印刷',
  toolbarFormat: '整形',
  toolbarLintFix: 'Lint 修正',
  toolbarSettings: '設定',

  // Settings modal
  settingsTitle: '設定',
  settingsClose: '設定を閉じる',
  settingsTheme: 'テーマ',
  settingsThemeAuto: '自動',
  settingsThemeLight: 'ライト',
  settingsThemeDark: 'ダーク',
  settingsFontSize: 'フォントサイズ',
  settingsLineWrap: '折り返し',
  settingsLineWrapOn: 'ON',
  settingsLineWrapOff: 'OFF',
  settingsScale: 'スケール（ピクセル/年）',
  settingsScaleAuto: '自動',
  settingsOrientation: '向き',
  settingsOrientationHorizontal: '横',
  settingsOrientationVertical: '縦',
  settingsGrid: 'グリッド',
  settingsGridNone: 'なし',
  settingsGridDecade: '10年',
  settingsGridYear: '年',
  settingsGridMonth: '月',
  settingsSvgTheme: 'SVGテーマ',
  settingsSvgThemeDefault: 'デフォルト',
  settingsSvgThemeDark: 'ダーク',
  settingsSvgThemePrint: '印刷',
  settingsSvgThemePastel: 'パステル',
  settingsShowEventLabels: 'イベントラベル常時表示',
  settingsShowEventLabelsOn: 'ON',
  settingsShowEventLabelsOff: 'OFF',
  settingsPngBg: 'PNG背景',
  settingsPngBgWhite: '白',
  settingsPngBgTransparent: '透明',
  settingsHistory: '履歴',
  settingsHistoryOn: 'ON',
  settingsHistoryOff: 'OFF',
  settingsAutoSave: '自動保存',
  settingsAutoSaveOn: 'ON',
  settingsAutoSaveOff: 'OFF',
  settingsLanguage: '言語',

  // Preview panel
  previewScaleTitle: 'プレビューのスケール（ピクセル/年）',
  previewReset: 'リセット',
  previewResetTitle: 'ビューをリセット（ダブルクリックでも可）',
  previewLegend: '凡例',
  previewLegendClose: '凡例 ✕',
  previewLegendTitle: '凡例を表示/非表示',
  previewFilter: 'フィルタ',
  previewFilterClose: 'フィルタ ✕',
  previewFilterTitle: 'フィルタパネルを表示/非表示',
  previewFilterLaneSection: 'レーン',
  previewFilterTagSection: 'タグ',
  previewFilterTagPlaceholder: 'タグで検索',
  previewFullscreen: '⛶',
  previewFullscreenExit: '✕ 全画面',
  previewFullscreenTitle: '全画面モードでプレビュー',
  previewFullscreenExitTitle: '全画面モードを終了（Escape）',
  previewDetailClose: '✕',

  // History modal
  historyTitle: '履歴',
  historyClose: '履歴を閉じる',
  historyManualSection: '手動保存',
  historyAutoSection: '自動スナップショット（最大 {count}/{max} 件）',
  historyRestoreTitle: 'このスナップショットを復元',
  historyRenameTitle: '名前を変更',
  historyDeleteTitle: '削除',
  historyRenameCommit: '確定',
  historyRenameCancel: 'キャンセル',

  // Gallery modal
  galleryTitle: 'テンプレートギャラリー',
  galleryClose: 'ギャラリーを閉じる',
  galleryNetworkNote:
    'ネットワーク必須テンプレートは CLI 専用・構文リファレンスです。WebUI では読み込めますが、import wikidata はオフライン診断エラーになります。',
  galleryCliOnly: 'CLI専用',
  galleryCliNoteItem:
    'Wikidata API が必要なため、WebUI ではプレビュー実行せず CLI で利用してください。',

  // Status bar
  statusErrors: 'エラー {count}',
  statusWarnings: '警告 {count}',
  statusOk: '問題なし',
  statusStale: '更新中…',

  // Diagnostics panel
  diagnosticsNoErrors: 'エラーはありません',
  diagnosticsHeader: '診断結果',

  // Export
  exportJsonIrFailed: 'JSON IR の生成に失敗しました: {msg}',
  exportJsonIrIncompleteConfirm:
    'import / map ブロックは WebUI では解決されないため、この JSON IR にインポート由来のアイテムは含まれません。完全な IR は CLI の tdsl build で取得できます。\n\n静的アイテムのみの JSON IR を保存しますか？',
  exportPngGenerateFailed: 'PNG の生成に失敗しました',
  exportSvgCopied: 'SVG をコピーしました',
  exportSvgCopyFailed: 'SVG のコピーに失敗しました',
  exportPngCopied: 'PNG をコピーしました',
  exportPngCopyFailed: 'PNG のコピーに失敗しました',
  exportMarkdownCopied: 'Markdown をコピーしました',
  exportMarkdownCopyFailed: 'Markdown のコピーに失敗しました',
  exportShareLinkCopied: 'Share link をコピーしました',
  exportShareLinkCopyFailed: 'Share link のコピーに失敗しました',
  exportShareLinkFailed: 'Share link の生成に失敗しました',
  exportPdfFailed: 'PDF の生成に失敗しました',
  exportPdfPrintHint: '印刷ダイアログで「PDF に保存」を選択してください',

  // History
  historySnapshotBeforeTemplate: 'テンプレートロード前',
  historySnapshotBeforeFileOpen: 'ファイルオープン前',
  historyAutoSnapshotLabel: '自動保存',
  historyManualSnapshotPrefix: '手動保存',
  historyManualSnapshotLabel: '手動保存 — {datetime}',
  historySavedToHistory: '履歴に保存しました: {label}',
  historyClearedAll: '履歴を全件削除しました',
  historyEmpty: '履歴はありません',
  historyClearAll: '全件削除',

  // App shell
  appFileOpenFailed: 'ファイルを開けませんでした: {msg}',

  // Toolbar — export menu
  toolbarExportDownloadSection: 'ダウンロード',
  toolbarExportTdsl: '.tdsl 保存',
  toolbarExportJsonIr: 'JSON IR 保存',
  toolbarExportPngWhite: 'PNG 保存（白背景）',
  toolbarExportPngTransparent: 'PNG 保存（透過）',
  toolbarExportClipboardSection: 'クリップボードへコピー',
  toolbarCopySvg: 'SVG をコピー',
  toolbarCopyPng: 'PNG をコピー',
  toolbarCopyMarkdown: 'Markdown をコピー',
  toolbarCopyShareLink: 'Share link をコピー',
  toolbarFileMenuTitle: 'ファイルメニュー',
  toolbarGalleryTitle: 'テンプレートギャラリー',
  toolbarAbout: 'About',
  toolbarAboutTitle: 'ランディングページ・ドキュメント',

  // Preview panel — detail panel
  previewLabel: '年表プレビュー',
  previewDetailTitle: '選択中アイテムの詳細',
  previewDetailName: '名前',
  previewDetailType: '種類',
  previewDetailLane: 'レーン',
  previewDetailSource: '出典',
  previewDetailInfo: '情報',
  previewStaleBadge: '直前の成功時プレビューを表示中',
  previewPlaceholderNoPreview: 'プレビューなし（エラーを確認してください）',
  previewPlaceholderLoading: '読み込み中...',
  previewEmptyValue: '—',

  // Settings modal — extra strings
  settingsThemeAutoTitle: 'OS の設定に追従（現在: {scheme}）',
  settingsThemeDarkLabel: 'ダーク',
  settingsThemeLightLabel: 'ライト',
  settingsSvgPreviewSection: 'SVG プレビュー設定',
  settingsShowEventLabelsTitle:
    'Event / event_range のラベルをドット・バー近傍に常時描画します（一覧表示・印刷向け）',
  settingsAutoSaveTitle: '編集内容をブラウザに自動保存します（リロード後も復元）',
  settingsAutoSaveOnHint: 'リロード後に復元されます',
  settingsAutoSaveOffHint: '保存しません（オフ時は既存の保存を削除）',
  settingsHistoryTitle: 'テンプレートロード・ファイルオープン・5分毎に自動スナップショットを保存',
  settingsHistoryOnHint: '自動スナップショット有効（最大5件）',
  settingsHistoryOffHint: '無効（既存履歴は保持）',
  settingsLocaleJa: '日本語',
  settingsLocaleEn: 'English',
  settingsShortcutsSection: 'キーボードショートカット',

  // Status bar
  statusInitializing: 'WASM を初期化中...',
  statusWasmInitError: 'WASM 初期化エラー: {msg}',

  // Mobile tab bar
  mobileTabEditor: 'エディタ',
  mobileTabPreview: 'プレビュー',

  // Toast
  toastCloseLabel: '通知を閉じる',

  // Editor completions
  completionTimelineDetail: '年表ブロック',
  completionLaneDetail: 'レーン定義',
  completionSpanDetail: 'スパン',
  completionEventDetail: 'イベント',
  completionEventRangeDetail: 'イベント範囲',
  completionImportDetail: 'Wikidataインポート',
  completionMapDetail: 'マッピング',
  completionQueryDetail: 'SPARQLクエリ',
  completionColorMapDetail: 'タグ→色マッピング',

  // Editor shortcuts
  shortcutSave: '.tdsl をダウンロード',
  shortcutFormat: 'エディタ内容を整形',
  shortcutSearch: '検索・置換パネルを開く',
  shortcutEscape: '検索パネルを閉じる / 全画面モードを終了',
  shortcutNextSuggestion: '次の候補へ',
  shortcutNextMatch: '次の一致へ',
  shortcutUndo: '元に戻す',
  shortcutRedo: 'やり直す',
  shortcutSelectSnippet: 'スニペット候補を選択',
  shortcutShowCompletions: '補完候補を表示',
  shortcutOpenSettings: '設定を開く',
}

// ─── English dictionary ───────────────────────────────────────────────────────
const en: Dictionary = {
  // App / toast
  appFormatFailed: 'Format failed: {msg}',
  appAlreadyFormatted: 'Already formatted',
  appFormatted: 'Formatted',
  appFormattedCommentWarning: 'Formatted (comments are not preserved)',
  appLintFixFailed: 'Lint fix failed: {msg}',
  appLintFixNoIssues: 'No auto-fixable lint issues',
  appLintFixCommentConfirm:
    'Apply lint --fix? Comments and formatting will not be preserved. Continue?',
  appLintFixConfirm: 'Apply lint --fix? Formatting will also be re-applied. Continue?',
  appLintFixed: 'Lint --fix applied',
  appLintFixedCommentWarning: 'Lint --fix applied (comments are not preserved)',
  fileAccessUnsupported: 'This browser does not support direct open/overwrite save. Falling back to file input and downloads.',
  fileAccessDownloadFallback: 'Direct save is unsupported, so the .tdsl file was downloaded.',
  fileAccessSaved: 'Saved to {name}',
  fileAccessSavedAs: 'Saved as {name}',
  fileAccessSaveFailed: 'Save failed: {msg}',
  pwaOfflineReady: 'Ready for offline launch',
  pwaUpdateAvailable: 'A new version is available',
  pwaUpdateMessage: 'A new version is available. Reload to update.',
  pwaReload: 'Reload',
  pwaRegistrationFailed: 'Service Worker registration failed: {msg}',
  pwaNetworkOffline:
    'You are offline. Static DSL editing and preview continue to work, but Wikidata imports are unavailable.',
  pwaNetworkOnline: 'Back online',

  // Split divider
  splitDividerTitle: 'Drag or use arrow keys to adjust split width',

  // Toolbar
  toolbarFileMenu: 'File',
  toolbarNew: 'New',
  toolbarOpen: 'Open',
  toolbarFileUnsupported: 'Direct open/overwrite save is not supported by this browser',
  toolbarCurrentFile: 'Current file: {name}',
  toolbarNoWritableFile: 'No writable file is selected',
  toolbarGallery: 'Gallery',
  toolbarSaveHistory: 'Save to History',
  toolbarHistory: 'History',
  toolbarExportMenu: 'Export',
  toolbarExportSvg: 'Download SVG',
  toolbarExportPng: 'Download PNG',
  toolbarExportHtml: 'Download HTML',
  toolbarExportPdf: 'Print as PDF',
  toolbarFormat: 'Format',
  toolbarLintFix: 'Lint Fix',
  toolbarSettings: 'Settings',

  // Settings modal
  settingsTitle: 'Settings',
  settingsClose: 'Close settings',
  settingsTheme: 'Theme',
  settingsThemeAuto: 'Auto',
  settingsThemeLight: 'Light',
  settingsThemeDark: 'Dark',
  settingsFontSize: 'Font Size',
  settingsLineWrap: 'Line Wrap',
  settingsLineWrapOn: 'ON',
  settingsLineWrapOff: 'OFF',
  settingsScale: 'Scale (px/year)',
  settingsScaleAuto: 'Auto',
  settingsOrientation: 'Orientation',
  settingsOrientationHorizontal: 'Horizontal',
  settingsOrientationVertical: 'Vertical',
  settingsGrid: 'Grid',
  settingsGridNone: 'None',
  settingsGridDecade: 'Decade',
  settingsGridYear: 'Year',
  settingsGridMonth: 'Month',
  settingsSvgTheme: 'SVG Theme',
  settingsSvgThemeDefault: 'Default',
  settingsSvgThemeDark: 'Dark',
  settingsSvgThemePrint: 'Print',
  settingsSvgThemePastel: 'Pastel',
  settingsShowEventLabels: 'Always-on Event Labels',
  settingsShowEventLabelsOn: 'ON',
  settingsShowEventLabelsOff: 'OFF',
  settingsPngBg: 'PNG Background',
  settingsPngBgWhite: 'White',
  settingsPngBgTransparent: 'Transparent',
  settingsHistory: 'History',
  settingsHistoryOn: 'ON',
  settingsHistoryOff: 'OFF',
  settingsAutoSave: 'Auto Save',
  settingsAutoSaveOn: 'ON',
  settingsAutoSaveOff: 'OFF',
  settingsLanguage: 'Language',

  // Preview panel
  previewScaleTitle: 'Preview scale (px/year)',
  previewReset: 'Reset',
  previewResetTitle: 'Reset view (or double-click)',
  previewLegend: 'Legend',
  previewLegendClose: 'Legend ✕',
  previewLegendTitle: 'Toggle legend',
  previewFilter: 'Filter',
  previewFilterClose: 'Filter ✕',
  previewFilterTitle: 'Toggle filter panel',
  previewFilterLaneSection: 'Lanes',
  previewFilterTagSection: 'Tags',
  previewFilterTagPlaceholder: 'Search by tag',
  previewFullscreen: '⛶',
  previewFullscreenExit: '✕ Fullscreen',
  previewFullscreenTitle: 'Preview in fullscreen',
  previewFullscreenExitTitle: 'Exit fullscreen (Escape)',
  previewDetailClose: '✕',

  // History modal
  historyTitle: 'History',
  historyClose: 'Close history',
  historyManualSection: 'Manual saves',
  historyAutoSection: 'Auto snapshots ({count}/{max} max)',
  historyRestoreTitle: 'Restore this snapshot',
  historyRenameTitle: 'Rename',
  historyDeleteTitle: 'Delete',
  historyRenameCommit: 'OK',
  historyRenameCancel: 'Cancel',

  // Gallery modal
  galleryTitle: 'Template Gallery',
  galleryClose: 'Close gallery',
  galleryNetworkNote:
    'Network-required templates are CLI-only syntax references. They load in WebUI but import wikidata will show offline diagnostic errors.',
  galleryCliOnly: 'CLI only',
  galleryCliNoteItem:
    'Requires Wikidata API. Use via CLI; WebUI preview will not execute imports.',

  // Status bar
  statusErrors: '{count} error(s)',
  statusWarnings: '{count} warning(s)',
  statusOk: 'No issues',
  statusStale: 'Updating…',

  // Diagnostics panel
  diagnosticsNoErrors: 'No errors',
  diagnosticsHeader: 'Diagnostics',

  // Export
  exportJsonIrFailed: 'Failed to generate JSON IR: {msg}',
  exportJsonIrIncompleteConfirm:
    'import / map blocks are not resolved in WebUI, so imported items will not be included in this JSON IR. The complete IR is available via the CLI\'s tdsl build.\n\nSave the JSON IR with static items only?',
  exportPngGenerateFailed: 'Failed to generate PNG',
  exportSvgCopied: 'Copied SVG',
  exportSvgCopyFailed: 'Failed to copy SVG',
  exportPngCopied: 'Copied PNG',
  exportPngCopyFailed: 'Failed to copy PNG',
  exportMarkdownCopied: 'Copied Markdown',
  exportMarkdownCopyFailed: 'Failed to copy Markdown',
  exportShareLinkCopied: 'Copied share link',
  exportShareLinkCopyFailed: 'Failed to copy share link',
  exportShareLinkFailed: 'Failed to generate share link',
  exportPdfFailed: 'Failed to generate PDF',
  exportPdfPrintHint: 'Choose "Save as PDF" in the print dialog',

  // History
  historySnapshotBeforeTemplate: 'Before template load',
  historySnapshotBeforeFileOpen: 'Before file open',
  historyAutoSnapshotLabel: 'Auto save',
  historyManualSnapshotPrefix: 'Manual save',
  historyManualSnapshotLabel: 'Manual save — {datetime}',
  historySavedToHistory: 'Saved to history: {label}',
  historyClearedAll: 'Cleared all history',
  historyEmpty: 'No history',
  historyClearAll: 'Clear all',

  // App shell
  appFileOpenFailed: 'Could not open file: {msg}',

  // Toolbar — export menu
  toolbarExportDownloadSection: 'Download',
  toolbarExportTdsl: 'Save .tdsl',
  toolbarExportJsonIr: 'Save JSON IR',
  toolbarExportPngWhite: 'Save PNG (white background)',
  toolbarExportPngTransparent: 'Save PNG (transparent)',
  toolbarExportClipboardSection: 'Copy to clipboard',
  toolbarCopySvg: 'Copy SVG',
  toolbarCopyPng: 'Copy PNG',
  toolbarCopyMarkdown: 'Copy Markdown',
  toolbarCopyShareLink: 'Copy share link',
  toolbarFileMenuTitle: 'File menu',
  toolbarGalleryTitle: 'Template gallery',
  toolbarAbout: 'About',
  toolbarAboutTitle: 'Landing page / documentation',

  // Preview panel — detail panel
  previewLabel: 'Timeline preview',
  previewDetailTitle: 'Selected item details',
  previewDetailName: 'Name',
  previewDetailType: 'Type',
  previewDetailLane: 'Lane',
  previewDetailSource: 'Source',
  previewDetailInfo: 'Info',
  previewStaleBadge: 'Showing last successful preview',
  previewPlaceholderNoPreview: 'No preview (check for errors)',
  previewPlaceholderLoading: 'Loading...',
  previewEmptyValue: '—',

  // Settings modal — extra strings
  settingsThemeAutoTitle: 'Follow OS setting (currently: {scheme})',
  settingsThemeDarkLabel: 'Dark',
  settingsThemeLightLabel: 'Light',
  settingsSvgPreviewSection: 'SVG preview settings',
  settingsShowEventLabelsTitle:
    'Always render Event / event_range labels near the dot/bar (useful for lists/printing)',
  settingsAutoSaveTitle: 'Automatically save edits to the browser (restored after reload)',
  settingsAutoSaveOnHint: 'Restored after reload',
  settingsAutoSaveOffHint: 'Not saved (turning off deletes any existing save)',
  settingsHistoryTitle: 'Auto-snapshot on template load, file open, and every 5 minutes',
  settingsHistoryOnHint: 'Auto snapshots enabled (5 max)',
  settingsHistoryOffHint: 'Disabled (existing history is kept)',
  settingsLocaleJa: '日本語',
  settingsLocaleEn: 'English',
  settingsShortcutsSection: 'Keyboard shortcuts',

  // Status bar
  statusInitializing: 'Initializing WASM...',
  statusWasmInitError: 'WASM initialization error: {msg}',

  // Mobile tab bar
  mobileTabEditor: 'Editor',
  mobileTabPreview: 'Preview',

  // Toast
  toastCloseLabel: 'Dismiss notification',

  // Editor completions
  completionTimelineDetail: 'Timeline block',
  completionLaneDetail: 'Lane definition',
  completionSpanDetail: 'Span',
  completionEventDetail: 'Event',
  completionEventRangeDetail: 'Event range',
  completionImportDetail: 'Wikidata import',
  completionMapDetail: 'Mapping',
  completionQueryDetail: 'SPARQL query',
  completionColorMapDetail: 'Tag→color mapping',

  // Editor shortcuts
  shortcutSave: 'Download .tdsl',
  shortcutFormat: 'Format editor content',
  shortcutSearch: 'Open search/replace panel',
  shortcutEscape: 'Close search panel / exit fullscreen mode',
  shortcutNextSuggestion: 'Next suggestion',
  shortcutNextMatch: 'Next match',
  shortcutUndo: 'Undo',
  shortcutRedo: 'Redo',
  shortcutSelectSnippet: 'Select snippet suggestion',
  shortcutShowCompletions: 'Show completions',
  shortcutOpenSettings: 'Open settings',
}

// ─── Dictionaries map ─────────────────────────────────────────────────────────
const dictionaries: Record<Locale, Dictionary> = { ja, en }

// ─── Translator ───────────────────────────────────────────────────────────────
export type Translator = {
  /** Return the raw string for `key`. */
  (key: keyof Dictionary): string
  /** Return the string for `key` with `{name}` placeholders replaced. */
  fmt(key: keyof Dictionary, vars: Record<string, string | number>): string
}

export function createTranslator(locale: Locale): Translator {
  const dict = dictionaries[locale] ?? dictionaries[DEFAULT_LOCALE]

  function t(key: keyof Dictionary): string {
    return dict[key]
  }

  t.fmt = (key: keyof Dictionary, vars: Record<string, string | number>): string => {
    let s = dict[key]
    for (const [k, v] of Object.entries(vars)) {
      s = s.replaceAll(`{${k}}`, String(v))
    }
    return s
  }

  return t as Translator
}
