import { GALLERY_EXAMPLES } from '../gallery-meta'

type GalleryModalProps = {
  onClose: () => void
  onSelect: (source: string) => void
}

export function GalleryModal({ onClose, onSelect }: GalleryModalProps) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-gallery" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span>テンプレートギャラリー</span>
          <button className="modal-close" onClick={onClose}>✕</button>
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
