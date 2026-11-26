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
      const fillEl = el.querySelector(
        '.tdsl-span, .tdsl-event-range, .tdsl-event-dot'
      )
      const style = fillEl?.getAttribute('style') || ''
      const m = style.match(/fill:([^;]+)/)
      if (m) colorMap.set(lane, m[1].trim())
    }
  })
  const result: LegendItem[] = []
  container
    .querySelectorAll<Element>('.tdsl-lane-label[data-lane]')
    .forEach((el) => {
      const lane = el.getAttribute('data-lane') || ''
      const label = el.textContent || lane
      result.push({ lane, label, color: colorMap.get(lane) || '#888' })
    })
  return result
}

/**
 * Read an item's tags from the rendered SVG.
 *
 * `data-tags-json` (a JSON string array, #1035) is authoritative: it keeps a
 * tag containing a comma as one tag. `data-tags` is the older comma-joined
 * form and cannot represent such tags; it is read only when the renderer
 * predates `data-tags-json` (e.g. `@keroway/tdsl-wasm` <= 3.0.0).
 */
export function readItemTags(el: Element): string[] {
  const json = el.getAttribute('data-tags-json')
  if (json !== null) {
    try {
      const parsed: unknown = JSON.parse(json)
      if (Array.isArray(parsed)) {
        return parsed.filter((t): t is string => typeof t === 'string')
      }
    } catch (err) {
      console.error('tdsl: invalid data-tags-json', err)
    }
    return []
  }
  const raw = el.getAttribute('data-tags') || ''
  return raw
    .split(',')
    .map((t) => t.trim())
    .filter(Boolean)
}

export function extractTags(container: Element): string[] {
  const tags = new Set<string>()
  container.querySelectorAll<Element>('[data-tags]').forEach((el) => {
    for (const tag of readItemTags(el)) tags.add(tag)
  })
  return [...tags].sort()
}

export function loadFilterState(): FilterState {
  try {
    const saved = sessionStorage.getItem(FILTER_STATE_KEY)
    if (saved) {
      const parsed: unknown = JSON.parse(saved)
      if (typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed)) {
        const { hiddenLanes, tagSearch } = parsed as Record<string, unknown>
        return {
          hiddenLanes: new Set(
            Array.isArray(hiddenLanes)
              ? hiddenLanes.filter((l): l is string => typeof l === 'string')
              : [],
          ),
          tagSearch: typeof tagSearch === 'string' ? tagSearch : '',
        }
      }
    }
  } catch {
    /* ignore */
  }
  return { hiddenLanes: new Set(), tagSearch: '' }
}
