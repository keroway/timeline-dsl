import { useEffect, type RefObject } from 'react'
import { DEBOUNCE_MS } from '../lib/constants'
import { EDITOR_SOURCE_KEY } from '../lib/initialSource'

// エディタソースを localStorage へデバウンス自動保存する。
// File System Access API のファイルハンドルは永続化せず、上書き保存は明示的な保存操作でのみ行う。
// `skipAutoSaveRef` が立っている場合（テンプレート/ファイル/履歴の読み込み直後）は
// 1 回スキップする。自動保存を OFF にした場合は既存の保存を削除する。
export function useSourcePersistence(
  source: string,
  autoSaveEnabled: boolean,
  skipAutoSaveRef: RefObject<boolean>,
): void {
  // Auto-save editor source to LocalStorage (debounced, skips template/file loads)
  useEffect(() => {
    if (skipAutoSaveRef.current) {
      skipAutoSaveRef.current = false
      return
    }
    if (!autoSaveEnabled) return
    const timer = setTimeout(() => {
      try {
        localStorage.setItem(EDITOR_SOURCE_KEY, source)
      } catch {/* quota exceeded or private browsing */}
    }, DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [source, autoSaveEnabled, skipAutoSaveRef])

  // When autoSaveEnabled is turned OFF, remove the stored source
  useEffect(() => {
    if (!autoSaveEnabled) {
      try {
        localStorage.removeItem(EDITOR_SOURCE_KEY)
      } catch {/* ignore */}
    }
  }, [autoSaveEnabled])
}
