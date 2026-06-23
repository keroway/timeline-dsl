import { GALLERY_EXAMPLES } from './gallery-meta'

export interface Example {
  label: string
  source: string
}

export const EXAMPLES: Example[] = GALLERY_EXAMPLES
  .filter((example) => !example.requiresNetwork)
  .map((example) => ({
    label: example.label,
    source: example.source,
  }))
