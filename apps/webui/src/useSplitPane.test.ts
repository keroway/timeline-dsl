import { describe, expect, it } from 'vitest'
import { splitRatioForKey } from './hooks/useSplitPane'

describe('splitRatioForKey', () => {
  it('ArrowRight increases the split ratio', () => {
    expect(splitRatioForKey(0.5, 'ArrowRight')).toBeCloseTo(0.52)
  })

  it('ArrowLeft decreases the split ratio', () => {
    expect(splitRatioForKey(0.5, 'ArrowLeft')).toBeCloseTo(0.48)
  })

  it('Home and End clamp to min and max', () => {
    expect(splitRatioForKey(0.5, 'Home')).toBe(0.15)
    expect(splitRatioForKey(0.5, 'End')).toBe(0.85)
  })

  it('ignores unrelated keys', () => {
    expect(splitRatioForKey(0.5, 'PageUp')).toBeNull()
  })
})
