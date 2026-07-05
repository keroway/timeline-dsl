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
