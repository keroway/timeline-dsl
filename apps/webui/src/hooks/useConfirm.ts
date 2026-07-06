import { useCallback, useRef, useState } from 'react'

export type ConfirmTone = 'default' | 'warn'

export type ConfirmOptions = {
  title: string
  body: string
  confirmLabel: string
  cancelLabel: string
  tone?: ConfirmTone
}

export type ConfirmState = ConfirmOptions & {
  resolve: (value: boolean) => void
}

// Promise ベースの confirm ダイアログ。呼び出し側は `await confirm({ ... })` で
// ユーザーの選択（true=確定 / false=キャンセル）を待てる。実際のモーダル描画は
// `confirmState` を `<ConfirmModal>` に渡すアプリ側（App.tsx 付近）が担当する。
export function useConfirm() {
  const [confirmState, setConfirmState] = useState<ConfirmState | null>(null)
  // 連続呼び出しで前の Promise が孤立しないよう、保留中の resolver を保持する。
  const pendingResolve = useRef<((value: boolean) => void) | null>(null)

  const confirm = useCallback((options: ConfirmOptions): Promise<boolean> => {
    // 前回の確認がまだ解決されていない場合は false で解決してから次を開く。
    if (pendingResolve.current) {
      pendingResolve.current(false)
      pendingResolve.current = null
    }
    return new Promise<boolean>((resolve) => {
      pendingResolve.current = resolve
      setConfirmState({
        ...options,
        resolve: (value: boolean) => {
          pendingResolve.current = null
          setConfirmState(null)
          resolve(value)
        },
      })
    })
  }, [])

  return { confirm, confirmState }
}
