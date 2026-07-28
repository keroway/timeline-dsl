import { useMemo } from 'react'
import { GALLERY_EXAMPLES } from '../gallery-meta'
import { useFocusTrap } from '../hooks/useFocusTrap'
import { createTranslator, type Locale } from '../lib/i18n'

type GalleryModalProps = {
  onClose: () => void
  onSelect: (source: string) => void
  locale: Locale
}

export function GalleryModal({ onClose, onSelect, locale }: GalleryModalProps) {
  const dialogRef = useFocusTrap<HTMLDivElement>({
    active: true,
    onEscape: onClose,
  })
  const t = useMemo(() => createTranslator(locale), [locale])
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal modal-gallery"
        onClick={(e) => e.stopPropagation()}
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="gallery-modal-title"
        tabIndex={-1}
      >
        <div className="modal-header">
          <span id="gallery-modal-title">{t('galleryTitle')}</span>
          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t('galleryClose')}
          >
            ✕
          </button>
        </div>
        <p className="gallery-note">{t('galleryNetworkNote')}</p>
        <ul className="gallery-list">
          {GALLERY_EXAMPLES.map((ex) => (
            <li key={ex.filename}>
              <button
                type="button"
                className={`gallery-item${ex.requiresNetwork ? ' gallery-item-network' : ''}`}
                onClick={() => onSelect(ex.source)}
                aria-describedby={
                  ex.requiresNetwork ? `${ex.filename}-network-note` : undefined
                }
              >
                <span className="gallery-item-header">
                  <span className="gallery-item-label">{ex.label}</span>
                  {ex.requiresNetwork ? (
                    <span className="gallery-badge">{t('galleryCliOnly')}</span>
                  ) : null}
                </span>
                <span className="gallery-item-desc">{ex.description}</span>
                {ex.requiresNetwork ? (
                  <span
                    className="gallery-item-network-note"
                    id={`${ex.filename}-network-note`}
                  >
                    {t('galleryCliNoteItem')}
                  </span>
                ) : null}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}
