import { describe, expect, test } from 'vite-plus/test'
import { categoryChips, isSuggested, SUGGESTED, suggestedFor } from '../app/utils/categories'
import { TEMPLATES } from '../app/utils/templates'

describe('pre-made categories', () => {
  test('ids and keys are unique and never empty', () => {
    expect(new Set(SUGGESTED.map((s) => s.id)).size).toBe(SUGGESTED.length)
    expect(new Set(SUGGESTED.map((s) => s.key)).size).toBe(SUGGESTED.length)
  })
  test.each(TEMPLATES.map((t) => [t.key] as const))('template %s has a category', (key) => {
    expect(suggestedFor(key)).not.toBeNull()
  })
  test('yours first; a suggestion you have (by id or name) is not offered again', () => {
    const chips = categoryChips([
      { id: SUGGESTED[0]!.id, name: 'TV', color: '', sort: 0 },
      { id: 'x', name: 'music', color: '', sort: 1 },
    ])
    expect(chips.slice(0, 2).map((c) => c.name)).toEqual(['TV', 'music'])
    expect(chips.filter(isSuggested).map((c) => c.key)).not.toContain('streaming')
    expect(chips.filter(isSuggested).map((c) => c.key)).not.toContain('music')
    expect(chips.length).toBe(2 + SUGGESTED.length - 2)
  })
})
