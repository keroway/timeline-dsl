import { afterEach, expect, it } from 'vitest'
import { MAX_SOURCE_BYTES } from '../shareLimits'
import { EDITOR_SOURCE_KEY, readInitialSource } from './initialSource'

afterEach(() => {
  history.replaceState(null, '', '/')
  localStorage.clear()
})
it('accepts an explicitly empty query source instead of restoring saved data', async () => {
  localStorage.setItem(EDITOR_SOURCE_KEY, 'saved')
  history.replaceState(null, '', '/?source=')
  expect(await readInitialSource()).toBe('')
})
it('limits query sources by UTF-8 bytes without modifying saved content', async () => {
  localStorage.setItem(EDITOR_SOURCE_KEY, 'saved')
  history.replaceState(
    null,
    '',
    `/?source=${encodeURIComponent('あ'.repeat(Math.ceil(MAX_SOURCE_BYTES / 3)))}`
  )
  await expect(readInitialSource()).rejects.toThrow('tooLarge')
  expect(localStorage.getItem(EDITOR_SOURCE_KEY)).toBe('saved')
})
