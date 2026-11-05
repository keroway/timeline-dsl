import { describe, expect, it } from 'vitest'
import { createLatestRequestGuard } from './latestRequest'

describe('createLatestRequestGuard', () => {
  it('完了順が逆転しても最新の要求だけが現行になる', () => {
    const g = createLatestRequestGuard()
    const a = g.begin()
    const b = g.begin()
    expect(g.isCurrent(b)).toBe(true) // B が先に完了
    expect(g.isCurrent(a)).toBe(false) // 遅れて来た A は破棄
  })

  it('invalidate で保留中の要求が無効になる', () => {
    const g = createLatestRequestGuard()
    const a = g.begin()
    g.invalidate()
    expect(g.isCurrent(a)).toBe(false)
  })

  it('invalidate 後の新しい要求は有効', () => {
    const g = createLatestRequestGuard()
    g.begin()
    g.invalidate()
    expect(g.isCurrent(g.begin())).toBe(true)
  })
})
