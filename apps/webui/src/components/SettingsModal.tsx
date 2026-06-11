import type { ChangeEvent } from 'react'
import type { ColorScheme, Settings } from '../lib/settings'
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
          <span id="settings-modal-title">設定</span>
          <button className="modal-close" onClick={onClose} aria-label="設定を閉じる">✕</button>
        </div>
        <div className="settings-body">
          <div className="settings-section">
            <div className="settings-label">テーマ</div>
            <div className="settings-row" role="radiogroup" aria-label="テーマ">
              <button
                type="button"
                role="radio"
                aria-checked={themePref === 'auto'}
                className={`btn${themePref === 'auto' ? ' btn-active' : ''}`}
                onClick={() => updateSetting('theme', 'auto')}
                title={`OS の設定に追従（現在: ${systemScheme === 'dark' ? 'ダーク' : 'ライト'}）`}
              >
                OS 追従
              </button>
              <button
                type="button"
                role="radio"
                aria-checked={themePref === 'light'}
                className={`btn${themePref === 'light' ? ' btn-active' : ''}`}
                onClick={() => updateSetting('theme', 'light')}
              >
                ライト
              </button>
              <button
                type="button"
                role="radio"
                aria-checked={themePref === 'dark'}
                className={`btn${themePref === 'dark' ? ' btn-active' : ''}`}
                onClick={() => updateSetting('theme', 'dark')}
              >
                ダーク
              </button>
            </div>
          </div>
          <div className="settings-section">
            <div className="settings-label">フォントサイズ</div>
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
            <div className="settings-label">行折り返し</div>
            <button
              className={`btn${lineWrap ? ' btn-active' : ''}`}
              onClick={() => updateSetting('lineWrap', !lineWrap)}
            >
              {lineWrap ? 'オン' : 'オフ'}
            </button>
          </div>
          <div className="settings-section">
            <div className="settings-label">スケール（ピクセル/年）</div>
            <select
              className="toolbar-select"
              value={scale}
              onChange={(e) => updateSetting('scale', Number(e.target.value))}
            >
              <option value={0}>Auto</option>
              <option value={0.5}>0.5×</option>
              <option value={1}>1×</option>
              <option value={2}>2×</option>
              <option value={4}>4×</option>
              <option value={8}>8×</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">PNG 背景色</div>
            <div className="settings-row">
              <button
                className={`btn${pngWhiteBg ? ' btn-active' : ''}`}
                onClick={() => updateSetting('pngWhiteBg', true)}
              >
                白背景
              </button>
              <button
                className={`btn${!pngWhiteBg ? ' btn-active' : ''}`}
                onClick={() => updateSetting('pngWhiteBg', false)}
              >
                透過
              </button>
            </div>
          </div>
          <hr className="settings-divider" />
          <div className="settings-section">
            <div className="settings-label">SVG プレビュー設定</div>
          </div>
          <div className="settings-section">
            <div className="settings-label">向き</div>
            <select
              className="toolbar-select"
              value={settings.svgOrientation}
              onChange={(e) => updateSetting('svgOrientation', e.target.value as Settings['svgOrientation'])}
            >
              <option value="horizontal">水平</option>
              <option value="vertical">垂直</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">グリッド密度</div>
            <select
              className="toolbar-select"
              value={settings.svgGrid}
              onChange={(e) => updateSetting('svgGrid', e.target.value as Settings['svgGrid'])}
            >
              <option value="none">なし</option>
              <option value="decade">10年</option>
              <option value="year">1年</option>
              <option value="month">月</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">SVG テーマ</div>
            <select
              className="toolbar-select"
              value={settings.svgTheme}
              onChange={(e) => updateSetting('svgTheme', e.target.value as Settings['svgTheme'])}
            >
              <option value="default">デフォルト</option>
              <option value="dark">ダーク</option>
              <option value="print">印刷</option>
              <option value="pastel">パステル</option>
            </select>
          </div>
          <div className="settings-section">
            <div className="settings-label">自動保存</div>
            <div className="settings-row">
              <button
                className={`btn${settings.autoSaveEnabled ? ' btn-active' : ''}`}
                onClick={() => updateSetting('autoSaveEnabled', !settings.autoSaveEnabled)}
                title="編集内容をブラウザに自動保存します（リロード後も復元）"
              >
                {settings.autoSaveEnabled ? 'オン' : 'オフ'}
              </button>
              <span className="settings-hint">
                {settings.autoSaveEnabled ? 'リロード後に復元されます' : '保存しません（オフ時は既存の保存を削除）'}
              </span>
            </div>
          </div>
          <div className="settings-section">
            <div className="settings-label">履歴スナップショット</div>
            <div className="settings-row">
              <button
                className={`btn${settings.historyEnabled ? ' btn-active' : ''}`}
                onClick={() => updateSetting('historyEnabled', !settings.historyEnabled)}
                title="テンプレートロード・ファイルオープン・5分毎に自動スナップショットを保存"
              >
                {settings.historyEnabled ? 'オン' : 'オフ'}
              </button>
              <span className="settings-hint">
                {settings.historyEnabled ? '自動スナップショット有効（最大5件）' : '無効（既存履歴は保持）'}
              </span>
            </div>
          </div>
          <hr className="settings-divider" />
          <div className="settings-section">
            <div className="settings-label">GitHub</div>
            <a
              className="btn"
              href="https://github.com/keroway/timeline-dsl"
              target="_blank"
              rel="noopener noreferrer"
            >
              keroway/timeline-dsl ↗
            </a>
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
