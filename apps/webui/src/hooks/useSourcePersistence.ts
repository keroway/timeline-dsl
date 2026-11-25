import { type RefObject, useEffect, useRef } from 'react'
import { DEBOUNCE_MS } from '../lib/constants'
import { EDITOR_SOURCE_KEY } from '../lib/initialSource'

// エディタソースを localStorage へデバウンス自動保存する。
// File System Access API のファイルハンドルは永続化せず、上書き保存は明示的な保存操作でのみ行う。
// `skipAutoSaveRef` が立っている場合（テンプレート/ファイル/履歴の読み込み直後）は
// 1 回スキップする。自動保存を OFF にした場合は既存の保存を削除する。
// 保存に失敗した場合は `onSaveError` で通知する（連続失敗中は最初の 1 回のみ。
// 成功すると再び通知対象に戻る）。自動保存 OFF 時の削除に失敗した場合は
// `onClearError` で通知する。
export function useSourcePersistence(
  source: string,
  autoSaveEnabled: boolean,
  skipAutoSaveRef: RefObject<boolean>,
  onSaveError?: () => void,
  onClearError?: () => void
): void {
  const onSaveErrorRef = useRef(onSaveError)
  const onClearErrorRef = useRef(onClearError)
  const failingRef = useRef(false)

  useEffect(() => {
    onSaveErrorRef.current = onSaveError
    onClearErrorRef.current = onClearError
  }, [onSaveError, onClearError])

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
        failingRef.current = false
      } catch {
        // quota exceeded or private browsing: 連続失敗中は 1 回だけ通知する
        if (!failingRef.current) {
          failingRef.current = true
          onSaveErrorRef.current?.()
        }
      }
    }, DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [source, autoSaveEnabled, skipAutoSaveRef])

  // When autoSaveEnabled is turned OFF, remove the stored source
  useEffect(() => {
    if (!autoSaveEnabled) {
      failingRef.current = false
      try {
        localStorage.removeItem(EDITOR_SOURCE_KEY)
      } catch {
        // 削除できないと保存済みソースが次回起動時に復元されるため通知する
        onClearErrorRef.current?.()
      }
    }
  }, [autoSaveEnabled])
}
