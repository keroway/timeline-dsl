// レンダリング済み SVG の DOM から凡例・タグを抽出し、フィルタ状態を永続化する
// 純粋ヘルパー群。SVG コンテナ要素を受け取り、React に依存しない。

export const FILTER_STATE_KEY = 'tdsl:filter-state'

export type LegendItem = { lane: string; label: string; color: string }

export type FilterState = { hiddenLanes: Set<string>; tagSearch: string }

export type SelectedItem = {
  label: string
  type: string
  lane: string
  source: string
  tooltip: string
}

export function extractLegend(container: Element): LegendItem[] {
  const colorMap = new Map<string, string>()
  container.querySelectorAll<Element>('[data-lane]').forEach((el) => {
    const lane = el.getAttribute('data-lane') || ''
    if (!colorMap.has(lane)) {
      const fillEl = el.querySelector('.tdsl-span, .tdsl-event-range, .tdsl-event-dot')
      const style = fillEl?.getAttribute('style') || ''
      const m = style.match(/fill:([^;]+)/)
      if (m) colorMap.set(lane, m[1].trim())
    }
  })
  const result: LegendItem[] = []
  container.querySelectorAll<Element>('.tdsl-lane-label[data-lane]').forEach((el) => {
    const lane = el.getAttribute('data-lane') || ''
    const label = el.textContent || lane
    result.push({ lane, label, color: colorMap.get(lane) || '#888' })
  })
  return result
}

export function extractTags(container: Element): string[] {
  const tags = new Set<string>()
  container.querySelectorAll<Element>('[data-tags]').forEach((el) => {
    const raw = el.getAttribute('data-tags') || ''
    raw.split(',').forEach((t) => { if (t.trim()) tags.add(t.trim()) })
  })
  return [...tags].sort()
}

export function loadFilterState(): FilterState {
  try {
    const saved = sessionStorage.getItem(FILTER_STATE_KEY)
    if (saved) {
      const parsed = JSON.parse(saved) as { hiddenLanes?: string[]; tagSearch?: string }
      return {
        hiddenLanes: new Set(parsed.hiddenLanes ?? []),
        tagSearch: parsed.tagSearch ?? '',
      }
    }
  } catch { /* ignore */ }
  return { hiddenLanes: new Set(), tagSearch: '' }
}
