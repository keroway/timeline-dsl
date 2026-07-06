import { describe, it, expect, beforeEach, vi } from 'vitest'
import {
  readAutoSnapshots,
  readManualSnapshots,
  pushAutoSnapshot,
  pushManualSnapshot,
  shouldAutoSnapshot,
  renameManualSnapshot,
  deleteManualSnapshot,
  clearAllHistory,
} from './history'

beforeEach(() => {
  localStorage.clear()
})

describe('pushAutoSnapshot', () => {
  it('スナップショットを保存できる', () => {
    pushAutoSnapshot('src1', 'load')
    const snaps = readAutoSnapshots()
    expect(snaps).toHaveLength(1)
    expect(snaps[0].source).toBe('src1')
    expect(snaps[0].kind).toBe('auto')
  })

  it('新しいスナップショットが先頭に挿入される', () => {
    pushAutoSnapshot('src1', 'load')
    pushAutoSnapshot('src2', 'load')
    const snaps = readAutoSnapshots()
    expect(snaps[0].source).toBe('src2')
    expect(snaps[1].source).toBe('src1')
  })

  it('上限 5 件を超えると古いものが切り詰められる', () => {
    for (let i = 0; i < 7; i++) {
      pushAutoSnapshot(`src${i}`, 'load')
    }
    const snaps = readAutoSnapshots()
    expect(snaps).toHaveLength(5)
    expect(snaps[0].source).toBe('src6')
  })
})

describe('shouldAutoSnapshot', () => {
  it('前回から interval 未満なら false', () => {
    const now = Date.now()
    expect(shouldAutoSnapshot('src', now - 1000)).toBe(false)
  })

  it('スナップショットが空なら true', () => {
    const old = Date.now() - 10 * 60 * 1000
    expect(shouldAutoSnapshot('src', old)).toBe(true)
  })

  it('最新スナップショットと同じソースなら false', () => {
    const old = Date.now() - 10 * 60 * 1000
    pushAutoSnapshot('src', 'load')
    expect(shouldAutoSnapshot('src', old)).toBe(false)
  })

  it('最新スナップショットと異なるソースなら true', () => {
    const old = Date.now() - 10 * 60 * 1000
    pushAutoSnapshot('src-old', 'load')
    expect(shouldAutoSnapshot('src-new', old)).toBe(true)
  })
})

describe('pushManualSnapshot', () => {
  it('スナップショットを保存して返す', () => {
    const snap = pushManualSnapshot('src', 'ラベル', '手動保存')
    expect(snap.source).toBe('src')
    expect(snap.label).toBe('ラベル')
    expect(snap.kind).toBe('manual')
    expect(readManualSnapshots()).toHaveLength(1)
  })

  it('空ラベルはデフォルトラベルになる', () => {
    const snap = pushManualSnapshot('src', '', '手動保存')
    expect(snap.label).toContain('手動保存')
  })
})

describe('renameManualSnapshot', () => {
  it('指定 ID のラベルを変更できる', () => {
    const snap = pushManualSnapshot('src', 'old', '手動保存')
    renameManualSnapshot(snap.id, 'new')
    const snaps = readManualSnapshots()
    expect(snaps[0].label).toBe('new')
  })
})

describe('deleteManualSnapshot', () => {
  it('指定 ID のスナップショットを削除できる', () => {
    const s1 = pushManualSnapshot('src1', 'a', '手動保存')
    const s2 = pushManualSnapshot('src2', 'b', '手動保存')
    deleteManualSnapshot(s1.id)
    const snaps = readManualSnapshots()
    expect(snaps).toHaveLength(1)
    expect(snaps[0].id).toBe(s2.id)
  })
})

describe('clearAllHistory', () => {
  it('auto / manual 両方をクリアする', () => {
    pushAutoSnapshot('src', 'load')
    pushManualSnapshot('src', 'a', '手動保存')
    clearAllHistory()
    expect(readAutoSnapshots()).toHaveLength(0)
    expect(readManualSnapshots()).toHaveLength(0)
  })
})

describe('localStorage 障害耐性', () => {
  it('getItem が throw しても空配列を返す', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('storage error')
    })
    expect(readAutoSnapshots()).toEqual([])
    vi.restoreAllMocks()
  })

  it('setItem が throw してもエラーを伝播しない', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota exceeded')
    })
    expect(() => pushAutoSnapshot('src', 'load')).not.toThrow()
    vi.restoreAllMocks()
  })
})
