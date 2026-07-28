import { EXAMPLES } from '../examples'
import { readSourceFromHash } from '../share'

export const EDITOR_SOURCE_KEY = 'tdsl:editor:source'

export type InitialSourceResult = { source: string; hashError: string | null }

// 初期ソースの決定順: 共有 URL ハッシュ → ?source クエリ → localStorage → 既定例。
export function readInitialSource(): InitialSourceResult {
  try {
    const fromHash = readSourceFromHash()
    if (fromHash !== null) return { source: fromHash, hashError: null }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    return {
      source: EXAMPLES[0].source,
      hashError: `共有 URL の展開に失敗しました: ${msg}`,
    }
  }
  const param = new URLSearchParams(location.search).get('source')
  if (param && param.length > 0) return { source: param, hashError: null }
  try {
    const saved = localStorage.getItem(EDITOR_SOURCE_KEY)
    if (saved) return { source: saved, hashError: null }
  } catch {
    /* private browsing or quota */
  }
  return { source: EXAMPLES[0].source, hashError: null }
}
