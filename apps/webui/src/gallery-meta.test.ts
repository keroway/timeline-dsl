import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { GALLERY_EXAMPLES } from './gallery-meta'

const examplesDir = new URL('../../../examples/', `file://${process.cwd()}/src/gallery-meta.test.ts`)

describe('GALLERY_EXAMPLES', () => {
  it('uses examples/*.tdsl as the source of truth', () => {
    for (const example of GALLERY_EXAMPLES) {
      const diskSource = readFileSync(new URL(example.filename, examplesDir), 'utf8')
      expect(example.source).toBe(diskSource)
    }
  })

  it('marks network-required templates as CLI-only references in descriptions', () => {
    const networkExamples = GALLERY_EXAMPLES.filter((example) => example.requiresNetwork)
    expect(networkExamples.map((example) => example.filename)).toEqual([
      'china_with_import.tdsl',
      'samurai_wikidata.tdsl',
      'china_dynasties_filtered.tdsl',
      'template_apply_example.tdsl',
      'officeholder_wikidata.tdsl',
    ])
    for (const example of networkExamples) {
      expect(example.description).toContain('CLI専用')
    }
  })
})
