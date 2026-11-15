import { EXAMPLES } from '../examples'
import { readSourceFromHash } from '../share'
import { checkSourceSize } from '../shareLimits'

export const EDITOR_SOURCE_KEY = 'tdsl:editor:source'
export function readSavedSource(): string {
  try {
    return localStorage.getItem(EDITOR_SOURCE_KEY) ?? EXAMPLES[0].source
  } catch {
    return EXAMPLES[0].source
  }
}
// Failure must be resolved explicitly before any editor persistence hooks mount.
export async function readInitialSource(signal?: AbortSignal): Promise<string> {
  const shared = await readSourceFromHash(signal)
  if (shared !== null) return shared
  const param = new URLSearchParams(location.search).get('source')
  if (param !== null) {
    checkSourceSize(param)
    return param
  }
  return readSavedSource()
}
