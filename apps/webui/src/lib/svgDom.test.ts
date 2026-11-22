import { expect, it } from 'vitest'
import { extractTags, readItemTags } from './svgDom'

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
