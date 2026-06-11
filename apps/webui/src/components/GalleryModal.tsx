import { GALLERY_EXAMPLES } from '../gallery-meta'
import { useFocusTrap } from '../hooks/useFocusTrap'

type GalleryModalProps = {
  onClose: () => void
  onSelect: (source: string) => void
}

export function GalleryModal({ onClose, onSelect }: GalleryModalProps) {
  const dialogRef = useFocusTrap<HTMLDivElement>({ active: true, onEscape: onClose })
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
          <span id="gallery-modal-title">テンプレートギャラリー</span>
          <button className="modal-close" onClick={onClose} aria-label="ギャラリーを閉じる">✕</button>
        </div>
        <ul className="gallery-list">
          {GALLERY_EXAMPLES.map((ex) => (
            <li key={ex.filename}>
              <button
                className="gallery-item"
                onClick={() => onSelect(ex.source)}
              >
                <span className="gallery-item-label">{ex.label}</span>
                <span className="gallery-item-desc">{ex.description}</span>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}
