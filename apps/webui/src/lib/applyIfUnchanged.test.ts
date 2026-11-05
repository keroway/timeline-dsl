import type { EditorView } from '@codemirror/view'
import { describe, expect, it } from 'vitest'
import { applyIfUnchanged } from './applyIfUnchanged'

function fakeView(initial: string) {
  const state = { doc: initial }
  const view = {
    state: {
      doc: {
        toString: () => state.doc,
        get length() {
          return state.doc.length
        },
      },
    },
    dispatch: (spec: { changes: { insert: string } }) => {
      state.doc = spec.changes.insert
    },
  } as unknown as EditorView
  return { view, state }
}

describe('applyIfUnchanged', () => {
  it('文書が変わっていなければ置換する', () => {
    const { view, state } = fakeView('old')
    expect(applyIfUnchanged(view, () => view, 'old', 'formatted')).toBe(true)
    expect(state.doc).toBe('formatted')
  })

  it('処理中に編集されていたら古い結果を適用しない', () => {
    const { view, state } = fakeView('old')
    state.doc = 'new user edits'
    expect(applyIfUnchanged(view, () => view, 'old', 'formatted')).toBe(false)
    expect(state.doc).toBe('new user edits')
  })

  it('エディタが差し替わっていたら適用しない', () => {
    const { view, state } = fakeView('old')
    const other = fakeView('old').view
    expect(applyIfUnchanged(view, () => other, 'old', 'formatted')).toBe(false)
    expect(state.doc).toBe('old')
  })
})
