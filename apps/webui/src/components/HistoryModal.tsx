import { useMemo, type Dispatch, type SetStateAction } from 'react'
import type { Snapshot } from '../history'
import { useFocusTrap } from '../hooks/useFocusTrap'
import { createTranslator, type Locale } from '../lib/i18n'

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
  locale: Locale
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
    locale,
  } = props

  const dialogRef = useFocusTrap<HTMLDivElement>({ active: true, onEscape: onClose })
  const t = useMemo(() => createTranslator(locale), [locale])

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal modal-history"
        onClick={(e) => e.stopPropagation()}
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="history-modal-title"
        tabIndex={-1}
      >
        <div className="modal-header">
          <span id="history-modal-title">{t('historyTitle')}</span>
          <button className="modal-close" onClick={onClose} aria-label={t('historyClose')}>✕</button>
        </div>
        <div className="history-body">
          {manualSnaps.length > 0 && (
            <section>
              <div className="history-section-title">{t('historyManualSection')}</div>
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
                        <button className="btn btn-sm" onClick={onRenameCommit}>{t('historyRenameCommit')}</button>
                        <button className="btn btn-sm" onClick={onRenameCancel}>{t('historyRenameCancel')}</button>
                      </div>
                    ) : (
                      <div className="history-item-row">
                        <button
                          className="history-restore-btn"
                          onClick={() => onRestore(snap.source)}
                          title={t('historyRestoreTitle')}
                        >
                          {snap.label}
                        </button>
                        <div className="history-item-actions">
                          <button className="btn btn-sm" onClick={() => onRenameStart(snap)} title={t('historyRenameTitle')}>✎</button>
                          <button className="btn btn-sm btn-danger" onClick={() => onDeleteManual(snap.id)} title={t('historyDeleteTitle')}>✕</button>
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
              <div className="history-section-title">{t.fmt('historyAutoSection', { count: autoSnaps.length, max: 5 })}</div>
              <ul className="history-list">
                {autoSnaps.map((snap) => (
                  <li key={snap.id} className="history-item">
                    <div className="history-item-row">
                      <button
                        className="history-restore-btn"
                        onClick={() => onRestore(snap.source)}
                        title={t('historyRestoreTitle')}
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
            <div className="history-empty">{t('historyEmpty')}</div>
          )}
          {(autoSnaps.length > 0 || manualSnaps.length > 0) && (
            <div className="history-footer">
              <button className="btn btn-danger" onClick={onClearAll}>
                {t('historyClearAll')}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
