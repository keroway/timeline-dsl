import { afterEach, expect, it, vi } from 'vitest'
import { extractTags, loadFilterState, readItemTags } from './svgDom'

function stubSaved(value: string | null): void {
  vi.stubGlobal('sessionStorage', { getItem: () => value })
}

afterEach(() => {
  vi.unstubAllGlobals()
})

it('loadFilterState restores valid saved state (#1055)', () => {
  stubSaved(JSON.stringify({ hiddenLanes: ['a', 'b'], tagSearch: 'x' }))
  const s = loadFilterState()
  expect([...s.hiddenLanes]).toEqual(['a', 'b'])
  expect(s.tagSearch).toBe('x')
})

it('loadFilterState resets invalid field types (#1055)', () => {
  stubSaved(JSON.stringify({ hiddenLanes: 'lane_a', tagSearch: 123 }))
  const s = loadFilterState()
  expect(s.hiddenLanes.size).toBe(0)
  expect(s.tagSearch).toBe('')
})

it('loadFilterState drops non-string lane entries (#1055)', () => {
  stubSaved(JSON.stringify({ hiddenLanes: ['a', 1, null], tagSearch: '' }))
  expect([...loadFilterState().hiddenLanes]).toEqual(['a'])
})

it('loadFilterState tolerates null, non-object and broken JSON (#1055)', () => {
  for (const raw of ['null', '123', '[]', '{', null]) {
    stubSaved(raw)
    const s = loadFilterState()
    expect(s.hiddenLanes.size).toBe(0)
    expect(s.tagSearch).toBe('')
  }
})

function item(attrs: Record<string, string>): Element {
  const el = document.createElement('g')
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v)
  return el
}

it('keeps a comma-containing tag as one tag when data-tags-json is present (#1035)', () => {
  const el = item({
    'data-tags': 'a,b,other',
    'data-tags-json': '["a,b","other"]',
  })
  expect(readItemTags(el)).toEqual(['a,b', 'other'])
})

it('falls back to comma-split data-tags for renderers without data-tags-json', () => {
  expect(readItemTags(item({ 'data-tags': 'war, dynasty' }))).toEqual([
    'war',
    'dynasty',
  ])
  expect(readItemTags(item({}))).toEqual([])
})

it('does not fall back to the lossy attribute when data-tags-json is malformed', () => {
  const el = item({ 'data-tags': 'a,b', 'data-tags-json': '[' })
  expect(readItemTags(el)).toEqual([])
})

it('collects distinct tags across items', () => {
  const container = document.createElement('div')
  container.append(
    item({ 'data-tags': 'a,b', 'data-tags-json': '["a,b"]' }),
    item({ 'data-tags': 'other,a,b', 'data-tags-json': '["other","a,b"]' })
  )
  expect(extractTags(container)).toEqual(['a,b', 'other'])
})
