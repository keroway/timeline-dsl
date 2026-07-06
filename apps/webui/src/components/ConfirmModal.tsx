import { useRef } from 'react'
import { useFocusTrap } from '../hooks/useFocusTrap'
import type { ConfirmState } from '../hooks/useConfirm'

type ConfirmModalProps = {
  state: ConfirmState
}

// window.confirm の代替となる、フォーカストラップ・Esc キャンセル対応のアプリ内モーダル。
// Enter は確定ボタンにフォーカスがある場合のみ確定として扱う（全体では自動確定しない）。
export function ConfirmModal({ state }: ConfirmModalProps) {
  const { title, body, confirmLabel, cancelLabel, tone = 'default', resolve } = state
  const confirmButtonRef = useRef<HTMLButtonElement>(null)

  function handleCancel() {
    resolve(false)
  }

  function handleConfirm() {
    resolve(true)
  }

  const dialogRef = useFocusTrap<HTMLDivElement>({ active: true, onEscape: handleCancel })

  return (
    <div className="modal-overlay" onClick={handleCancel}>
      <div
        className="modal modal-confirm"
        onClick={(e) => e.stopPropagation()}
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-modal-title"
        aria-describedby="confirm-modal-body"
        tabIndex={-1}
      >
        <div className="modal-header">
          <span id="confirm-modal-title">{title}</span>
        </div>
        <div className="confirm-body" id="confirm-modal-body">
          {body}
        </div>
        <div className="confirm-actions">
          <button type="button" className="btn" onClick={handleCancel}>
            {cancelLabel}
          </button>
          <button
            type="button"
            ref={confirmButtonRef}
            className={`btn${tone === 'warn' ? ' btn-danger' : ' btn-active'}`}
            onClick={handleConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  )
}
