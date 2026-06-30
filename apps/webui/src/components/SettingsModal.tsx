import { useMemo, type ChangeEvent } from 'react'
import type { ColorScheme, Settings } from '../lib/settings'
import { createTranslator, SUPPORTED_LOCALES, type Locale } from '../lib/i18n'
import { SHORTCUTS } from '../editor/shortcuts'
import { useFocusTrap } from '../hooks/useFocusTrap'

type SettingsModalProps = {
  onClose: () => void
  settings: Settings
  updateSetting: <K extends keyof Settings>(key: K, value: Settings[K]) => void
  systemScheme: ColorScheme
}

export function SettingsModal({ onClose, settings, updateSetting, systemScheme }: SettingsModalProps) {
  const { theme: themePref, fontSize, lineWrap, scale, pngWhiteBg } = settings
  const dialogRef = useFocusTrap<HTMLDivElement>({ active: true, onEscape: onClose })
  const t = useMemo(() => createTranslator(settings.locale), [settings.locale])

  function handleFontSizeChange(e: ChangeEvent<HTMLSelectElement>) {
    updateSetting('fontSize', parseInt(e.target.value, 10))
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal modal-settings"
        onClick={(e) => e.stopPropagation()}
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-modal-title"
        tabIndex={-1}
      >
        <div className="modal-header">
          <span id="settings-modal-title">{t('settingsTitle')}</span>
          <button className="modal-close" onClick={onClose} aria-label={t('settingsClose')}>✕</button>
        </div>
        <div className="settings-body">
          <div className="settings-section">
            <div className="settings-label">{t('settingsTheme')}</div>
            <div className="settings-row" role="radiogroup" aria-label={t('settingsTheme')}>
              <button
                type="button"
                role="radio"
                aria-checked={themePref === 'auto'}
                className={`btn${themePref === 'auto' ? ' btn-active' : ''}`}
                onClick={() => updateSetting('theme', 'auto')}
                title={`OS の設定に追従（現在: ${systemScheme === 'dark' ? 'ダーク' : 'ライト'}）`}
              >
                {t('settingsThemeAuto')}
              </button>
              <button
                type="button"
                role="radio"
                aria-checked={themePref === 'light'}
                className={`btn${themePref === 'light' ? ' btn-active' : ''}`}
                onClick={() => updateSetting('theme', 'light')}
              >
                {t('settingsThemeLight')}
              </button>
              <button
                type="button"
                role="radio"
                aria-checked={themePref === 'dark'}
                className={`btn${themePref === 'dark' ? ' btn-active' : ''}`}
                onClick={() => updateSetting('theme', 'dark')}
              >
                {t('settingsThemeDark')}
              </button>
            </div>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsFontSize')}</div>
            <select
              className="toolbar-select"
              value={fontSize}
              onChange={handleFontSizeChange}
            >
              <option value={12}>12px</option>
              <option value={13}>13px</option>
              <option value={14}>14px</option>
              <option value={16}>16px</option>
              <option value={18}>18px</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsLineWrap')}</div>
            <button
              className={`btn${lineWrap ? ' btn-active' : ''}`}
              onClick={() => updateSetting('lineWrap', !lineWrap)}
            >
              {lineWrap ? t('settingsLineWrapOn') : t('settingsLineWrapOff')}
            </button>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsScale')}</div>
            <select
              className="toolbar-select"
              value={scale}
              onChange={(e) => updateSetting('scale', Number(e.target.value))}
            >
              <option value={0}>{t('settingsScaleAuto')}</option>
              <option value={0.5}>0.5×</option>
              <option value={1}>1×</option>
              <option value={2}>2×</option>
              <option value={4}>4×</option>
              <option value={8}>8×</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsPngBg')}</div>
            <div className="settings-row">
              <button
                className={`btn${pngWhiteBg ? ' btn-active' : ''}`}
                onClick={() => updateSetting('pngWhiteBg', true)}
              >
                {t('settingsPngBgWhite')}
              </button>
              <button
                className={`btn${!pngWhiteBg ? ' btn-active' : ''}`}
                onClick={() => updateSetting('pngWhiteBg', false)}
              >
                {t('settingsPngBgTransparent')}
              </button>
            </div>
          </div>
          <hr className="settings-divider" />
          <div className="settings-section">
            <div className="settings-label">SVG プレビュー設定</div>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsOrientation')}</div>
            <select
              className="toolbar-select"
              value={settings.svgOrientation}
              onChange={(e) => updateSetting('svgOrientation', e.target.value as Settings['svgOrientation'])}
            >
              <option value="horizontal">{t('settingsOrientationHorizontal')}</option>
              <option value="vertical">{t('settingsOrientationVertical')}</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsGrid')}</div>
            <select
              className="toolbar-select"
              value={settings.svgGrid}
              onChange={(e) => updateSetting('svgGrid', e.target.value as Settings['svgGrid'])}
            >
              <option value="none">{t('settingsGridNone')}</option>
              <option value="decade">{t('settingsGridDecade')}</option>
              <option value="year">{t('settingsGridYear')}</option>
              <option value="month">{t('settingsGridMonth')}</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsSvgTheme')}</div>
            <select
              className="toolbar-select"
              value={settings.svgTheme}
              onChange={(e) => updateSetting('svgTheme', e.target.value as Settings['svgTheme'])}
            >
              <option value="default">{t('settingsSvgThemeDefault')}</option>
              <option value="dark">{t('settingsSvgThemeDark')}</option>
              <option value="print">{t('settingsSvgThemePrint')}</option>
              <option value="pastel">{t('settingsSvgThemePastel')}</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsAutoSave')}</div>
            <div className="settings-row">
              <button
                className={`btn${settings.autoSaveEnabled ? ' btn-active' : ''}`}
                onClick={() => updateSetting('autoSaveEnabled', !settings.autoSaveEnabled)}
                title="編集内容をブラウザに自動保存します（リロード後も復元）"
              >
                {settings.autoSaveEnabled ? t('settingsAutoSaveOn') : t('settingsAutoSaveOff')}
              </button>
              <span className="settings-hint">
                {settings.autoSaveEnabled ? 'リロード後に復元されます' : '保存しません（オフ時は既存の保存を削除）'}
              </span>
            </div>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsHistory')}</div>
            <div className="settings-row">
              <button
                className={`btn${settings.historyEnabled ? ' btn-active' : ''}`}
                onClick={() => updateSetting('historyEnabled', !settings.historyEnabled)}
                title="テンプレートロード・ファイルオープン・5分毎に自動スナップショットを保存"
              >
                {settings.historyEnabled ? t('settingsHistoryOn') : t('settingsHistoryOff')}
              </button>
              <span className="settings-hint">
                {settings.historyEnabled ? '自動スナップショット有効（最大5件）' : '無効（既存履歴は保持）'}
              </span>
            </div>
          </div>
          <div className="settings-section">
            <div className="settings-label">{t('settingsLanguage')}</div>
            <select
              className="toolbar-select"
              value={settings.locale}
              onChange={(e) => updateSetting('locale', e.target.value as Locale)}
              aria-label={t('settingsLanguage')}
            >
              {SUPPORTED_LOCALES.map((loc) => (
                <option key={loc} value={loc}>
                  {loc === 'ja' ? '日本語' : 'English'}
                </option>
              ))}
            </select>
          </div>
          <hr className="settings-divider" />
          <div className="settings-section">
            <div className="settings-label">GitHub</div>
            <button
              type="button"
              className="btn"
              onClick={() => window.open('https://github.com/keroway/timeline-dsl', '_blank', 'noopener,noreferrer')}
            >
              keroway/timeline-dsl ↗
            </button>
          </div>
          <hr className="settings-divider" />
          <div className="settings-section">
            <div className="settings-label">キーボードショートカット</div>
            <table className="shortcuts-table">
              <tbody>
                {SHORTCUTS.map(({ key, desc }) => (
                  <tr key={key}>
                    <td><kbd className="kbd">{key}</kbd></td>
                    <td>{desc}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  )
}
