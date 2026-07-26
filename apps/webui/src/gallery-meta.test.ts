import { describe, expect, it } from 'vitest'
import { readdirSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { GALLERY_EXAMPLES } from './gallery-meta'

const examplesDir = new URL('../../../examples/', `file://${process.cwd()}/src/gallery-meta.test.ts`)

describe('GALLERY_EXAMPLES', () => {
  it('uses examples/*.tdsl as the source of truth', () => {
    for (const example of GALLERY_EXAMPLES) {
      const diskSource = readFileSync(new URL(example.filename, examplesDir), 'utf8')
      expect(example.source).toBe(diskSource)
    }
  })

  // ドリフト防止: examples/*.tdsl を新規追加したのに GALLERY_EXAMPLES への登録を
  // 忘れると、上の「disk と一致するか」テストだけでは検出できない
  // （登録されていないファイルはそもそも比較対象にならないため）。
  // examples ディレクトリの全 .tdsl ファイルが登録されていることを保証する。
  it('registers every examples/*.tdsl file (no silently-dropped additions)', () => {
    const onDisk = readdirSync(fileURLToPath(examplesDir))
      .filter((name) => name.endsWith('.tdsl'))
      .sort()
    const registered = GALLERY_EXAMPLES.map((example) => example.filename).sort()
    expect(registered).toEqual(onDisk)
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
