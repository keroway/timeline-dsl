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
        <p className="gallery-note">
          ネットワーク必須テンプレートは CLI 専用・構文リファレンスです。WebUI では読み込めますが、import wikidata はオフライン診断エラーになります。
        </p>
        <ul className="gallery-list">
          {GALLERY_EXAMPLES.map((ex) => (
            <li key={ex.filename}>
              <button
                className={`gallery-item${ex.requiresNetwork ? ' gallery-item-network' : ''}`}
                onClick={() => onSelect(ex.source)}
                aria-describedby={ex.requiresNetwork ? `${ex.filename}-network-note` : undefined}
              >
                <span className="gallery-item-header">
                  <span className="gallery-item-label">{ex.label}</span>
                  {ex.requiresNetwork ? <span className="gallery-badge">CLI専用</span> : null}
                </span>
                <span className="gallery-item-desc">{ex.description}</span>
                {ex.requiresNetwork ? (
                  <span className="gallery-item-network-note" id={`${ex.filename}-network-note`}>
                    Wikidata API が必要なため、WebUI ではプレビュー実行せず CLI で利用してください。
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
