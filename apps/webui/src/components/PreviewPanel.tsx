import { useMemo, type ChangeEvent, type Dispatch, type KeyboardEvent, type MouseEvent, type RefObject, type SetStateAction } from 'react'
import type { FilterState, LegendItem, SelectedItem } from '../lib/svgDom'
import { createTranslator, type Locale } from '../lib/i18n'

type PreviewPanelProps = {
  hidden: boolean
  scale: number
  onScaleChange: (value: number) => void
  svgContent: string
  isStalePreview: boolean
  wasmReady: boolean
  previewRef: RefObject<HTMLDivElement | null>
  svgContainerRef: RefObject<HTMLDivElement | null>
  cursorGrab: boolean
  resetPanZoom: () => void
  previewFullscreen: boolean
  setPreviewFullscreen: Dispatch<SetStateAction<boolean>>
  showLegend: boolean
  setShowLegend: Dispatch<SetStateAction<boolean>>
  showFilterPanel: boolean
  setShowFilterPanel: Dispatch<SetStateAction<boolean>>
  legendItems: LegendItem[]
  allTags: string[]
  filterState: FilterState
  setFilterState: Dispatch<SetStateAction<FilterState>>
  selectedItem: SelectedItem | null
  setSelectedItem: Dispatch<SetStateAction<SelectedItem | null>>
  onMouseDown: (e: MouseEvent<HTMLDivElement>) => void
  onMouseMove: (e: MouseEvent<HTMLDivElement>) => void
  onMouseUp: () => void
  onMouseLeave: () => void
  onDoubleClick: () => void
  onClick: (e: MouseEvent<HTMLDivElement>) => void
  onKeyDown: (e: KeyboardEvent<HTMLDivElement>) => void
  locale: Locale
}

export function PreviewPanel(props: PreviewPanelProps) {
  const {
    hidden,
    scale,
    onScaleChange,
    svgContent,
    isStalePreview,
    wasmReady,
    previewRef,
    svgContainerRef,
    cursorGrab,
    resetPanZoom,
    previewFullscreen,
    setPreviewFullscreen,
    showLegend,
    setShowLegend,
    showFilterPanel,
    setShowFilterPanel,
    legendItems,
    allTags,
    filterState,
    setFilterState,
    selectedItem,
    setSelectedItem,
    onMouseDown,
    onMouseMove,
    onMouseUp,
    onMouseLeave,
    onDoubleClick,
    onClick,
    onKeyDown,
    locale,
  } = props

  const t = useMemo(() => createTranslator(locale), [locale])

  return (
    <div className={`preview-area${hidden ? ' mobile-hidden' : ''}`}>
      {/* Preview controls overlay */}
      <div className="preview-controls">
        <select
          className="scale-select"
          value={scale}
          onChange={(e) => onScaleChange(Number(e.target.value))}
          title={t('previewScaleTitle')}
        >
          <option value={0}>Auto</option>
          <option value={0.5}>0.5×</option>
          <option value={1}>1×</option>
          <option value={2}>2×</option>
          <option value={4}>4×</option>
          <option value={8}>8×</option>
        </select>
        {svgContent && (
          <>
            <button
              className="btn btn-preview-ctrl"
              onClick={resetPanZoom}
              title={t('previewResetTitle')}
            >
              {t('previewReset')}
            </button>
            <button
              className="btn btn-preview-ctrl"
              onClick={() => setShowLegend((v) => !v)}
              title={t('previewLegendTitle')}
            >
              {showLegend ? t('previewLegendClose') : t('previewLegend')}
            </button>
            <button
              className={`btn btn-preview-ctrl${filterState.hiddenLanes.size > 0 || filterState.tagSearch ? ' btn-preview-ctrl-active' : ''}`}
              onClick={() => setShowFilterPanel((v) => !v)}
              title={t('previewFilterTitle')}
            >
              {showFilterPanel ? t('previewFilterClose') : t('previewFilter')}
            </button>
          </>
        )}
        <button
          className={`btn btn-preview-ctrl${previewFullscreen ? ' btn-preview-ctrl-active' : ''}`}
          onClick={() => setPreviewFullscreen((v) => !v)}
          title={previewFullscreen ? t('previewFullscreenExitTitle') : t('previewFullscreenTitle')}
          aria-label={previewFullscreen ? t('previewFullscreenExitTitle') : t('previewFullscreenTitle')}
        >
          {previewFullscreen ? t('previewFullscreenExit') : t('previewFullscreen')}
        </button>
      </div>
      {/* Legend panel */}
      {showLegend && legendItems.length > 0 && (
        <div className="legend-panel" aria-label={t('previewLegend')}>
          <div className="legend-header">{t('previewLegend')}</div>
          {legendItems.map((item) => (
            <div key={item.lane} className="legend-item">
              <span className="legend-swatch" style={{ background: item.color }} />
              <span className="legend-label">{item.label}</span>
            </div>
          ))}
        </div>
      )}
      {/* Filter panel */}
      {showFilterPanel && legendItems.length > 0 && (
        <div className="filter-panel" aria-label={t('previewFilter')}>
          <div className="filter-header">{t('previewFilter')}</div>
          <div className="filter-section">
            <div className="filter-section-title">{t('previewFilterLaneSection')}</div>
            {legendItems.map((item) => (
              <label key={item.lane} className="filter-item">
                <input
                  type="checkbox"
                  checked={!filterState.hiddenLanes.has(item.lane)}
                  onChange={(e: ChangeEvent<HTMLInputElement>) => {
                    setFilterState((prev) => {
                      const next = new Set(prev.hiddenLanes)
                      if (e.target.checked) next.delete(item.lane)
                      else next.add(item.lane)
                      return { ...prev, hiddenLanes: next }
                    })
                  }}
                />
                <span className="filter-swatch" style={{ background: item.color }} />
                <span className="filter-label">{item.label}</span>
              </label>
            ))}
          </div>
          {allTags.length > 0 && (
            <div className="filter-section">
              <div className="filter-section-title">{t('previewFilterTagSection')}</div>
              <input
                type="text"
                className="filter-tag-input"
                value={filterState.tagSearch}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  setFilterState((prev) => ({ ...prev, tagSearch: e.target.value }))
                }
                placeholder={t('previewFilterTagPlaceholder')}
              />
            </div>
          )}
          {(filterState.hiddenLanes.size > 0 || filterState.tagSearch) && (
            <button
              className="filter-reset-btn"
              onClick={() => setFilterState({ hiddenLanes: new Set(), tagSearch: '' })}
            >
              {t('previewReset')}
            </button>
          )}
        </div>
      )}
      {/* Selected item detail panel */}
      {selectedItem && (
        <div className="detail-panel" aria-label="選択中アイテムの詳細">
          <div className="detail-header">
            <span>詳細</span>
            <button className="detail-close" onClick={() => setSelectedItem(null)} aria-label="詳細を閉じる">✕</button>
          </div>
          <dl className="detail-list">
            <dt>名前</dt><dd>{selectedItem.label || '—'}</dd>
            <dt>種類</dt><dd>{selectedItem.type || '—'}</dd>
            <dt>レーン</dt><dd>{selectedItem.lane || '—'}</dd>
            {selectedItem.source && <><dt>出典</dt><dd>{selectedItem.source}</dd></>}
            {selectedItem.tooltip && (
              <><dt>情報</dt><dd className="detail-tooltip">{selectedItem.tooltip}</dd></>
            )}
          </dl>
        </div>
      )}
      <div
        ref={previewRef}
        className={`preview-pane${cursorGrab ? ' grabbing' : ''}`}
        onMouseDown={onMouseDown}
        onMouseMove={onMouseMove}
        onMouseUp={onMouseUp}
        onMouseLeave={onMouseLeave}
        onDoubleClick={onDoubleClick}
        onClick={onClick}
        onKeyDown={onKeyDown}
        aria-label="年表プレビュー"
      >
        {svgContent ? (
          <>
            {isStalePreview && (
              <div className="stale-preview-badge">直前の成功時プレビューを表示中</div>
            )}
            <div
              ref={svgContainerRef}
              className="svg-container"
              dangerouslySetInnerHTML={{ __html: svgContent }}
            />
          </>
        ) : (
          <div className="preview-placeholder">
            {wasmReady ? 'プレビューなし（エラーを確認してください）' : '読み込み中...'}
          </div>
        )}
      </div>
    </div>
  )
}
