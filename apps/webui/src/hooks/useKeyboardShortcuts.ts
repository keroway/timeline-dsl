import type { EditorView } from '@codemirror/view'
import {
  type Dispatch,
  type RefObject,
  type SetStateAction,
  useEffect,
} from 'react'

type Params = {
  editorViewRef: RefObject<EditorView | null>
  setShowSettings: Dispatch<SetStateAction<boolean>>
  setPreviewFullscreen: Dispatch<SetStateAction<boolean>>
  onSave: () => void
  onFormat: () => void
  source: string
  wasmReady: boolean
}

// グローバルキーボードショートカットを登録する:
// `?`（設定トグル）/ Escape（設定・全画面解除）/ Ctrl+S（保存）/ Ctrl+Shift+F（整形）。
export function useKeyboardShortcuts(params: Params): void {
  const {
    editorViewRef,
    setShowSettings,
    setPreviewFullscreen,
    onSave,
    onFormat,
    source,
    wasmReady,
  } = params

  // Global `?` key to toggle settings modal (only when editor doesn't have focus)
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (
        e.key === '?' &&
        !(e.target instanceof HTMLTextAreaElement) &&
        !editorViewRef.current?.hasFocus
      ) {
        e.preventDefault()
        setShowSettings((v) => !v)
      }
      if (e.key === 'Escape') {
        setShowSettings(false)
        setPreviewFullscreen(false)
      }
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [editorViewRef, setShowSettings, setPreviewFullscreen])

  // Ctrl/Cmd+S: .tdsl ファイルをダウンロード
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault()
        onSave()
      }
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [source])

  // Ctrl/Cmd+Shift+F: エディタ内容を整形
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (
        (e.ctrlKey || e.metaKey) &&
        e.shiftKey &&
        (e.key === 'F' || e.key === 'f')
      ) {
        e.preventDefault()
        onFormat()
      }
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wasmReady])
}
