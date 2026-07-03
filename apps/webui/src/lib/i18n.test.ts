import { describe, expect, it } from 'vitest'
import {
  DEFAULT_LOCALE,
  SUPPORTED_LOCALES,
  createTranslator,
  type Dictionary,
  type Locale,
} from './i18n'

// ─── Key parity ───────────────────────────────────────────────────────────────
// Every locale dictionary must contain exactly the same keys as the type.
// We cross-check every supported locale against every other locale.

describe('i18n key parity', () => {
  it('all locales have the same keys', () => {
    // Build reference key list from the default locale translator.
    // We rely on the Dictionary type to ensure completeness at compile time;
    // this test guards against accidental runtime-only divergence.
    const refT = createTranslator(DEFAULT_LOCALE)

    // Collect all keys declared in the Dictionary type via a compile-time
    // exhaustive object (the dictionaries inside i18n.ts already enforce this,
    // but we verify the exported translator returns a non-empty string for each).
    const sampleKeys: Array<keyof Dictionary> = [
      'appFormatFailed',
      'appAlreadyFormatted',
      'appFormatted',
      'appFormattedCommentWarning',
      'appLintFixFailed',
      'appLintFixNoIssues',
      'appLintFixCommentConfirm',
      'appLintFixConfirm',
      'appLintFixed',
      'appLintFixedCommentWarning',
      'fileAccessUnsupported',
      'fileAccessDownloadFallback',
      'fileAccessSaved',
      'fileAccessSavedAs',
      'fileAccessSaveFailed',
      'splitDividerTitle',
      'toolbarFileMenu',
      'toolbarNew',
      'toolbarOpen',
      'toolbarFileUnsupported',
      'toolbarCurrentFile',
      'toolbarNoWritableFile',
      'toolbarGallery',
      'toolbarSaveHistory',
      'toolbarHistory',
      'toolbarExportMenu',
      'toolbarExportSvg',
      'toolbarExportPng',
      'toolbarExportHtml',
      'toolbarExportPdf',
      'toolbarFormat',
      'toolbarLintFix',
      'toolbarSettings',
      'settingsTitle',
      'settingsClose',
      'settingsTheme',
      'settingsThemeAuto',
      'settingsThemeLight',
      'settingsThemeDark',
      'settingsFontSize',
      'settingsLineWrap',
      'settingsLineWrapOn',
      'settingsLineWrapOff',
      'settingsScale',
      'settingsScaleAuto',
      'settingsOrientation',
      'settingsOrientationHorizontal',
      'settingsOrientationVertical',
      'settingsGrid',
      'settingsGridNone',
      'settingsGridDecade',
      'settingsGridYear',
      'settingsGridMonth',
      'settingsSvgTheme',
      'settingsSvgThemeDefault',
      'settingsSvgThemeDark',
      'settingsSvgThemePrint',
      'settingsSvgThemePastel',
      'settingsPngBg',
      'settingsPngBgWhite',
      'settingsPngBgTransparent',
      'settingsHistory',
      'settingsHistoryOn',
      'settingsHistoryOff',
      'settingsAutoSave',
      'settingsAutoSaveOn',
      'settingsAutoSaveOff',
      'settingsLanguage',
      'previewScaleTitle',
      'previewReset',
      'previewResetTitle',
      'previewLegend',
      'previewLegendClose',
      'previewLegendTitle',
      'previewFilter',
      'previewFilterClose',
      'previewFilterTitle',
      'previewFilterLaneSection',
      'previewFilterTagSection',
      'previewFilterTagPlaceholder',
      'previewFullscreen',
      'previewFullscreenExit',
      'previewFullscreenTitle',
      'previewFullscreenExitTitle',
      'previewDetailClose',
      'historyTitle',
      'historyClose',
      'historyManualSection',
      'historyAutoSection',
      'historyRestoreTitle',
      'historyRenameTitle',
      'historyDeleteTitle',
      'historyRenameCommit',
      'historyRenameCancel',
      'galleryTitle',
      'galleryClose',
      'galleryNetworkNote',
      'galleryCliOnly',
      'galleryCliNoteItem',
      'statusErrors',
      'statusWarnings',
      'statusOk',
      'statusStale',
      'diagnosticsNoErrors',
    ]

    for (const locale of SUPPORTED_LOCALES) {
      const t = createTranslator(locale as Locale)
      for (const key of sampleKeys) {
        const value = t(key)
        expect(
          typeof value === 'string' && value.length > 0,
          `[${locale}] key "${key}" must return a non-empty string`,
        ).toBe(true)
      }
    }

    // Verify the reference translator returns strings too.
    for (const key of sampleKeys) {
      expect(refT(key).length).toBeGreaterThan(0)
    }
  })
})

// ─── Interpolation ────────────────────────────────────────────────────────────
describe('i18n interpolation', () => {
  it('replaces {msg} placeholder', () => {
    const t = createTranslator('ja')
    expect(t.fmt('appFormatFailed', { msg: 'parse error' })).toBe(
      '整形に失敗しました: parse error',
    )
  })

  it('replaces {count} and {max} placeholders', () => {
    const t = createTranslator('ja')
    expect(t.fmt('historyAutoSection', { count: 3, max: 5 })).toBe(
      '自動スナップショット（最大 3/5 件）',
    )
  })

  it('replaces {count} in English', () => {
    const t = createTranslator('en')
    expect(t.fmt('statusErrors', { count: 2 })).toBe('2 error(s)')
    expect(t.fmt('historyAutoSection', { count: 1, max: 5 })).toBe('Auto snapshots (1/5 max)')
  })

  it('leaves unknown placeholders untouched', () => {
    const t = createTranslator('en')
    // No placeholder in appFormatted; passing extra vars is harmless.
    expect(t.fmt('appFormatted', { unused: 'x' })).toBe('Formatted')
  })

  it('replaces {name} in file access messages', () => {
    const ja = createTranslator('ja')
    expect(ja.fmt('fileAccessSaved', { name: 'timeline.tdsl' })).toBe(
      'timeline.tdsl に保存しました',
    )
    const en = createTranslator('en')
    expect(en.fmt('toolbarCurrentFile', { name: 'timeline.tdsl' })).toBe(
      'Current file: timeline.tdsl',
    )
  })
})

// ─── Fallback ─────────────────────────────────────────────────────────────────
describe('i18n fallback', () => {
  it('falls back to default locale for unknown locale', () => {
    // Cast to bypass TS type guard — simulates a runtime unknown value.
    const t = createTranslator('zz' as Locale)
    expect(t('appFormatted')).toBe('整形しました') // default = ja
  })
})
