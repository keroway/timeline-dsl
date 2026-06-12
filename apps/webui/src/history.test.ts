import { describe, it, expect, beforeEach, vi } from 'vitest'
import {
  pushAutoSnapshot,
  readAutoSnapshots,
  pushManualSnapshot,
  readManualSnapshots,
  renameManualSnapshot,
  deleteManualSnapshot,
  clearAllHistory,
  shouldAutoSnapshot,
} from './history.ts'

beforeEach(() => {
  localStorage.clear()
})

describe('pushAutoSnapshot / readAutoSnapshots', () => {
  it('stores a snapshot and reads it back', () => {
    pushAutoSnapshot('timeline "t" {}', 'initial')
    const snaps = readAutoSnapshots()
    expect(snaps).toHaveLength(1)
    expect(snaps[0].source).toBe('timeline "t" {}')
    expect(snaps[0].kind).toBe('auto')
  })

  it('trims to AUTO_MAX (5) entries when more are pushed', () => {
    for (let i = 0; i < 7; i++) {
      pushAutoSnapshot(`source-${i}`, `reason-${i}`)
    }
    const snaps = readAutoSnapshots()
    expect(snaps).toHaveLength(5)
    // most recent entry is first
    expect(snaps[0].source).toBe('source-6')
  })

  it('prepends new snapshots so the most recent is first', () => {
    pushAutoSnapshot('first', 'r1')
    pushAutoSnapshot('second', 'r2')
    const snaps = readAutoSnapshots()
    expect(snaps[0].source).toBe('second')
    expect(snaps[1].source).toBe('first')
  })
})

describe('shouldAutoSnapshot', () => {
  it('returns true when no snapshots exist yet', () => {
    // lastAutoMs far in the past to skip interval check
    expect(shouldAutoSnapshot('some source', 0)).toBe(true)
  })

  it('returns false when the interval has not elapsed', () => {
    // lastAutoMs is essentially now
    expect(shouldAutoSnapshot('some source', Date.now())).toBe(false)
  })

  it('returns false when the latest snapshot has the same source', () => {
    const src = 'timeline "same" {}'
    pushAutoSnapshot(src, 'initial')
    // interval elapsed (lastAutoMs=0), but source unchanged
    expect(shouldAutoSnapshot(src, 0)).toBe(false)
  })

  it('returns true when interval elapsed and source changed', () => {
    pushAutoSnapshot('old source', 'initial')
    expect(shouldAutoSnapshot('new source', 0)).toBe(true)
  })
})

describe('pushManualSnapshot / readManualSnapshots', () => {
  it('stores a manual snapshot with the provided label', () => {
    const snap = pushManualSnapshot('src', 'My Save')
    expect(snap.kind).toBe('manual')
    expect(snap.label).toBe('My Save')
    const snaps = readManualSnapshots()
    expect(snaps).toHaveLength(1)
    expect(snaps[0].id).toBe(snap.id)
  })

  it('uses a default label when an empty string is provided', () => {
    const snap = pushManualSnapshot('src', '')
    expect(snap.label).toMatch(/手動保存/)
  })

  it('prepends new manual snapshots so the most recent is first', () => {
    pushManualSnapshot('a', 'first')
    pushManualSnapshot('b', 'second')
    const snaps = readManualSnapshots()
    expect(snaps[0].label).toBe('second')
    expect(snaps[1].label).toBe('first')
  })
})

describe('renameManualSnapshot', () => {
  it('renames the snapshot with the matching id', () => {
    const snap = pushManualSnapshot('src', 'original')
    renameManualSnapshot(snap.id, 'renamed')
    const snaps = readManualSnapshots()
    expect(snaps[0].label).toBe('renamed')
  })

  it('leaves other snapshots unchanged', () => {
    const s1 = pushManualSnapshot('a', 'first')
    pushManualSnapshot('b', 'second')
    renameManualSnapshot(s1.id, 'updated-first')
    const snaps = readManualSnapshots()
    const found = snaps.find((s) => s.id === s1.id)!
    expect(found.label).toBe('updated-first')
  })
})

describe('deleteManualSnapshot', () => {
  it('removes the snapshot with the matching id', () => {
    const snap = pushManualSnapshot('src', 'to-delete')
    deleteManualSnapshot(snap.id)
    expect(readManualSnapshots()).toHaveLength(0)
  })

  it('does not affect other snapshots', () => {
    const s1 = pushManualSnapshot('a', 'keep')
    const s2 = pushManualSnapshot('b', 'delete')
    deleteManualSnapshot(s2.id)
    const snaps = readManualSnapshots()
    expect(snaps).toHaveLength(1)
    expect(snaps[0].id).toBe(s1.id)
  })
})

describe('clearAllHistory', () => {
  it('removes both auto and manual snapshots', () => {
    pushAutoSnapshot('auto-src', 'auto')
    pushManualSnapshot('manual-src', 'manual')
    clearAllHistory()
    expect(readAutoSnapshots()).toHaveLength(0)
    expect(readManualSnapshots()).toHaveLength(0)
  })
})

describe('localStorage failure resilience', () => {
  it('readAutoSnapshots returns [] when localStorage.getItem throws', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('storage unavailable')
    })
    expect(readAutoSnapshots()).toEqual([])
    vi.restoreAllMocks()
  })

  it('pushAutoSnapshot does not throw when localStorage.setItem throws', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota exceeded')
    })
    expect(() => pushAutoSnapshot('src', 'reason')).not.toThrow()
    vi.restoreAllMocks()
  })
})
