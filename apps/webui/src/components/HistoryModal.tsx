import type { Dispatch, SetStateAction } from 'react'
import type { Snapshot } from '../history'

type HistoryModalProps = {
  onClose: () => void
  manualSnaps: Snapshot[]
  autoSnaps: Snapshot[]
  renamingId: string | null
  renameValue: string
  setRenameValue: Dispatch<SetStateAction<string>>
  onRestore: (source: string) => void
  onRenameStart: (snap: Snapshot) => void
  onRenameCommit: () => void
  onRenameCancel: () => void
  onDeleteManual: (id: string) => void
  onClearAll: () => void
}

export function HistoryModal(props: HistoryModalProps) {
  const {
    onClose,
    manualSnaps,
    autoSnaps,
    renamingId,
    renameValue,
    setRenameValue,
    onRestore,
    onRenameStart,
    onRenameCommit,
    onRenameCancel,
    onDeleteManual,
    onClearAll,
  } = props

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-history" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span>履歴</span>
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="history-body">
          {manualSnaps.length > 0 && (
            <section>
              <div className="history-section-title">手動保存</div>
              <ul className="history-list">
                {manualSnaps.map((snap) => (
                  <li key={snap.id} className="history-item">
                    {renamingId === snap.id ? (
                      <div className="history-rename-row">
                        <input
                          className="history-rename-input"
                          value={renameValue}
                          autoFocus
                          onChange={(e) => setRenameValue(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') onRenameCommit()
                            if (e.key === 'Escape') onRenameCancel()
                          }}
                        />
                        <button className="btn btn-sm" onClick={onRenameCommit}>確定</button>
                        <button className="btn btn-sm" onClick={onRenameCancel}>キャンセル</button>
                      </div>
                    ) : (
                      <div className="history-item-row">
                        <button
                          className="history-restore-btn"
                          onClick={() => onRestore(snap.source)}
                          title="このスナップショットを復元"
                        >
                          {snap.label}
                        </button>
                        <div className="history-item-actions">
                          <button className="btn btn-sm" onClick={() => onRenameStart(snap)} title="名前を変更">✎</button>
                          <button className="btn btn-sm btn-danger" onClick={() => onDeleteManual(snap.id)} title="削除">✕</button>
                        </div>
                      </div>
                    )}
                  </li>
                ))}
              </ul>
            </section>
          )}
          {autoSnaps.length > 0 && (
            <section>
              <div className="history-section-title">自動スナップショット（最大 {autoSnaps.length}/5 件）</div>
              <ul className="history-list">
                {autoSnaps.map((snap) => (
                  <li key={snap.id} className="history-item">
                    <div className="history-item-row">
                      <button
                        className="history-restore-btn"
                        onClick={() => onRestore(snap.source)}
                        title="このスナップショットを復元"
                      >
                        {snap.label}
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            </section>
          )}
          {autoSnaps.length === 0 && manualSnaps.length === 0 && (
            <div className="history-empty">履歴はありません</div>
          )}
          {(autoSnaps.length > 0 || manualSnaps.length > 0) && (
            <div className="history-footer">
              <button className="btn btn-danger" onClick={onClearAll}>
                全件削除
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
