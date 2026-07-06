// 設定モーダルに表示するキーボードショートカット一覧。

import type { Translator } from '../lib/i18n'

export function getShortcuts(t: Translator) {
  return [
    { key: 'Ctrl/Cmd + S', desc: t('shortcutSave') },
    { key: 'Ctrl/Cmd + Shift + F', desc: t('shortcutFormat') },
    { key: 'Ctrl/Cmd + F', desc: t('shortcutSearch') },
    { key: 'Escape', desc: t('shortcutEscape') },
    { key: 'Ctrl/Cmd + Enter', desc: t('shortcutNextSuggestion') },
    { key: 'Ctrl/Cmd + G', desc: t('shortcutNextMatch') },
    { key: 'Ctrl/Cmd + Z', desc: t('shortcutUndo') },
    { key: 'Ctrl/Cmd + Shift + Z', desc: t('shortcutRedo') },
    { key: 'Tab / Space', desc: t('shortcutSelectSnippet') },
    { key: 'Ctrl/Cmd + Space', desc: t('shortcutShowCompletions') },
    { key: '? (outside editor)', desc: t('shortcutOpenSettings') },
  ]
}
